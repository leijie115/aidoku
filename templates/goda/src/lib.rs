#![no_std]
use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider,
	Listing, ListingProvider, Manga, MangaPageResult, MangaStatus, Page, PageContent, PageContext,
	Result, Source, Viewer,
	alloc::{String, Vec, borrow::Cow, string::ToString as _},
	helpers::uri::encode_uri_component,
	imports::{defaults::defaults_get, html::Document, net::Request, std::send_partial_result},
	prelude::*,
};
use serde::Deserialize;

mod decoder;

/// Where chapter lists and page lists are loaded from.
pub enum Backend {
	/// `{base}/manga/get?mid=..&mode=all` and `{base}/chapter/getcontent?m=..&c=..` (HTML).
	Html,
	/// The JSON API at `api_url`; decoded image paths are prefixed with `image_url`.
	Api {
		api_url: Cow<'static, str>,
		image_url: Cow<'static, str>,
	},
}

pub struct Params {
	/// Default base url (overridden by the `url` setting when present).
	pub base_url: Cow<'static, str>,
	pub backend: Backend,
	/// Mark every manga as NSFW.
	pub nsfw: bool,
}

pub trait Impl {
	fn new() -> Self;
	fn params(&self) -> Params;
}

pub struct GoDa<T: Impl> {
	params: Params,
	_inner: T,
}

impl<T: Impl> GoDa<T> {
	fn base_url(&self) -> String {
		defaults_get::<String>("url")
			.filter(|url| !url.is_empty())
			.map(|url| url.trim_end_matches('/').into())
			.unwrap_or_else(|| self.params.base_url.to_string())
	}

	fn get(&self, url: &str) -> Result<Request> {
		let base_url = self.base_url();
		Ok(Request::get(url)?
			.header("Referer", &format!("{base_url}/"))
			.header("Origin", &base_url))
	}

	fn manga_list(&self, url: &str) -> Result<MangaPageResult> {
		let html = self.get(url)?.html()?;
		let entries = html
			.select(".cardlist .pb-2 a")
			.map(|els| {
				els.filter_map(|el| {
					let key = manga_key(&el.attr("href")?)?;
					let title = el.select_first("h3")?.text()?;
					let cover = el
						.select_first("img")
						.and_then(|img| img.attr("src"))
						.map(cover_url);
					Some(Manga {
						key,
						title,
						cover,
						..Default::default()
					})
				})
				.collect::<Vec<_>>()
			})
			.unwrap_or_default();
		let has_next_page = html.select_first("a[aria-label='下一頁']").is_some();
		Ok(MangaPageResult {
			entries,
			has_next_page,
		})
	}

	fn update_details(&self, html: &Document, manga: &mut Manga, url: String) {
		let h1 = html
			.select_first("main h1")
			.or_else(|| html.select_first("h1"));
		if let Some(title) = h1
			.as_ref()
			.and_then(|h1| h1.own_text())
			.map(|t| t.trim().to_string())
			.filter(|t| !t.is_empty())
		{
			manga.title = title;
		}
		manga.cover = html
			.select_first("meta[property='og:image']")
			.and_then(|el| el.attr("content"))
			.map(cover_url)
			.or(manga.cover.take());
		manga.authors = html
			.select("a[href*='/manga-author/'] span")
			.map(|els| {
				els.filter_map(|el| el.text())
					.map(|t| clean_tag(&t))
					.filter(|t| !t.is_empty())
					.collect::<Vec<_>>()
			})
			.filter(|v| !v.is_empty());
		manga.tags = html
			.select("a[href*='/manga-genre/'] span, a[href*='/manga-tag/'] span")
			.map(|els| {
				els.filter_map(|el| el.text())
					.map(|t| clean_tag(&t))
					.filter(|t| {
						!t.is_empty() && !matches!(t.as_str(), "热门漫画" | "熱門漫畫" | "热门推荐")
					})
					.collect::<Vec<_>>()
			})
			.filter(|v| !v.is_empty());
		manga.description = html
			.select_first("#info p.line-clamp-4, p.line-clamp-4")
			.and_then(|el| el.text())
			.map(|t| t.trim().to_string());
		manga.status = h1
			.and_then(|h1| h1.select_first("span"))
			.and_then(|el| el.text())
			.map(|t| match t.trim() {
				"連載中" | "连载中" | "Ongoing" => MangaStatus::Ongoing,
				"完結" | "已完結" | "完结" | "已完结" => MangaStatus::Completed,
				"停止更新" => MangaStatus::Cancelled,
				"休刊" => MangaStatus::Hiatus,
				_ => MangaStatus::Unknown,
			})
			.unwrap_or_default();
		let tags = manga.tags.as_deref().unwrap_or_default();
		let has = |names: &[&str]| tags.iter().any(|t| names.contains(&t.as_str()));
		manga.viewer = if has(&["韩漫", "韓漫", "国漫", "國漫"]) {
			Viewer::Webtoon
		} else if has(&["日漫"]) {
			Viewer::RightToLeft
		} else {
			Viewer::Unknown
		};
		manga.content_rating = if self.params.nsfw {
			ContentRating::NSFW
		} else {
			ContentRating::Unknown
		};
		manga.url = Some(url);
	}

