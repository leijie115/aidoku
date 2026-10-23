#![no_std]

mod html;
mod net;

use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider, Listing,
	ListingProvider, Manga, MangaPageResult, Page, Result, Source,
	alloc::{String, Vec},
	imports::{defaults::defaults_get, net::Request},
	prelude::*,
};
use html::{ChapterPage as _, MangaPage as _};
use net::Url;

pub const BASE_URL: &str = "https://bakamh.com";

/// Returns the domain set in the settings, or the default one.
pub fn base_url() -> String {
	defaults_get::<String>("url")
		.map(|url| url.trim().trim_end_matches('/').into())
		.filter(|url: &String| url.starts_with("http"))
		.unwrap_or_else(|| BASE_URL.into())
}

/// Creates a GET request with the headers the site expects.
pub fn get<T: AsRef<str>>(url: T) -> Result<Request> {
	Ok(Request::get(url)?
		.header("Accept-Language", "zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7")
		.header("Referer", &format!("{}/", base_url())))
}

struct Bakamh;

impl Source for Bakamh {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let is_search = query.is_some();
		let url = Url::from_query_or_filters(query.as_deref(), page, &filters)?;
		let html = url.request()?.html()?;
		html.manga_page_result(is_search)
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let url = format!("{}/manga/{}/", base_url(), manga.key);
		let html = get(url)?.html()?;

		if needs_details {
			html.update_details(&mut manga)?;
		}

		if needs_chapters {
			manga.chapters = Some(html.chapters(&manga.key)?);
		}

		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		html::get_page_list(&chapter.key)
	}
}

impl ListingProvider for Bakamh {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let url = match listing.id.as_str() {
			"latest" | "rating" | "views" | "new-manga" => format!(
				"{}/page/{}/?s&post_type=wp-manga&m_orderby={}",
				base_url(),
				page,
				listing.id
			),
			_ => bail!("Invalid listing"),
		};
		let html = get(url)?.html()?;
		html.manga_page_result(true)
	}
}

impl ImageRequestProvider for Bakamh {
	fn get_image_request(
		&self,
		url: String,
		_context: Option<aidoku::PageContext>,
	) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{}/", base_url())))
	}
}

impl DeepLinkHandler for Bakamh {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let base_url = base_url();
		let Some(path) = url
			.strip_prefix(&base_url)
			.or_else(|| url.strip_prefix(BASE_URL))
		else {
			return Ok(None);
		};
		let path = path.trim_start_matches('/');
		let mut parts = path.split('/').filter(|s| !s.is_empty());
		let result = match (parts.next(), parts.next(), parts.next()) {
			(Some("manga"), Some(manga_key), None) => Some(DeepLinkResult::Manga {
				key: manga_key.into(),
			}),
			(Some("manga"), Some(manga_key), Some(chapter_slug)) => Some(DeepLinkResult::Chapter {
				manga_key: manga_key.into(),
				key: format!("/manga/{}/{}/", manga_key, chapter_slug),
			}),
			_ => None,
		};
		Ok(result)
	}
}

register_source!(
	Bakamh,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
