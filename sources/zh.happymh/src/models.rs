use aidoku::{
	Manga, MangaPageResult,
	alloc::{String, Vec},
	prelude::*,
};
use serde::Deserialize;

const BASE_URL: &str = crate::BASE_URL;

/// `isEnd` is a bool in list responses and an int in chapter responses.
#[derive(Deserialize, Default)]
#[serde(untagged)]
pub enum Flag {
	Bool(bool),
	Int(i64),
	#[default]
	None,
}

impl Flag {
	fn is_set(&self) -> bool {
		match self {
			Self::Bool(value) => *value,
			Self::Int(value) => *value != 0,
			Self::None => false,
		}
	}
}

#[derive(Deserialize)]
pub struct PopularResponse {
	pub data: PopularData,
}

#[derive(Deserialize)]
pub struct PopularData {
	#[serde(default)]
	pub items: Vec<MangaDto>,
	#[serde(rename = "isEnd", default)]
	pub is_end: Flag,
}

#[derive(Deserialize)]
pub struct MangaDto {
	pub name: String,
	pub manga_code: String,
	#[serde(default)]
	pub cover: Option<String>,
}

impl From<MangaDto> for Manga {
	fn from(value: MangaDto) -> Self {
		Self {
			url: Some(format!("{BASE_URL}/manga/{}", value.manga_code)),
			key: value.manga_code,
			title: value.name,
			cover: value.cover.filter(|cover| !cover.is_empty()),
			..Default::default()
		}
	}
}

impl From<PopularData> for MangaPageResult {
	fn from(value: PopularData) -> Self {
		let has_next_page = !value.is_end.is_set() && !value.items.is_empty();
		Self {
			entries: value.items.into_iter().map(Into::into).collect(),
			has_next_page,
		}
	}
}

#[derive(Deserialize)]
pub struct ChapterByPageResponse {
	pub data: ChapterByPageData,
}

#[derive(Deserialize)]
pub struct ChapterByPageData {
	#[serde(default)]
	pub items: Vec<ChapterItem>,
	#[serde(rename = "isEnd", default)]
	pub is_end: Flag,
}

impl ChapterByPageData {
	pub fn is_last_page(&self) -> bool {
		self.is_end.is_set() || self.items.is_empty()
	}
}

#[derive(Deserialize)]
pub struct ChapterItem {
	pub id: i64,
	#[serde(rename = "chapterName")]
	pub chapter_name: String,
}

#[derive(Deserialize)]
pub struct PageListResponse {
	pub data: PageListData,
}

#[derive(Deserialize)]
pub struct PageListData {
	pub scans: Vec<PageDto>,
}

#[derive(Deserialize)]
pub struct PageDto {
	#[serde(default)]
	pub n: i32,
	pub url: String,
	#[serde(default)]
	pub width: i32,
	#[serde(default)]
	pub height: i32,
}