	fn chapters_html(&self, base_url: &str, slug: &str, mid: &str) -> Result<Vec<Chapter>> {
		let url = format!("{base_url}/manga/get?mid={mid}&mode=all");
		let html = self.get(&url)?.html()?;
		// some responses contain separate newest-first / oldest-first lists;
		// otherwise (`mode=all`) a single oldest-first list is returned
		let (selector, newest_first) = if html.select_first("#chapterListDesc").is_some() {
			("#chapterListDesc .chapteritem a", true)
		} else {
			(".chapteritem a", false)
		};
		let mut chapters: Vec<Chapter> = Vec::new();
		for el in html.select(selector).into_iter().flatten() {
			let Some(cid) = el.attr("data-cs") else {
				continue;
			};
			let key = format!("{mid}/{cid}");
			if chapters.iter().any(|c| c.key == key) {
				continue;
			}
			let Some(title) = el
				.attr("data-ct")
				.filter(|t| !t.is_empty())
				.or_else(|| el.text())
			else {
				continue;
			};
			let url = el.attr("href").and_then(|href| {
				chapter_slug(&href).map(|cslug| format!("{base_url}/manga/{slug}/{cslug}"))
			});
			chapters.push(make_chapter(mid, &cid, title, None, url));
		}
		if !newest_first {
			chapters.reverse();
		}
		Ok(chapters)
	}

	fn chapters_api(
		&self,
		api_url: &str,
		base_url: &str,
		slug: &str,
		mid: &str,
	) -> Result<Vec<Chapter>> {
		let url = format!("{api_url}/api/manga/get?mid={mid}&mode=all");
		let response: ApiResponse<ChapterListData> = self.get(&url)?.json_owned()?;
		let mut chapters = response
			.data
			.chapters
			.into_iter()
			.map(|item| {
				let id = match item.id {
					serde_json::Value::String(s) => s,
					other => other.to_string(),
				};
				let attrs = item.attributes;
				let url = attrs
					.slug
					.map(|cslug| format!("{base_url}/manga/{slug}/{cslug}"));
				let date = attrs
					.updated_at
					.and_then(|d| chrono::DateTime::parse_from_rfc3339(&d).ok())
					.map(|d| d.timestamp());
				make_chapter(mid, &id, attrs.title, date, url)
			})
			.collect::<Vec<_>>();
		chapters.reverse();
		Ok(chapters)
	}

	fn find_mid(html: &Document) -> Result<String> {
		html.select_first("#mangachapters[data-mid], #allchapters[data-mid], [data-mid]")
			.and_then(|el| el.attr("data-mid"))
			.filter(|mid| !mid.is_empty())
			.ok_or_else(|| error!("Unable to find manga id"))
	}
}

impl<T: Impl> Source for GoDa<T> {
	fn new() -> Self {
		let inner = T::new();
		Self {
			params: inner.params(),
			_inner: inner,
		}
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let base_url = self.base_url();
		if let Some(query) = query.filter(|q| !q.trim().is_empty()) {
			let url = format!(
				"{base_url}/s/{}?page={page}",
				encode_uri_component(query.trim())
			);
			return self.manga_list(&url);
		}
		let mut path = String::from("manga");
		for filter in filters {
			if let FilterValue::Select { id, value } = filter
				&& id == "genre"
				&& !value.is_empty()
			{
				path = value;
			}
		}
		self.manga_list(&format!("{base_url}/{path}/page/{page}"))
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let base_url = self.base_url();
		let url = format!("{base_url}/manga/{}", manga.key);
		let html = self.get(&url)?.html()?;

		if needs_details {
			self.update_details(&html, &mut manga, url);
			if needs_chapters {
				send_partial_result(&manga);
			}
		}

		if needs_chapters {
			let mid = Self::find_mid(&html)?;
			manga.chapters = Some(match &self.params.backend {
				Backend::Html => self.chapters_html(&base_url, &manga.key, &mid)?,
				Backend::Api { api_url, .. } => {
					self.chapters_api(api_url, &base_url, &manga.key, &mid)?
				}
			});
		}

		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let (mid, cid) = chapter
			.key
			.split_once('/')
			.ok_or_else(|| error!("Invalid chapter key, please refresh the chapter list"))?;
		match &self.params.backend {
			Backend::Html => {
				let url = format!("{}/chapter/getcontent?m={mid}&c={cid}", self.base_url());
				let html = self.get(&url)?.html()?;
				Ok(html
					.select("#chapcontent > div > img, #chapcontent img")
					.map(|els| {
						els.filter_map(|el| {
							el.attr("abs:data-src")
								.filter(|s| !s.is_empty())
								.or_else(|| el.attr("abs:src"))
								.filter(|s| !s.is_empty())
						})
						.collect::<Vec<_>>()
					})
					.unwrap_or_default()
					.into_iter()
					.fold(Vec::new(), |mut acc: Vec<String>, url| {
						if !acc.contains(&url) {
							acc.push(url);
						}
						acc
					})
					.into_iter()
					.map(|url| Page {
						content: PageContent::url(url),
						..Default::default()
					})
					.collect())
			}
			Backend::Api { api_url, image_url } => {
				let url = format!("{api_url}/api/v2/chapter/getinfo?m={mid}&c={cid}");
				let response: ApiResponse<PageListData> = self.get(&url)?.json_owned()?;
				let decoded =
					decoder::ChapterImageDecoder::decode(&response.data.info.images.images)?;
				let images: Vec<ImageItem> = serde_json::from_str(&decoded)
					.map_err(|_| error!("Failed to parse image list"))?;
				Ok(images
					.into_iter()
					.map(|item| {
						let url = if item.url.starts_with("http") {
							item.url
						} else {
							format!("{image_url}{}", item.url)
						};
						Page {
							content: PageContent::url(url),
							..Default::default()
						}
					})
					.collect())
			}
		}
	}
}

