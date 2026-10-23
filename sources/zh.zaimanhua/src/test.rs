#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::alloc::vec;
use aidoku_test::aidoku_test;

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Zaimanhua::new();

	let result = source
		.get_search_manga_list(Some("大理寺日志".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");

	let manga = result.entries.into_iter().next().unwrap();
	println!("manga: {} {}", manga.key, manga.title);
	let manga = match source.get_manga_update(manga, true, true) {
		Ok(m) => m,
		Err(e) => panic!("{e:?}"),
	};
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
fn listing() {
	let source = Zaimanhua::new();
	let result = source
		.get_manga_list(
			Listing {
				id: "latest".into(),
				name: "更新".into(),
				..Default::default()
			},
			1,
		)
		.unwrap();
	assert!(!result.entries.is_empty(), "listing returned nothing");
}
