#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{alloc::vec, imports::html::Html};
use aidoku_test::aidoku_test;

// Trimmed markup captured from manga18.club in a browser (the site is behind a
// Cloudflare challenge that blocks non-browser clients such as the test runner).
const LIST_HTML: &str = r#"<html><body>
<div class="story_item"> <div class="story_images"> <a href="https://manga18.club/manhwa/the-most-shameless-guy-girl-friendship-ever-official" title="The Most Shameless Guy-Girl Friendship Ever (Official)"><img src="https://cdn.manga18.club/manga/the-most-shameless-guy-girl-friendship-ever-official/cover/cover_thumb_2.webp" alt="" class="img-responsive"></a> </div> <div class="mg_info"> <div class="mg_name"> <a href="https://manga18.club/manhwa/the-most-shameless-guy-girl-friendship-ever-official">The Most Shameless Guy-Girl Friendship Ever (Official)</a> </div> <div class="mg_chapter"> <div class="item"> <div class="chapter_count"><a href="/manhwa/the-most-shameless-guy-girl-friendship-ever-official/chapter-47">Chapter 47</a></div> <div class="chapter_time"><span>46 minutes ago</span></div> </div> </div> </div> </div>
<ul class="pagination"> <li class="active"><a href="/list-manga/1">1</a></li> <li class=""><a href="/list-manga/2">2</a></li> <li><a href="/list-manga/2">›</a></li> <li><a href="/list-manga/166">»</a></li> </ul>
</body></html>"#;

const LAST_PAGE_HTML: &str = r#"<html><body>
<ul class="pagination"> <li><a href="/list-manga/165">‹</a></li> <li class=""><a href="/list-manga/165">165</a></li> <li class="active"><a href="/list-manga/166">166</a></li> </ul>
</body></html>"#;

const DETAIL_HTML: &str = r#"<html><body>
<div class="detail_name"> <h1>The Most Shameless Guy-Girl Friendship Ever (Official)</h1> </div>
<div class="detail_avatar"> <img src="https://cdn.manga18.club/manga/the-most-shameless-guy-girl-friendship-ever-official/cover/cover_250x350.jpg" alt="" class="img-responsive"> </div>
<div class="detail_reviewContent"> The Most Shameless Guy-Girl Friendship Ever (Official) There’s never been a guy-girl friendship this wild! </div>
<div class="detail_listInfo"> <div class="item"> <div class="info_label"><i class="lnr lnr-tag"></i> Other name</div> <div class="info_value"><span>MangMoongMing</span></div> </div> <div class="item"> <div class="info_label"><i class="lnr lnr-user"></i> Author</div> <div class="info_value"><a href="/list-manga">MangMoongMing</a></div> </div> <div class="item"> <div class="info_label"><i class="lnr lnr-user"></i> Artist</div> <div class="info_value"><a href="/list-manga">Updating</a></div> </div> <div class="item"> <div class="info_label"><i class="ti-rss-alt"></i> Status</div> <div class="info_value"> <span class="label label-success">On Going</span> </div> </div> <div class="item"> <div class="info_label"><i class="lnr lnr-tag"></i> Categories</div> <div class="info_value"> <a href="https://manga18.club/manga-list/drama">Drama</a> - <a href="https://manga18.club/manga-list/slice-of-life">Slice of Life</a> </div> </div> <div class="item"> <div class="info_label"><i class="lnr lnr-eye"></i> Views</div> <div class="info_value"><span>315,253</span></div> </div> </div>
<div class="chapter_box"> <ul> <li class=""> <div class="item"> <a href="/manhwa/the-most-shameless-guy-girl-friendship-ever-official/chapter-47" class="chapter_num">Chapter 47 </a> <p class="chapter_info">23-10-2026</p> <p class="chapter_info">72</p> <p class="chapter_info hide"> <a href="https://azmin.manga18.club/download/the-most-shameless-guy-girl-friendship-ever-official/168235"> <i class="glyphicon glyphicon-download-alt"></i> </a> </p> </div> </li> <li class=""> <div class="item"> <a href="/manhwa/the-most-shameless-guy-girl-friendship-ever-official/chapter-46" class="chapter_num">Chapter 46 </a> <p class="chapter_info">23-10-2026</p> <p class="chapter_info">46</p> <p class="chapter_info hide"> <a href="https://azmin.manga18.club/download/the-most-shameless-guy-girl-friendship-ever-official/168236"> <i class="glyphicon glyphicon-download-alt"></i> </a> </p> </div> </li> <li class="hide"> <div class="item"> <a href="/manhwa/the-most-shameless-guy-girl-friendship-ever-official/chapter-1" class="chapter_num">Chapter 1 </a> <p class="chapter_info">02-03-2026</p> <p class="chapter_info">37,405</p> <p class="chapter_info hide"> <a href="https://azmin.manga18.club/download/the-most-shameless-guy-girl-friendship-ever-official/143784"> <i class="glyphicon glyphicon-download-alt"></i> </a> </p> </div> </li> </ul> </div>
</body></html>"#;

