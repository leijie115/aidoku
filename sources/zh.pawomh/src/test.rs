#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::alloc::vec;
use aidoku_test::aidoku_test;

fn check_manga(source: &Pawomh, manga: Manga) {
	println!("manga: {} {} {:?}", manga.key, manga.title, manga.cover);
	let manga = source.get_manga_update(manga, true, true).unwrap();
	println!(
		"details: {:?} {:?} {:?} {:?} {:?}",
		manga.authors, manga.tags, manga.status, manga.viewer, manga.description
	);
	assert!(!manga.title.is_empty());
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");
	let first = chapters.first().unwrap();
	println!(
		"chapters: {} (newest: {} {:?} #{:?} {:?})",
		chapters.len(),
		first.key,
		first.title,
		first.chapter_number,
		first.date_uploaded
	);

	let pages = source
		.get_page_list(manga, chapters.into_iter().last().unwrap())
		.unwrap();
	assert!(!pages.is_empty(), "no pages");
	println!("pages: {}", pages.len());
}

#[aidoku_test]
fn browse_details_chapters_pages() {
	let source = Pawomh::new();
	let result = source.get_search_manga_list(None, 1, vec![]).unwrap();
	assert!(!result.entries.is_empty(), "booklist returned nothing");
	assert!(result.has_next_page);
	println!("booklist: {}", result.entries.len());
	check_manga(&source, result.entries.into_iter().next().unwrap());
}

#[aidoku_test]
fn search() {
	let source = Pawomh::new();
	let result = source
		.get_search_manga_list(Some("LINDA".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");
	println!("search: {}", result.entries.len());
	check_manga(&source, result.entries.into_iter().next().unwrap());
}

#[aidoku_test]
fn filters_and_listing() {
	let source = Pawomh::new();
	let result = source
		.get_search_manga_list(
			None,
			2,
			vec![
				FilterValue::Select {
					id: "area".into(),
					value: "1".into(),
				},
				FilterValue::Select {
					id: "end".into(),
					value: "1".into(),
				},
			],
		)
		.unwrap();
	assert!(
		!result.entries.is_empty(),
		"filtered booklist returned nothing"
	);
	println!("filtered: {}", result.entries.len());

	let result = source
		.get_manga_list(
			Listing {
				id: "update".into(),
				name: "最近更新".into(),
				..Default::default()
			},
			1,
		)
		.unwrap();
	assert!(
		!result.entries.is_empty(),
		"update listing returned nothing"
	);
	println!("update: {}", result.entries.len());

	let link = source
		.handle_deep_link(format!("{BASE_URL}/manhuabook/10687"))
		.unwrap();
	assert!(matches!(link, Some(DeepLinkResult::Manga { key }) if key == "10687"));
}
