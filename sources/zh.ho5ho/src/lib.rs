#![no_std]
use aidoku::{
	Chapter, ContentRating, DeepLinkResult, Manga, Page, Result, Source,
	alloc::{String, Vec},
	imports::{html::Document, net::Request},
	prelude::*,
};
use madara::{Impl, LoadMoreStrategy, Madara, Params};

const BASE_URL: &str = "https://www.ho5ho.com";

struct Ho5Ho;

impl Impl for Ho5Ho {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			source_path: "中字h漫".into(),
			use_new_chapter_endpoint: true,
			use_load_more_request: LoadMoreStrategy::Never,
			datetime_format: "MMMM d, yyyy".into(),
			search_manga_selector: "article.ho5ho-v2-card".into(),
			search_manga_url_selector: "h3.ho5ho-v2-card-title a".into(),
			search_manga_title_selector: "h3.ho5ho-v2-card-title a".into(),
			search_manga_cover_selector: "img.ho5ho-v2-card-image".into(),
			details_title_selector: "#ho5ho-detail-title".into(),
			details_cover_selector: "a.ho5ho-v3-cover img".into(),
			details_author_selector:
				"section.ho5ho-v3-taxonomy-group:has(a[href*='/h漫作者/']) .ho5ho-v3-term-chips a"
					.into(),
			details_artist_selector: "".into(),
			details_description_selector: "meta[name=description]".into(),
			details_tag_selector:
				"section.ho5ho-v3-taxonomy-group .ho5ho-v3-term-chips a[href*='/h漫分類/'], \
				 section.ho5ho-v3-taxonomy-group .ho5ho-v3-term-chips a[href*='/h漫標籤/']"
					.into(),
			details_status_selector: "".into(),
			details_type_selector: "".into(),
			page_list_selector: ".reading-content img.wp-manga-chapter-img".into(),
			..Default::default()
		}
	}

	fn get_manga_content_rating(&self, _html: &Document, _manga: &Manga) -> ContentRating {
		ContentRating::NSFW
	}

	fn get_page_list(&self, params: &Params, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		// chapters are paginated by default, the list style shows every image on one page
		let url = format!("{}{}", params.base_url, chapter.key);
		let list_url = format!("{url}?style=list");
		let html = Request::get(&list_url)?.html()?;
		self.parse_page_list(params, &html, url)
	}

	fn handle_deep_link(&self, params: &Params, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(path) = url.strip_prefix(params.base_url.as_ref()) else {
			return Ok(None);
		};
		let path = path.split(['?', '#']).next().unwrap_or_default();
		let mut parts = path.split('/').filter(|s| !s.is_empty());
		// urls look like /中字h漫/<manga>/<chapter>/
		Ok(match (parts.next(), parts.next(), parts.next()) {
			(Some(category), Some(manga), None) => Some(DeepLinkResult::Manga {
				key: format!("/{category}/{manga}/"),
			}),
			(Some(category), Some(manga), Some(chapter)) => Some(DeepLinkResult::Chapter {
				manga_key: format!("/{category}/{manga}/"),
				key: format!("/{category}/{manga}/{chapter}/"),
			}),
			_ => None,
		})
	}
}

register_source!(Madara<Ho5Ho>, DeepLinkHandler, ImageRequestProvider);

#[cfg(test)]
mod test;
