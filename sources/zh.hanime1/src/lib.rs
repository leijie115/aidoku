#![no_std]

use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, Listing, ListingProvider,
	Manga, MangaPageResult, Page, PageContent, Result, Source,
	alloc::{String, Vec, string::ToString as _, vec},
	helpers::uri::QueryParameters,
	imports::{
		html::{Document, Element},
		net::Request,
	},
	prelude::*,
};

const BASE_URL: &str = "https://hanime1.me";

const NEXT_PAGE_SELECTOR: &str = "ul.pagination a[rel=next]";

struct Hanime1;

fn request(url: &str) -> Result<Document> {
	Ok(Request::get(url)?.html()?)
}

/// Extracts the numeric comic id from `https://hanime1.me/comic/{id}[/...]`.
fn comic_id(href: &str) -> Option<String> {
	let path = href.strip_prefix(BASE_URL).unwrap_or(href);
	let id = path
		.strip_prefix("/comic/")?
		.split(['/', '?', '#'])
		.next()?;
	(!id.is_empty()).then(|| id.into())
}

/// The cover is lazy loaded; the real url is the first entry of `data-srcset`.
fn image_src(img: &Element) -> Option<String> {
	img.attr("data-srcset")
		.filter(|s| !s.is_empty())
		.or_else(|| img.attr("data-src"))
		.or_else(|| img.attr("src"))
		.and_then(|s| s.split(',').next().map(|s| s.trim().into()))
		.filter(|s: &String| !s.is_empty())
}

/// Collapses the whitespace left over from the page's indentation.
fn clean_text(element: &Element) -> Option<String> {
	let text = element.text()?;
	let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
	(!text.is_empty()).then_some(text)
}

fn parse_comic_div(element: &Element) -> Option<Manga> {
	let key = element
		.select_first("a")
		.and_then(|a| a.attr("href"))
		.and_then(|href| comic_id(&href))?;
	let title = element
		.select_first("div.comic-rows-videos-title")
		.and_then(|el| clean_text(&el))
		.unwrap_or_default();
	Some(Manga {
		url: Some(format!("{BASE_URL}/comic/{key}")),
		key,
		title,
		cover: element.select_first("img").and_then(|img| image_src(&img)),
		content_rating: ContentRating::NSFW,
		..Default::default()
	})
}

/// Finds the `div.comic-rows-videos-div` items in the section titled `heading`.
fn section_items(html: &Document, heading: &str) -> Vec<Element> {
	html.select("div.comic-rows-wrapper")
		.map(|wrappers| {
			wrappers
				.filter(|wrapper| {
					wrapper
						.select_first("h3")
						.and_then(|h3| h3.text())
						.is_some_and(|text| text.contains(heading))
				})
				.flat_map(|wrapper| {
					wrapper
						.select("div.comic-rows-videos-div")
						.map(|items| items.collect::<Vec<_>>())
						.unwrap_or_default()
				})
				.collect()
		})
		.unwrap_or_default()
}

fn manga_page(items: Vec<Element>, html: &Document) -> MangaPageResult {
	MangaPageResult {
		entries: items.iter().filter_map(parse_comic_div).collect(),
		has_next_page: html.select_first(NEXT_PAGE_SELECTOR).is_some(),
	}
}

/// Returns the `div.no-select` texts of the metadata row starting with `key`.
fn metadata(info: &Element, key: &str) -> Option<Vec<String>> {
	let row = info.select("h5")?.find(|h5| {
		h5.text()
			.is_some_and(|text| text.trim_start().starts_with(key))
	})?;
	let values: Vec<String> = row
		.select("div.no-select")?
		.filter_map(|el| el.text())
		.filter(|text| !text.is_empty())
		.collect();
	(!values.is_empty()).then_some(values)
}

fn parse_search(html: &Document) -> MangaPageResult {
	let mut items: Vec<Element> = html
		.select("div#comics-search-tag-top-row + div div.comic-rows-videos-div")
		.map(|items| items.collect())
		.unwrap_or_default();
	if items.is_empty() {
		items = html
			.select("div.comic-rows-wrapper div.comic-rows-videos-div")
			.map(|items| items.collect())
			.unwrap_or_default();
	}
	manga_page(items, html)
}

