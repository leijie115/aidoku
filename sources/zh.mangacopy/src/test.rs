use aidoku::alloc::String;

#[aidoku_test::aidoku_test]
fn collect_button_page_extracts_nonempty_uuid() {
	use crate::html::CollectButtonPage as _;
	use aidoku::imports::html::Html;

	let document = Html::parse(r#"<button onclick="collect('comic-uuid')"></button>"#).unwrap();
	assert_eq!(document.collect_uuid(), Some(String::from("comic-uuid")));

	let empty = Html::parse(r#"<button onclick="collect('')"></button>"#).unwrap();
	assert_eq!(empty.collect_uuid(), None);

	let missing = Html::parse("<button></button>").unwrap();
	assert_eq!(missing.collect_uuid(), None);
}

#[aidoku_test::aidoku_test]
fn newest_page_reads_the_discovery_page_cards() {
	use crate::html::NewestPage as _;
	use aidoku::imports::html::Html;

	let document = Html::parse(
		r#"<div class="exemptComic_Item">
			<a href="/comic/new-comic"><img data-src="https://img.example/cover.jpg.328x422.jpg"></a>
			<div class="exemptComicItem-txt"><a href="/comic/new-comic"><p>全新作品</p></a><span><a>作者甲</a></span></div>
		</div><li class="page-all-item active"><a>1</a></li><li class="page-all-item"><a>2</a></li>"#,
	)
	.unwrap();
	let page = document.newest_manga_page_result().unwrap();
	assert_eq!(page.entries.len(), 1);
	assert_eq!(page.entries[0].key, "new-comic");
	assert_eq!(page.entries[0].title, "全新作品");
	assert_eq!(
		page.entries[0].cover.as_deref(),
		Some("https://img.example/cover.jpg")
	);
	assert_eq!(page.entries[0].authors, None);
	assert!(page.has_next_page);
}

#[aidoku_test::aidoku_test]
fn favorite_deep_links_require_a_known_operation_and_comic_id() {
	use crate::parse_fav_deep_link;

	assert_eq!(
		parse_fav_deep_link("https://www.copy5000.com/__fav/add/abc123"),
		Some((String::from("abc123"), true))
	);
	assert_eq!(
		parse_fav_deep_link("https://www.copy5000.com/__fav/remove/abc123"),
		Some((String::from("abc123"), false))
	);
	assert_eq!(
		parse_fav_deep_link("https://www.copy5000.com/__fav/add/"),
		None
	);
	assert_eq!(
		parse_fav_deep_link("https://www.copy5000.com/__fav/toggle/abc123"),
		None
	);
}

#[aidoku_test::aidoku_test]
fn live_search_details_chapters_pages() {
	use crate::MangaCopy;
	use aidoku::{Source as _, alloc::vec};

	let source = MangaCopy::new();
	let result = source
		.get_search_manga_list(Some("海贼王".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");
	let manga = result.entries.into_iter().next().unwrap();
	aidoku::println!("manga: {} {}", manga.key, manga.title);

	let manga = source.get_manga_update(manga, true, true).unwrap();
	aidoku::println!(
		"details: status={:?} viewer={:?} authors={:?} tags={:?}",
		manga.status,
		manga.viewer,
		manga.authors,
		manga.tags
	);
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");
	aidoku::println!(
		"chapters: {} first={:?}",
		chapters.len(),
		chapters
			.first()
			.map(|c| (&c.key, &c.title, c.chapter_number, &c.scanlators))
	);

	let pages = source
		.get_page_list(manga, chapters.into_iter().next().unwrap())
		.unwrap();
	assert!(!pages.is_empty(), "no pages");
	aidoku::println!(
		"pages: {} first={:?}",
		pages.len(),
		pages.first().map(|p| &p.content)
	);
}

#[aidoku_test::aidoku_test]
fn live_listing() {
	use crate::MangaCopy;
	use aidoku::{Listing, ListingProvider as _, Source as _};

	let source = MangaCopy::new();
	for id in ["update", "recent"] {
		let result = source
			.get_manga_list(
				Listing {
					id: id.into(),
					..Default::default()
				},
				1,
			)
			.unwrap();
		aidoku::println!(
			"{id}: {} next={}",
			result.entries.len(),
			result.has_next_page
		);
		assert!(!result.entries.is_empty(), "listing {id} empty");
	}
}

#[aidoku_test::aidoku_test]
fn live_home_and_filters() {
	use crate::MangaCopy;
	use aidoku::{DynamicFilters as _, Home as _, Source as _};

	let source = MangaCopy::new();
	let home = source.get_home().unwrap();
	aidoku::println!("home components: {}", home.components.len());
	assert!(!home.components.is_empty(), "empty home");
	let filters = source.get_dynamic_filters().unwrap();
	assert!(!filters.is_empty(), "no dynamic filters");
}
