#![expect(clippy::unwrap_used)]

use super::*;
use aidoku_test::aidoku_test;

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Rouman5::new();

	let result = source
		.get_search_manga_list(Some("越操越强".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");

	let manga = result.entries.into_iter().next().unwrap();
	println!("manga: {} {} {:?}", manga.key, manga.title, manga.cover);
	let manga = source.get_manga_update(manga, true, true).unwrap();
	println!(
		"details: {:?} {:?} {:?} {:?}",
		manga.authors, manga.tags, manga.status, manga.description
	);
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");
	println!("chapters: {}", chapters.len());
	for chapter in chapters.iter().take(2).chain(chapters.iter().rev().take(1)) {
		println!(
			"  {} {:?} {:?} {:?}",
			chapter.key, chapter.title, chapter.chapter_number, chapter.date_uploaded
		);
	}

	let pages = source
		.get_page_list(manga, chapters.into_iter().next().unwrap())
		.unwrap();
	assert!(!pages.is_empty(), "no pages");
	println!("pages: {}", pages.len());
	let PageContent::Url(url, _) = &pages[0].content else {
		panic!("unexpected page content");
	};
	let image = get(url).unwrap().image().unwrap();
	println!("page image: {url} {}x{}", image.width(), image.height());
	assert!(image.width() > 0.0);
}

#[aidoku_test]
fn filters() {
	let source = Rouman5::new();
	let result = source
		.get_search_manga_list(
			None,
			2,
			vec![FilterValue::Sort {
				id: "sort".into(),
				index: 1,
				ascending: false,
			}],
		)
		.unwrap();
	println!(
		"popular: {} next {}",
		result.entries.len(),
		result.has_next_page
	);
	assert!(!result.entries.is_empty());

	let result = source
		.get_search_manga_list(
			None,
			1,
			vec![
				FilterValue::Text {
					id: "tag".into(),
					value: "人妻".into(),
				},
				FilterValue::Select {
					id: "continued".into(),
					value: "true".into(),
				},
			],
		)
		.unwrap();
	println!(
		"tag: {} next {}",
		result.entries.len(),
		result.has_next_page
	);
	assert!(!result.entries.is_empty());

	let slices = scramble_slices(
		"https://v1.kelv47.xyz/m/9WDX4hnyX7GauscwivOrN67yZzNvPEK18QwxXXrwYgY/wm:0/sr:1/czM6Ly9yb3VtYW4vaW1hZ2VzL2NtY3d0ZjRsYjAwMDNzNm1paTh0MGg0MWEvMThjLzE0ODQ3NDYvMDAwMDEud2VicA.webp",
	);
	println!("slices: {slices:?}");
	assert_eq!(slices, Some(9));
	assert!(scramble_slices("https://v1.kelv47.xyz/m/abc/wm:0/sr:0/abc.jpg").is_none());
}
