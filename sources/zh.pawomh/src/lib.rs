#![no_std]

use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider,
	Listing, ListingProvider, Manga, MangaPageResult, MangaStatus, Page, PageContent, PageContext,
	Result, Source, Viewer,
	alloc::{String, Vec, string::ToString as _},
	helpers::uri::QueryParameters,
	imports::{
		html::{Document, Element},
		net::Request,
		std::parse_date_with_options,
	},
	prelude::*,
};

const BASE_URL: &str = "https://pawomh1.cc";
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";
const MANGA_PATH: &str = "manhuabook";
const CHAPTER_PATH: &str = "manhuach";

fn get(url: &str) -> Result<Request> {
	Ok(Request::get(url)?
		.header("User-Agent", USER_AGENT)
		.header("Referer", &format!("{BASE_URL}/")))
}

fn manga_url(key: &str) -> String {
	format!("{BASE_URL}/{MANGA_PATH}/{key}")
}

fn chapter_url(key: &str) -> String {
	format!("{BASE_URL}/{CHAPTER_PATH}/{key}")
}

/// Last non-empty path segment of a link, e.g. `/book/1254` -> `1254`
fn last_segment(href: &str) -> Option<String> {
	href.split(['?', '#'])
		.next()?
		.rsplit('/')
		.find(|s| !s.is_empty())
		.map(Into::into)
}

/// Covers are either in `data-original` or in `style="background-image: url(...)"`
fn list_cover(item: &Element) -> Option<String> {
	let cover = item.select_first("p.mh-cover")?;
	if let Some(url) = cover.attr("abs:data-original").filter(|s| !s.is_empty()) {
		return Some(url);
	}
	let style = cover.attr("style")?;
	let start = style.find("url(")? + 4;
	let end = start + style[start..].find(')')?;
	let url = style[start..end].trim_matches(|c| c == '\'' || c == '"' || c == ' ');
	(!url.is_empty()).then(|| url.into())
}

fn parse_manga_list(html: &Document) -> MangaPageResult {
	let entries = html
		.select("ul.mh-list div.mh-item")
		.map(|items| {
			items
				.filter_map(|item| {
					let link = item.select_first("h2.title a")?;
					let key = last_segment(&link.attr("href")?)?;
					let title = link
						.attr("title")
						.filter(|s| !s.trim().is_empty())
						.or_else(|| link.text())?
						.trim()
						.to_string();
					Some(Manga {
						cover: list_cover(&item),
						url: Some(manga_url(&key)),
						key,
						title,
						..Default::default()
					})
				})
				.collect::<Vec<_>>()
		})
		.unwrap_or_default();
	let has_next_page = html.select_first("a#nextPage").is_some();
	MangaPageResult {
		entries,
		has_next_page,
	}
}

/// Text after the full-width colon of the first `selector` element starting with `label`
fn labeled(html: &Document, selector: &str, label: &str) -> Option<String> {
	html.select(selector)?
		.filter_map(|el| el.text())
		.find(|text| text.trim_start().starts_with(label))
		.map(|text| {
			text.split_once('：')
				.map(|(_, v)| v)
				.unwrap_or_default()
				.trim()
				.to_string()
		})
}

fn chapter_number(title: &str) -> Option<f32> {
	let rest = &title[title.find('第')? + '第'.len_utf8()..];
	let end = rest
		.find(|c: char| !c.is_ascii_digit() && c != '.')
		.unwrap_or(rest.len());
	rest[..end].parse().ok()
}

struct Pawomh;

