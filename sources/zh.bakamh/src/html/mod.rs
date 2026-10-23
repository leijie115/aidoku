use crate::{base_url, get};
use aidoku::{
	Chapter, ContentRating, Manga, MangaPageResult, MangaStatus, Page, PageContent, Result, Viewer,
	alloc::{String, Vec, string::ToString as _},
	helpers::uri::decode_uri,
	imports::{
		html::{Document, Element},
		std::current_date,
	},
	prelude::*,
};

/// Parses an absolute date like "2026 年 1 月 8 日" (UTC+8).
fn parse_absolute_date(s: &str) -> Option<i64> {
	let mut numbers = s
		.split(|c: char| !c.is_ascii_digit())
		.filter(|n| !n.is_empty())
		.map(|n| n.parse::<i64>().ok());
	let (year, month, day) = (numbers.next()??, numbers.next()??, numbers.next()??);
	if !s.contains('年') || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
		return None;
	}
	// days since unix epoch from a civil date (Howard Hinnant's algorithm)
	let y = if month <= 2 { year - 1 } else { year };
	let era = y.div_euclid(400);
	let yoe = y - era * 400;
	let mp = (month + 9) % 12;
	let doy = (153 * mp + 2) / 5 + day - 1;
	let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
	let days = era * 146097 + doe - 719468;
	Some(days * 86400 - 8 * 3600)
}

fn parse_chapter_date(s: &str) -> Option<i64> {
	if let Some(ts) = parse_absolute_date(s) {
		return Some(ts);
	}

	let number = s
		.split_whitespace()
		.find_map(|w| w.parse::<i64>().ok())
		.unwrap_or(0);

	const MINUTE: i64 = 60;
	const HOUR: i64 = 60 * MINUTE;
	const DAY: i64 = 24 * HOUR;
	const WEEK: i64 = 7 * DAY;
	const MONTH: i64 = 30 * DAY;
	const YEAR: i64 = 365 * DAY;

	let offset = if s.contains("年") {
		number * YEAR
	} else if s.contains("月") {
		number * MONTH
	} else if s.contains("周") || s.contains("週") {
		number * WEEK
	} else if s.contains("天") || s.contains("日") {
		number * DAY
	} else if s.contains("小时") || s.contains("小時") {
		number * HOUR
	} else if s.contains("分") {
		number * MINUTE
	} else {
		return None;
	};

	Some(current_date() - offset)
}

pub trait MangaPage {
	fn manga_page_result(&self, is_search: bool) -> Result<MangaPageResult>;
	fn update_details(&self, manga: &mut Manga) -> Result<()>;
}

pub trait ChapterPage {
	fn chapters(&self, manga_key: &str) -> Result<Vec<Chapter>>;
}

impl MangaPage for Document {
	fn manga_page_result(&self, is_search: bool) -> Result<MangaPageResult> {
		let entries = if is_search {
			self.select(".c-tabs-item__content")
				.map(|items| {
					items
						.filter_map(|item| {
							let href = item
								.select_first(".col-4>.tab-thumb>a")
								.and_then(|a| a.attr("href"))?;
							let id = href.split('/').rfind(|s| !s.is_empty())?.to_string();
							let cover = item
								.select_first(".col-4>.tab-thumb>a>img")
								.and_then(|img| img.attr("src"))
								.map(|s| s.replace("-193x278", ""));
							let title = item
								.select_first(".col-8>.tab-summary>.post-title>h3>a")
								.and_then(|a| a.text())?;
							let url = format!("{}/manga/{}/", base_url(), id);
							Some(Manga {
								key: id,
								cover,
								title,
								url: Some(url),
								content_rating: ContentRating::NSFW,
								viewer: Viewer::Webtoon,
								..Default::default()
							})
						})
						.collect::<Vec<Manga>>()
				})
				.unwrap_or_default()
		} else {
			self.select(".page-item-detail")
				.map(|items| {
					items
						.filter_map(|item| {
							let href = item
								.select_first(".item-thumb>a")
								.and_then(|a| a.attr("href"))?;
							let id = href.split('/').rfind(|s| !s.is_empty())?.to_string();
							let cover = item
								.select_first(".item-thumb>a>img")
								.and_then(|img| img.attr("src"))
								.map(|s| s.replace("-175x238", ""));
							let title = item
								.select_first(".item-summary>.post-title>h3>a")
								.and_then(|a| a.text())?;
							let url = format!("{}/manga/{}/", base_url(), id);
							Some(Manga {
								key: id,
								cover,
								title,
								url: Some(url),
								content_rating: ContentRating::NSFW,
								viewer: Viewer::Webtoon,
								..Default::default()
							})
						})
						.collect::<Vec<Manga>>()
				})
				.unwrap_or_default()
		};

		let has_next_page = self.select_first("a.nextpostslink").is_some();

		Ok(MangaPageResult {
			entries,
			has_next_page,
		})
	}

