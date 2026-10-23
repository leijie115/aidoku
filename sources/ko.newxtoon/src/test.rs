#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::alloc::vec;
use aidoku_test::aidoku_test;

#[aidoku_test]
fn helpers() {
	assert_eq!(
		parse_chapter_number("제215화 최종화 가인&예린"),
		Some(215.0)
	);
	assert_eq!(parse_chapter_number("12화"), Some(12.0));
	assert_eq!(parse_chapter_number("[후기]"), None);
	assert_eq!(
		origin("https://newxtoon3.com/"),
		Some("https://newxtoon3.com".into())
	);
	assert_eq!(
		comic_id("https://newxtoon3.com/comics/5261/chapters/1"),
		Some("5261".into())
	);
}

#[aidoku_test]
fn listing_and_filters() {
	let source = Newxtoon::new();
	let popular = source
		.get_manga_list(
			Listing {
				id: "popular".into(),
				..Default::default()
			},
			1,
		)
		.unwrap();
	assert!(!popular.entries.is_empty(), "popular listing empty");
	assert!(popular.has_next_page);
	println!(
		"popular: {} first={}",
		popular.entries.len(),
		popular.entries[0].title
	);

	let filtered = source
		.get_search_manga_list(
			None,
			1,
			vec![
				FilterValue::Select {
					id: "category".into(),
					value: "".into(),
				},
				FilterValue::Select {
					id: "status".into(),
					value: "완결".into(),
				},
				FilterValue::Select {
					id: "genre".into(),
					value: "1".into(),
				},
			],
		)
		.unwrap();
	assert!(!filtered.entries.is_empty(), "filtered list empty");
	println!("filtered: {}", filtered.entries.len());
}

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Newxtoon::new();

	let result = source
		.get_search_manga_list(Some("동아리".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");
	let manga = result
		.entries
		.into_iter()
		.find(|m| m.title == "동아리")
		.expect("expected title not found");
	println!("manga: {} {}", manga.key, manga.title);

	let manga = source.get_manga_update(manga, true, true).unwrap();
	println!(
		"details: status={:?} rating={:?} authors={:?} tags={:?} cover={:?}",
		manga.status, manga.content_rating, manga.authors, manga.tags, manga.cover
	);
	assert!(manga.description.is_some());
	assert_eq!(manga.status, MangaStatus::Completed);
	let chapters = manga.chapters.clone().unwrap();
	assert!(chapters.len() > 20, "chapter pagination failed");
	println!(
		"chapters: {} first={:?} #{:?} date={:?}",
		chapters.len(),
		chapters[0].title,
		chapters[1].chapter_number,
		chapters[0].date_uploaded
	);
	assert!(chapters[0].date_uploaded.is_some());

	let pages = source
		.get_page_list(manga, chapters.into_iter().nth(1).unwrap())
		.unwrap();
	assert!(!pages.is_empty(), "no pages");
	println!("pages: {}", pages.len());
}
