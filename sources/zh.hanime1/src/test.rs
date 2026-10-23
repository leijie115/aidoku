#![expect(clippy::unwrap_used)]

use super::*;
use aidoku_test::aidoku_test;

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Hanime1::new();

	let result = source
		.get_search_manga_list(Some("NTR".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");
	println!(
		"search: {} next: {}",
		result.entries.len(),
		result.has_next_page
	);

	let manga = result.entries.into_iter().next().unwrap();
	println!("manga: {} {} {:?}", manga.key, manga.title, manga.cover);
	let manga = source.get_manga_update(manga, true, true).unwrap();
	println!("authors: {:?} tags: {:?}", manga.authors, manga.tags);
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
fn listings() {
	let source = Hanime1::new();
	for id in ["latest", "popular"] {
		let result = source
			.get_manga_list(
				Listing {
					id: id.into(),
					..Default::default()
				},
				1,
			)
			.unwrap();
		assert!(!result.entries.is_empty(), "listing {id} returned nothing");
	}
}
