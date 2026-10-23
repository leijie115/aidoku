#![no_std]
use aidoku::{Source, prelude::*};
use mangathemesia::{Impl, MangaThemesia, Params};

const BASE_URL: &str = "https://2.komikindo.shop";

struct KomikindoLive;

impl Impl for KomikindoLive {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			date_format: "MMMM d, yyyy".into(),
			..Default::default()
		}
	}
}

register_source!(
	MangaThemesia<KomikindoLive>,
	Home,
	ImageRequestProvider,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
