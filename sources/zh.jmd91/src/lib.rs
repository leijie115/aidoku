#![no_std]

use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider,
	ImageResponse, Manga, MangaPageResult, MangaStatus, Page, PageContent, PageContext,
	PageImageProcessor, Result, Source, Viewer,
	alloc::{String, Vec, string::ToString as _},
	helpers::uri::encode_uri_component,
	imports::{
		canvas::ImageRef,
		html::{Document, Element, Html},
		net::Request,
		std::current_date,
	},
	prelude::*,
};
use chrono::{DateTime, Datelike as _, NaiveDate};
use regex::Regex;

const BASE_URL: &str = "https://91jmd.com";
const GENRES: [&str; 5] = ["韩漫", "日漫", "3D漫画", "真人", "短篇"];
const SORTS: [&str; 2] = ["time", "hits"];
const IMAGE_HOSTS: [&str; 7] = [
	"img.nnpic.xyz",
	"p4.nnpic.xyz",
	"p6.nnpic.xyz",
	"p8.nnpic.xyz",
	"p9.nnpic.xyz",
	"p10.nnpic.xyz",
	"p11.nnpic.xyz",
];

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

fn unescape(text: String) -> String {
	Html::unescape(&text).unwrap_or(text).trim().to_string()
}

fn text_of(element: Option<Element>) -> Option<String> {
	element
		.and_then(|element| element.text())
		.map(unescape)
		.filter(|text| !text.is_empty())
}

fn parse_manga_list(html: &Document) -> MangaPageResult {
	let entries = html
		.select("div.module-items > div.module-item")
		.map(|list| {
			list.filter_map(|element| {
				let link = element.select_first("a.module-item-title")?;
				let key = path_of(&link.attr("href")?)
					.strip_prefix("/manga/")?
					.trim_end_matches('/')
					.to_string();
				let title = link
					.attr("title")
					.filter(|title| !title.is_empty())
					.or_else(|| link.text())
					.map(unescape)?;
				let cover = element
					.select_first("img")
					.and_then(|img| img.attr("abs:data-original"));
				Some(Manga {
					url: Some(format!("{BASE_URL}/manga/{key}")),
					key,
					title,
					cover,
					..Default::default()
				})
			})
			.collect::<Vec<_>>()
		})
		.unwrap_or_default();
	let has_next_page = html.select_first("#page a.page-next").is_some();
	MangaPageResult {
		entries,
		has_next_page,
	}
}

/// Returns the `.video-info-item` element of the info row with the given title.
fn info_item(html: &Document, title: &str) -> Option<Element> {
	html.select(".video-info-items")?
		.find(|item| {
			text_of(item.select_first(".video-info-itemtitle"))
				.is_some_and(|text| text.trim_end_matches(['：', ':']) == title)
		})
		.and_then(|item| item.select_first(".video-info-item"))
}

/// Parses dates like "6小时之前", "前天 20:55", "10/22/2026" or "10-22".
fn parse_date(text: &str) -> Option<i64> {
	let now = current_date();
	if let Some(caps) = Regex::new(r"(\d+)\s*(秒|分钟|小时|天|周|个月|月|年)[之以]?前")
		.ok()?
		.captures(text)
	{
		let amount: i64 = caps[1].parse().ok()?;
		let unit = match &caps[2] {
			"秒" => 1,
			"分钟" => 60,
			"小时" => 3600,
			"天" => 86400,
			"周" => 7 * 86400,
			"个月" | "月" => 30 * 86400,
			_ => 365 * 86400,
		};
		return Some(now - amount * unit);
	}

	// the site shows times in China Standard Time
	const OFFSET: i64 = 8 * 3600;
	let today = DateTime::from_timestamp(now + OFFSET, 0)?.date_naive();
	let to_timestamp = |date: NaiveDate, hour: u32, minute: u32| {
		date.and_hms_opt(hour, minute, 0)
			.map(|date| date.and_utc().timestamp() - OFFSET)
	};

	if let Some(caps) = Regex::new(r"(今天|昨天|前天)\s*(?:(\d{1,2}):(\d{2}))?")
		.ok()?
		.captures(text)
	{
		let days = match &caps[1] {
			"今天" => 0,
			"昨天" => 1,
			_ => 2,
		};
		let date = today.checked_sub_days(chrono::Days::new(days))?;
		let hour = caps
			.get(2)
			.and_then(|m| m.as_str().parse().ok())
			.unwrap_or(0);
		let minute = caps
			.get(3)
			.and_then(|m| m.as_str().parse().ok())
			.unwrap_or(0);
		return to_timestamp(date, hour, minute);
	}

	let text = text.trim();
	if let Ok(date) = NaiveDate::parse_from_str(text, "%m/%d/%Y") {
		return to_timestamp(date, 0, 0);
	}
	let (month, day) = text.split_once('-')?;
	let (month, day) = (month.trim().parse().ok()?, day.trim().parse().ok()?);
	let mut date = NaiveDate::from_ymd_opt(today.year(), month, day)?;
	if date > today {
		date = NaiveDate::from_ymd_opt(today.year() - 1, month, day)?;
	}
	to_timestamp(date, 0, 0)
}

struct Jmd91;

impl Source for Jmd91 {
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
		let mut genre = "all".to_string();
		let mut sort = SORTS[0];
		let mut status = "all".to_string();

		for filter in filters {
			match filter {
				FilterValue::Text { value, .. } => query = value.trim().into(),
				FilterValue::Sort { index, .. } => {
					sort = SORTS.get(index as usize).copied().unwrap_or(sort);
				}
				FilterValue::Select { id, value } => match id.as_str() {
					"status" => status = value,
					_ if value == "all" || GENRES.contains(&value.as_str()) => genre = value,
					// tags of a manga are searched by keyword on the site
					_ => query = value,
				},
				_ => {}
			}
		}

