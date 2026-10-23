use super::*;
use aidoku_test::aidoku_test;

#[aidoku_test]
fn recent_update_returns_entries() {
	let source = KomiicSource;
	let result = source
		.get_manga_list(
			Listing {
				id: String::from("recent_update"),
				name: String::from("最近更新"),
				..Default::default()
			},
			1,
		)
		.expect("recent update request should succeed");

	assert!(
		!result.entries.is_empty(),
		"recent update should return entries"
	);
}

#[aidoku_test]
fn home_returns_components() {
	let source = KomiicSource;
	let result = source.get_home().expect("home request should succeed");

	assert!(
		result.components.len() >= 3,
		"home should return multiple components"
	);
}

#[aidoku_test]
fn sort_filter_uses_index_value() {
	let filters = Vec::from([FilterValue::Sort {
		id: String::from("sort"),
		index: 2,
		ascending: false,
	}]);

	let (order_by, _, _, _, _) = KomiicSource::parse_search_filters(None, &filters);

	assert_eq!(order_by, "FAVORITE_COUNT");
}

#[aidoku_test]
fn author_filter_uses_author_search() {
	let filters = Vec::from([FilterValue::Text {
		id: String::from("author"),
		value: String::from("米二"),
	}]);

	let (_, _, _, keyword, author) = KomiicSource::parse_search_filters(None, &filters);

	assert_eq!(keyword, None);
	assert_eq!(author.as_deref(), Some("米二"));
}

#[aidoku_test]
fn author_search_returns_author_works() {
	let source = KomiicSource;
	let result = source
		.get_search_manga_list(
			None,
			1,
			Vec::from([FilterValue::Text {
				id: String::from("author"),
				value: String::from("米二"),
			}]),
		)
		.expect("author search should succeed");

	assert!(
		result.entries.iter().any(|entry| entry.title == "一人之下"),
		"author search should use Komiic author results"
	);
}

#[aidoku_test]
fn manga_list_reports_raw_count_before_deduplication() {
	let value = serde_json::json!({
		"data": {
			"comics": [
				{ "id": "1", "title": "A" },
				{ "id": "1", "title": "A duplicate" }
			]
		}
	});

	let (entries, raw_count) =
		KomiicSource::parse_manga_list_with_raw_count(&value, &["data", "comics"])
			.expect("list should parse");

	assert_eq!(raw_count, 2);
	assert_eq!(entries.len(), 1);
}

#[aidoku_test]
fn manga_list_uses_minimal_fields() {
	let value = serde_json::json!({
		"data": {
			"comics": [
				{
					"id": "1",
					"title": "A",
					"status": "ONGOING",
					"imageUrl": "https://example.com/cover.jpg",
					"description": "detail-only",
					"authors": [
						{ "name": "作者A" },
						{ "name": "作者B" }
					],
					"categories": [
						{ "name": "熱血" }
					]
				}
			]
		}
	});

	let (entries, _) = KomiicSource::parse_manga_list_with_raw_count(&value, &["data", "comics"])
		.expect("list should parse");

	let manga = &entries[0];
	assert_eq!(manga.key, "1");
	assert_eq!(manga.title, "A");
	assert_eq!(
		manga.cover.as_deref(),
		Some("https://example.com/cover.jpg")
	);
	assert_eq!(manga.url.as_deref(), Some("https://komiic.com/comic/1"));
	assert_eq!(manga.status, MangaStatus::Ongoing);
	assert_eq!(manga.content_rating, ContentRating::Safe);
	assert_eq!(manga.viewer, Viewer::RightToLeft);
	assert_eq!(manga.authors, None);
	assert_eq!(manga.description, None);
	assert_eq!(manga.tags, None);
}

#[aidoku_test]
fn home_manga_list_preserves_authors_and_tags() {
	let value = serde_json::json!({
		"data": {
			"comics": [
				{
					"id": "1",
					"title": "A",
					"authors": [
						{ "name": "作者A" },
						{ "name": "作者B" }
					],
					"categories": [
						{ "name": "熱血" },
						{ "name": "冒險" }
					]
				}
			]
		}
	});

	let (entries, _) =
		KomiicSource::parse_manga_list_with_raw_count_and_mode(&value, &["data", "comics"], false)
			.expect("home list should parse");

	assert_eq!(
		entries[0].authors,
		Some(Vec::from([String::from("作者A"), String::from("作者B")]))
	);
	assert_eq!(
		entries[0].tags,
		Some(Vec::from([String::from("熱血"), String::from("冒險")]))
	);
}

#[aidoku_test]
fn recommendations_require_login() {
	let source = KomiicSource;

	let result = source.get_manga_list(
		Listing {
			id: String::from("recommendations"),
			name: String::from("个性化推荐"),
			..Default::default()
		},
		1,
	);

	assert!(result.is_err(), "recommendations should require login");
}

