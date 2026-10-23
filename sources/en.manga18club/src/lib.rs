#![no_std]

use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider,
	Listing, ListingProvider, Manga, MangaPageResult, MangaStatus, Page, PageContent, PageContext,
	Result, Source, Viewer,
	alloc::{String, Vec, vec},
	helpers::uri::QueryParameters,
	imports::{
		html::{Document, Element},
		net::Request,
		std::parse_date,
	},
	prelude::*,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};

const BASE_URL: &str = "https://manga18.club";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// Genre names and their url slugs, taken from the site's genre menu.
const GENRES: &[(&str, &str)] = &[
	("18+", "18"),
	("Action", "action"),
	("Adult", "adult"),
	("Adventure", "Adventure"),
	("Anime", "anime"),
	("Comedy", "comedy"),
	("Comic", "comic"),
	("Doujinshi", "doujinshi"),
	("Drama", "drama"),
	("Ecchi", "Ecchi"),
	("Fantasy", "Fantasy"),
	("Gender Bender", "Gender-Bender"),
	("Harem", "Harem"),
	("Historical", "historical"),
	("Horror", "Horror"),
	("Josei", "Josei"),
	("Live action", "live-action"),
	("Manhua", "Manhua"),
	("Manhwa", "Manhwa"),
	("Martial Art", "Martial-art"),
	("Mature", "Mature"),
	("Mecha", "mecha"),
	("Mystery", "mystery"),
	("One shot", "one-shot"),
	("Psychological", "psychological"),
	("Raw", "raw"),
	("Romance", "romance"),
	("School Life", "school-life"),
	("Sci-fi", "sci-fi"),
	("Seinen", "seinen"),
	("Shojou Ai", "shojou-ai"),
	("Shoujo", "Shoujo"),
	("Shounen", "Shounen"),
	("Shounen Ai", "shounen-ai"),
	("Slice of Life", "slice-of-life"),
	("Smut", "Smut"),
	("Sports", "Sports"),
	("Supernatural", "supernatural"),
	("Tragedy", "Tragedy"),
	("Uncensored", "Uncensored"),
	("Yaoi", "Yaoi"),
];

struct Manga18Club;

fn request(url: &str) -> Result<Request> {
	Ok(Request::get(url)?
		.header("User-Agent", USER_AGENT)
		.header("Referer", &format!("{BASE_URL}/")))
}

/// Strip the scheme and host from a url, returning the path.
fn path_of(url: &str) -> &str {
	match url.find("://") {
		Some(pos) => {
			let rest = &url[pos + 3..];
			rest.find('/').map_or("", |i| &rest[i..])
		}
		None => url,
	}
}

/// Map a genre filter value (slug or display name from a tag search) to a slug.
fn genre_slug(value: &str) -> String {
	GENRES
		.iter()
		.find(|(name, slug)| *slug == value || name.eq_ignore_ascii_case(value))
		.map(|(_, slug)| (*slug).into())
		.unwrap_or_else(|| value.trim().replace(' ', "-"))
}

fn parse_manga_list(html: &Document) -> MangaPageResult {
	let entries = html
		.select("div.story_item")
		.map(|items| items.filter_map(parse_manga_item).collect())
		.unwrap_or_default();
	let has_next_page = html
		.select_first(".pagination li.active + li a, .pagination a[rel=next]")
		.is_some();
	MangaPageResult {
		entries,
		has_next_page,
	}
}

fn parse_manga_item(el: Element) -> Option<Manga> {
	let link = el.select_first("a")?;
	let key = path_of(&link.attr("abs:href")?).into();
	let title = el
		.select_first("div.mg_info > div.mg_name a")
		.and_then(|a| a.text())
		.or_else(|| link.attr("title"))?;
	let cover = el.select_first("img").and_then(|img| img.attr("abs:src"));
	Some(Manga {
		key,
		title,
		cover,
		..Default::default()
	})
}

/// Value of an info row whose label contains one of `labels` (case-insensitive).
fn info_value(info: &Element, labels: &[&str]) -> Option<Element> {
	info.select("div.item")?.find_map(|item| {
		let label = item.select_first("div.info_label")?.text()?.to_lowercase();
		if labels.iter().any(|l| label.contains(l)) {
			item.select_first("div.info_value")
		} else {
			None
		}
	})
}

fn info_text(info: &Element, labels: &[&str]) -> Option<String> {
	info_value(info, labels)
		.and_then(|v| v.text())
		.filter(|t| !t.is_empty() && t != "Updating")
}

fn parse_details(html: &Document, mut manga: Manga) -> Result<Manga> {
	let info = html
		.select_first("div.detail_listInfo")
		.ok_or_else(|| error!("Missing manga info"))?;

	if let Some(title) = html
		.select_first("div.detail_name > h1")
		.and_then(|e| e.text())
	{
		manga.title = title;
	}
	manga.cover = html
		.select_first("div.detail_avatar > img")
		.and_then(|img| img.attr("abs:src"))
		.or(manga.cover);

	let mut description: Vec<String> = html
		.select("div.detail_reviewContent")
		.map(|els| els.filter_map(|e| e.text()).collect())
		.unwrap_or_default();
	if let Some(alt) = info_text(&info, &["other name"]) {
		description.push(format!("Alternative Names:\n{alt}"));
	}
	manga.description = (!description.is_empty()).then(|| description.join("\n\n"));

	manga.authors = info_text(&info, &["author", "autor"]).map(|a| vec![a]);
	manga.artists = info_text(&info, &["artist"]).map(|a| vec![a]);
	manga.status = match info_text(&info, &["status"]).as_deref() {
		Some("On Going" | "Ongoing") => MangaStatus::Ongoing,
		Some("Completed") => MangaStatus::Completed,
		_ => MangaStatus::Unknown,
	};
	let tags: Vec<String> = info
		.select("div.info_value > a[href*=\"/manga-list/\"]")
		.map(|els| els.filter_map(|a| a.text()).collect())
		.unwrap_or_default();
	manga.tags = (!tags.is_empty()).then_some(tags);
	manga.content_rating = ContentRating::NSFW;
	manga.viewer = Viewer::Webtoon;
	manga.url = Some(format!("{BASE_URL}{}", manga.key));
	Ok(manga)
}

