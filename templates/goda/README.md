# GoDa Template

Shared implementation for GoDa-family sites (Astro frontend: `/hots`, `/dayup`, `/newss`,
`/manga-genre/*`, `/manga-tag/*`, `/s/{query}`).

Two backends are supported for chapters and pages:

- `Backend::Html`: legacy site endpoints `/manga/get?mid=..&mode=all` (`.chapteritem` list) and
  `/chapter/getcontent?m=..&c=..` (`#chapcontent img`).
- `Backend::Api`: the `mgsearcher` JSON API (`/api/manga/get`, `/api/v2/chapter/getinfo`, the latter
  returning an obfuscated image list, see `decoder.rs`).

Genre filters are a `select` filter with id `genre` whose ids are site paths, e.g. `manga-genre/cn`
or `manga-tag/gufeng` (empty = `manga`).

Chapter keys are `{mid}/{chapter id}` so that the page list needs no extra request.
