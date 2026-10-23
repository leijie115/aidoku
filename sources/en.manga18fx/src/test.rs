#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{ListingProvider, alloc::vec};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Madara::<Manga18fx>::new();

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
		"details: {} {:?} {:?} {:?} {:?} {:?}",
		manga.title, manga.authors, manga.artists, manga.tags, manga.status, manga.description
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
fn listings_and_filters() {
	let source = Madara::<Manga18fx>::new();
	for id in ["trending", "latest"] {
		let result = source
			.get_manga_list(
				Listing {
					id: id.into(),
					name: id.into(),
					..Default::default()
				},
				1,
			)
			.unwrap();
		assert!(!result.entries.is_empty(), "{id} returned nothing");
		println!(
			"{id}: {} entries, next: {}, first: {:?}",
			result.entries.len(),
			result.has_next_page,
			result.entries[0].key
		);
	}
	let result = source
		.get_search_manga_list(
			None,
			2,
			vec![
				FilterValue::Select {
					id: "genre".into(),
					value: "romance".into(),
				},
				FilterValue::Select {
					id: "orderby".into(),
					value: "views".into(),
				},
			],
		)
		.unwrap();
	assert!(!result.entries.is_empty(), "genre returned nothing");
	println!(
		"genre: {} entries, next: {}",
		result.entries.len(),
		result.has_next_page
	);
}
