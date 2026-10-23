#![no_std]
use aidoku::{Source, prelude::*};
use goda::{Backend, GoDa, Impl, Params};

struct GoDaManhua;

impl Impl for GoDaManhua {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: "https://baozimh.org".into(),
			backend: Backend::Api {
				api_url: "https://api-get-v3.mgsearcher.com".into(),
				image_url: "https://c-nd2-1.6wm.top".into(),
			},
			nsfw: false,
		}
	}
}

register_source!(
	GoDa<GoDaManhua>,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
