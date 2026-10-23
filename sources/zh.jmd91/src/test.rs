#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::alloc::vec;
use aidoku_test::aidoku_test;

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Jmd91::new();

	let result = source
		.get_search_manga_list(Some("上司".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");

	let manga = result.entries.into_iter().next().unwrap();
	println!("manga: {} {} {:?}", manga.key, manga.title, manga.cover);
	let manga = source.get_manga_update(manga, true, true).unwrap();
	println!(
		"details: {:?} {:?} {:?} {:?} {:?}",
		manga.authors, manga.tags, manga.status, manga.viewer, manga.description
	);
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");
	println!(
		"chapters: {} first: {} {:?} {:?} {:?}",
		chapters.len(),
		chapters[0].key,
		chapters[0].title,
		chapters[0].chapter_number,
		chapters[0].date_uploaded
	);

	let pages = source
		.get_page_list(manga, chapters.into_iter().next().unwrap())
		.unwrap();
	assert!(!pages.is_empty(), "no pages");
	println!("pages: {}", pages.len());
	let PageContent::Url(url, _) = &pages[0].content else {
		panic!("unexpected page content");
	};
	let image = get(url).unwrap().image().unwrap();
	println!("page image: {url} {}x{}", image.width(), image.height());
	assert!(image.width() > 0.0);
}

#[aidoku_test]
fn filters() {
	let source = Jmd91::new();
	let result = source
		.get_search_manga_list(
			None,
			2,
			vec![
				FilterValue::Select {
					id: "genre".into(),
					value: "日漫".into(),
				},
				FilterValue::Sort {
					id: "sort".into(),
					index: 1,
					ascending: false,
				},
				FilterValue::Select {
					id: "status".into(),
					value: "completed".into(),
				},
			],
		)
		.unwrap();
	println!(
		"filtered: {} next {}",
		result.entries.len(),
		result.has_next_page
	);
	assert!(!result.entries.is_empty());
}

#[aidoku_test]
fn dates() {
	let now = current_date();
	assert_eq!(parse_date("6小时之前"), Some(now - 6 * 3600));
	println!("{:?}", parse_date("前天 20:55"));
	assert!(parse_date("前天 20:55").is_some());
	assert!(parse_date("10/22/2026").is_some());
	assert!(parse_date("01-05").is_some());
}
