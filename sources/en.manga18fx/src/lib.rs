#![no_std]
use aidoku::{
	ContentRating, FilterValue, Listing, Manga, MangaPageResult, Result, Source, Viewer,
	alloc::{String, Vec},
	helpers::{string::StripPrefixOrSelf, uri::QueryParameters},
	imports::{
		html::{Document, Element},
		net::Request,
	},
	prelude::*,
};
use madara::{Impl, Madara, Params};

const BASE_URL: &str = "https://manga18fx.com";

struct Manga18fx;

impl Manga18fx {
	fn parse_manga(params: &Params, element: &Element) -> Option<Manga> {
		let key: String = element
			.attr("abs:href")?
			.strip_prefix_or_self(&params.base_url)
			.trim_end_matches('/')
			.into();
		let title = element
			.attr("title")
			.or_else(|| element.text())?
			.trim()
			.into();
		let cover = element
			.select_first("img")
			.and_then(|img| img.attr("abs:data-src").or_else(|| img.attr("abs:src")));
		Some(Manga {
			url: Some(format!("{}{key}", params.base_url)),
			key,
			title,
			cover,
			content_rating: ContentRating::NSFW,
			viewer: Viewer::Webtoon,
			..Default::default()
		})
	}

	// parses a page with a grid of manga (latest updates, search, genre)
	fn parse_manga_grid(params: &Params, html: &Document) -> MangaPageResult {
		let entries = html
			.select(".bsx-item")
			.map(|els| {
				els.filter_map(|el| Self::parse_manga(params, &el.select_first("a")?))
					.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		let has_next_page = html
			.select_first(".pagination li.next")
			.is_some_and(|el| !el.has_class("disabled"));
		MangaPageResult {
			entries,
			has_next_page,
		}
	}
}

impl Impl for Manga18fx {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			default_viewer: Viewer::Webtoon,
			datetime_format: "dd MMM yy".into(),
			details_description_selector: ".dsct".into(),
			chapter_selector: "ul.row-content-chapter > li".into(),
			chapter_date_selector: "span.chapter-time".into(),
			..Default::default()
		}
	}

	fn get_search_manga_list(
		&self,
		params: &Params,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let url = if let Some(query) = query {
			let mut qs = QueryParameters::new();
			qs.push("q", Some(&query));
			qs.push("page", Some(&format!("{page}")));
			format!("{}/search?{qs}", params.base_url)
		} else {
			let mut genre = String::new();
			let mut orderby = String::new();
			for filter in filters {
				if let FilterValue::Select { id, value } = filter {
					match id.as_str() {
						"genre" => genre = value,
						"orderby" => orderby = value,
						_ => {}
					}
				}
			}
			if genre.is_empty() {
				format!("{}/page/{page}", params.base_url)
			} else if orderby.is_empty() {
				format!("{}/manga-genre/{genre}/{page}", params.base_url)
			} else {
				format!(
					"{}/manga-genre/{genre}/{page}?orderby={orderby}",
					params.base_url
				)
			}
		};
		let html = Request::get(url)?.html()?;
		Ok(Self::parse_manga_grid(params, &html))
	}

	fn get_manga_list(
		&self,
		params: &Params,
		listing: Listing,
		page: i32,
	) -> Result<MangaPageResult> {
		match listing.id.as_str() {
			"trending" => {
				let html = Request::get(params.base_url.as_ref())?.html()?;
				let entries = html
					.select(".trending-block a")
					.map(|els| {
						els.filter_map(|el| Self::parse_manga(params, &el))
							.collect::<Vec<_>>()
					})
					.unwrap_or_default();
				Ok(MangaPageResult {
					entries,
					has_next_page: false,
				})
			}
			"latest" => {
				let html = Request::get(format!("{}/page/{page}", params.base_url))?.html()?;
				Ok(Self::parse_manga_grid(params, &html))
			}
			_ => bail!("Invalid listing"),
		}
	}

	fn get_manga_content_rating(&self, _html: &Document, _manga: &Manga) -> ContentRating {
		ContentRating::NSFW
	}
}

register_source!(
	Madara<Manga18fx>,
	ListingProvider,
	DeepLinkHandler,
	ImageRequestProvider
);

#[cfg(test)]
mod test;