	fn update_details(&self, manga: &mut Manga) -> Result<()> {
		manga.cover = self
			.select_first("meta[property='og:image']")
			.and_then(|m| m.attr("content"));

		manga.title = self
			.select_first("meta[property='og:title']")
			.and_then(|m| m.attr("content"))
			.unwrap_or_default();

		manga.authors = self
			.select(".author-content>a")
			.map(|els| els.filter_map(|el| el.text()).collect());

		let len = self
			.select(".post-content>div")
			.map(|e| e.count())
			.unwrap_or(0);
		if len > 0 {
			manga.description = self
				.select_first(format!(".post-content>div:nth-child({})>div>p", len))
				.and_then(|e| e.text())
				.map(|s| s.trim().to_string());
		}

		manga.tags = self
			.select(".tags-content>a")
			.map(|els| els.filter_map(|el| el.text()).collect());

		if len >= 3 {
			manga.status = self
				.select_first(format!(
					".post-content>div:nth-child({})>.summary-content",
					len - 2
				))
				.and_then(|e| e.text())
				.map(|s| match s.trim() {
					"OnGoing" | "Ongoing" | "连载中" => MangaStatus::Ongoing,
					"Completed" | "End" | "完结" => MangaStatus::Completed,
					_ => MangaStatus::Unknown,
				})
				.unwrap_or(MangaStatus::Unknown);
		}

		manga.content_rating = ContentRating::NSFW;
		manga.viewer = Viewer::Webtoon;

		Ok(())
	}
}

/// Decodes and lowercases a url so differently encoded urls can be compared.
fn normalize_url(url: &str) -> String {
	decode_uri(url).to_lowercase()
}

/// Finds the chapter url of a chapter link.
///
/// The site hides the real url in a random attribute, so every attribute value
/// is checked for a url pointing to a chapter of the current manga.
fn chapter_url(a: &Element, manga_url: &str) -> Option<String> {
	if let Some(url) = a.attr("storage-chapter-url").filter(|u| !u.is_empty()) {
		return Some(url);
	}
	let is_chapter_url = |value: &str| {
		let value = normalize_url(value);
		value.starts_with(manga_url)
			&& value != manga_url
			&& !value.starts_with(&format!("{manga_url}#comment"))
	};
	if let Some(href) = a.attr("href").filter(|href| is_chapter_url(href)) {
		return Some(href);
	}
	// check the values of all attributes in the opening tag
	let outer_html = a.outer_html()?;
	let tag = outer_html.split_once('>').map(|(tag, _)| tag)?;
	tag.split(['"', '\''])
		.skip(1)
		.step_by(2)
		.map(|value| value.replace("&amp;", "&"))
		.find(|value| is_chapter_url(value))
}

impl ChapterPage for Document {
	fn chapters(&self, manga_key: &str) -> Result<Vec<Chapter>> {
		let manga_url = normalize_url(&format!("{}/manga/{}/", base_url(), manga_key));

		let mut items: Vec<(Element, Element)> = self
			.select(".listing-chapters_main li.chapter-loveYou")
			.filter(|els| !els.is_empty())
			.or_else(|| self.select(".listing-chapters_main li"))
			.map(|els| {
				els.filter_map(|li| Some((li.select_first("a")?, li)))
					.collect()
			})
			.unwrap_or_default();
		items.retain(|(a, _)| chapter_url(a, &manga_url).is_some());
		if items.is_empty() {
			// fall back to checking every link on the page
			items = self
				.select(".chapter-loveYou a, li:not(.menu-item) a[onclick], li:not(.menu-item) a")
				.map(|els| {
					els.filter(|a| chapter_url(a, &manga_url).is_some())
						.filter_map(|a| {
							let li = a.parent()?;
							Some((a, li))
						})
						.collect()
				})
				.unwrap_or_default();
		}

		let len = items.len();
		let chapters: Vec<Chapter> = items
			.into_iter()
			.enumerate()
			.filter_map(|(index, (a, li))| {
				let chapter_url = chapter_url(&a, &manga_url)?;

				let key = chapter_url
					.trim_start_matches("https://")
					.trim_start_matches("http://")
					.split_once('/')
					.map(|x| x.1)
					.map(|s| format!("/{}", s))
					.unwrap_or_else(|| chapter_url.clone());

				let title = a.text().map(|t| t.trim().to_string());
				let date_uploaded = li
					.select_first(".chapter-release-date i")
					.and_then(|el| el.text())
					.and_then(|s| parse_chapter_date(s.trim()));
				let chapter_number = (len - index) as f32;

				Some(Chapter {
					key,
					title,
					chapter_number: Some(chapter_number),
					date_uploaded,
					url: Some(chapter_url),
					..Default::default()
				})
			})
			.collect();

		Ok(chapters)
	}
}

pub fn get_page_list(chapter_key: &str) -> Result<Vec<Page>> {
	let url = format!("{}{}", base_url(), chapter_key);
	let html = get(url)?.html()?;

	let parse_images = |selector: &str| -> Vec<Page> {
		html.select(selector)
			.map(|imgs| {
				imgs.filter_map(|img| {
					let src = img
						.attr("abs:data-manga-src")
						.or_else(|| img.attr("abs:data-src"))
						.or_else(|| img.attr("abs:data-lazy-src"))
						.or_else(|| img.attr("abs:src"))
						.map(|s| s.trim().to_string())
						.filter(|s| !s.is_empty())?;
					Some(Page {
						content: PageContent::url(src),
						..Default::default()
					})
				})
				.collect::<Vec<Page>>()
			})
			.unwrap_or_default()
	};

	let pages = parse_images(".reading-content div.page-break img, .reading-content img[id]");
	if pages.is_empty() {
		Ok(parse_images("img[id]"))
	} else {
		Ok(pages)
	}
}
