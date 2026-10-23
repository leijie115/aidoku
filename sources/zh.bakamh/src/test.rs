#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::alloc::vec;
use aidoku_test::aidoku_test;

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Bakamh::new();

	let result = source
		.get_search_manga_list(Some("老师".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");
	println!("search: {} entries", result.entries.len());

	let manga = result.entries.into_iter().next().unwrap();
	println!("manga: {} {}", manga.key, manga.title);
	let manga = source.get_manga_update(manga, true, true).unwrap();
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");
	println!("chapters: {} first: {:?}", chapters.len(), chapters[0]);

	let pages = source
		.get_page_list(manga, chapters.into_iter().next().unwrap())
		.unwrap();
	assert!(!pages.is_empty(), "no pages");
	println!("pages: {}", pages.len());
}
