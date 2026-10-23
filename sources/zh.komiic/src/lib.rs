#![no_std]

mod helpers;
mod home;
mod settings;

use aidoku::{
	BasicLoginHandler, Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue,
	HashMap, Home as HomeProvider, HomeComponent, HomeComponentValue, HomeLayout,
	ImageRequestProvider, Link, Listing, ListingProvider, Manga, MangaPageResult, MangaStatus,
	NotificationHandler, Page, PageContent, PageContext, Result, Source, Viewer,
	alloc::{String, Vec, format},
	imports::{
		defaults::{DefaultValue, defaults_get, defaults_set},
		net::Request,
		std::send_partial_result,
	},
	prelude::*,
};
use serde_json::{Value, json};

const BASE_URL: &str = "https://komiic.com";
const QUERY_URL: &str = "https://komiic.com/api/query";
const LOGIN_URL: &str = "https://komiic.com/api/login";
const IMAGE_URL: &str = "https://komiic.com/api/image";
const REFERER_URL: &str = "https://komiic.com/";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36";
const PAGE_SIZE: i32 = 20;
const CATEGORY_PAGE_SIZE: i32 = 30;
const RECOMMENDATION_PAGE_SIZE: i32 = 25;
const SORT_ORDER_IDS: [&str; 3] = ["DATE_UPDATED", "VIEWS", "FAVORITE_COUNT"];
const CHAPTER_FILTER_KEY: &str = "chapterFilter";
const CHECK_IMAGE_LIMIT_KEY: &str = "checkImageLimit";
const TOKEN_KEY: &str = "token";
const JUST_LOGGED_IN_KEY: &str = "justLoggedIn";

const COMIC_FIELDS: &str = r#"
id
title
status
imageUrl
__typename
"#;

const HOME_COMIC_FIELDS: &str = r#"
id
title
status
imageUrl
authors { id name __typename }
categories { id name __typename }
__typename
"#;

const DETAIL_COMIC_FIELDS: &str = r#"
id
title
status
year
imageUrl
authors { id name __typename }
categories { id name __typename }
dateCreated
dateUpdated
monthViews
views
favoriteCount
lastBookUpdate
lastChapterUpdate
description
__typename
"#;

struct KomiicSource;

