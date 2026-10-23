use aes::Aes128;
use aidoku::{
	Result,
	alloc::{String, Vec, format, string::ToString as _},
	bail,
	imports::{
		canvas::{Canvas, ImageRef, Rect},
		html::Document,
	},
};
use base64::{
	Engine as _, alphabet,
	engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
};
use cbc::cipher::{BlockDecryptMut as _, KeyIvInit as _, block_padding::Pkcs7};
use regex::Regex;
use serde::Deserialize;

/// Images are split into chunks which are decrypted one by one and then joined
pub const MODE_LEGACY: &str = "bmi2";
/// Images are split into chunks which are joined and then decrypted
pub const MODE_V3: &str = "bmi3";

const KEY: &[u8; 16] = b"aaaaaaaaaaaaaaaa";
const IV: &[u8; 16] = b"0123456789aaaaaa";

const DEFAULT_HOSTS: [&str; 2] = [
	"https://bmigmij-wuwu.sqxxov.com/break_2",
	"https://bmigmih-wuwu.sqxxov.com/break_2",
];

const BASE64: GeneralPurpose = GeneralPurpose::new(
	&alphabet::URL_SAFE,
	GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

/// Image host information embedded in every page of the site.
pub struct ImageInfo {
	hosts: Vec<String>,
	cache_key: Option<String>,
}

impl ImageInfo {
	pub fn from(html: &Document) -> Self {
		let scripts = html
			.select("script:not([src])")
			.map(|list| {
				list.filter_map(|script| script.data())
					.collect::<Vec<_>>()
					.join("\n")
			})
			.unwrap_or_default();

		let hosts =
			Regex::new(r#"recover_img_url_with_multiple_hosts\(\s*[\w.]+\s*,\s*["']([^"']+)["']"#)
				.ok()
				.and_then(|re| re.captures(&scripts))
				.and_then(|caps| caps.get(1))
				.map(|m| {
					m.as_str()
						.split(',')
						.map(str::trim)
						.filter(|host| !host.is_empty())
						.map(String::from)
						.collect::<Vec<_>>()
				})
				.filter(|hosts| hosts.len() >= 2)
				.unwrap_or_else(|| DEFAULT_HOSTS.iter().map(|host| host.to_string()).collect());

		let head = html
			.select_first("head")
			.and_then(|head| head.html())
			.unwrap_or_default();
		let cache_key = Regex::new(r#"BMI_CACHE_KEY\s*=\s*["']([^"']+)["']"#)
			.ok()
			.and_then(|re| re.captures(&head))
			.and_then(|caps| caps.get(1))
			.map(|m| m.as_str().to_string());

		Self { hosts, cache_key }
	}

	/// Converts an original image url to the urls of its encrypted chunks.
	pub fn legacy_urls(&self, url: &str) -> Vec<String> {
		// strip scheme, host and query
		let path = url
			.find("://")
			.map(|pos| &url[pos + 3..])
			.and_then(|rest| rest.find('/').map(|slash| &rest[slash..]))
			.unwrap_or(url);
		let path = path.split(['?', '#']).next().unwrap_or_default();

		let path = Regex::new(r"/break[^/]*/")
			.map(|re| re.replace(path, "/").into_owned())
			.unwrap_or_else(|_| path.into());
		let path = Regex::new(r"(?i)\.(jpe?g|png|gif|avif|webp)$")
			.map(|re| re.replace(&path, "").into_owned())
			.unwrap_or(path);

		self.hosts
			.iter()
			.take(2)
			.enumerate()
			.map(|(i, host)| {
				let mut url = format!("{}{path}.b_{i}", host.trim_end_matches('/'));
				if let Some(key) = &self.cache_key {
					url.push_str("?v=");
					url.push_str(key);
				}
				url
			})
			.collect()
	}
}

#[derive(Deserialize)]
struct Manifest {
	variants: Variants,
}

#[derive(Deserialize)]
struct Variants {
	jpeg: Variant,
}

#[derive(Deserialize)]
struct Variant {
	renditions: Vec<Rendition>,
}

#[derive(Deserialize)]
struct Rendition {
	role: Option<String>,
	chunks: Vec<Chunk>,
}

#[derive(Deserialize)]
struct Chunk {
	index: i32,
	url: String,
}

/// Parses a base64 encoded `data-bmi-manifest` into its chunk urls.
pub fn manifest_urls(manifest: &str) -> Option<Vec<String>> {
	let decoded = BASE64.decode(manifest).ok()?;
	let manifest: Manifest = serde_json::from_slice(&decoded).ok()?;
	let mut chunks = manifest
		.variants
		.jpeg
		.renditions
		.into_iter()
		.find(|rendition| rendition.role.as_deref() == Some("page"))?
		.chunks;
	chunks.sort_by_key(|chunk| chunk.index);
	Some(chunks.into_iter().map(|chunk| chunk.url).collect())
}

fn decrypt(mut data: Vec<u8>) -> Result<Vec<u8>> {
	let len = cbc::Decryptor::<Aes128>::new(KEY.into(), IV.into())
		.decrypt_padded_mut::<Pkcs7>(&mut data)
		.map(|plain| plain.len());
	let Ok(len) = len else {
		bail!("Failed to decrypt image");
	};
	data.truncate(len);
	Ok(data)
}

/// Decrypts the chunks of an image and joins them.
pub fn decrypt_chunks(mode: &str, chunks: Vec<Vec<u8>>) -> Result<Vec<u8>> {
	if mode == MODE_LEGACY {
		let mut data = Vec::new();
		for chunk in chunks {
			data.extend(decrypt(chunk)?);
		}
		Ok(data)
	} else {
		decrypt(chunks.concat())
	}
}

fn magic_number(kind: u8) -> Option<&'static [u8]> {
	Some(match kind {
		0 => &[
			0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
		],
		1 => &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
		3 => &[0x47, 0x49, 0x46, 0x38, 0x39, 0x61],
		4 => &[
			0x00, 0x00, 0x00, 0x20, 0x66, 0x74, 0x79, 0x70, 0x61, 0x76, 0x69, 0x66,
		],
		_ => return None,
	})
}

fn slice_count(key: &str) -> u32 {
	let digest = md5::compute(key.as_bytes());
	let last_hex_char = b"0123456789abcdef"[(digest[15] & 15) as usize];
	(u32::from(last_hex_char) % 10) * 4 + 44
}

/// Restores the image header of decrypted data and returns the decoded image
/// together with the number of slices it was scrambled into.
pub fn restore_header(mut data: Vec<u8>) -> Result<(ImageRef, u32)> {
	if data.len() < 12 {
		bail!("Invalid image data");
	}
	let Some(magic) = magic_number(data[0]) else {
		bail!("Unknown image type: {}", data[0]);
	};
	let key = if data[1] == 0 {
		let book_id = u32::from(data[2]) * 256 + u32::from(data[3]);
		let page = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
		format!("{book_id}{page}")
	} else {
		"undefinedundefined".into()
	};
	data[..magic.len()].copy_from_slice(magic);
	Ok((ImageRef::new(&data), slice_count(&key)))
}

/// Reverses the order of horizontal slices of a scrambled image.
pub fn unscramble(image: ImageRef, rows: u32) -> ImageRef {
	let width = image.width();
	let height = image.height() as u32;
	if rows == 0 || height < rows {
		return image;
	}
	let remainder = height % rows;
	let mut canvas = Canvas::new(width, height as f32);
	for i in 0..rows {
		let mut copy_height = height / rows;
		let mut dst_y = copy_height * i;
		let src_y = height - (i + 1) * copy_height - remainder;
		if i == 0 {
			copy_height += remainder;
		} else {
			dst_y += remainder;
		}
		canvas.copy_image(
			&image,
			Rect::new(0.0, src_y as f32, width, copy_height as f32),
			Rect::new(0.0, dst_y as f32, width, copy_height as f32),
		);
	}
	canvas.get_image()
}
