#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{FilterValue, alloc::vec};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn listing_details_chapters_pages() {
	let source = Bilimanga::new();

	let result = source
		.get_manga_list(
			Listing {
				id: "weekvisit".into(),
				name: "周點擊榜".into(),
				..Default::default()
			},
			1,
		)
		.unwrap();
	assert!(!result.entries.is_empty(), "listing returned nothing");
	println!("listing: {} entries", result.entries.len());

	let filtered = source
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::Sort {
				id: "排序".into(),
				index: 0,
				ascending: false,
			}],
		)
		.unwrap();
	assert!(!filtered.entries.is_empty(), "filter returned nothing");
	println!(
		"filter: {} entries, next={}",
		filtered.entries.len(),
		filtered.has_next_page
	);

	let manga = Manga {
		key: "1714".into(),
		..Default::default()
	};
	let manga = source.get_manga_update(manga, true, true).unwrap();
	println!(
		"manga: {} status={:?} viewer={:?} tags={:?}",
		manga.title, manga.status, manga.viewer, manga.tags
	);
	println!("desc: {:?}", manga.description);
	assert!(!manga.title.is_empty());
	assert!(
		!manga.tags.clone().unwrap_or_default().is_empty(),
		"no tags"
	);
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");
	println!(
		"chapters: {} first={:?} last={:?}",
		chapters.len(),
		chapters.first().map(|c| (&c.key, &c.title)),
		chapters.last().map(|c| (&c.key, &c.title))
	);
	assert!(
		chapters.iter().all(|c| c.key.parse::<u32>().is_ok()),
		"unresolved chapter key"
	);

	let pages = source
		.get_page_list(manga, chapters.into_iter().next().unwrap())
		.unwrap();
	assert!(!pages.is_empty(), "no pages");
	println!("pages: {}", pages.len());
}

#[aidoku_test]
fn search() {
	let source = Bilimanga::new();
	let result = source
		.get_search_manga_list(Some("葬送的芙莉蓮".into()), 1, vec![])
		.unwrap();
	println!(
		"search: {:?}",
		result
			.entries
			.iter()
			.map(|m| (&m.key, &m.title))
			.collect::<Vec<_>>()
	);
	assert!(!result.entries.is_empty(), "search returned nothing");
}
