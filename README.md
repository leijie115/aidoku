# Aidoku Source Repository

Custom source repository for [Aidoku](https://aidoku.app/) (0.7+).

## Available Sources

| Source | Language | NSFW | URL |
|--------|----------|------|-----|
| Manga18fx | English | Yes | https://manga18fx.com |
| Manga18.Club | English | Yes | https://manga18.club |
| 漫小肆韓漫 (Ikanwzd) | 中文 | Yes | https://www.ikanwzd.cc |
| 18漫畫 (EighteenMH) | 中文 | Yes | https://18mh.org |
| 巴卡漫画 (Bakamh) | 中文 | Yes | https://bakamh.com |
| 包子漫画 (Baozi Manhua) | 中文 | No | https://www.baozimh.com |
| GoDa漫画 (BaozimhOrg) | 中文 | No | https://baozimh.org |
| 拷貝漫畫 (MangaCopy) | 中文 | Yes | https://www.mangacopy.com |
| Hanime1 | 中文 | Yes | https://hanime1.me |
| 嗨皮漫画 (HappyMH) | 中文 | No | https://m.happymh.com |
| Ho5Ho | 中文 | Yes | https://www.ho5ho.com |
| 哔哩漫画 (BiliManga) | 中文 | Yes | https://www.bilimanga.net |
| Komiic | 中文 | Yes | https://komiic.com |
| MyComic | 中文 | Yes | https://mycomic.com |
| 紳士漫畫 (WNACG) | 中文 | Yes | https://www.wnacg.com |
| 再漫画 (Zaimanhua) | 中文 | No | https://www.zaimanhua.com |
| 禁漫天堂 (Jinmantiantang) | 中文 | Yes | https://18comic.ink |
| pawo漫画 (Pawomh) | 中文 | Yes | https://pawomh1.cc |
| 污污漫畫 (55comic) | 中文 | Yes | https://www.55comic.com |
| 肉漫屋 (Rouman5) | 中文 | Yes | https://rouman5.com |
| 漫小肆韓漫 (jjmhw) | 中文 | Yes | https://www.jjmhw.cc |
| 禁漫岛 (91jmd) | 中文 | Yes | https://91jmd.com |
| Temple Scan | Español | Yes | https://templescanesp.net |
| Komikindo.live | Bahasa Indonesia | Yes | https://2.komikindo.shop |
| Mangaforfree.com | English | Yes | https://mangaforfree.com |
| 뉴엑스툰 (NewXToon) | 한국어 | Yes | https://newxtoon3.com |

## How to Use

On a device with Aidoku installed, open <https://leijie115.github.io/aidoku/> and tap **Add Repository**.

Or open Aidoku, go to **Settings → Source Lists → +**, and paste:

```
https://leijie115.github.io/aidoku/index.min.json
```

Then go to **Browse**, find the source in the list, and tap **Get** to install it.

## Development

Requirements: [Rust](https://rustup.rs/) with the `wasm32-unknown-unknown` target, and the Aidoku CLI:

```sh
rustup target add wasm32-unknown-unknown
cargo install --git https://github.com/Aidoku/aidoku-rs aidoku-cli
cargo install --git https://github.com/Aidoku/aidoku-rs aidoku-test-runner
```

Build a single source:

```sh
cd sources/zh.baozimanhua
aidoku package          # -> package.aix
cargo test -- --nocapture   # live tests against the website
```

Build the whole source list locally (output in `public/`):

```sh
for s in sources/*; do (cd "$s" && aidoku package); done
aidoku build sources/*/package.aix --name "leijie115"
aidoku serve sources/*/package.aix   # serve over LAN to test on a device
```

Pushing to `main` builds every source with GitHub Actions and deploys the source list to the `gh-pages`
branch. Enable GitHub Pages for the `gh-pages` branch in the repository settings.

Some sources and the `madara` / `mangathemesia` templates are based on
[Aidoku-Community/sources](https://github.com/Aidoku-Community/sources) (MIT / Apache-2.0).

---

# Aidoku 图源仓库

适用于 [Aidoku](https://aidoku.app/)（0.7 及以上）的自定义图源仓库。

## 可用图源

| 图源 | 语言 | NSFW | 网址 |
|------|------|------|------|
| Manga18fx | English | 是 | https://manga18fx.com |
| Manga18.Club | English | 是 | https://manga18.club |
| 漫小肆韓漫 (Ikanwzd) | 中文 | 是 | https://www.ikanwzd.cc |
| 18漫畫 (EighteenMH) | 中文 | 是 | https://18mh.org |
| 巴卡漫画 (Bakamh) | 中文 | 是 | https://bakamh.com |
| 包子漫画 (Baozi Manhua) | 中文 | 否 | https://www.baozimh.com |
| GoDa漫画 (BaozimhOrg) | 中文 | 否 | https://baozimh.org |
| 拷貝漫畫 (MangaCopy) | 中文 | 是 | https://www.mangacopy.com |
| Hanime1 | 中文 | 是 | https://hanime1.me |
| 嗨皮漫画 (HappyMH) | 中文 | 否 | https://m.happymh.com |
| Ho5Ho | 中文 | 是 | https://www.ho5ho.com |
| 哔哩漫画 (BiliManga) | 中文 | 是 | https://www.bilimanga.net |
| Komiic | 中文 | 是 | https://komiic.com |
| MyComic | 中文 | 是 | https://mycomic.com |
| 紳士漫畫 (WNACG) | 中文 | 是 | https://www.wnacg.com |
| 再漫画 (Zaimanhua) | 中文 | 否 | https://www.zaimanhua.com |
| 禁漫天堂 (Jinmantiantang) | 中文 | 是 | https://18comic.ink |
| pawo漫画 (Pawomh) | 中文 | 是 | https://pawomh1.cc |
| 污污漫畫 (55comic) | 中文 | 是 | https://www.55comic.com |
| 肉漫屋 (Rouman5) | 中文 | 是 | https://rouman5.com |
| 漫小肆韓漫 (jjmhw) | 中文 | 是 | https://www.jjmhw.cc |
| 禁漫岛 (91jmd) | 中文 | 是 | https://91jmd.com |
| Temple Scan | Español | 是 | https://templescanesp.net |
| Komikindo.live | Bahasa Indonesia | 是 | https://2.komikindo.shop |
| Mangaforfree.com | English | 是 | https://mangaforfree.com |
| 뉴엑스툰 (NewXToon) | 한국어 | 是 | https://newxtoon3.com |

## 使用方法

在装有 Aidoku 的设备上打开 <https://leijie115.github.io/aidoku/>，点击 **Add Repository**。

或者打开 Aidoku，进入 **设置 → 图源列表 → +**，粘贴：

```
https://leijie115.github.io/aidoku/index.min.json
```

然后在 **浏览** 页找到对应图源，点击 **获取** 安装即可。

## 开发

见上方英文 Development 部分。

## 已知问题

- **嗨皮漫画 (zh.happymh)**：开发时网站无法访问，按旧版 Tachiyomi 扩展的接口实现，未经实测，接口可能已变化。
- **Hanime1 / 18漫畫 / 巴卡漫画 / Manga18.Club**：有 Cloudflare 验证，只用存档页面验证过解析；阅读页（图片列表）未实测。首次打开时需在 App 弹出的网页中完成验证。
- **污污漫畫 (zh.comic55)**：图片和封面为加密 + 切片打乱，需要 Aidoku 0.9.0+。
- **禁漫天堂 (zh.jinmantiantang)**：需要 Aidoku 0.8.3+。
- **拷貝漫畫**：网站接口每个分组最多返回 400 话，超长连载的早期章节会缺失。
- **Temple Scan / NewXToon / wnacg** 等域名经常变动，可在图源设置里修改网址。推送到 `main` 后 GitHub Actions 会自动构建全部图源并发布到 `gh-pages`
分支，需要在仓库设置中为 `gh-pages` 分支开启 GitHub Pages。
