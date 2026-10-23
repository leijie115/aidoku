#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{FilterValue, alloc::vec};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Madara::<Ho5Ho>::new();

	let result = source
		.get_search_manga_list(Some("調教".into()), 1, vec![])
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
		"details: {} {:?} {:?} {:?} {:?} {:?}",
		manga.title,
		manga.cover,
		manga.authors,
		manga.tags,
		manga.content_rating,
		manga.description
	);
	assert!(
		manga.tags.as_ref().is_some_and(|t| !t.is_empty()),
		"no tags"
	);
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");
	println!("chapters: {} first: {:?}", chapters.len(), chapters[0]);

	let pages = source
		.get_page_list(manga, chapters.into_iter().next().unwrap())
		.unwrap();
	assert!(pages.len() > 3, "too few pages");
	println!("pages: {} first: {:?}", pages.len(), pages[0]);
}

#[aidoku_test]
fn filters() {
	let source = Madara::<Ho5Ho>::new();
	let result = source
		.get_search_manga_list(
			None,
			2,
			vec![
				FilterValue::MultiSelect {
					id: "genre[]".into(),
					included: vec!["亂倫".into()],
					excluded: vec![],
				},
				FilterValue::Sort {
					id: "m_orderby".into(),
					index: 5,
					ascending: false,
				},
			],
		)
		.unwrap();
	assert!(!result.entries.is_empty(), "genre filter returned nothing");
	println!(
		"genre page 2: {} entries, first: {}",
		result.entries.len(),
		result.entries[0].title
	);
}
