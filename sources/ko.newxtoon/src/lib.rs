#![no_std]

use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider,
	Listing, ListingProvider, Manga, MangaPageResult, MangaStatus, Page, PageContent, PageContext,
	Result, Source, Viewer,
	alloc::{String, Vec, string::ToString as _},
	helpers::uri::QueryParameters,
	imports::{
		defaults::defaults_get,
		html::{Document, Element},
		net::Request,
		std::parse_date,
	},
	prelude::*,
};
use serde::Deserialize;

const DEFAULT_BASE_URL: &str = "https://newxtoon3.com";
const DEFAULT_CATEGORY: &str = "성인";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

struct Newxtoon;

/// Reduce a url to its `scheme://host` origin.
fn origin(url: &str) -> Option<String> {
	let url = url.trim();
	let scheme_end = url.find("://")? + 3;
	let host_end = url[scheme_end..]
		.find('/')
		.map_or(url.len(), |i| scheme_end + i);
	(host_end > scheme_end).then(|| url[..host_end].into())
}

impl Newxtoon {
	fn base_url(&self) -> String {
		// the site rotates domains, so allow overriding it in the settings
		defaults_get::<String>("baseUrl")
			.as_deref()
			.filter(|url| url.trim().starts_with("http"))
			.and_then(origin)
			.unwrap_or_else(|| DEFAULT_BASE_URL.into())
	}

	fn request(&self, url: &str, referer: &str) -> Result<Request> {
		Ok(Request::get(url)?
			.header("User-Agent", USER_AGENT)
			.header("Referer", referer))
	}

	fn get_html(&self, url: &str) -> Result<Document> {
		let referer = format!("{}/", self.base_url());
		Ok(self.request(url, &referer)?.html()?)
	}

	fn comics_url(&self, page: i32, params: &[(&str, &str)]) -> String {
		let mut qs = QueryParameters::new();
		for (key, value) in params {
			if !value.is_empty() {
				qs.push(key, Some(value));
			}
		}
		qs.push("page", Some(&page.to_string()));
		format!("{}/comics?{qs}", self.base_url())
	}

	fn parse_manga_list(&self, url: &str) -> Result<MangaPageResult> {
		let html = self.get_html(url)?;
		let entries = html
			.select("a.comic-link[href*=\"/comics/\"]")
			.map(|els| els.filter_map(parse_manga_item).collect())
			.unwrap_or_default();
		let has_next_page = html.select_first("a[aria-label=\"다음 페이지\"]").is_some();
		Ok(MangaPageResult {
			entries,
			has_next_page,
		})
	}
}

/// Extract the comic id from a `/comics/{id}` url.
fn comic_id(href: &str) -> Option<String> {
	let rest = &href[href.find("/comics/")? + 8..];
	let id = rest.split(['/', '?', '#']).next()?;
	(!id.is_empty()).then(|| id.into())
}

fn parse_manga_item(el: Element) -> Option<Manga> {
	let key = comic_id(&el.attr("href")?)?;
	let title = el
		.select_first("h3[title]")
		.and_then(|h| h.attr("title"))
		.filter(|t| !t.trim().is_empty())
		.or_else(|| el.select_first("h3").and_then(|h| h.text()))?;
	let cover = el
		.select_first("img.cover-image")
		.and_then(|img| img.attr("abs:src"));
	Some(Manga {
		key,
		title,
		cover,
		..Default::default()
	})
}

fn parse_status(text: &str) -> Option<MangaStatus> {
	match text {
		"연재중" | "연재" => Some(MangaStatus::Ongoing),
		"완결" => Some(MangaStatus::Completed),
		"휴재" | "휴재중" => Some(MangaStatus::Hiatus),
		_ => None,
	}
}

/// Parse the number from titles such as `제215화 ...` or `12화`.
fn parse_chapter_number(title: &str) -> Option<f32> {
	let end = title.find('화')?;
	let digits = title[..end]
		.trim_end()
		.rsplit(|c: char| !(c.is_ascii_digit() || c == '.'))
		.next()?;
	digits.parse().ok()
}

#[derive(Deserialize)]
struct ChapterPageDto {
	#[serde(default)]
	chapters: Vec<ChapterDto>,
	next_page: Option<i32>,
	#[serde(default)]
	has_more: bool,
}

#[derive(Deserialize)]
struct ChapterDto {
	id: i64,
	#[serde(default)]
	title: String,
	date: Option<String>,
}