impl<T: Impl> ListingProvider for GoDa<T> {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let path = match listing.id.as_str() {
			"hots" | "dayup" | "newss" => listing.id,
			_ => bail!("Invalid listing"),
		};
		self.manga_list(&format!("{}/{path}/page/{page}", self.base_url()))
	}
}

impl<T: Impl> ImageRequestProvider for GoDa<T> {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{}/", self.base_url())))
	}
}

impl<T: Impl> DeepLinkHandler for GoDa<T> {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		Ok(manga_key(&url).map(|key| DeepLinkResult::Manga { key }))
	}
}

/// Extracts the manga slug from `.../manga/{slug}[/...]`.
fn manga_key(href: &str) -> Option<String> {
	let (_, rest) = href.split_once("/manga/")?;
	let slug = rest.split(['/', '?', '#']).next()?;
	(!slug.is_empty() && slug != "get").then(|| slug.into())
}

/// Extracts the chapter slug from `.../manga/{slug}/{chapter}`.
fn chapter_slug(href: &str) -> Option<&str> {
	let (_, rest) = href.split_once("/manga/")?;
	rest.trim_end_matches('/')
		.split('/')
		.nth(1)
		.filter(|s| !s.is_empty())
}

/// Unwraps `/_next/image?url=...` style cover urls.
fn cover_url(url: String) -> String {
	match url.split_once("url=") {
		Some((_, rest)) => rest
			.split('&')
			.next()
			.unwrap_or(rest)
			.replace("%3A", ":")
			.replace("%2F", "/"),
		None => url,
	}
}

fn clean_tag(text: &str) -> String {
	text.trim()
		.trim_end_matches(',')
		.trim()
		.trim_start_matches('#')
		.trim()
		.into()
}

fn make_chapter(
	mid: &str,
	cid: &str,
	title: String,
	date_uploaded: Option<i64>,
	url: Option<String>,
) -> Chapter {
	let title = title.trim().to_string();
	let number = parse_number(&title);
	let is_volume = title.ends_with('卷');
	Chapter {
		key: format!("{mid}/{cid}"),
		chapter_number: if is_volume { None } else { number },
		volume_number: if is_volume { number } else { None },
		title: Some(title),
		date_uploaded,
		url,
		..Default::default()
	}
}

/// Parses `第12话`, `第12.5話`, or a leading number like `12 标题`.
fn parse_number(title: &str) -> Option<f32> {
	let digits = |s: &str| -> Option<f32> {
		let end = s
			.char_indices()
			.find(|(_, c)| !(c.is_ascii_digit() || *c == '.'))
			.map(|(i, _)| i)
			.unwrap_or(s.len());
		s[..end].trim_end_matches('.').parse::<f32>().ok()
	};
	if let Some((_, rest)) = title.split_once('第')
		&& let Some(num) = digits(rest.trim_start())
	{
		return Some(num);
	}
	let lower = title.to_lowercase();
	for prefix in ["chapter", "ch.", "ep.", "ep"] {
		if let Some(rest) = lower.strip_prefix(prefix)
			&& let Some(num) = digits(rest.trim_start())
		{
			return Some(num);
		}
	}
	digits(title)
}

#[derive(Deserialize)]
struct ApiResponse<D> {
	data: D,
}

#[derive(Deserialize)]
struct ChapterListData {
	chapters: Vec<ChapterItem>,
}

#[derive(Deserialize)]
struct ChapterItem {
	id: serde_json::Value,
	attributes: ChapterAttributes,
}

#[derive(Deserialize)]
struct ChapterAttributes {
	title: String,
	slug: Option<String>,
	#[serde(rename = "updatedAt")]
	updated_at: Option<String>,
}

#[derive(Deserialize)]
struct PageListData {
	info: PageListInfo,
}

#[derive(Deserialize)]
struct PageListInfo {
	images: PageListImages,
}

#[derive(Deserialize)]
struct PageListImages {
	images: String,
}

#[derive(Deserialize)]
struct ImageItem {
	url: String,
}
