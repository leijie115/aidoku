#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{FilterValue, Manga, alloc::vec};
use aidoku_test::aidoku_test;

type Src = MangaThemesia<KomikindoLive>;

fn details_chapters_pages(source: &Src, manga: Manga) {
	println!("manga: {} {}", manga.key, manga.title);
	let manga = source.get_manga_update(manga, true, true).unwrap();
	println!(
		"details: {} status={:?} viewer={:?} authors={:?} tags={:?} cover={:?}",
		manga.title, manga.status, manga.viewer, manga.authors, manga.tags, manga.cover
	);
	assert!(manga.description.is_some(), "no description");
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");
	let first = chapters.first().unwrap();
	println!(
		"chapters: {} first={} {:?} ch={:?} date={:?}",
		chapters.len(),
		first.key,
		first.title,
		first.chapter_number,
		first.date_uploaded
	);
	let pages = source.get_page_list(manga, first.clone()).unwrap();
	assert!(!pages.is_empty(), "no pages");
	println!("pages: {}", pages.len());
}

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Src::new();
	let result = source
		.get_search_manga_list(Some("money".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");
	details_chapters_pages(&source, result.entries.into_iter().next().unwrap());
}

#[aidoku_test]
fn filter_list() {
	let source = Src::new();
	let result = source
		.get_search_manga_list(
			None,
			1,
			vec![
				FilterValue::Sort {
					id: "order".into(),
					index: 3,
					ascending: false,
				},
				FilterValue::Select {
					id: "type".into(),
					value: "manhwa".into(),
				},
			],
		)
		.unwrap();
	assert!(!result.entries.is_empty(), "filter returned nothing");
	assert!(result.has_next_page);
	println!("filtered: {}", result.entries.len());
}
