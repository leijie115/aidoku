#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::alloc::vec;
use aidoku_test::aidoku_test;

#[aidoku_test]
fn listing_details_chapters_pages() {
	let source = Happymh::new();

	let result = source
		.get_manga_list(
			Listing {
				id: "views".into(),
				..Default::default()
			},
			1,
		)
		.unwrap();
	assert!(!result.entries.is_empty(), "listing returned nothing");

	let manga = result.entries.into_iter().next().unwrap();
	println!("manga: {} {}", manga.key, manga.title);
	let manga = source.get_manga_update(manga, true, true).unwrap();
	println!("authors: {:?} tags: {:?}", manga.authors, manga.tags);
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");
	println!("chapters: {}", chapters.len());

	let pages = source
		.get_page_list(manga, chapters.into_iter().next().unwrap())
		.unwrap();
	assert!(!pages.is_empty(), "no pages");
	println!("pages: {}", pages.len());
}

#[aidoku_test]
fn search() {
	let source = Happymh::new();
	let result = source
		.get_search_manga_list(Some("海贼王".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");
	println!("search: {}", result.entries.len());
}

#[aidoku_test]
fn filters() {
	let source = Happymh::new();
	let result = source
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::Select {
				id: "area".into(),
				value: "japan".into(),
			}],
		)
		.unwrap();
	assert!(!result.entries.is_empty(), "filter returned nothing");
}

/// Offline check of the response models (field names follow the Tachiyomi extension DTOs).
#[aidoku_test]
fn parse_models() {
	let list: models::PopularResponse = serde_json::from_str(
		r#"{"status":0,"data":{"items":[{"name":"N","manga_code":"code","cover":"https://c/x.jpg","url":"/manga/code"}],"isEnd":false}}"#,
	)
	.unwrap();
	let result: MangaPageResult = list.data.into();
	assert!(result.has_next_page);
	assert_eq!(result.entries[0].key, "code");

	let chapters: models::ChapterByPageResponse = serde_json::from_str(
		r#"{"data":{"items":[{"id":123,"chapterName":"第1话","order":1}],"isEnd":1}}"#,
	)
	.unwrap();
	assert!(chapters.data.is_last_page());

	let pages: models::PageListResponse = serde_json::from_str(
		r#"{"data":{"scans":[{"n":0,"url":"https://i/1.jpg?q=50","width":800,"height":20000},{"n":1,"url":"https://ad"}]}}"#,
	)
	.unwrap();
	assert_eq!(pages.data.scans.len(), 2);
}
