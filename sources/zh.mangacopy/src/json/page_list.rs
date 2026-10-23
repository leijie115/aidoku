use aidoku::{
	AidokuError, Page, PageContent, Result,
	alloc::{String, format},
	error,
	imports::defaults::defaults_get,
	serde::Deserialize,
};

#[derive(Deserialize)]
pub struct Item {
	url: String,
}

impl TryFrom<Item> for Page {
	type Error = AidokuError;

	fn try_from(page_item: Item) -> Result<Self> {
		let quality = defaults_get_string("image.quality").unwrap_or_else(|_| "c1500x".into());
		let format = defaults_get_string("image.format").unwrap_or_else(|_| "webp".into());
		let url = page_item
			.url
			.rsplit_once('c')
			.map(|(url, _)| format!("{url}{quality}.{format}"))
			.unwrap_or(page_item.url);
		let content = PageContent::Url(url, None);
		Ok(Self {
			content,
			..Default::default()
		})
	}
}

fn defaults_get_string(key: &str) -> Result<String> {
	defaults_get(key).ok_or_else(|| error!("Default not exist or not string"))
}
