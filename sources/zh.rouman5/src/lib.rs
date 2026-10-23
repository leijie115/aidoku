#![no_std]

use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider,
	ImageResponse, Manga, MangaPageResult, MangaStatus, Page, PageContent, PageContext,
	PageImageProcessor, Result, Source, Viewer,
	alloc::{String, Vec, string::ToString as _, vec},
	helpers::uri::{QueryParameters, encode_uri_component},
	imports::{
		canvas::{Canvas, ImageRef, Rect},
		html::{Document, Element, Html},
		net::Request,
	},
	prelude::*,
};
use base64::{
	Engine as _, alphabet,
	engine::{DecodePaddingMode, GeneralPurpose, GeneralPurposeConfig},
};
use chrono::NaiveDateTime;
use regex::Regex;

const BASE_URL: &str = "https://rouman5.com";
const ALL_TERM: &str = "%";
const SORTS: [&str; 4] = ["relevance", "views", "updated", "shelved"];

const BASE64: GeneralPurpose = GeneralPurpose::new(
	&alphabet::URL_SAFE,
	GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

fn get(url: &str) -> Result<Request> {
	Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
}

/// Extracts the path of an url, without scheme, host, query and fragment.
fn path_of(url: &str) -> &str {
	let path = url
		.find("://")
		.map(|pos| &url[pos + 3..])
		.map(|rest| rest.find('/').map(|slash| &rest[slash..]).unwrap_or("/"))
		.unwrap_or(url);
	path.split(['?', '#']).next().unwrap_or_default()
}

fn text_of(element: Option<Element>) -> Option<String> {
	element
		.and_then(|element| element.text())
		.map(|text| text.trim().to_string())
		.filter(|text| !text.is_empty())
}

fn parse_manga_list(html: &Document) -> MangaPageResult {
	let entries = html
		.select("a.site-comic")
		.map(|list| {
			list.filter_map(|element| {
				let href = element.attr("href")?;
				let key = path_of(&href).strip_prefix("/books/")?.to_string();
				let title = text_of(element.select_first("h3"))?;
				let cover = element
					.select_first(".site-comic-cover img")
					.and_then(|img| img.attr("abs:src"));
				Some(Manga {
					url: Some(format!("{BASE_URL}/books/{key}")),
					key,
					title,
					cover,
					..Default::default()
				})
			})
			.collect::<Vec<_>>()
		})
		.unwrap_or_default();
	let has_next_page = html
		.select_first("nav.site-pagination > a:last-child")
		.is_some();
	MangaPageResult {
		entries,
		has_next_page,
	}
}

fn add_page(qs: &mut QueryParameters, page: i32) {
	if page > 1 {
		qs.push("page", Some(&(page - 1).to_string()));
	}
}

fn with_query(url: String, qs: &QueryParameters) -> String {
	if qs.is_empty() {
		url
	} else {
		format!("{url}?{qs}")
	}
}

fn search_url(page: i32, term: &str, sort: &str) -> String {
	let mut qs = QueryParameters::new();
	qs.push("term", Some(term));
	if sort != "relevance" {
		qs.push("sort", Some(sort));
	}
	add_page(&mut qs, page);
	with_query(format!("{BASE_URL}/search"), &qs)
}

fn list_url(path: String, page: i32, continued: Option<&str>) -> String {
	let mut qs = QueryParameters::new();
	if let Some(continued) = continued {
		qs.push("continued", Some(continued));
	}
	add_page(&mut qs, page);
	with_query(path, &qs)
}

fn parse_date(date: &str) -> Option<i64> {
	NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S%.fZ")
		.ok()
		.map(|date| date.and_utc().timestamp())
}

struct Rouman5;

impl Source for Rouman5 {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let query = query.map(|q| q.trim().to_string()).unwrap_or_default();
		let mut sort = "relevance";
		let mut continued = None;
		let mut tag = String::new();

		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => {
					sort = SORTS.get(index as usize).copied().unwrap_or(sort);
				}
				FilterValue::Select { id, value } => {
					if id == "continued" {
						continued = Some(value).filter(|v| !v.is_empty());
					} else {
						// tags tapped in the manga details
						tag = value.trim().into();
					}
				}
				FilterValue::Text { value, .. } => tag = value.trim().into(),
				_ => {}
			}
		}

		let url = if !query.is_empty() {
			search_url(page, &query, sort)
		} else if !tag.is_empty() {
			list_url(
				format!("{BASE_URL}/tags/{}", encode_uri_component(&tag)),
				page,
				continued.as_deref(),
			)
		} else if sort != "relevance" {
			search_url(page, ALL_TERM, sort)
		} else {
			list_url(format!("{BASE_URL}/books"), page, continued.as_deref())
		};

		Ok(parse_manga_list(&get(&url)?.html()?))
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let url = format!("{BASE_URL}/books/{}", manga.key);
		let html_text = get(&url)?.string()?;
		let html = Html::parse_with_url(&html_text, &url)?;

		if needs_details {
			let Some(info) = html.select_first(".site-book-info") else {
				bail!("Missing book info");
			};
			let mut data: Vec<(String, String)> = Vec::new();
			if let Some(list) = info.select("dl.site-book-data dt") {
				for dt in list {
					let Some(key) = dt.text() else { continue };
					let value = dt.next().and_then(|dd| dd.text()).unwrap_or_default();
					data.push((key.trim().into(), value.trim().into()));
				}
			}
			let field = |name: &str| {
				data.iter()
					.find(|(key, _)| key == name)
					.map(|(_, value)| value.as_str())
					.filter(|value| !value.is_empty())
			};

			if let Some(title) = text_of(info.select_first("h1")) {
				manga.title = title;
			}
			if let Some(cover) = html
				.select_first("img.site-detail-cover")
				.and_then(|img| img.attr("abs:src"))
			{
				manga.cover = Some(cover);
			}
			manga.authors = field("作者")
				.filter(|author| *author != "N/A")
				.map(|author| vec![author.into()]);
			manga.status = match field("狀態") {
				Some(status) if status.contains("已完結") => MangaStatus::Completed,
				Some(status) if status.contains("連載中") => MangaStatus::Ongoing,
				_ => MangaStatus::Unknown,
			};

			let mut description =
				text_of(html.select_first(".site-book-synopsis")).unwrap_or_default();
			if let Some(alias) = text_of(info.select_first("p.site-book-alias"))
				&& alias != manga.title
			{
				if !description.is_empty() {
					description.push_str("\n\n");
				}
				description.push_str("別名：");
				description.push_str(&alias);
			}
			let region = field("地區");
			if let Some(region) = region {
				if !description.is_empty() {
					description.push('\n');
				}
				description.push_str("地區：");
				description.push_str(region);
			}
			manga.description = Some(description).filter(|d| !d.is_empty());
			manga.viewer = match region {
				Some("日本") => Viewer::RightToLeft,
				_ => Viewer::Webtoon,
			};

			let mut tags: Vec<String> = Vec::new();
			if let Some(list) = html.select("nav.site-book-tags a") {
				for tag in list.filter_map(|tag| text_of(Some(tag))) {
					if !tags.contains(&tag) {
						tags.push(tag);
					}
				}
			}
			if tags.is_empty()
				&& let Some(array) =
					Regex::new(r"activeResource:\$R\[\d+\]=\{[^{}]*?tags:\$R\[\d+\]=(\[[^\]]*\])")
						.ok()
						.and_then(|re| re.captures(&html_text))
						.and_then(|caps| caps.get(1))
				&& let Ok(list) = serde_json::from_str::<Vec<String>>(array.as_str())
			{
				for tag in list {
					if !tags.contains(&tag) {
						tags.push(tag);
					}
				}
			}
			manga.tags = Some(tags).filter(|tags| !tags.is_empty());
			manga.content_rating = ContentRating::NSFW;
			manga.url = Some(url.clone());
		}

		if needs_chapters {
			// chapter dates are only available in the serialized page data
			let mut dates: Vec<(String, i64)> = Vec::new();
			let book_start = html_text.find("book:$R[").unwrap_or(0);
			if let Some(start) = html_text[book_start..]
				.find("chapters:$R[")
				.map(|pos| pos + book_start)
				&& let Ok(re) =
					Regex::new(r#"ind:(\d+),[^{}]*?createdAt:\$R\[\d+\]=new Date\("([^"]+)"\)"#)
			{
				for caps in re.captures_iter(&html_text[start..]) {
					let ind = caps[1].to_string();
					if dates.iter().any(|(key, _)| *key == ind) {
						continue;
					}
					if let Some(date) = parse_date(&caps[2]) {
						dates.push((ind, date));
					}
				}
			}

			let number_re = Regex::new(r"第\s*(\d+(?:\.\d+)?)\s*[話话回]").ok();
			let prefix = format!("/books/{}/", manga.key);
			let mut chapters = html
				.select("a.site-chapter-link")
				.map(|list| {
					list.filter_map(|element| {
						let href = element.attr("href")?;
						let key = path_of(&href).strip_prefix(&prefix)?.to_string();
						let span = element.select_first("span");
						let title = span
							.as_ref()
							.and_then(|span| span.attr("title"))
							.map(|title| title.trim().to_string())
							.filter(|title| !title.is_empty())
							.or_else(|| text_of(span))
							.or_else(|| text_of(Some(element)));
						let chapter_number = title
							.as_deref()
							.zip(number_re.as_ref())
							.and_then(|(title, re)| re.captures(title))
							.and_then(|caps| caps.get(1))
							.and_then(|m| m.as_str().parse::<f32>().ok());
						let date_uploaded = dates
							.iter()
							.find(|(ind, _)| *ind == key)
							.map(|(_, date)| *date);
						Some(Chapter {
							url: Some(format!("{BASE_URL}{prefix}{key}")),
							key,
							title,
							chapter_number,
							date_uploaded,
							..Default::default()
						})
					})
					.collect::<Vec<_>>()
				})
				.unwrap_or_default();
			chapters.reverse();
			manga.chapters = Some(chapters);
		}

		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{BASE_URL}/books/{}/{}", manga.key, chapter.key);
		let html = get(&url)?.string()?;
		let Some(array) = Regex::new(r"imagePaths:\$R\[\d+\]=(\[[^\]]*\])")
			.ok()
			.and_then(|re| re.captures(&html))
			.and_then(|caps| caps.get(1))
		else {
			bail!("找不到圖片列表");
		};
		let urls: Vec<String> = serde_json::from_str(array.as_str())?;
		Ok(urls
			.into_iter()
			.map(|url| Page {
				content: PageContent::url(url),
				..Default::default()
			})
			.collect())
	}
}

