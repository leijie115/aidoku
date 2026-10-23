#![no_std]
use aidoku::{Source, prelude::*};
use goda::{Backend, GoDa, Impl, Params};

struct EighteenMH;

impl Impl for EighteenMH {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: "https://18mh.org".into(),
			backend: Backend::Html,
			nsfw: true,
		}
	}
}

register_source!(
	GoDa<EighteenMH>,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