const READER_SCRIPT: &str = r#"
  var slides_page;
  var slides_p_path = ["aHR0cHM6Ly9jZG4ubWFuZ2ExOC5jbHViL21hbmdhL3RoZS1tb3N0LXNoYW1lbGVzcy1ndXktZ2lybC1mcmllbmRzaGlwLWV2ZXItb2ZmaWNpYWwvY2hhcHRlcnMvY2hhcHRlci00Ny8wMS5qcGc=","aHR0cHM6Ly9jZG4ubWFuZ2ExOC5jbHViL21hbmdhL3RoZS1tb3N0LXNoYW1lbGVzcy1ndXktZ2lybC1mcmllbmRzaGlwLWV2ZXItb2ZmaWNpYWwvY2hhcHRlcnMvY2hhcHRlci00Ny8wMi5qcGc=",];
  $(document)['ready'](function() {});
"#;

#[aidoku_test]
fn parse_list_fixture() {
	let html = Html::parse_with_url(LIST_HTML, BASE_URL).unwrap();
	let result = parse_manga_list(&html);
	assert_eq!(result.entries.len(), 1);
	let manga = &result.entries[0];
	assert_eq!(
		manga.key,
		"/manhwa/the-most-shameless-guy-girl-friendship-ever-official"
	);
	assert_eq!(
		manga.title,
		"The Most Shameless Guy-Girl Friendship Ever (Official)"
	);
	assert!(
		manga
			.cover
			.as_deref()
			.unwrap()
			.ends_with("cover_thumb_2.webp")
	);
	assert!(result.has_next_page);

	let html = Html::parse_with_url(LAST_PAGE_HTML, BASE_URL).unwrap();
	assert!(!parse_manga_list(&html).has_next_page);
}

#[aidoku_test]
fn parse_details_fixture() {
	let html = Html::parse_with_url(DETAIL_HTML, BASE_URL).unwrap();
	let manga = parse_details(
		&html,
		Manga {
			key: "/manhwa/the-most-shameless-guy-girl-friendship-ever-official".into(),
			..Default::default()
		},
	)
	.unwrap();
	println!("{manga:?}");
	assert_eq!(
		manga.title,
		"The Most Shameless Guy-Girl Friendship Ever (Official)"
	);
	assert_eq!(manga.status, MangaStatus::Ongoing);
	assert_eq!(manga.authors, Some(vec!["MangMoongMing".into()]));
	assert_eq!(manga.artists, None);
	assert_eq!(
		manga.tags,
		Some(vec!["Drama".into(), "Slice of Life".into()])
	);
	assert!(
		manga
			.description
			.as_deref()
			.unwrap()
			.ends_with("Alternative Names:\nMangMoongMing")
	);
	assert!(
		manga
			.cover
			.as_deref()
			.unwrap()
			.ends_with("cover_250x350.jpg")
	);

	let chapters = parse_chapters(&html);
	println!("{:?}", chapters[0]);
	assert_eq!(chapters.len(), 3);
	assert_eq!(
		chapters[0].key,
		"/manhwa/the-most-shameless-guy-girl-friendship-ever-official/chapter-47"
	);
	assert_eq!(chapters[0].title.as_deref(), Some("Chapter 47"));
	assert_eq!(chapters[0].chapter_number, Some(47.0));
	assert!(chapters[0].date_uploaded.is_some());
	assert_eq!(chapters[2].chapter_number, Some(1.0));
}

#[aidoku_test]
fn parse_pages_fixture() {
	let urls = parse_image_urls(READER_SCRIPT);
	assert_eq!(
		urls,
		vec![
			String::from(
				"https://cdn.manga18.club/manga/the-most-shameless-guy-girl-friendship-ever-official/chapters/chapter-47/01.jpg"
			),
			String::from(
				"https://cdn.manga18.club/manga/the-most-shameless-guy-girl-friendship-ever-official/chapters/chapter-47/02.jpg"
			),
		]
	);
	assert_eq!(parse_chapter_number("Chapter 12.5 - End"), Some(12.5));
}

#[aidoku_test]
fn deep_links() {
	let source = Manga18Club::new();
	assert_eq!(
		source
			.handle_deep_link("https://manga18.club/manhwa/foo-bar/chapter-3".into())
			.unwrap(),
		Some(DeepLinkResult::Chapter {
			manga_key: "/manhwa/foo-bar".into(),
			key: "/manhwa/foo-bar/chapter-3".into(),
		})
	);
	assert_eq!(
		source
			.handle_deep_link("https://manga18.club/manhwa/foo-bar".into())
			.unwrap(),
		Some(DeepLinkResult::Manga {
			key: "/manhwa/foo-bar".into(),
		})
	);
}

/// Live test against the real site. manga18.club sits behind a Cloudflare
/// challenge (HTTP 403 to non-browser clients), so this is skipped when the
/// challenge page is returned instead of the site.
#[aidoku_test]
fn live_search_details_chapters_pages() {
	let source = Manga18Club::new();

	let html = request(&format!("{BASE_URL}/list-manga/1"))
		.unwrap()
		.html()
		.unwrap();
	if html.select_first("div.story_item").is_none() {
		let title = html
			.select_first("title")
			.and_then(|t| t.text())
			.unwrap_or_default();
		println!("SKIPPED live test: site returned challenge page ({title})");
		return;
	}

	let result = source
		.get_search_manga_list(Some("friendship".into()), 1, vec![])
		.unwrap();
	assert!(!result.entries.is_empty(), "search returned nothing");
	let manga = result.entries.into_iter().next().unwrap();
	let manga = source.get_manga_update(manga, true, true).unwrap();
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty(), "no chapters");
	let pages = source
		.get_page_list(manga, chapters.into_iter().next().unwrap())
		.unwrap();
	assert!(!pages.is_empty(), "no pages");
	println!("pages: {}", pages.len());
}