#[aidoku_test]
fn category_listing_returns_entries() {
	let source = KomiicSource;
	let result = source
		.get_manga_list(
			Listing {
				id: String::from("category:1:DATE_UPDATED"),
				name: String::from("愛情"),
				..Default::default()
			},
			1,
		)
		.expect("category listing request should succeed");

	assert!(
		!result.entries.is_empty(),
		"category listing should return entries"
	);
}

#[aidoku_test]
fn detail_returns_description_and_clean_chapter_titles() {
	let source = KomiicSource;
	defaults_set(
		CHAPTER_FILTER_KEY,
		DefaultValue::String(String::from("book")),
	);
	let manga = source
		.get_manga_update(
			Manga {
				key: String::from("2100"),
				..Default::default()
			},
			true,
			true,
		)
		.expect("detail request should succeed");

	let description = manga
		.description
		.expect("detail should include description");
	assert!(
		description.contains("阿波羅"),
		"description should come from comicById"
	);

	let chapters = manga.chapters.expect("detail should include chapters");
	assert!(
		chapters
			.iter()
			.all(|chapter| chapter.volume_number.is_some() && chapter.chapter_number.is_none()),
		"default chapter list should only include volumes"
	);
	assert!(
		chapters
			.iter()
			.any(|chapter| chapter.volume_number == Some(9.0)),
		"expected known volume 9"
	);
	assert!(
		chapters
			.iter()
			.filter(|chapter| chapter.chapter_number.is_some() || chapter.volume_number.is_some())
			.all(|chapter| chapter.title.is_none()),
		"numeric serials should not be repeated as chapter titles"
	);
}

#[aidoku_test]
fn iso_dates_are_parsed_as_utc() {
	assert_eq!(parse_iso_date("2023-09-20T17:39:43Z"), Some(1_695_231_583));
	assert_eq!(parse_iso_date("1970-01-01T00:00:00Z"), Some(0));
	assert_eq!(parse_iso_date("bogus"), None);
}

#[aidoku_test]
fn chapter_version_selection_matches_preference() {
	let books = Vec::from([Chapter {
		key: String::from("book-1"),
		volume_number: Some(1.0),
		..Default::default()
	}]);
	let web_chapters = Vec::from([Chapter {
		key: String::from("chapter-1"),
		chapter_number: Some(1.0),
		..Default::default()
	}]);

	let selected =
		KomiicSource::select_chapter_version(books.clone(), web_chapters.clone(), "prefer_book");
	assert_eq!(selected[0].key, "book-1");

	let selected =
		KomiicSource::select_chapter_version(books.clone(), web_chapters.clone(), "chapter");
	assert_eq!(selected.len(), 1);
	assert_eq!(selected[0].key, "chapter-1");

	let selected = KomiicSource::select_chapter_version(books.clone(), web_chapters.clone(), "all");
	assert_eq!(selected.len(), 2);
	assert_eq!(selected[0].key, "chapter-1");

	let selected = KomiicSource::select_chapter_version(Vec::new(), web_chapters, "prefer_book");
	assert_eq!(selected[0].key, "chapter-1");

	let selected = KomiicSource::select_chapter_version(books, Vec::new(), "book");
	assert_eq!(selected[0].key, "book-1");
}

#[aidoku_test]
fn search_details_chapters_pages() {
	let source = KomiicSource;
	defaults_set(
		CHAPTER_FILTER_KEY,
		DefaultValue::String(String::from("all")),
	);
	let result = source
		.get_search_manga_list(Some(String::from("一人之下")), 1, Vec::new())
		.expect("search should succeed");
	let manga = result
		.entries
		.into_iter()
		.next()
		.expect("search should return entries");
	println!("manga: {} {}", manga.key, manga.title);

	let manga = source
		.get_manga_update(manga, true, true)
		.expect("detail request should succeed");
	println!(
		"details: status={:?} authors={:?} tags={:?}",
		manga.status, manga.authors, manga.tags
	);
	let chapters = manga.chapters.clone().expect("chapters");
	assert!(!chapters.is_empty(), "no chapters");
	let first = chapters.first().expect("first chapter");
	println!(
		"chapters: {} first={} ch={:?} vol={:?} date={:?} scanlators={:?}",
		chapters.len(),
		first.key,
		first.chapter_number,
		first.volume_number,
		first.date_uploaded,
		first.scanlators
	);
	assert!(first.date_uploaded.is_some(), "missing chapter date");

	let chapter = chapters.into_iter().next().expect("chapter");
	let pages = source
		.get_page_list(manga, chapter)
		.expect("page list should succeed");
	assert!(!pages.is_empty(), "no pages");
	println!("pages: {}", pages.len());
}