fn parse_chapters(html: &Document) -> Vec<Chapter> {
	html.select("div.chapter_box .item")
		.map(|items| {
			items
				.filter_map(|item| {
					let link = item.select_first("a")?;
					let chapter_url = link.attr("abs:href")?;
					let title = link.text();
					let date_uploaded = item
						.select_first("p")
						.and_then(|p| p.text())
						// explicit time so that strict parsers accept it
						.and_then(|d| parse_date(format!("{d} 00:00"), "dd-MM-yyyy HH:mm"));
					Some(Chapter {
						key: path_of(&chapter_url).into(),
						chapter_number: title.as_deref().and_then(parse_chapter_number),
						title,
						date_uploaded,
						url: Some(chapter_url),
						..Default::default()
					})
				})
				.collect()
		})
		.unwrap_or_default()
}

/// Parse the number from titles such as `Chapter 47` or `Chapter 12.5 - Title`.
fn parse_chapter_number(title: &str) -> Option<f32> {
	let lower = title.to_lowercase();
	let start = lower.find("chapter")? + 7;
	let rest = lower[start..].trim_start();
	let end = rest
		.find(|c: char| !(c.is_ascii_digit() || c == '.'))
		.unwrap_or(rest.len());
	rest[..end].trim_end_matches('.').parse().ok()
}

/// Decode the base64 image urls listed in the `slides_p_path` script.
fn parse_image_urls(script: &str) -> Vec<String> {
	let Some(start) = script.find('[') else {
		return Vec::new();
	};
	let data = &script[start + 1..];
	let data = data
		.find(",]")
		.or_else(|| data.find(']'))
		.map_or(data, |end| &data[..end]);
	data.replace(['"', '\''], "")
		.split(',')
		.map(str::trim)
		.filter(|s| !s.is_empty())
		.filter_map(|s| STANDARD.decode(s).ok())
		.filter_map(|bytes| String::from_utf8(bytes).ok())
		.map(|url| {
			if url.starts_with('/') {
				format!("{BASE_URL}{url}")
			} else {
				url
			}
		})
		.collect()
}

impl Source for Manga18Club {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut sort = "";
		let mut genre = String::new();
		for filter in &filters {
			if let FilterValue::Select { id, value } = filter {
				match id.as_str() {
					"sort" => sort = value.as_str(),
					"genre" => genre = genre_slug(value),
					_ => {}
				}
			}
		}

		let mut qs = QueryParameters::new();
		let url = match query.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
			Some(query) => {
				qs.push("search", Some(query));
				format!("{BASE_URL}/list-manga/{page}?{qs}")
			}
			None => {
				if !sort.is_empty() {
					qs.push("order_by", Some(sort));
				}
				if genre.is_empty() {
					format!("{BASE_URL}/list-manga/{page}?{qs}")
				} else {
					format!("{BASE_URL}/manga-list/{genre}/{page}?{qs}")
				}
			}
		};
		Ok(parse_manga_list(&request(&url)?.html()?))
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let html = request(&format!("{BASE_URL}{}", manga.key))?.html()?;
		if needs_details {
			manga = parse_details(&html, manga)?;
		}
		if needs_chapters {
			manga.chapters = Some(parse_chapters(&html));
		}
		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{BASE_URL}{}", chapter.key);
		let html = request(&url)?.html()?;
		let script = html
			.select("script")
			.and_then(|scripts| {
				scripts
					.filter_map(|s| s.data())
					.find(|data| data.contains("slides_p_path"))
			})
			.ok_or_else(|| error!("Unable to find script with image data"))?;
		Ok(parse_image_urls(&script)
			.into_iter()
			.map(|url| Page {
				content: PageContent::url(url),
				..Default::default()
			})
			.collect())
	}
}

impl ListingProvider for Manga18Club {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let url = if listing.id == "popular" {
			format!("{BASE_URL}/list-manga/{page}?order_by=views")
		} else {
			format!("{BASE_URL}/list-manga/{page}")
		};
		Ok(parse_manga_list(&request(&url)?.html()?))
	}
}

impl ImageRequestProvider for Manga18Club {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		request(&url)
	}
}

impl DeepLinkHandler for Manga18Club {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let path = path_of(&url);
		let path = path.split(['?', '#']).next().unwrap_or_default();
		let mut segments = path.split('/').filter(|s| !s.is_empty());
		let (Some(kind), Some(slug)) = (segments.next(), segments.next()) else {
			return Ok(None);
		};
		if kind == "list-manga" || kind == "manga-list" {
			return Ok(None);
		}
		let manga_key = format!("/{kind}/{slug}");
		Ok(Some(match segments.next() {
			Some(chapter) => DeepLinkResult::Chapter {
				key: format!("{manga_key}/{chapter}"),
				manga_key,
			},
			None => DeepLinkResult::Manga { key: manga_key },
		}))
	}
}

register_source!(
	Manga18Club,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
