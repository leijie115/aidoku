#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{FilterValue, Home, alloc::vec};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Madara::<Mangaforfreecom>::new();

	let result = source
		.get_search_manga_list(Some("love".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");
	println!(
		"search: {} entries, has_next_page: {}",
		result.entries.len(),
		result.has_next_page
	);

	let manga = result.entries.into_iter().next().unwrap();
	println!("manga: {} {} {:?}", manga.key, manga.title, manga.cover);
	let manga = source.get_manga_update(manga, true, true).unwrap();
	println!(
		"details: {:?} {:?} {:?} {:?} {:?}",
		manga.authors, manga.tags, manga.status, manga.content_rating, manga.description
	);
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");
	println!("chapters: {} first: {:?}", chapters.len(), chapters[0]);

	let pages = source
		.get_page_list(manga, chapters.into_iter().next().unwrap())
		.unwrap();
	assert!(!pages.is_empty(), "no pages");
	println!("pages: {} first: {:?}", pages.len(), pages[0]);
}

#[aidoku_test]
fn filters_and_home() {
	let source = Madara::<Mangaforfreecom>::new();
	let result = source
		.get_search_manga_list(
			None,
			2,
			vec![FilterValue::MultiSelect {
				id: "genre[]".into(),
				included: vec!["romance".into()],
				excluded: vec![],
			}],
		)
		.unwrap();
	assert!(!result.entries.is_empty(), "genre filter returned nothing");
	println!("genre page 2: {} entries", result.entries.len());

	let home = source.get_home().unwrap();
	assert!(!home.components.is_empty(), "empty home");
	println!("home components: {}", home.components.len());
}