impl Source for Newxtoon {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		if let Some(query) = query.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
			let mut qs = QueryParameters::new();
			qs.push("q", Some(query));
			qs.push("page", Some(&page.to_string()));
			let url = format!("{}/search?{qs}", self.base_url());
			return self.parse_manga_list(&url);
		}

		let mut category = DEFAULT_CATEGORY;
		let mut sort = "";
		let mut status = "";
		let mut genre = "";
		let mut platform = "";
		let mut weekday = "";
		for filter in &filters {
			if let FilterValue::Select { id, value } = filter {
				let value = value.as_str();
				match id.as_str() {
					"category" => category = value,
					"sort" => sort = value,
					"status" => status = value,
					"genre" => genre = value,
					"platform" => platform = value,
					"weekday" => weekday = value,
					_ => {}
				}
			}
		}
		let url = self.comics_url(
			page,
			&[
				("category", category),
				("sort", sort),
				("status", status),
				("genre", genre),
				("platform", platform),
				("weekday", weekday),
			],
		);
		self.parse_manga_list(&url)
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let base_url = self.base_url();
		let manga_url = format!("{base_url}/comics/{}", manga.key);

		if needs_details {
			let html = self.get_html(&manga_url)?;
			let root = html
				.select_first("section[aria-labelledby=\"comic-title\"]")
				.or_else(|| html.select_first("body"))
				.ok_or_else(|| error!("Missing comic section"))?;

			if let Some(title) = root.select_first("h1#comic-title").and_then(|e| e.text()) {
				manga.title = title;
			}
			let authors: Vec<String> = root
				.select("h1#comic-title + p a")
				.map(|els| {
					els.filter_map(|a| a.text())
						.filter(|t| !t.trim().is_empty())
						.collect()
				})
				.unwrap_or_default();
			manga.authors = (!authors.is_empty()).then_some(authors);
			manga.description = root
				.select_first("p[data-comic-description]")
				.and_then(|p| p.untrimmed_text())
				.map(|t| t.trim().into());
			let mut tags: Vec<String> = Vec::new();
			if let Some(links) = root.select("a[href*=\"category=\"]") {
				for link in links {
					if let Some(text) = link.text() {
						let tag = text.trim_start_matches('#').trim();
						if !tag.is_empty() && !tags.iter().any(|t| t == tag) {
							tags.push(tag.into());
						}
					}
				}
			}
			manga.status = root
				.select("strong")
				.and_then(|els| {
					els.filter_map(|s| s.text())
						.find_map(|t| parse_status(t.trim()))
				})
				.unwrap_or_default();
			manga.cover = root
				.select_first("img[alt$=\"표지\"]")
				.and_then(|img| img.attr("abs:src"))
				.or_else(|| {
					html.select_first("meta[property=\"og:image\"]")
						.and_then(|m| m.attr("content"))
				})
				.or(manga.cover);
			let adult = root.select_first("img[alt^=\"19세\"]").is_some()
				|| tags.iter().any(|t| t == "성인");
			manga.content_rating = if adult {
				ContentRating::NSFW
			} else {
				ContentRating::Safe
			};
			manga.tags = (!tags.is_empty()).then_some(tags);
			manga.viewer = Viewer::Webtoon;
			manga.url = Some(manga_url.clone());
		}

		if needs_chapters {
			let mut chapters: Vec<Chapter> = Vec::new();
			let mut page = 1;
			loop {
				let mut qs = QueryParameters::new();
				qs.push("sort", Some("latest"));
				qs.push("page", Some(&page.to_string()));
				qs.push("mode", Some("desktop"));
				let url = format!("{manga_url}/chapters?{qs}");
				let result = self
					.request(&url, &manga_url)?
					.header("Accept", "application/json")
					.json_owned::<ChapterPageDto>()?;
				let is_empty = result.chapters.is_empty();
				for dto in result.chapters {
					let key = dto.id.to_string();
					if chapters.iter().any(|c| c.key == key) {
						continue;
					}
					let title = dto.title.trim();
					chapters.push(Chapter {
						chapter_number: parse_chapter_number(title),
						title: (!title.is_empty()).then(|| title.into()),
						date_uploaded: dto.date.as_deref().and_then(|d| {
							// explicit time so that strict parsers accept it
							parse_date(format!("{} 00:00", d.trim()), "yyyy.MM.dd HH:mm")
						}),
						url: Some(format!("{manga_url}/chapters/{key}")),
						key,
						..Default::default()
					});
				}
				match result.next_page {
					Some(next) if result.has_more && next > page && !is_empty => page = next,
					_ => break,
				}
			}
			manga.chapters = Some(chapters);
		}

		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let base_url = self.base_url();
		let manga_url = format!("{base_url}/comics/{}", manga.key);
		let url = format!("{manga_url}/chapters/{}", chapter.key);
		let html = self.request(&url, &manga_url)?.html()?;
		let pages = html
			.select("img.reader-page[data-reader-image]")
			.map(|els| {
				els.filter_map(|img| {
					img.attr("abs:src")
						.filter(|s| !s.trim().is_empty())
						.or_else(|| img.attr("abs:data-src"))
						.filter(|s| !s.trim().is_empty())
				})
				.map(|src| Page {
					content: PageContent::url(src),
					..Default::default()
				})
				.collect()
			})
			.unwrap_or_default();
		Ok(pages)
	}
}

impl ListingProvider for Newxtoon {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let sort = if listing.id == "popular" {
			"popular"
		} else {
			""
		};
		let url = self.comics_url(page, &[("category", DEFAULT_CATEGORY), ("sort", sort)]);
		self.parse_manga_list(&url)
	}
}

impl ImageRequestProvider for Newxtoon {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		let referer = format!("{}/", self.base_url());
		self.request(&url, &referer)
	}
}

impl DeepLinkHandler for Newxtoon {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(manga_key) = comic_id(&url) else {
			return Ok(None);
		};
		let rest = &url[url.find("/chapters/").map_or(url.len(), |i| i + 10)..];
		let chapter_key = rest.split(['/', '?', '#']).next().unwrap_or_default();
		if chapter_key.is_empty() {
			Ok(Some(DeepLinkResult::Manga { key: manga_key }))
		} else {
			Ok(Some(DeepLinkResult::Chapter {
				manga_key,
				key: chapter_key.into(),
			}))
		}
	}
}

register_source!(
	Newxtoon,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
