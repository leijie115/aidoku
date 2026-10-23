#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::alloc::vec;
use aidoku_test::aidoku_test;

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Mycomic::new();

	let result = source
		.get_search_manga_list(Some("航海王".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");

	let manga = result.entries.into_iter().next().unwrap();
	println!("manga: {} {}", manga.key, manga.title);
	let manga = source.get_manga_update(manga, true, true).unwrap();
	println!("status: {:?}, authors: {:?}", manga.status, manga.authors);
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
fn rank_listing() {
	let source = Mycomic::new();
	let result = source
		.get_manga_list(
			Listing {
				id: "rank-week".into(),
				name: "週排行".into(),
				..Default::default()
			},
			1,
		)
		.unwrap();
	assert!(!result.entries.is_empty(), "rank returned nothing");
}

#[aidoku_test]
fn simplified_chinese() {
	use aidoku::imports::defaults::{DefaultValue, defaults_set};
	defaults_set("lang", DefaultValue::String("zh-hans".into()));
	let source = Mycomic::new();
	let result = source
		.get_search_manga_list(Some("航海王".into()), 1, vec![])
		.unwrap();
	let manga = result.entries.into_iter().next().unwrap();
	let manga = source.get_manga_update(manga, true, false).unwrap();
	println!("cn: {} {:?} {:?}", manga.title, manga.status, manga.url);
	assert!(manga.url.unwrap().contains("/cn/"));
}