/// Parse a UTC timestamp like `2023-09-20T17:39:43Z` into seconds since epoch.
fn parse_iso_date(date: &str) -> Option<i64> {
	let (date, time) = date.trim_end_matches('Z').split_once('T')?;
	let mut ymd = date.split('-').map(|part| part.parse::<i64>().ok());
	let (year, month, day) = (ymd.next()??, ymd.next()??, ymd.next()??);
	let mut hms = time
		.split('.')
		.next()?
		.split(':')
		.map(|part| part.parse::<i64>().ok());
	let (hour, minute, second) = (hms.next()??, hms.next()??, hms.next()??);
	// Days from civil (Howard Hinnant's algorithm).
	let y = if month <= 2 { year - 1 } else { year };
	let era = y.div_euclid(400);
	let yoe = y - era * 400;
	let mp = (month + 9) % 12;
	let doy = (153 * mp + 2) / 5 + day - 1;
	let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
	let days = era * 146_097 + doe - 719_468;
	Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

impl KomiicSource {
	fn chapter_title(value: &Value) -> Option<String> {
		let serial = value.get("serial").and_then(Value::as_str)?.trim();
		if serial.is_empty() {
			return None;
		}
		if serial.parse::<f32>().is_ok() {
			return None;
		}
		if value.get("type").and_then(Value::as_str) == Some("book") {
			Some(format!("卷{serial}"))
		} else {
			Some(String::from(serial))
		}
	}

	fn parse_chapter(value: &Value, manga_key: &str) -> Option<Chapter> {
		let key = Self::string_field(value, "id")?;
		let serial = value
			.get("serial")
			.and_then(Value::as_str)
			.unwrap_or_default();
		let serial_number = serial.parse::<f32>().ok();
		let is_book = value.get("type").and_then(Value::as_str) == Some("book");
		let url = format!("{BASE_URL}/comic/{manga_key}/chapter/{key}/images/all");
		let date_uploaded = value
			.get("dateCreated")
			.and_then(Value::as_str)
			.and_then(parse_iso_date);
		let scanlator = if is_book { "單行本" } else { "連載" };
		Some(Chapter {
			key,
			title: Self::chapter_title(value),
			chapter_number: if is_book { None } else { serial_number },
			volume_number: if is_book { serial_number } else { None },
			date_uploaded,
			scanlators: Some(Vec::from([String::from(scanlator)])),
			url: Some(url),
			language: Some(String::from("zh")),
			..Default::default()
		})
	}

	/// Pick which releases to show according to the `chapterFilter` setting:
	/// `all` (web chapters followed by volumes), `chapter`, `book`, or
	/// `prefer_book` (volumes when available, otherwise web chapters).
	fn select_chapter_version(
		mut books: Vec<Chapter>,
		mut web_chapters: Vec<Chapter>,
		filter: &str,
	) -> Vec<Chapter> {
		let by_number_desc = |a: &Chapter, b: &Chapter| {
			let a_number = a.chapter_number.or(a.volume_number).unwrap_or(0.0);
			let b_number = b.chapter_number.or(b.volume_number).unwrap_or(0.0);
			b_number
				.partial_cmp(&a_number)
				.unwrap_or(core::cmp::Ordering::Equal)
		};
		books.sort_by(by_number_desc);
		web_chapters.sort_by(by_number_desc);
		match filter {
			"chapter" => web_chapters,
			"book" => books,
			"prefer_book" if !books.is_empty() => books,
			"prefer_book" => web_chapters,
			_ => {
				web_chapters.extend(books);
				web_chapters
			}
		}
	}

	fn chapters(manga_key: &str) -> Result<Vec<Chapter>> {
		let json = Self::query(json!({
			"operationName": "chapterByComicId",
			"variables": { "comicId": manga_key },
			"query": "query chapterByComicId($comicId: ID!) { chaptersByComicId(comicId: $comicId) { id serial type dateCreated dateUpdated size __typename } }"
		}))?;
		let values = json
			.get("data")
			.and_then(|value| value.get("chaptersByComicId"))
			.and_then(Value::as_array)
			.ok_or_else(|| error!("Komiic missing chapters"))?;
		let mut books = Vec::new();
		let mut web_chapters = Vec::new();
		for value in values {
			if let Some(chapter) = Self::parse_chapter(value, manga_key) {
				if value.get("type").and_then(Value::as_str) == Some("book") {
					books.push(chapter);
				} else {
					web_chapters.push(chapter);
				}
			}
		}
		Ok(Self::select_chapter_version(
			books,
			web_chapters,
			&Self::chapter_filter(),
		))
	}
}

impl Source for KomiicSource {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let (order_by, status, category, keyword, author) =
			Self::parse_search_filters(query.as_deref(), &filters);
		if let Some(author) = author {
			Self::search_by_author(author, page)
		} else if let Some(keyword) = keyword {
			if let Some(category_id) = Self::category_filter_value(keyword.as_str()) {
				Self::comics_by_category(
					category_id.as_str(),
					order_by.as_str(),
					status.as_str(),
					page,
				)
			} else {
				Self::search(keyword)
			}
		} else {
			Self::comics_by_category(category.as_str(), order_by.as_str(), status.as_str(), page)
		}
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		if needs_details {
			if let Some(updated) = Self::comic_by_id(manga.key.clone())? {
				manga.copy_from(updated);
			}
			if needs_chapters {
				send_partial_result(&manga);
			}
		}
		if needs_chapters {
			manga.chapters = Some(Self::chapters(&manga.key)?);
		}
		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let json = Self::query(json!({
			"operationName": "imagesByChapterId",
			"variables": { "chapterId": chapter.key },
			"query": "query imagesByChapterId($chapterId: ID!) { reachedImageLimit imagesByChapterId(chapterId: $chapterId) { id kid height width __typename } }"
		}))?;
		let reached_limit = json
			.get("data")
			.and_then(|value| value.get("reachedImageLimit"))
			.and_then(Value::as_bool)
			.unwrap_or(false);
		if reached_limit && Self::checks_image_limit() {
			bail!("今日圖片讀取次數已達上限，請登入或明天再來！");
		}
		let values = json
			.get("data")
			.and_then(|value| value.get("imagesByChapterId"))
			.and_then(Value::as_array)
			.ok_or_else(|| error!("Komiic missing images"))?;
		let mut pages = Vec::new();
		for value in values {
			if let Some(kid) = value.get("kid").and_then(Value::as_str) {
				let mut context: PageContext = HashMap::new();
				context.insert(
					String::from("referer"),
					format!(
						"{BASE_URL}/comic/{}/chapter/{}/images/all",
						manga.key, chapter.key
					),
				);
				pages.push(Page {
					content: PageContent::url_context(format!("{IMAGE_URL}/{kid}"), context),
					..Default::default()
				});
			}
		}
		Ok(pages)
	}
}

impl ListingProvider for KomiicSource {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		match listing.id.as_str() {
			"recent_update" => Self::get_comic_list("recentUpdate", "DATE_UPDATED", page),
			"recommendations" => Self::recommendations(page),
			"month_views" => Self::get_comic_list("hotComics", "MONTH_VIEWS", page),
			"views" => Self::get_comic_list("hotComics", "VIEWS", page),
			id => Self::category_listing(id, page).unwrap_or_else(|| Ok(Self::empty_page())),
		}
	}
}

impl DeepLinkHandler for KomiicSource {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let mut splits = url.split('/').skip(3);
		let result = match splits.next() {
			Some("comic") => match (splits.next(), splits.next(), splits.next(), splits.next()) {
				(Some(key), None, None, None) => Some(DeepLinkResult::Manga { key: key.into() }),
				(Some(manga_key), Some("chapter"), Some(key), _) => Some(DeepLinkResult::Chapter {
					manga_key: manga_key.into(),
					key: key.into(),
				}),
				_ => None,
			},
			_ => None,
		};
		Ok(result)
	}
}

impl ImageRequestProvider for KomiicSource {
	fn get_image_request(
		&self,
		url: String,
		context: Option<aidoku::PageContext>,
	) -> Result<Request> {
		let referer = context
			.and_then(|value| value.get("referer").cloned())
			.unwrap_or_else(|| String::from(REFERER_URL));
		let mut request = Request::get(url)?;
		request.set_header("User-Agent", USER_AGENT);
		request.set_header("Referer", referer.as_str());
		request.set_header(
			"Accept",
			"image/avif,image/webp,image/apng,image/svg+xml,image/*,*/*;q=0.8",
		);
		if let Some(token) = Self::auth_token() {
			let authorization = format!("Bearer {token}");
			request.set_header("Authorization", authorization.as_str());
		}
		Ok(request)
	}
}

register_source!(
	KomiicSource,
	ListingProvider,
	Home,
	DeepLinkHandler,
	ImageRequestProvider,
	BasicLoginHandler,
	NotificationHandler
);

#[cfg(test)]
mod test;