		let url = if !query.is_empty() {
			format!(
				"{BASE_URL}/search/{}/page/{page}",
				encode_uri_component(&query)
			)
		} else {
			format!(
				"{BASE_URL}/catalog/{}/ob/{sort}/st/{status}/page/{page}",
				encode_uri_component(&genre)
			)
		};
		Ok(parse_manga_list(&get(&url)?.html()?))
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let url = format!("{BASE_URL}/manga/{}", manga.key);
		let html = get(&url)?.html()?;
		let updated = text_of(info_item(&html, "更新"));

		if needs_details {
			if let Some(title) = text_of(html.select_first("h1.page-title")) {
				manga.title = title;
			}
			if let Some(cover) = html
				.select_first(".video-cover img")
				.and_then(|img| img.attr("abs:data-original"))
			{
				manga.cover = Some(cover);
			}
			manga.authors = info_item(&html, "作者")
				.and_then(|item| item.select("a"))
				.map(|list| list.filter_map(|a| text_of(Some(a))).collect::<Vec<_>>())
				.filter(|authors| !authors.is_empty());
			manga.description = text_of(html.select_first(".video-info-content span"));

			let categories = html
				.select(".video-info-aux .tag-link a")
				.map(|list| list.filter_map(|a| text_of(Some(a))).collect::<Vec<_>>())
				.unwrap_or_default();
			manga.viewer = if categories.iter().any(|c| c == "日漫") {
				Viewer::RightToLeft
			} else {
				Viewer::Webtoon
			};
			let mut tags: Vec<String> = Vec::new();
			let tag_items = info_item(&html, "TAG")
				.and_then(|item| item.select("a"))
				.map(|list| list.filter_map(|a| text_of(Some(a))).collect::<Vec<_>>())
				.unwrap_or_default();
			for tag in categories.into_iter().chain(tag_items) {
				if !tags.contains(&tag) {
					tags.push(tag);
				}
			}
			manga.tags = Some(tags).filter(|tags| !tags.is_empty());

			manga.status = match updated.as_deref().and_then(|text| text.split(' ').next()) {
				Some("连载中") => MangaStatus::Ongoing,
				Some("已完结") => MangaStatus::Completed,
				_ => MangaStatus::Unknown,
			};
			manga.content_rating = ContentRating::NSFW;
			manga.url = Some(url.clone());
		}

		if needs_chapters {
			let number_re = Regex::new(r"第\s*(\d+(?:\.\d+)?)\s*[話话回]").ok();
			let prefix = format!("/manga/{}/", manga.key);
			let mut chapters = html
				.select("#sort-item-3 a.detail-write")
				.map(|list| {
					list.filter_map(|element| {
						let href = element.attr("href")?;
						let key = path_of(&href).strip_prefix(&prefix)?.to_string();
						let title = text_of(Some(element))?;
						let chapter_number = number_re
							.as_ref()
							.and_then(|re| re.captures(&title))
							.and_then(|caps| caps.get(1))
							.and_then(|m| m.as_str().parse::<f32>().ok());
						Some(Chapter {
							url: Some(format!("{BASE_URL}{prefix}{key}")),
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
				first.date_uploaded = updated
					.as_deref()
					.and_then(|text| text.split_once(' '))
					.map(|(_, date)| date.trim())
					.filter(|date| !date.is_empty())
					.and_then(parse_date);
			}
			manga.chapters = Some(chapters);
		}

		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{BASE_URL}/manga/{}/{}", manga.key, chapter.key);
		let html = get(&url)?.html()?;
		Ok(html
			.select("#m_r_imgbox_0 img[data-src]")
			.map(|list| {
				list.filter_map(|img| img.attr("abs:data-src"))
					.map(|url| Page {
						content: PageContent::url(url),
						..Default::default()
					})
					.collect()
			})
			.unwrap_or_default())
	}
}

impl ImageRequestProvider for Jmd91 {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		get(&url)
	}
}

impl PageImageProcessor for Jmd91 {
	fn process_page_image(
		&self,
		response: ImageResponse,
		_context: Option<PageContext>,
	) -> Result<ImageRef> {
		if (200..300).contains(&response.code) && response.image.width() > 0.0 {
			return Ok(response.image);
		}
		// the image hosts mirror each other, so try the others if one fails
		let Some(url) = response.request.url else {
			return Ok(response.image);
		};
		let Some(rest) = url.strip_prefix("https://") else {
			return Ok(response.image);
		};
		let Some((host, path)) = rest.split_once('/') else {
			return Ok(response.image);
		};
		let Some(index) = IMAGE_HOSTS.iter().position(|h| *h == host) else {
			return Ok(response.image);
		};
		for offset in 1..IMAGE_HOSTS.len() {
			let host = IMAGE_HOSTS[(index + offset) % IMAGE_HOSTS.len()];
			let Ok(response) = get(&format!("https://{host}/{path}"))?.send() else {
				continue;
			};
			if (200..300).contains(&response.status_code())
				&& let Ok(image) = response.get_image()
			{
				return Ok(image);
			}
		}
		Ok(response.image)
	}
}

impl DeepLinkHandler for Jmd91 {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let path = path_of(&url).trim_end_matches('/');
		let Some(rest) = path.strip_prefix("/manga/") else {
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
	Jmd91,
	ImageRequestProvider,
	PageImageProcessor,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