fn parse_details(html: &Document, manga: &mut Manga) {
	let title_el = html.select_first("h3.title.comics-metadata-top-row");
	let info = title_el.as_ref().and_then(|el| el.parent());
	if let Some(title) = title_el.and_then(|el| clean_text(&el)) {
		manga.title = title;
	}
	if let Some(cover) = info
		.as_ref()
		.and_then(|el| el.parent())
		.and_then(|row| row.select_first("div.col-md-4 img"))
		.and_then(|img| image_src(&img))
	{
		manga.cover = Some(cover);
	}
	if let Some(info) = info {
		manga.authors = metadata(&info, "作者").or_else(|| metadata(&info, "社團"));
		manga.tags = metadata(&info, "標籤");

		let mut description = Vec::new();
		if let Some(subtitle) = info.select_first("h4.title").and_then(|el| clean_text(&el)) {
			description.push(subtitle);
		}
		for key in ["分類", "語言", "角色", "社團", "頁數"] {
			if let Some(values) = metadata(&info, key) {
				description.push(format!("{key}：{}", values.join(", ")));
			}
		}
		manga.description = (!description.is_empty()).then(|| description.join("\n"));
	}
	manga.content_rating = ContentRating::NSFW;
	manga.url = Some(format!("{BASE_URL}/comic/{}", manga.key));
}

/// Every entry of the "related episodes" section is a separate comic; expose them as chapters.
fn parse_chapters(html: &Document, manga_key: &str) -> Vec<Chapter> {
	let related: Vec<Chapter> = section_items(html, "相關集數列表")
		.iter()
		.filter_map(parse_comic_div)
		.map(|item| {
			let prefix = if item.key == manga_key {
				"當前"
			} else {
				"關聯"
			};
			Chapter {
				title: Some(format!("{prefix}：{}", item.title)),
				url: item.url,
				key: item.key,
				..Default::default()
			}
		})
		.collect();
	if related.is_empty() {
		vec![Chapter {
			key: manga_key.into(),
			title: Some("單章節".into()),
			url: Some(format!("{BASE_URL}/comic/{manga_key}")),
			..Default::default()
		}]
	} else {
		related
	}
}

fn parse_pages(html: &Document) -> Result<Vec<Page>> {
	let image = html
		.select_first("img#current-page-image")
		.ok_or_else(|| error!("Missing page image"))?;
	let prefix = image.attr("data-prefix").unwrap_or_default();
	let extension = image.attr("data-extension").unwrap_or_default();
	let count: usize = html
		.select_first(".comic-show-content-nav")
		.and_then(|el| el.attr("data-pages"))
		.and_then(|pages| pages.trim().parse().ok())
		.ok_or_else(|| error!("Missing page count"))?;
	Ok((1..=count)
		.map(|i| Page {
			content: PageContent::url(format!("{prefix}{i}.{extension}")),
			..Default::default()
		})
		.collect())
}

impl Source for Hanime1 {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut params = QueryParameters::new();
		params.push("query", Some(query.as_deref().unwrap_or_default()));
		params.push("page", Some(&page.to_string()));
		for filter in filters {
			if let FilterValue::Select { id, value } = filter
				&& id == "sort"
				&& !value.is_empty()
			{
				params.push("sort", Some(&value));
			}
		}
		let html = request(&format!("{BASE_URL}/comics/search?{params}"))?;
		Ok(parse_search(&html))
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let html = request(&format!("{BASE_URL}/comic/{}", manga.key))?;
		if needs_details {
			parse_details(&html, &mut manga);
		}
		if needs_chapters {
			manga.chapters = Some(parse_chapters(&html, &manga.key));
		}
		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let html = request(&format!("{BASE_URL}/comic/{}/1", chapter.key))?;
		parse_pages(&html)
	}
}

impl ListingProvider for Hanime1 {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		if listing.id == "popular" {
			// the popular section is a fixed list on the homepage
			if page > 1 {
				return Ok(MangaPageResult::default());
			}
			let html = request(&format!("{BASE_URL}/comics"))?;
			let mut result = manga_page(section_items(&html, "發燒漫畫"), &html);
			result.has_next_page = false;
			Ok(result)
		} else {
			let html = request(&format!("{BASE_URL}/comics?page={page}"))?;
			Ok(manga_page(section_items(&html, "最新上傳"), &html))
		}
	}
}

impl DeepLinkHandler for Hanime1 {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		if !url.starts_with(BASE_URL) {
			return Ok(None);
		}
		Ok(comic_id(&url).map(|key| DeepLinkResult::Manga { key }))
	}
}

register_source!(Hanime1, ListingProvider, DeepLinkHandler);

#[cfg(test)]
mod test;
