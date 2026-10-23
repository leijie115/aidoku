#![no_std]
use aidoku::{
	PageContext, Result, Source, Viewer, alloc::String, imports::net::Request, prelude::*,
};
use madara::{Impl, Madara, Params};

const BASE_URL: &str = "https://mangaforfree.com";

struct Mangaforfreecom;

impl Impl for Mangaforfreecom {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			default_viewer: Viewer::Webtoon,
			..Default::default()
		}
	}

	fn get_image_request(
		&self,
		params: &Params,
		url: String,
		_context: Option<PageContext>,
	) -> Result<Request> {
		// some image urls are served over http, but the site only works over https
		let url = url.replace("http://", "https://");
		Ok(Request::get(url)?.header("Referer", &format!("{}/", params.base_url)))
	}
}

register_source!(
	Madara<Mangaforfreecom>,
	Home,
	DeepLinkHandler,
	ImageRequestProvider
);

#[cfg(test)]
mod test;
