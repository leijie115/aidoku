#![no_std]

mod image;

use aidoku::{
	Chapter, ContentRating, CoverImageProcessor, DeepLinkHandler, DeepLinkResult, FilterValue,
	ImageRequestProvider, ImageResponse, Listing, ListingProvider, Manga, MangaPageResult,
	MangaStatus, Page, PageContent, PageContext, PageImageProcessor, Result, Source, Viewer,
	alloc::{String, Vec, string::ToString as _, vec},
	helpers::uri::QueryParameters,
	imports::{
		canvas::ImageRef,
		html::{Document, Element},
		net::Request,
	},
	prelude::*,
};
use chrono::NaiveDate;
use image::{ImageInfo, MODE_LEGACY, MODE_V3};
use regex::Regex;

const BASE_URL: &str = "https://www.55comic.com";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";

fn get(url: &str) -> Result<Request> {
	Ok(Request::get(url)?
		.header("User-Agent", USER_AGENT)
		.header("Referer", &format!("{BASE_URL}/")))
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

/// Finds the value `span` of a `div.sp-book-meta-item` by its label.
fn meta_item(html: &Document, label: &str) -> Option<Element> {
	html.select("div.sp-book-meta-item")?
		.find(|item| item.text().is_some_and(|text| text.contains(label)))
		.and_then(|item| item.select_first("span"))
}

struct Comic55;

impl Comic55 {
	fn parse_manga_list(&self, html: &Document) -> MangaPageResult {
		let image = ImageInfo::from(html);
		let entries = html
			.select("a.sp-search-card, a.sp-booklist-card, a.sp-rank-card")
			.map(|list| {
				list.filter_map(|element| {
					let href = element.attr("href")?;
					let key = path_of(&href)
						.trim_end_matches('/')
						.rsplit('/')
						.next()?
						.to_string();
					let title = text_of(element.select_first(
						".sp-search-card-title, .sp-booklist-title, .sp-rank-card-title",
					))
					.or_else(|| element.attr("title").map(|t| t.trim().to_string()))?;
					let cover = element
						.select_first("div.cropped[data-src]")
						.and_then(|cover| cover.attr("abs:data-src"))
						.and_then(|url| image.legacy_urls(&url).into_iter().next());
					Some(Manga {
						url: Some(format!("{BASE_URL}/book/{key}")),
						key,
						title,
						cover,
						..Default::default()
					})
				})
				.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		let has_next_page = html.select_first("div.pagination a#nextPage").is_some();
		MangaPageResult {
			entries,
			has_next_page,
		}
	}
}

impl Source for Comic55 {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut query = query.map(|q| q.trim().to_string()).unwrap_or_default();
		let mut tag = None;
		let mut area = String::from("-1");
		let mut end = String::from("-1");

		for filter in filters {
			match filter {
				FilterValue::Text { value, .. } => query = value.trim().into(),
				FilterValue::Select { id, value } => match id.as_str() {
					"area" => area = value,
					"end" => end = value,
					_ => {
						if !value.is_empty() && value != "全部" {
							tag = Some(value)
						}
					}
				},
				_ => {}
			}
		}

		let mut qs = QueryParameters::new();
		let url = if !query.is_empty() {
			if page > 1 {
				return Ok(MangaPageResult::default());
			}
			qs.push("keyword", Some(&query));
			format!("{BASE_URL}/search?{qs}")
		} else {
			if let Some(tag) = &tag {
				qs.push("tag", Some(tag));
			}
			qs.push("area", Some(&area));
			qs.push("end", Some(&end));
			qs.push("page", Some(&page.to_string()));
			format!("{BASE_URL}/booklist?{qs}")
		};

		let html = get(&url)?.html()?;
		let mut result = self.parse_manga_list(&html);
		if !query.is_empty() {
			result.has_next_page = false;
		}
		Ok(result)
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let url = format!("{BASE_URL}/book/{}", manga.key);
		let html = get(&url)?.html()?;

		if needs_details {
			let Some(info) = html.select_first("div.sp-book-info") else {
				bail!("Missing book info");
			};
			if let Some(title) = text_of(info.select_first("h1.sp-book-title")) {
				manga.title = title;
			}
			manga.authors = text_of(info.select_first("p.sp-book-author"))
				.map(|text| {
					let text = text.split_once("作者：").map(|(_, a)| a).unwrap_or(&text);
					// split on "&" that is not surrounded by spaces
					let mut authors = Vec::new();
					let mut current = String::new();
					let chars: Vec<char> = text.chars().collect();
					for (i, c) in chars.iter().enumerate() {
						let surrounded = i > 0
							&& i + 1 < chars.len()
							&& !chars[i - 1].is_whitespace()
							&& !chars[i + 1].is_whitespace();
						if *c == '&' && surrounded {
							authors.push(current.trim().to_string());
							current.clear();
						} else {
							current.push(*c);
						}
					}
					authors.push(current.trim().to_string());
					authors.retain(|a| !a.is_empty());
					authors
				})
				.filter(|authors| !authors.is_empty());
			manga.description = text_of(info.select_first("p.sp-book-summary"));

			let area = text_of(meta_item(&html, "地區"));
			let mut tags: Vec<String> = area.iter().cloned().collect();
			let tag_elements = info
				.select("ul.sp-book-tags-full li a")
				.filter(|list| !list.is_empty())
				.or_else(|| info.select("div.sp-book-tags a"));
			if let Some(list) = tag_elements {
				for tag in list.filter_map(|tag| text_of(Some(tag))) {
					if !tags.contains(&tag) {
						tags.push(tag);
					}
				}
			}
			manga.tags = Some(tags);

			manga.status = match text_of(meta_item(&html, "狀態")).as_deref() {
				Some("連載中") => MangaStatus::Ongoing,
				Some("已完結") => MangaStatus::Completed,
				_ => MangaStatus::Unknown,
			};
			manga.viewer = match area.as_deref() {
				Some("韓漫") => Viewer::Webtoon,
				_ => Viewer::RightToLeft,
			};
			manga.content_rating = ContentRating::NSFW;
			if let Some(cover) = html
				.select_first("div.sp-book-cover div.cropped[data-src]")
				.and_then(|cover| cover.attr("abs:data-src"))
				.and_then(|url| ImageInfo::from(&html).legacy_urls(&url).into_iter().next())
			{
				manga.cover = Some(cover);
			}
			manga.url = Some(url);
		}

		if needs_chapters {
			let number_re = Regex::new(r"第\s*(\d+(?:\.\d+)?)\s*[話话回]").ok();
			let mut chapters = html
				.select("div.sp-chapter-grid a.sp-chapter-item")
				.map(|list| {
					list.filter_map(|element| {
						let href = element.attr("href")?;
						let key = path_of(&href).to_string();
						let title = element
							.attr("title")
							.map(|t| t.trim().to_string())
							.filter(|t| !t.is_empty())
							.or_else(|| text_of(Some(element)))?;
						let chapter_number = number_re
							.as_ref()
							.and_then(|re| re.captures(&title))
							.and_then(|caps| caps.get(1))
							.and_then(|m| m.as_str().parse::<f32>().ok());
						Some(Chapter {
							url: Some(format!("{BASE_URL}{key}")),
							key,
							title: Some(title),
							chapter_number,
							..Default::default()
						})
					})
					.collect::<Vec<_>>()
				})
				.unwrap_or_default();
			chapters.reverse();

			if let Some(first) = chapters.first_mut() {
				first.date_uploaded = text_of(meta_item(&html, "更新"))
					.and_then(|date| NaiveDate::parse_from_str(&date, "%Y-%m-%d").ok())
					.and_then(|date| date.and_hms_opt(0, 0, 0))
					.map(|date| date.and_utc().timestamp());
			}
			manga.chapters = Some(chapters);
		}

		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let html = get(&format!("{BASE_URL}{}", chapter.key))?.html()?;
		let image = ImageInfo::from(&html);
		let pages = html
			.select("div.comicpage div.cropped[data-src]")
			.map(|list| {
				list.filter_map(|element| {
					let (mode, mut urls) = match element
						.attr("data-bmi-manifest")
						.filter(|manifest| !manifest.is_empty())
						.and_then(|manifest| image::manifest_urls(&manifest))
					{
						Some(urls) => (MODE_V3, urls),
						None => (
							MODE_LEGACY,
							image.legacy_urls(&element.attr("abs:data-src")?),
						),
					};
					if urls.is_empty() {
						return None;
					}
					let first = urls.remove(0);
					let mut context = PageContext::new();
					context.insert("mode".into(), mode.into());
					context.insert("chunks".into(), urls.join("\n"));
					Some(Page {
						content: PageContent::url_context(first, context),
						..Default::default()
					})
				})
				.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		Ok(pages)
	}
}

impl ListingProvider for Comic55 {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		if listing.id != "rank" || page > 1 {
			return Ok(MangaPageResult::default());
		}
		let html = get(&format!("{BASE_URL}/rank"))?.html()?;
		let mut result = self.parse_manga_list(&html);
		result.has_next_page = false;
		Ok(result)
	}
}

impl ImageRequestProvider for Comic55 {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		get(&url)
	}
}

/// Fetches the remaining chunks of an image and decodes it.
fn decode_chunks(
	first: Vec<u8>,
	first_url: Option<&str>,
	mode: &str,
	rest: &[&str],
) -> Result<ImageRef> {
	let first = if first.is_empty() {
		let Some(url) = first_url else {
			bail!("Missing image data");
		};
		get(url)?.data()?
	} else {
		first
	};
	let mut chunks = vec![first];
	for url in rest {
		chunks.push(get(url)?.data()?);
	}
	let data = image::decrypt_chunks(mode, chunks)?;
	let (image, rows) = image::restore_header(data)?;
	Ok(image::unscramble(image, rows))
}

impl PageImageProcessor for Comic55 {
	fn process_page_image(
		&self,
		response: ImageResponse,
		context: Option<PageContext>,
	) -> Result<ImageRef> {
		let Some(context) = context else {
			return Ok(response.image);
		};
		let Some(mode) = context
			.get("mode")
			.filter(|mode| *mode == MODE_LEGACY || *mode == MODE_V3)
		else {
			return Ok(response.image);
		};
		let rest = context
			.get("chunks")
			.map(|chunks| {
				chunks
					.split('\n')
					.filter(|url| !url.is_empty())
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		decode_chunks(
			response.image.data(),
			response.request.url.as_deref(),
			mode,
			&rest,
		)
	}
}

impl CoverImageProcessor for Comic55 {
	fn process_cover_image(&self, response: ImageResponse) -> Result<ImageRef> {
		// covers are legacy encrypted images split into `.b_0` and `.b_1`
		let Some(url) = response.request.url.as_deref() else {
			return Ok(response.image);
		};
		if !path_of(url).ends_with(".b_0") {
			return Ok(response.image);
		}
		let second = url.replacen(".b_0", ".b_1", 1);
		decode_chunks(response.image.data(), Some(url), MODE_LEGACY, &[&second])
	}
}

impl DeepLinkHandler for Comic55 {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let path = path_of(&url);
		if let Some(id) = path.strip_prefix("/book/") {
			let id = id.trim_end_matches('/');
			if !id.is_empty() && !id.contains('/') {
				return Ok(Some(DeepLinkResult::Manga { key: id.into() }));
			}
		}
		Ok(None)
	}
}

register_source!(
	Comic55,
	ListingProvider,
	ImageRequestProvider,
	PageImageProcessor,
	CoverImageProcessor,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
