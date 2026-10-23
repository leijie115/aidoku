#![no_std]
use aidoku::{
	Chapter, ContentRating, Manga, Page, Result, Source, Viewer,
	alloc::{String, Vec, string::ToString},
	helpers::uri::QueryParameters,
	imports::{defaults::defaults_get, html::Document, net::Request},
	prelude::*,
};
use madara::{Impl, LoadMoreStrategy, Madara, Params, helpers::extract_between};

// the site changes domains often, the current one is published through this endpoint
const DEFAULT_BASE_URL: &str = "https://aedexnox.syakaihoken-web.com";
const DOMAIN_API_URL: &str = "https://ysilhsqbtixygcgscvbb.supabase.co/rest/v1/parameters?select=value&name=eq.redirect_url_templescan";
const DOMAIN_API_KEY: &str = "sb_publishable_y5ZlqOnxowq6W7JTSZHSBQ_AQfHg77U";

struct TempleScanEsp;

// returns the domain set in the settings, or the default one
fn settings_base_url() -> String {
	defaults_get::<String>("url")
		.map(|url| url.trim().trim_end_matches('/').into())
		.filter(|url: &String| url.starts_with("http"))
		.unwrap_or_else(|| DEFAULT_BASE_URL.into())
}

// fetches the current domain from the site's domain api
fn fetch_base_url() -> Option<String> {
	let response = Request::get(DOMAIN_API_URL)
		.ok()?
		.header("apikey", DOMAIN_API_KEY)
		.header("Accept", "application/json")
		.string()
		.ok()?;
	let value: String = extract_between(&response, "\"value\":\"", "\"")?
		.replace("\\/", "/")
		.trim()
		.trim_end_matches('/')
		.into();
	if value.is_empty() {
		None
	} else if value.starts_with("http") {
		Some(value)
	} else {
		Some(format!("https://{value}"))
	}
}

impl Impl for TempleScanEsp {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		let fetch_domain = defaults_get::<bool>("fetchDomain").unwrap_or(true);
		let base_url = fetch_domain
			.then(fetch_base_url)
			.flatten()
			.unwrap_or_else(settings_base_url);
		Params {
			base_url: base_url.into(),
			source_path: "serie".into(),
			use_new_chapter_endpoint: true,
			use_style_images: true,
			use_load_more_request: LoadMoreStrategy::Always,
			default_viewer: Viewer::Webtoon,
			datetime_format: "MMMM dd, yyyy".into(),
			datetime_locale: "es".into(),
			search_manga_selector: "button.group > div.grid".into(),
			search_manga_url_selector: "a".into(),
			search_manga_title_selector: "h3".into(),
			search_manga_cover_selector: "div[style].bg-cover".into(),
			details_title_selector: "div.wp-manga div.grid > h1".into(),
			details_cover_selector: "div.wp-manga > div.grid div.grid > div[style].bg-cover".into(),
			details_status_selector: "div.wp-manga div[alt=type]:first-child > span".into(),
			details_tag_selector: "div.wp-manga div[alt=type] ~ div[alt=type] > span".into(),
			details_description_selector: "div.wp-manga div#expand_content".into(),
			chapter_selector: "ul#list-chapters li > a".into(),
			chapter_title_selector: "div.grid > span".into(),
			chapter_date_selector: "div.grid > div".into(),
			chapter_thumbnail_selector: "div[style].bg-cover".into(),
			..Default::default()
		}
	}

	fn get_page_list(&self, params: &Params, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{}{}", params.base_url, chapter.key);
		let html = Request::get(&url)?.html()?;

		// chapter pages may be behind a redirect form that has to be submitted to get the images
		let Some(form) = html.select_first("form#redirect-form") else {
			return self.parse_page_list(params, &html, url);
		};
		let action = form
			.attr("abs:action")
			.or_else(|| form.attr("action"))
			.ok_or_else(|| error!("Missing redirect form action"))?;
		let mut body = QueryParameters::new();
		if let Some(inputs) = form.select("input[name]") {
			for input in inputs {
				let Some(name) = input.attr("name") else {
					continue;
				};
				body.push(&name, Some(&input.attr("value").unwrap_or_default()));
			}
		}
		let redirected = Request::post(&action)?
			.header("Referer", &url)
			.header("Content-Type", "application/x-www-form-urlencoded")
			.body(body.to_string())
			.html()?;
		self.parse_page_list(params, &redirected, url)
	}

	fn get_manga_content_rating(&self, _html: &Document, _manga: &Manga) -> ContentRating {
		ContentRating::NSFW
	}
}

register_source!(
	Madara<TempleScanEsp>,
	DeepLinkHandler,
	ImageRequestProvider,
	BaseUrlProvider
);

#[cfg(test)]
mod test;
