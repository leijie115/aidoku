#![no_std]

mod html;
mod net;
#[cfg(test)]
mod test;

use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, ImageRequestProvider, Listing, ListingProvider,
	Manga, MangaPageResult, Page, Result, Source,
	alloc::{String, Vec},
	imports::{defaults::defaults_get, net::Request},
	prelude::*,
};
use html::{ChapterPage as _, MangaPage as _, PageList as _};
use net::Url;

pub const BASE_URL: &str = "https://mycomic.com";

/// Base URL for page requests, honoring the preferred language setting
/// (traditional Chinese at `/`, simplified Chinese at `/cn`).
pub fn request_url() -> String {
	if defaults_get::<String>("lang").is_some_and(|lang| lang == "zh-hans") {
		format!("{BASE_URL}/cn")
	} else {
		BASE_URL.into()
	}
}

struct Mycomic;

impl Source for Mycomic {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<aidoku::FilterValue>,
	) -> Result<MangaPageResult> {
		let url = Url::from_query_or_filters(query.as_deref(), page, &filters)?;
		let html = url.request()?.html()?;
		html.manga_page_result()
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		if needs_details {
			let manga_page = Url::manga(manga.key.clone()).request()?.html()?;
			manga_page.update_details(&mut manga)?;
		}

		if needs_chapters {
			let chapter_list_page = Url::chapter_list(manga.key.clone()).request()?.html()?;
			manga.chapters = Some(chapter_list_page.chapters(&manga.key)?);
		}

		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		Url::chapter(chapter.key).request()?.html()?.pages()
	}
}

impl ListingProvider for Mycomic {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let url = Url::from_listing(&listing.id, page)
			.ok_or_else(|| error!("Unknown listing: {}", listing.id))?;
		if url.is_rank() {
			if page > 1 {
				return Ok(MangaPageResult::default());
			}
			url.request()?.html()?.rank_page_result()
		} else {
			url.request()?.html()?.manga_page_result()
		}
	}
}

impl ImageRequestProvider for Mycomic {
	fn get_image_request(
		&self,
		url: String,
		_context: Option<aidoku::PageContext>,
	) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for Mycomic {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(url) = url
			.strip_prefix(BASE_URL)
			.or_else(|| url.strip_prefix("https://www.mycomic.com"))
		else {
			return Ok(None);
		};
		let url = url.strip_prefix("/cn").unwrap_or(url);
		let url = url.split(['?', '#']).next().unwrap_or_default();
		let mut splits = url.split('/').skip(1);
		let deep_link_result = match splits.next() {
			Some("comics") => splits
				.next()
				.map(|id| DeepLinkResult::Manga { key: id.into() }),
			Some("chapters") => match (splits.next(), splits.next()) {
				(Some(id), None) => Some(DeepLinkResult::Chapter {
					manga_key: "".into(),
					key: id.into(),
				}),
				_ => None,
			},
			_ => None,
		};
		Ok(deep_link_result)
	}
}

register_source!(
	Mycomic,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);