impl ImageRequestProvider for Rouman5 {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		get(&url)
	}
}

/// Returns the number of slices if the image url points to a scrambled image.
fn scramble_slices(url: &str) -> Option<u32> {
	let path = path_of(url);
	let mut segments = path.trim_start_matches('/').split('/');
	if segments.next() != Some("m") {
		return None;
	}
	if !path.split('/').any(|segment| segment == "sr:1") {
		return None;
	}
	let last = path.rsplit('/').next()?;
	let encoded = last.rsplit_once('.').map(|(name, _)| name).unwrap_or(last);
	let decoded = BASE64.decode(encoded).ok()?;
	let digest = md5::compute(decoded);
	Some(u32::from(digest[15]) % 10 + 5)
}

impl PageImageProcessor for Rouman5 {
	fn process_page_image(
		&self,
		response: ImageResponse,
		_context: Option<PageContext>,
	) -> Result<ImageRef> {
		let Some(slices) = response.request.url.as_deref().and_then(scramble_slices) else {
			return Ok(response.image);
		};
		let image = response.image;
		let width = image.width();
		let height = image.height() as u32;
		if height < slices {
			return Ok(image);
		}
		let remainder = height % slices;
		let mut canvas = Canvas::new(width, height as f32);
		for i in 0..slices {
			let mut slice_height = height / slices;
			let mut dst_y = slice_height * i;
			let src_y = height - (i + 1) * slice_height - remainder;
			if i == 0 {
				slice_height += remainder;
			} else {
				dst_y += remainder;
			}
			canvas.copy_image(
				&image,
				Rect::new(0.0, src_y as f32, width, slice_height as f32),
				Rect::new(0.0, dst_y as f32, width, slice_height as f32),
			);
		}
		Ok(canvas.get_image())
	}
}

impl DeepLinkHandler for Rouman5 {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let path = path_of(&url).trim_end_matches('/');
		let Some(rest) = path.strip_prefix("/books/") else {
			return Ok(None);
		};
		Ok(match rest.split_once('/') {
			None if !rest.is_empty() => Some(DeepLinkResult::Manga { key: rest.into() }),
			Some((manga, chapter)) if !chapter.contains('/') => Some(DeepLinkResult::Chapter {
				manga_key: manga.into(),
				key: chapter.into(),
			}),
			_ => None,
		})
	}
}

register_source!(
	Rouman5,
	ImageRequestProvider,
	PageImageProcessor,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
