#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{ImageRequest, PageContent, alloc::vec, imports::net::Request};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = Jinmantiantang::new();

	let result = source
		.get_search_manga_list(Some("無修正".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");

	let manga = result.entries.into_iter().next().unwrap();
	println!("manga: {} {}", manga.key, manga.title);
	let manga = source.get_manga_update(manga, true, true).unwrap();
	println!("authors: {:?}, url: {:?}", manga.authors, manga.url);
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");
	println!("chapters: {}", chapters.len());

	let pages = source
		.get_page_list(manga, chapters.into_iter().next().unwrap())
		.unwrap();
	assert!(!pages.is_empty(), "no pages");
	println!("pages: {}", pages.len());
}

/// Fetches a scrambled page and runs it through the descrambler.
/// Note: the test runner's `copy_image` is a stub, so this only checks the
/// slice count and that processing succeeds with the original dimensions.
#[aidoku_test]
fn descramble_page() {
	let source = Jinmantiantang::new();
	let result = source
		.get_manga_list(
			Listing {
				id: "latest".into(),
				name: "最新上傳".into(),
				..Default::default()
			},
			1,
		)
		.unwrap();
	let manga = result.entries.into_iter().next().unwrap();
	let manga = source.get_manga_update(manga, false, true).unwrap();
	let chapter = manga.chapters.clone().unwrap().into_iter().next().unwrap();
	let page = source
		.get_page_list(manga, chapter)
		.unwrap()
		.into_iter()
		.next()
		.unwrap();
	let PageContent::Url(url, context) = page.content else {
		panic!("unexpected page content");
	};
	let ctx = context.unwrap();
	let ep_id: u64 = ctx.get("ep_id").unwrap().parse().unwrap();
	let rows = Jinmantiantang::calc_scramble_num(ep_id, ctx.get("filename").unwrap());
	println!("url: {url}, rows: {rows}");
	assert!(rows > 1, "page is not scrambled");

	let image = Request::get(&url).unwrap().image().unwrap();
	let (w, h) = (image.width(), image.height());
	let processed = source
		.process_page_image(
			ImageResponse {
				code: 200,
				headers: HashMap::new(),
				request: ImageRequest {
					url: Some(url),
					headers: HashMap::new(),
				},
				image,
			},
			Some(ctx),
		)
		.unwrap();
	assert_eq!((processed.width(), processed.height()), (w, h));
}
