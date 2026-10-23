#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{FilterValue, alloc::vec};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Wnacg::new();

	let result = source
		.get_search_manga_list(Some("漢化".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");

	let manga = result.entries.into_iter().next().unwrap();
	println!("manga: {} {}", manga.key, manga.title);
	let manga = source.get_manga_update(manga, true, true).unwrap();
	println!("tags: {:?}", manga.tags);
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");

	let pages = source
		.get_page_list(manga, chapters.into_iter().next().unwrap())
		.unwrap();
	assert!(!pages.is_empty(), "no pages");
	println!("pages: {}", pages.len());
}

#[aidoku_test]
fn filter_and_listing() {
	let source = Wnacg::new();
	let result = source
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::Select {
				id: "韩漫".into(),
				value: "19".into(),
			}],
		)
		.unwrap();
	assert!(!result.entries.is_empty(), "filter returned nothing");

	let result = source
		.get_manga_list(
			Listing {
				id: "single-volume".into(),
				name: "单行本".into(),
				..Default::default()
			},
			1,
		)
		.unwrap();
	assert!(!result.entries.is_empty(), "listing returned nothing");
}
