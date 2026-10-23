#![no_std]

mod models;

use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider, Listing,
	ListingProvider, Manga, MangaPageResult, Page, PageContent, PageContext, Result, Source,
	alloc::{String, Vec, string::ToString as _},
	helpers::uri::{QueryParameters, encode_uri_component},
	imports::{html::Document, net::Request},
	prelude::*,
};
use models::{ChapterByPageResponse, PageListResponse, PopularResponse};

pub const BASE_URL: &str = "https://m.happymh.com";

struct Happymh;

fn get(url: &str, referer: &str) -> Result<Request> {
	Ok(Request::get(url)?.header("Referer", referer))
}

fn manga_list(query: &str, page: i32) -> Result<MangaPageResult> {
	let referer = format!("{BASE_URL}/latest/{query}");
	let url = format!("{BASE_URL}/apis/c/index?{query}&pn={page}");
	let response = get(&url, &referer)?.json_owned::<PopularResponse>()?;
	Ok(response.data.into())
}

fn chapter_url(manga_key: &str, chapter_key: &str) -> String {
	format!("{BASE_URL}/mangaread/{manga_key}/{chapter_key}")
}

fn text_of(html: &Document, selector: &str) -> Option<String> {
	html.select_first(selector)
		.and_then(|el| el.text())
		.filter(|text| !text.is_empty())
}

impl Source for Happymh {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		if let Some(query) = query.filter(|q| !q.is_empty()) {
			// the search api returns every result at once
			if page > 1 {
				return Ok(MangaPageResult::default());
			}
			let body = format!("searchkey={}&v=v2.13", encode_uri_component(query));
			let response = Request::post(format!("{BASE_URL}/v2.0/apis/manga/ssearch"))?
				.header("Referer", &format!("{BASE_URL}/sssearch"))
				.header("Content-Type", "application/x-www-form-urlencoded")
				.body(body)
				.json_owned::<PopularResponse>()?;
			let mut result: MangaPageResult = response.data.into();
			result.has_next_page = false;
			return Ok(result);
		}

		let mut params = QueryParameters::new();
		let mut has_status = false;
		for filter in filters {
			if let FilterValue::Select { id, value } = filter
				&& !value.is_empty()
			{
				has_status |= id == "series_status";
				params.push(&id, Some(&value));
			}
		}
		if !has_status {
			params.push("series_status", Some("-1"));
		}
		manga_list(&params.to_string(), page)
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let manga_url = format!("{BASE_URL}/manga/{}", manga.key);

		if needs_details {
			let html = get(&manga_url, BASE_URL)?.html()?;
			if let Some(title) = text_of(&html, "div.mg-property > h2.mg-title") {
				manga.title = title;
			}
			if let Some(cover) = html
				.select_first("div.mg-cover > mip-img")
				.and_then(|el| el.attr("abs:src"))
				.filter(|src| !src.is_empty())
			{
				manga.cover = Some(cover);
			}
			let author = text_of(&html, "div.mg-property > p.mg-sub-title:nth-of-type(2)");
			manga.authors = author.map(|author| {
				author
					.split(['/', ',', '，', '、'])
					.map(|s| s.trim().to_string())
					.filter(|s| !s.is_empty())
					.collect()
			});
			manga.tags = html.select("div.mg-property > p.mg-cate > a").map(|els| {
				els.filter_map(|el| el.text())
					.filter(|s| !s.is_empty())
					.collect()
			});
			manga.description = text_of(&html, "div.manga-introduction > mip-showmore#showmore");
			manga.url = Some(manga_url);
		}

		if needs_chapters {
			let mut chapters = Vec::new();
			let mut page = 1;
			loop {
				let url = format!(
					"{BASE_URL}/v2.0/apis/manga/chapterByPage?code={}&lang=cn&order=asc&page={page}",
					encode_uri_component(&manga.key)
				);
				let data = get(&url, &format!("{BASE_URL}/manga/{}", manga.key))?
					.json_owned::<ChapterByPageResponse>()?
					.data;
				let is_end = data.is_last_page();
				chapters.extend(data.items.into_iter().map(|item| {
					let key = item.id.to_string();
					Chapter {
						url: Some(chapter_url(&manga.key, &key)),
						key,
						title: Some(item.chapter_name),
						..Default::default()
					}
				}));
				if is_end {
					break;
				}
				page += 1;
			}
			chapters.reverse();
			manga.chapters = Some(chapters);
		}

		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!(
			"{BASE_URL}/v2.0/apis/manga/reading?code={}&cid={}&v=v3.1919111",
			encode_uri_component(&manga.key),
			encode_uri_component(&chapter.key)
		);
		let response = get(&url, &chapter_url(&manga.key, &chapter.key))?
			.header("X-Requested-With", "XMLHttpRequest")
			.json_owned::<PageListResponse>()?;
		Ok(response
			.data
			.scans
			.into_iter()
			// n != 0 entries are ads
			.filter(|scan| scan.n == 0)
			.map(|scan| {
				// the resize service rejects images larger than 16383px
				let url = if scan.width > 16383 || scan.height > 16383 {
					scan.url
						.split_once("?q=")
						.map(|(url, _)| url.into())
						.unwrap_or(scan.url)
				} else {
					scan.url
				};
				Page {
					content: PageContent::url(url),
					..Default::default()
				}
			})
			.collect())
	}
}

impl ListingProvider for Happymh {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let order = match listing.id.as_str() {
			"last_date" => "last_date",
			_ => "views",
		};
		let referer = format!("{BASE_URL}/latest");
		let url = format!("{BASE_URL}/apis/c/index?pn={page}&series_status=-1&order={order}");
		let response = get(&url, &referer)?.json_owned::<PopularResponse>()?;
		Ok(response.data.into())
	}
}

impl ImageRequestProvider for Happymh {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for Happymh {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(path) = url.strip_prefix(BASE_URL) else {
			return Ok(None);
		};
		let path = path.split(['?', '#']).next().unwrap_or_default();
		let mut segments = path.split('/').filter(|s| !s.is_empty());
		Ok(match (segments.next(), segments.next(), segments.next()) {
			(Some("manga"), Some(key), None) => Some(DeepLinkResult::Manga { key: key.into() }),
			(Some("mangaread"), Some(manga_key), Some(key)) => Some(DeepLinkResult::Chapter {
				manga_key: manga_key.into(),
				key: key.into(),
			}),
			_ => None,
		})
	}
}

register_source!(
	Happymh,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