impl Source for Pawomh {
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
			// search results are not paginated
			if page > 1 {
				return Ok(MangaPageResult::default());
			}
			let mut qs = QueryParameters::new();
			qs.push("keyword", Some(query));
			let html = get(&format!("{BASE_URL}/search?{qs}"))?.html()?;
			let mut result = parse_manga_list(&html);
			result.has_next_page = false;
			return Ok(result);
		}

		let mut area = String::from("-1");
		let mut end = String::from("-1");
		for filter in filters {
			if let FilterValue::Select { id, value } = filter {
				match id.as_str() {
					"area" => area = value,
					"end" => end = value,
					_ => {}
				}
			}
		}
		// the site's `cate` (题材) filter has no options and breaks the listing when sent
		let mut qs = QueryParameters::new();
		qs.push("page", Some(&page.to_string()));
		qs.push("area", Some(&area));
		qs.push("end", Some(&end));
		let html = get(&format!("{BASE_URL}/booklist?{qs}"))?.html()?;
		Ok(parse_manga_list(&html))
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let url = manga_url(&manga.key);
		let html = get(&url)?.html()?;

		if needs_details {
			if let Some(title) = html
				.select_first(".banner_detail_form .info h1")
				.and_then(|el| el.text())
			{
				manga.title = title.trim().into();
			}
			manga.cover = html
				.select_first(".banner_detail_form .cover img")
				.and_then(|img| {
					img.attr("abs:data-original")
						.filter(|s| !s.is_empty())
						.or_else(|| img.attr("abs:src"))
				})
				.or(manga.cover);
			manga.authors = labeled(&html, "p.subtitle", "作者").map(|authors| {
				authors
					.split('&')
					.map(str::trim)
					.filter(|s| !s.is_empty())
					.map(Into::into)
					.collect()
			});

			let synopsis = html
				.select_first(".banner_detail_form p.content")
				.and_then(|el| el.text())
				.map(|s| s.trim().to_string())
				.filter(|s| !s.is_empty());
			let alt = labeled(&html, "p.subtitle", "别名").filter(|s| !s.is_empty());
			manga.description = match (synopsis, alt) {
				(Some(s), Some(a)) => Some(format!("{s}\n\n别名：{a}")),
				(Some(s), None) => Some(s),
				(None, Some(a)) => Some(format!("别名：{a}")),
				(None, None) => None,
			};

			// area + tag links; unfilled tag placeholders look like `[db:分类]`
			let mut tags: Vec<String> = Vec::new();
			if let Some(links) = html.select(".banner_detail_form p.tip a") {
				for tag in links.filter_map(|a| a.text()) {
					let tag = tag.trim();
					if !tag.is_empty() && !tag.starts_with("[db:") && !tags.iter().any(|t| t == tag)
					{
						tags.push(tag.into());
					}
				}
			}
			manga.tags = Some(tags);

			let status = labeled(&html, "p.tip span.block", "状态").unwrap_or_default();
			manga.status = if status.contains("连载") {
				MangaStatus::Ongoing
			} else if status.contains("完结") {
				MangaStatus::Completed
			} else {
				MangaStatus::Unknown
			};

			let area = labeled(&html, "p.tip span.block", "地区").unwrap_or_default();
			// 韩国 = manhwa, 其他 = mostly Japanese doujinshi
			manga.viewer = if area.contains("韩国") {
				Viewer::Webtoon
			} else {
				Viewer::RightToLeft
			};
			manga.content_rating = ContentRating::NSFW;
			manga.url = Some(url);
		}

		if needs_chapters {
			let updated = labeled(&html, "p.tip span.block", "更新时间").and_then(|date| {
				parse_date_with_options(
					format!("{date} 00:00:00"),
					"yyyy-MM-dd HH:mm:ss",
					"en_US",
					"Asia/Shanghai",
				)
			});
			let mut chapters: Vec<Chapter> = html
				.select(format!("ul#detail-list-select a[href*='/{CHAPTER_PATH}/']"))
				.map(|links| {
					links
						.filter_map(|a| {
							let key = last_segment(&a.attr("href")?)?;
							let title = a
								.text()
								.map(|s| s.trim().to_string())
								.filter(|s| !s.is_empty())
								.or_else(|| a.attr("title"));
							Some(Chapter {
								chapter_number: title.as_deref().and_then(chapter_number),
								url: Some(chapter_url(&key)),
								key,
								title,
								..Default::default()
							})
						})
						.collect()
				})
				.unwrap_or_default();
			chapters.reverse();
			if let Some(first) = chapters.first_mut() {
				first.date_uploaded = updated;
			}
			manga.chapters = Some(chapters);
		}

		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let html = get(&chapter_url(&chapter.key))?.html()?;
		Ok(html
			.select("div.comiclist img[data-original]")
			.map(|imgs| {
				imgs.filter_map(|img| img.attr("abs:data-original"))
					.filter(|url| !url.is_empty())
					.map(|url| Page {
						content: PageContent::url(url),
						..Default::default()
					})
					.collect()
			})
			.unwrap_or_default())
	}
}

impl ListingProvider for Pawomh {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		match listing.id.as_str() {
			// the update page is not paginated
			"update" if page > 1 => Ok(MangaPageResult::default()),
			"update" => {
				let html = get(&format!("{BASE_URL}/update"))?.html()?;
				let mut result = parse_manga_list(&html);
				result.has_next_page = false;
				Ok(result)
			}
			_ => bail!("Invalid listing"),
		}
	}
}

impl ImageRequestProvider for Pawomh {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		get(&url)
	}
}

impl DeepLinkHandler for Pawomh {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(path) = url.strip_prefix(BASE_URL) else {
			return Ok(None);
		};
		let mut segments = path.split('/').filter(|s| !s.is_empty());
		Ok(match (segments.next(), segments.next()) {
			(Some(MANGA_PATH), Some(key)) => Some(DeepLinkResult::Manga { key: key.into() }),
			_ => None,
		})
	}
}

register_source!(
	Pawomh,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
