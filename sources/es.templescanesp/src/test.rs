#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{FilterValue, alloc::vec};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Madara::<TempleScanEsp>::new();
	println!("base url: {}", TempleScanEsp.params().base_url);

	let result = source
		.get_search_manga_list(Some("diario".into()), 1, vec![])
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
		"details: {} {:?} {:?} {:?} {:?}",
		manga.title, manga.cover, manga.tags, manga.status, manga.description
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
fn browse_with_filters() {
	let source = Madara::<TempleScanEsp>::new();
	let result = source.get_search_manga_list(None, 1, vec![]).unwrap();
	assert!(!result.entries.is_empty(), "browse returned nothing");
	println!(
		"browse: {} entries, next: {}",
		result.entries.len(),
		result.has_next_page
	);
	let result = source
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::MultiSelect {
				id: "genre[]".into(),
				included: vec!["romance".into()],
				excluded: vec![],
			}],
		)
		.unwrap();
	println!("romance: {} entries", result.entries.len());
}
