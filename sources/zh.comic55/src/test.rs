#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::alloc::vec;
use aidoku_test::aidoku_test;

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Comic55::new();

	let result = source
		.get_search_manga_list(Some("七色".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");

	let manga = result.entries.into_iter().next().unwrap();
	println!("manga: {} {} {:?}", manga.key, manga.title, manga.cover);
	let cover = manga.cover.clone().unwrap();
	let manga = source.get_manga_update(manga, true, true).unwrap();
	println!(
		"details: {:?} {:?} {:?} {:?}",
		manga.authors, manga.tags, manga.status, manga.description
	);
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");
	println!(
		"chapters: {} first: {:?} {:?} {:?}",
		chapters.len(),
		chapters[0].key,
		chapters[0].title,
		chapters[0].date_uploaded
	);

	let pages = source
		.get_page_list(manga, chapters.into_iter().next().unwrap())
		.unwrap();
	assert!(!pages.is_empty(), "no pages");
	println!("pages: {}", pages.len());

	// decrypt the first page and the cover (canvas operations are not supported by the runner)
	let PageContent::Url(url, Some(context)) = &pages[0].content else {
		panic!("unexpected page content");
	};
	let mode = context.get("mode").unwrap();
	let mut chunks = vec![get(url).unwrap().data().unwrap()];
	for chunk in context.get("chunks").unwrap().split('\n') {
		chunks.push(get(chunk).unwrap().data().unwrap());
	}
	let data = image::decrypt_chunks(mode, chunks).unwrap();
	let (image, rows) = image::restore_header(data).unwrap();
	println!(
		"page image: {}x{} rows {rows}",
		image.width(),
		image.height()
	);
	assert!(image.width() > 0.0);

	let chunks = vec![
		get(&cover).unwrap().data().unwrap(),
		get(&cover.replacen(".b_0", ".b_1", 1))
			.unwrap()
			.data()
			.unwrap(),
	];
	let data = image::decrypt_chunks(MODE_LEGACY, chunks).unwrap();
	let (image, rows) = image::restore_header(data).unwrap();
	println!(
		"cover image: {}x{} rows {rows}",
		image.width(),
		image.height()
	);
	assert!(image.width() > 0.0);
}

#[aidoku_test]
fn filters_and_listing() {
	let source = Comic55::new();
	let result = source
		.get_search_manga_list(
			None,
			2,
			vec![
				FilterValue::Select {
					id: "tag".into(),
					value: "巨乳".into(),
				},
				FilterValue::Select {
					id: "area".into(),
					value: "2".into(),
				},
			],
		)
		.unwrap();
	println!(
		"filtered: {} next {}",
		result.entries.len(),
		result.has_next_page
	);
	assert!(!result.entries.is_empty());

	let rank = source
		.get_manga_list(
			Listing {
				id: "rank".into(),
				name: "排行榜".into(),
				..Default::default()
			},
			1,
		)
		.unwrap();
	println!("rank: {}", rank.entries.len());
	assert!(!rank.entries.is_empty());
}
