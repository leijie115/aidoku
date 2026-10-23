#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{FilterValue, Listing, ListingProvider, Manga, alloc::vec};
use aidoku_test::aidoku_test;

type Src = GoDa<GoDaManhua>;

fn details_chapters_pages(source: &Src, manga: Manga) {
	println!("manga: {} {}", manga.key, manga.title);
	let manga = source.get_manga_update(manga, true, true).unwrap();
	println!(
		"details: {} status={:?} viewer={:?} authors={:?} tags={:?} cover={:?}\n{:?}",
		manga.title,
		manga.status,
		manga.viewer,
		manga.authors,
		manga.tags,
		manga.cover,
		manga.description
	);
	assert!(!manga.title.is_empty(), "no title");
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");
	let first = chapters.first().unwrap();
	println!(
		"chapters: {} first={} {:?} ch={:?} date={:?} url={:?}",
		chapters.len(),
		first.key,
		first.title,
		first.chapter_number,
		first.date_uploaded,
		first.url
	);
	let pages = source.get_page_list(manga, first.clone()).unwrap();
	assert!(!pages.is_empty(), "no pages");
	println!(
		"pages: {} first={:?}",
		pages.len(),
		pages.first().unwrap().content
	);
}

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Src::new();
	let result = source
		.get_search_manga_list(Some("武炼巅峰".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");
	details_chapters_pages(&source, result.entries.into_iter().next().unwrap());
}

#[aidoku_test]
fn listing_and_filter() {
	let source = Src::new();
	let result = source
		.get_manga_list(
			Listing {
				id: "hots".into(),
				..Default::default()
			},
			1,
		)
		.unwrap();
	assert!(!result.entries.is_empty(), "listing returned nothing");
	assert!(result.has_next_page);
	let first = result.entries.first().unwrap();
	println!(
		"hots: {} first={} {} {:?}",
		result.entries.len(),
		first.key,
		first.title,
		first.cover
	);

	let result = source
		.get_search_manga_list(
			None,
			2,
			vec![FilterValue::Select {
				id: "genre".into(),
				value: "manga-genre/hots".into(),
			}],
		)
		.unwrap();
	assert!(!result.entries.is_empty(), "filter returned nothing");
	println!("genre: {}", result.entries.len());
}
