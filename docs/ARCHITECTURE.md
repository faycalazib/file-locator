# Prospector: architecture

> Last updated: 2026-09-28 (lots 8.3–8.4: meaning search; lot 8.2: meaning index; lot 8.1: meaning module; lot 7.4: term lists; lot 7.3: shared index; lot 7.2: site groups; lot 7.1: command line; lot 6.8: portable mode; lot 6.7: extraction; lot 6.6: images; lot 6.5: duplicates). The reference spec is [`goal.md`](../goal.md).

## Overview

```text
my-file-locator/
├── goal.md                  Reference spec (tracking checkboxes)
├── Cargo.toml               Rust workspace: core + src-tauri
├── core/                    `prospector-core` crate: engine, has no Tauri dependency (can be tested on its own)
├── src-tauri/               Tauri v2 shell: commands, window, bundle, icons
├── ui/                      Svelte 5 + Vite frontend (SPA, no SvelteKit)
│   ├── index.html
│   ├── public/favicon.svg
│   └── src/
│       ├── main.ts          Entry point: fonts → tokens → theme → base, i18n.init(), mount
│       ├── App.svelte       Masthead, then rail | results | preview
│       ├── assets/doodles/  Fluent Emoji "Color" icons (MIT), bundled for offline use
│       ├── styles/          tokens.css (structure), themes/crayons.css (theme), fonts.css, base.css
│       └── lib/
│           ├── components/  SearchBar (masthead), IndexRail, FilterPanel, ResultsList, PreviewPanel,
│           │                LangSwitcher, ThemeToggle, SettingsMenu, Marked, Icon, ThemeDefs (SVG filters)
│           ├── doodles.ts   Doodle icon per file type + decorations
│           ├── i18n/        Runtime (index.svelte.ts) + locales/{en,fr,es,ar}.json
│           ├── stores/      search.svelte.ts (query, filters, results, snapshots), tabs.svelte.ts (tabs + history),
│           │                ui.svelte.ts (variant, panels), saved, sites, updates, notices, preview
│           ├── results-file.ts / results-io.ts  Saved results (`.prospector`): format, save / open
│           ├── mock/        Multilingual mock data (Étape 0 only)
│           ├── types.ts     Types that mirror the future backend payloads
│           ├── text.ts      ⟦match⟧ segmentation, normalization, path splitting
│           ├── export.ts    CSV / JSON export
│           └── actions.ts   `dismissable` (popovers)
├── scripts/i18n-check.mjs   Checks that the 4 locales are consistent
├── test_fixtures/           One sample file per format (Étape 1+)
└── docs/                    ARCHITECTURE, I18N, THEMES, BUGS, references/, mockups/, screenshots/
```

## Commands

| Command | Purpose |
| --- | --- |
| `pnpm tauri dev` | Runs the desktop app in dev mode (**started by the user**) |
| `pnpm dev` | Vite only, on http://localhost:1420 (browser preview) |
| `pnpm check` | svelte-check + TypeScript (target: 0 errors, 0 warnings) |
| `pnpm build` | Frontend build → `dist/` |
| `pnpm i18n:check` | Consistency of the 4 locales |
| `cargo check --workspace` / `cargo clippy --workspace -- -D warnings` | Rust |
| `cargo test -p prospector-core` | Engine tests |

URL parameters (for screenshots): `?lang=ar` forces the language, `?v=journal|ledger|strata` forces the layout, `?theme=crayons|sonar` forces the theme.

**Screenshots without a server.** `vite.config.ts` uses `base: './'`, so `dist/index.html` also opens from `file://`. Run:

```bash
chrome --headless=new --allow-file-access-from-files --window-size=1600,1000   --virtual-time-budget=6000 --screenshot=out.png "file:///<repo>/dist/index.html?lang=ar&v=journal"
```

The static mockups in `docs/mockups/` have their own helper, `shoot.sh`.

## Frontend

- **Svelte 5 with runes.** Global state lives in `*.svelte.ts` classes (`$state`, `$derived`) and in `SvelteSet` for reactive sets.
- **Themes** (see [THEMES.md](THEMES.md)).
  - `tokens.css` holds only structure: type scale, spacing, motion.
  - `themes/<name>.css` defines colors, fonts and treatments under `:root[data-theme="<name>"]`. Available themes: `crayons` (light, default) and `sonar` (dark).
  - The theme is switched by the sun / moon button at the bottom of the rail (`ThemeToggle`, `ui.setTheme`: light ↔ dark), next to the Settings cog, and applied before mount (`ui.initTheme`). It also aligns the Tauri window chrome.
  - Components only use semantic variables (`--text`, `--accent`, `--mark-bg`…) and the `.sketch` / `.hatch` classes.
- **"Colored pencils" treatments.**
  - `.sketch::before`: an ink outline shaken by the SVG filter `#wobble` (`ThemeDefs.svelte`). Only the stroke is filtered; text stays sharp.
  - `.hatch`: pencil hatching. `--k` sets the stroke color, `--h` the hatching color, `--sw` the stroke width.
- **Masthead** (`SearchBar.svelte`):
  - notebook ribbon, then PROSPECTOR in hatched letters (one pencil per letter, always LTR);
  - dashed "I'm looking for:" box with the FOUILLER button;
  - chips: fuzzy tolerance, `Aa` / `ab` / `.*`, file types, "+ Refine", active-filter tokens, indexed / live-scan mode.
- **Progressive disclosure.** Size, date, language and excluded folders are folded away behind "+ Refine". Each result shows a single excerpt. A site's roots and size are in its tooltip.
- **Results.**
  - Each result is a white card, sketch-outlined and slightly rotated (±0.3°), with a doodle icon per file type.
  - The match count sits in a hatched pink circle.
  - The selected card gets a thicker blue stroke.
- **Motion.**
  - `.sweep`: a band of pink hatching sweeps across the results while digging.
  - `.row.ping::after`: a yellow highlighter flash on each new find.
  - Both are disabled under `prefers-reduced-motion`.
- **Rich preview** (`PreviewPanel.svelte` + `lib/rich.ts`). The engine returns a `layout`: `code` (syntax colors), `sheet` (Excel), `slides` (PowerPoint) or `prose`.
  - `sheet`: one table per `— Sheet —` block. Cells are split on tabs, and empty cells are kept by the extractor so columns stay aligned. The first row is bold. Numbers are aligned to the end and short cells do not wrap. The table follows the **document's** direction, not the interface's.
  - `slides`: one card per `— N —` block, with the slide number in a circle and the first line as the title.
  - A match split by a tab keeps its index (`occ`), so ↑ ↓ / F3 still work. Lines skipped in a long document show as `⋯`.
- **PDF report** (`ReportView.svelte`, `styles/print.css`). "Download → PDF report…" (or Ctrl+P) mounts a printable report of the search on screen, then opens the print dialog (`print_report` → `WebviewWindow::print`), whose "Save as PDF" writes the file.
  - The report holds the query, mode, options, filters, excluded folders, sites and totals, then each file with its path, details and excerpts (matches highlighted).
  - Printing gives the fonts, **Arabic shaping and RTL** for free: a PDF library would need its own shaping.
  - In print, only the report is shown, on white paper whatever the theme. `color-scheme: light` is forced, because Sonar's dark scheme would darken the page margins.
  - `afterprint` unmounts the report. `?report` shows it in the browser demo; sample PDFs are in `docs/screenshots/report-*.pdf`.
- **User layouts** (Settings → Layout, `ui.setVariant`, stored under `prospector.layout`):

  | Layout | Rail | Filters ("+ Refine") | Results | Preview |
  | --- | --- | --- | --- | --- |
  | `journal` (Log, default) | 280 px | Sheet over the results | Cards: name, path, date, 1 excerpt | 38 % |
  | `ledger` | 290 px, holds the filters | In the rail | Table; the excerpt shows on the selected row only | 34 % |
  | `strata` | Compact, 64 px (sites as circles) | Drawer | Grouped by folder | 46 % |

- **Performance (to watch).** The `#wobble` filter is applied to every `.sketch` outline. It is fine for a few hundred cards. If long lists stutter at Étape 1, virtualize the list or set `--wobble: none` beyond N items.

## Backend

### `core/`: the `prospector-core` crate, testable without Tauri

| Module | Role |
| --- | --- |
| `crawler.rs` | Parallel walk (`ignore`). Skips hidden files and default folders (`.git`, `node_modules`, `target`, `$RECYCLE.BIN`…). User exclusions are absolute paths (`D:\Clients\_old`) or globs (`**\node_modules`), case-insensitive. |
| `kind.rs` | File family from the extension (`FileKind`, mirrored in the UI). |
| `extract/` | `text` (BOM, then UTF-8, then `chardetng` guess; binary files rejected), `pdf` (pdf-extract inside `catch_unwind`), `docx` (zip + quick-xml, one line per paragraph), `office` (Excel xlsx/xls/ods with calamine, one `— Sheet —` block per sheet, cells separated by tabs with **empty cells kept** so columns align; PowerPoint pptx, one `— N —` block per slide), `archive` (ZIP with `zip`, **7z** with `sevenz-rust2` in pure Rust, **RAR 4/5** with `unrar`, the official UnRAR library compiled with MSVC. Nested ZIP and 7z are read in memory up to 3 levels; a RAR can only be opened from disk, so a RAR inside an archive is skipped. In a solid 7z, a skipped entry is still read through. Multi-volume RAR sets are read from their first part. A fully password-protected archive counts as `encrypted`. Guards: entry size, total budget, entry count and ratio > 100), `mail` (.msg with msg_parser; .pst with Microsoft's pure-Rust `outlook-pst`, one document per message). A file gives **one or more** documents (`ExtractedDoc`); an inner document's path is `file › inner` (`INNER_SEP`). Files over 64 MB are skipped (containers are streamed). Each failure becomes a `SkipReason` (counted, never fatal). |
| `extract/mail.rs` | E-mails (lot 5.4).
- `.msg` (msg_parser) and `.eml` (**mail-parser**: MIME, base64, HTML turned into text) give the message, then **one document per attachment** (`mail.msg › contrat.pdf`, family `email`), through `archive::attachment_docs`. Same limits as archive entries; an attached ZIP or message is opened too.
- `.mbox` is streamed (`mailbox::mbox::MessageIterator`).
- `.pst`/`.ost`: `Mailbox` (Unicode or ANSI store) reads the typed messages to get at their attachment table.
- A message's own document has no inner path; the name criterion checks it against the file name (BUG-031). |
| `extract/doc.rs`, `ppt.rs`, `rtf.rs`, `odf.rs`, `epub.rs` | Older and other formats (lot 5.3), in pure Rust, with neither Office nor IFilters.
- **`.doc`** (CFB + FIB → piece table: 8-bit Windows-1252 or UTF-16 pieces; field instructions dropped, results kept).
- **`.ppt`** (records of `PowerPoint Document`: placeholder text from `SlideListWithText`, text boxes from each `Slide`; masters and notes skipped). The output uses the same `— N —` blocks as `.pptx`, so slides show as cards.
- **RTF** (groups, `\\uN`/`\\ucN`, `\\'hh` in `\\ansicpg`, non-text destinations skipped).
- **`.odt`/`.odp`** (`content.xml`; notes skipped).
- **EPUB** (container → OPF spine order → XHTML text).
- **PDFs protected only against copying** are opened with the empty password.
- `archive.rs` also reads `.jar`, `.tar`, `.tar.gz`/`.tgz`, `.tar.bz2`, and single `.gz`/`.bz2` files (`serveur.log.gz › serveur.log`). |
| `extract/ocr.rs`, `extract/pdf_ocr.rs` | **OCR** (lot 5.2), with the Windows recognizer (`Windows.Media.Ocr` through the `windows` crate). Nothing to install.
- **Recognizers:** one per thread (BUG-029). When Latin and Arabic are both installed, both read the image and the reading with more letters of its own script wins.
- **Images:** new family `FileKind::Image` (png, jpg, tif, bmp, gif, webp). Images smaller than 200 px on a side are icons and are skipped.
- **PDFs:** read page by page (`extract_text_from_mem_by_pages`). A page with fewer than 32 letters is sent to OCR: its images come out through `lopdf`. DCT goes as is, CCITT is wrapped in a TIFF, Flate or raw pixels are converted to BGRA (PNG predictors undone). JBIG2 and JPX are not decoded.
- **Setting:** on by default (`settings.json` → `ocr`). The manifest stamp records `ocr`: switching OCR on re-reads only the images and PDFs read without it (`set_ocr` updates every site). |
| `lang/` | `WordTokenizer`: keeps Arabic combining marks inside words and emits the normalized text **with the original offsets**. `fold`: lowercase, no diacritics, unified alef/ya/ta marbuta. Analyzers: `generic`, plus `stem_en/fr/es/ar` (Snowball). Language detection with `whatlang`. |
| `index.rs` | Tantivy schema: `path`, `name_raw` (folded name(s): the document's, plus its archive's for inner documents, for the name criterion), `file` (the file on disk a document comes from: itself, or the ZIP / PST holding it; deleting this term removes all its documents), `name`, `kind`, `lang`, `size`, `modified`, `body` (generic analyzer, positions, stored), and `stem_<lang>` (the text stemmed in its detected language). An index written with another schema is recreated empty, then rebuilt by the next update. |
| `manifest.rs` | `indexes/<id>.manifest.json`: for each file, size and date when indexed, number of documents, or skip reason. The difference with the folders gives the added / changed / removed files. `VERSION` bumped → full rebuild of every site. |
| `saved.rs` (alerts) | Lot 6.1: `SavedSearch.alert` = `Alert { request, site_ids, seen, fresh }`. `request` is the engine request built by the UI (`buildRequest`, relative dates left out); `seen` = documents already matching (never announced); `fresh` = announced, not looked at (the badge). `record()` keeps only the documents never seen, and forgets those of removed files (announced again if they come back). |
| `saved.rs` | Saved searches in `<data>/saved-searches.json` (they follow the data folder). `settings` (mode, options, filters, sites) belongs to the UI and is stored as is. Same query + same settings = one entry (saving again renames it). |
| `fsutil.rs` | `write_atomic`: temporary file + rename, retried for up to 1 s on "access denied" (antivirus, BUG-026). Used by every JSON store. |
| `watch.rs` | `FolderWatcher` (notify + notify-debouncer-full, **without** file-id cache: it would walk every folder when the watch starts). Changed paths are grouped until 2 s of quiet, then handed over as one `Changes` batch (`rescan` if events were lost). |
| `query.rs` | User language: words, `"phrase"` / `«…»`, `OR` / `\|`, `AND` (default), `NOT` / `-word`, `/regex/`, and FileLocator Pro's `a NEAR b` / `NEAR:n` (one `Clause::Near`, distance in characters, 100 by default; `NOT a NEAR b` excludes the pair), `LIKE word` (`Clause::Like`: typo tolerance for that word only) and `LINES:a-b` / `LINES:a+` (`ParsedQuery.lines`). Parentheses are ignored. Operators are recognized in upper case only. The `.*` chip turns the whole input into one regex (`ParsedQuery::whole_regex`). |
| `names.rs` | File-name criterion (lot 5.1).
- Syntax: `*.pdf; facture-2024-??.xlsx`; `!` excludes; a plain word means "contains"; `/regex/`.
- Case and accents are ignored (`fold`).
- `matches_any`: a document inside an archive answers to its own name **or** its archive's; any excluded name wins.
- `query()`: the same rules as `RegexQuery`s on `name_raw`, which always match the whole term, so regex anchors become the absence of `.*`. |
| `scope.rs` | "Search in this folder" (lot 5.7, `SearchRequest.inFolder`).
- `inside(root, path)`: case- and separator-insensitive containment (unlike `manifest::is_within`, for paths read from disk). The part below the root is **respelled as on disk** (BUG-032), since the index is case-sensitive.
- Index (`site_scope`): a site holding the folder answers only under it (`RangeQuery` on `path` from `prefix\` to `prefix]`); a site inside the folder answers whole; others are skipped.
- Live scan and folder search (`restrict_targets`): the folder alone, labelled with the site holding it, or `@folder` (`FOLDER_SITE`) when none does: that walk covers the sites inside it. |
| `thumb.rs` | Thumbnails (lot 6.6) with the imaging built into Windows (`BitmapDecoder` → `BitmapTransform` scaled with Fant interpolation, EXIF orientation respected — scale on the stored frame, output turned — → `BitmapEncoder` JPEG). Nothing to ship. `extract/ocr.rs` → `image_words`: the words read with their boxes (fractions of the turned image), same Latin / Arabic choice as the indexed reading. `Engine::image_matches(path, req)`: each OCR line's text matched with the request's `Matcher`, the words overlapping a match merged into one box. |
| `dupes.rs` | Duplicates (lot 6.5). **Identical**: the manifests give every file's size (no walk); only files sharing a non-zero size are hashed (SHA-256, in path order), grouped by digest, `wasted` = size × extra copies. **Near-identical**: each document's stored text (≥ 300 characters, first 200,000) folded, 5-word shingles, 64-slot MinHash (multiply-add hashes over one 64-bit hash, in parallel); 16 bands × 4 rows (LSH) give candidate pairs (a bucket over 200 documents is chained, not squared), kept at similarity ≥ threshold (0.7–0.95), union-find groups with their lowest similarity. Pairs of the same identical group are skipped. `Engine::find_duplicates` (cancel, progress `hashing` / `comparing`). |
| `report.rs` | Keyword report (lot 6.4): the positive terms of the query (word, "phrase", /regex/, LIKE, NEAR; detectors excluded), each once. `Engine::keyword_report(docs, req)` counts each term (one `Matcher` per clause) in each document's stored text (or the file read from disk for live-scan results) → rows + totals per term. |
| `sense/` | **Meaning search** (Étape 8). `SenseModel::load(dir)`: ONNX Runtime loaded once per process from the module folder (`ort` 2.0.0-rc.13, `load-dynamic`: nothing linked or downloaded at build time; any runtime ≥ 1.17, the module ships 1.28), session (graph optimisations, all cores but one), `tokenizer.json` (`tokenizers`, `fancy-regex`: no C code), truncation at 512 tokens, **no padding** (BUG-035). `embed(texts)`: texts grouped by exact token length, one run per group, CLS vector of the last hidden state, normalized (the model's pooling). `similarity` = dot product. `sense/module.rs`: the module in `<personal>/modules/sense` (manifest: module, version, model, files with SHA-256); `install_zip(zip, dir, expected_sha)` (ZIP SHA-256 for a download, then every file against the manifest, unpacked aside then swapped), `remove` (marked and removed at the next start when the DLL is in use), `cleanup`, `status`; `MODULE_URL` (None until the release exists), `MODULE_SHA256`, `MODULE_BYTES`. Built by `scripts/sense-module.py` (reproducible ZIP). |
| `sense/chunk.rs`, `sense/store.rs`, `sense/pool.rs`, `engine/meaning.rs` | **Meaning index** (lot 8.2). `chunk::passages`: about 120 words, cut at the end of a sentence (`.` `!` `?` `؟` line break; 160 words at most), 12 per document, the file name leading the first; `count` for the estimate. `store::SenseIndex`: a Tantivy index per site (`indexes/<id>.sense`), one document per passage (path, file, passage, byte range stored); its vector quantized on 8 bits (scale in an `f64` fast field, 384 signed bytes packed in 48 `u64` fast fields, `v0`…`v47`); `search` reads the columns in blocks of 512 documents and keeps the best in a heap (exact, deleted documents skipped); `delete_file`, `remap_paths` (portable drive letter). `pool::SensePool`: 2 models on half of the cores minus one (normal) or 1 model on 2 cores (economy), every thread — ours and ONNX Runtime's, through `ThreadManager` — in the background mode of Windows (`THREAD_MODE_BACKGROUND_BEGIN`); `speed` measures passages per second. `engine/meaning.rs`: `SiteRecord.sense`, `set_site_sense` (off deletes the index), `sense_plan` (files to do from the main manifest against `indexes/<id>.sense.json`; passages estimated from the stored text), `sense_update` (batches of 32 files read from the main index by their `file` term, passages embedded 32 at a time, committed then recorded: an interruption resumes, a changed file is computed again alone; source code left out, even inside archives), `sense_search`, `sense_passages`; readers of a shared index open it read-only and reload it. |
| `terms.rs` | **Term lists** (lot 7.4): `read_list` (≤ 8 MB, encoding by the text reader), `parse_list` (one term per line, first CSV / TSV column with its quotes, `#` comments, repeats ignoring case, ≤ 5,000 terms; `skipped` counted), `clause` (`/…/` regex, several words a phrase, else a word, like the search box). `SearchRequest.term_list` / `term_list_all` → `parsed()` adds one AND group (any term) or one group per term (all): highlighting, live scan, keyword report and its columns follow without other code. |
| `share.rs` | **Shared index** (lot 7.3). `Lease` on `maintainer.json` in the data folder (computer name, instance, heartbeat): `claim(now)` takes it if free, stale (> `TTL` = 120 s) or ours, writes it (atomic) and reads it back (two PCs at once: the last write wins); `holder(now)` without claiming; `release` on closing. `is_network` (UNC, or `GetDriveTypeW` = remote); `universal` (`WNetGetUniversalNameW`: `Z:\x` → `\\server\share\x`). Engine side: `Engine::open_with(data, personal)` — saved searches and groups in the personal folder (moved there once from the data folder); `set_reader_of(Some(pc))` → `add_site` / `remove_site` / `index_site` / `update_paths` refused (`CoreError::ReadOnly`), indexes opened with `SiteIndex::open_existing` (nothing created or migrated, another schema refused), `refresh_shared` (reload of the open readers, catalog read again when `sites.json` changed) before each search and site list. |
| `groups.rs` | **Site groups** (lot 7.2): `site-groups.json` next to the indexes (id, name, site ids, color index — the first one no group has). `Engine::create_site_group` / `update_site_group` (rename, or new sites) / `remove_site_group`; unknown site ids are left out; `remove_site` takes the site out of its groups and drops a group left empty. Used by the app and by `prospector-cli --group`. |
| `locate.rs` | **Where the data is** (lot 7.1), shared by the app and `prospector-cli`: `portable_root` (a `portable` file next to the program → `ProspectorData`), `Location` (settings file + default data folder: portable, or `dirs::config_dir()` / `dirs::data_dir()` + the identifier, the same folders as Tauri), `Location::data_dir` (portable always; else `PROSPECTOR_DATA_DIR`, the saved choice, the default), `read_settings` (data folder + OCR switch for the command line), `identifier` (`.portable` suffix). |
| `userpath.rs` | **The user's PATH** (lot 7.1): `HKCU\Environment\Path` read raw to keep its type (`REG_EXPAND_SZ`), one entry added or removed (case, final `\` and quotes ignored), then `WM_SETTINGCHANGE "Environment"` broadcast so that new terminals see it. |
| `portable.rs` | **Portable mode** (lot 6.8): `drive_of` (drive letter of a path, none for UNC), `remap_path` (`E:\…` / `\\?\E:\…` → `F:\…`, letter without case, `E:tude` untouched), `move_drive(data, from, to)`: for each site with a root on `from`, `index::remap_paths` (documents rewritten from their stored fields, one commit, nothing read on the disk) and its manifest's keys; then `saved-searches.json` (every string, alerts' seen / new files included) and `sites.json` last. Every step can run again. |
| `unpack.rs` | **Extraction** (lot 6.7). `extract::extract_raw(file, inners)` reads a container again **the way it was indexed** (same walk, same inner paths — `(2)` of same-subject messages included —, same bomb guards) in *capture* mode: a thread-local `Capture` (wanted inner paths, bytes found) set only for that call; `admit` skips before reading any entry that is neither wanted nor an archive on the way to one (`capture_skips`), `push_entry` keeps the wanted bytes without extracting text (`offer`), `Budget::spent` stops once all are found, the mbox / PST walks skip other messages. Indexing never sets it. `inner_kind(path)`: the family of the last inner name (`None` on disk and for a mailbox message, which is not a file) — also sent on each `Hit` as `innerKind`. `read` (name + bytes), `extract_to` (save dialog target), `open_copy` (into `<data>/opened/<stamp>/<name>`, emptied at the next start by `clear_opened`, and before moving the data folder), `safe_name` (characters Windows refuses). |
| `copy.rs` | Bulk copy (lot 6.4): the files on disk behind the results (a container once), to a folder (`std::fs::copy`, never over an existing file: " (2)") or a ZIP (`zip` writer, deflate, zip64 for big files), flat or below their common parent (drive letter as first folder across drives); cancel flag, progress, report (copied, skipped, from containers). Lot 6.7: with `extract`, documents inside containers are extracted instead (`Item::Inner`; each container read once for all its documents, `Extracted`), placed flat under their own name or, in a tree, in a folder named after the container (`clients.zip\notes\devis.txt`); a mailbox message still brings its mailbox. |
| `facets.rs` | Counts per criterion (lot 6.3, `SearchRequest.facets`). A Tantivy `Collector` reads fast fields: `kind` / `lang` (raw text, made fast: schema change handled by the index migration) and the year (UTC) of `modified` or `created` (the date the filter uses). Each criterion is counted **without its own filter** (one pass per criterion, the query rebuilt without it). Sites: the searched ones get their total; the others a `Count` of the same query (what they would bring). Queries checked on the text (regex, NEAR, detectors, disk): counted on the documents checked, current filters included. → `SearchResponse.facets`. |
| `detect.rs` | Detectors (lot 6.2): 13 `Detector`s, each a regex **and a validator** (IBAN mod 97 + country length, RIB key, Luhn + card brand, FR VAT key, BE VAT mod 97, SIREN/SIRET Luhn after the word, NIR key 97 with 2A/2B, DNI/NIE letter, ICAO MRZ check digits, BIC country + digit or context, IPv4 ≤ 255, phone 9–15 digits, amount with a currency). Arabic-Indic digits count. `Clause::Detector` is one more AND group of the parsed query (`SearchRequest.detectors`, any of them); the index cannot answer it, so it goes through the text check like a regex (same results in live scan). `Matcher::detections` counts per kind → `Hit.detections`; `search_site` keeps counting **past the display limit** when detectors are asked → `SearchResponse.detections` (files, occurrences per kind). |
| `disk.rs` | What is read on the disk at search time, never indexed (lot 5.8): `Facts` (creation, last access, attributes, from the metadata the crawl already reads), `Digest` (MD5 / SHA-256, by hex length, read in 256 KiB blocks), `DiskCheck` (last-access range, required attribute bits, digest). A document inside an archive has no digest of its own. |
| `pipeline.rs` | Shared by indexing and live scan: files sorted by path, **one reader thread** (sequential on hard disks), read-ahead of 256, CPU work in parallel (`par_bridge`). See BUG-025. |
| `scan.rs` | Live scan (non-indexed search): walk the folders, extract, apply the query's AND/OR/NOT logic (`search::QueryLogic`, shared with the index) and highlight; each hit is streamed through `on_hit`.
- The name criterion is applied **before reading**.
- With no text, plain files are answered from their name alone (nothing is opened); archives and mailboxes are still opened to check their entries.
- `find_folders`: folders by name (`crawler::crawl_folders`), used by both modes, because folders are not indexed. |
| `search.rs` | **Dates and disk checks** (lot 5.8): `dateField` (`modified` / `created` → `RangeQuery` `[dateFrom, dateTo)`; `accessed` → disk), `attributes`, `hash`. With a disk check, Tantivy gives up to 100k candidates and `DiskCheck` decides, last; "find copies" = digest + exact size, so only same-size files are hashed. In live scan, facts filter the walk (`crawl_with(hidden)` when `hidden` is asked); a digest without text answers whole files (archives included), nothing is extracted. **Text check** (lot 5.5): when the index cannot answer alone (`SearchRequest::verified`: regex, NEAR, LINES, detectors, or `Aa`, BUG-037), Tantivy narrows the candidates (NEAR = all its terms, LIKE = fuzzy word) and `QueryLogic` checks each one on its stored text, exactly as the live scan does. `SearchRequest` carries the chips: `fuzzy`, `caseSensitive` (Aa), `wholeWord` (ab, on by default), `regex` (.*). Regexes: Tantivy narrows the candidates using the rest of the query (up to 100k), then each regex is applied to the stored text. `ab` off: `Contains::expand` lists the indexed words that contain the word, once per site and search (one walk of the dictionary), and a `TermSetQuery` on `body` is reused by the counts. `Aa`: the index looks for the exact word only (no stems, no typos), every candidate is checked on its text (counts included), `NOT` is decided on the text. Candidates are checked in parallel batches of 256. The highlighter judges each spelling once per text and stems only words within one letter of a searched word's length. Tantivy query per word: a union of `body` + `name` (×2) + stemmed fields (×0.7) + fuzzy (×0.5; no tolerance up to 5 letters, distance 1 for 6 to 9 letters, 2 from 10). A file that only matches approximately has its score ×0.2. Filters: `TermSetQuery` for kind and language, `RangeQuery` for size and date. |
| `highlight.rs` | `Matcher` also keeps NEAR groups (one sub-matcher per term: an occurrence is kept when every other term has one within the distance, counted in characters) and the LINES range (matches outside are dropped). `Matcher` finds matches **in the original text**: folded word, same stem in the document's language **and at most one letter of difference** (inflections only, not derived words; the Arabic article is ignored), or phrase (exact, `⟦ ⟧`); typo (approximate, `⟪ ⟫`, shown with a wavy underline and a dashed counter). From those it builds snippets (2 lines, ±100/140 characters on long lines) and preview lines (the whole document up to 400 lines, otherwise start + matches ±2 lines; for sheets and slides the `— … —` title of each kept line is kept too), with ⟦ ⟧ markers. |
| `engine.rs` | `Engine` facade: `sites.json` catalog, one index per site (`indexes/<id>/`), **incremental** indexing: `index_site` crawls and compares with the manifest (first time or new version: everything; a missing root keeps its documents), `update_paths` handles the paths reported by the watcher (file → read again; folder → compared; gone → its documents leave), both through `apply` (deletes by `file` term, then `process_files`, one commit, manifest saved after the commit). The data folder is always excluded. multi-site search, preview, `relocate_data` (moves the data folder: rename, or copy + delete across disks). |
| `error.rs` | `CoreError`, serialized as `{ code, params }`. |

### `src-tauri/`

| Command | Purpose |
| --- | --- |
| `app_info` | Version, current data folder, default folder |
| `list_sites` / `add_site` / `remove_site` | Dig sites (`SiteView` = record + `watching` / `ready` / `indexing` / `error` / `empty` status) |
| `index_site` / `cancel_index` | Update in a thread (only changed files are read). Emits `index://progress` (phase `scanning` / `reading` / `saving`) and `index://finished`, then watches the site |
| (startup, `watch.rs`) | Every indexed site is watched, then brought up to date one after the other (changes made while closed). Each watcher batch runs `update_paths` in the site's worker thread (retry while busy) and emits `index://finished`; progress only for batches ≥ 50 files. The UI then runs `search.refresh()`: the last query again, quietly, keeping the selection. |
| `search` / `preview` | Run off the UI thread (`spawn_blocking`). The preview takes the same `SearchRequest` (options). It reads from disk when the document is not in an index (live-scan results). |
| `live_scan` / `cancel_scan` | Live scan in a thread: `scan://hit` for each find, `scan://progress`, then `scan://finished` |
| `list_saved_searches` / `save_search` / `remove_saved_search` | Saved searches (★) |
| `print_report` | Print dialog of the window (PDF report, see Frontend) |
| `get_shortcut` / `set_shortcut` | Global shortcut (`shortcut.rs`, tauri-plugin-global-shortcut). Default `CmdOrCtrl+Shift+Space` (Ctrl+Space switches the keyboard layout on Windows); `""` = none; saved in `settings.json`. If the OS refuses one (used by another app), the previous one is restored and `shortcutUnavailable` is returned. Pressed: window shown + focused + `app://summon` (the UI focuses the search box); pressed while in front: minimized. |
| `set_data_dir` | Moves catalog + indexes, then saves the choice in `settings.json` (config folder) |
| `open_file` / `reveal_file` / `save_text_file` | Open, show in folder, write an export. A file inside an archive or an e-mail is extracted into `<data>/opened` and opened with its application (lot 6.7); a mailbox message opens the mailbox; "show in folder" shows the container. |
| `extract_to` | "Extract to…" (lot 6.7): writes a file inside an archive or an e-mail where the save dialog said. |
| `read_text_file` | Reads saved results (`.prospector`) picked in the open dialog; refused above 64 MB (`readFailed`). |
| (startup, `launch.rs`) | **Single instance** (`tauri-plugin-single-instance`, registered first): a second launch brings the window up (`shortcut::bring_to_front`) and hands its `argv` over. `--in <folder>` (Explorer; `D:"` is read back as `D:\`) and a `.prospector` path are kept in `Launches`; `app://launch` tells the UI. |
| `take_launch_requests` | The waiting requests: `{kind: 'folder', path, siteId}` (`siteId` = indexed site holding it, `Engine::site_holding`) or `{kind: 'results', path}`. The UI (`lib/launch.ts`) opens a tab: indexed + `inFolder` when `siteId`, else live scan; results through `openResultsAt`. |
| `get_integration` / `sync_explorer_menu` / `set_explorer_menu` | Explorer menu (`explorer.rs`, `winreg`): `HKCU\Software\Classes\{Directory,Directory\Background,Drive}\shell\Prospector` → `"exe" --in "%1"/"%V"`. Owned by the app: written again at each start and on a language change (label in the UI language). Setting `explorerMenu`, default on in release, off in a dev build. |
| `set_editor` / `open_in_editor` | Code editor (`editor.rs`): VS Code, VSCodium, Cursor (`--goto file:line`), Notepad++ (`-nLINE`), Sublime Text (`file:line`) detected in their install folders, or a custom command split on spaces outside quotes, `{file}` / `{line}` filled in each argument. Started directly (no shell). Refused for inner documents. |

**Tabs, history, saved results (lot 5.6).**
- Only the tab on screen lives in `search.svelte.ts`. The others are `SearchSnapshot`s (query, name criterion, mode, options, filters, sites, sort, hits, selection, instant filter, "within"). `search.switchTo()` swaps them; `tabs.svelte.ts` keeps the list and, per tab, the back / forward stacks.
- `run()` hands the search it replaces to `beforeRun` (history). Pressing Enter on the same search again adds no step. Restoring shows the stored hits: nothing is searched again, unless the tab was left before the engine answered.
- A live scan left in a hidden tab keeps running: its events go to that tab's snapshot (`#background`); closing the tab cancels it.
- `ranSignature`: the settings of the search on screen. App.svelte re-runs the search only when filters / options / sites differ from it, so a tab, a history step or a saved search never runs twice.
- "Search within these results": `SearchRequest.withinPaths` (engine: `TermSetQuery` on `path`; live scan reads only the files holding them). The instant filter is UI-only (`quickFilter`, name or folder).
- `.prospector` files: JSON `{format: "prospector-results", version: 1, search, hits}`; opened in a new tab, shown whatever the sites selected (`openedFrom`); the preview reads the current file.

**Where the data lives.**
- `settings.json` sits in the OS config folder (`%APPDATA%\app.prospector.desktop`).
- Catalog and indexes go by default to `%APPDATA%\app.prospector.desktop`, can be moved from Settings, and can be forced with the `PROSPECTOR_DATA_DIR` environment variable (dev).

**Browser mode.** Outside Tauri (`inTauri` in `ui/src/lib/api.ts`), the stores fall back to the Étape 0 mocks. Headless screenshots therefore always work.

### Tests

| Command | What it covers |
| --- | --- |
| `cargo test -p prospector-core` | 30 unit tests, 11 end-to-end tests (Étape 1), 9 (Étape 2, `tests/formats.rs`) on `test_fixtures/`, and 6 (Étape 3, `tests/incremental.rs`: update, watcher paths, unplugged root, old index, watcher events, saved searches) |
| `cargo test -p prospector-core --release --test formats -- --ignored --nocapture` | Diagnostics: extraction time per fixture, indexing phases, per-stage costs (`PROSPECTOR_DIAG_ROOT`, `PROSPECTOR_DIAG_DIR`) |
| `python scripts/make-fixtures.py` | Regenerates the fixtures (the PDFs are printed by headless Chrome) |
| `cargo test -p prospector-core --release --test perf -- --ignored --nocapture` | 100k-file benchmark, corpus in `target/tmp/perf-corpus`, target < 200 ms |

**Measured on 2026-09-25**, after BUG-025 (release build, 100,000 FR/ES/EN/AR text files **cold on an external USB hard disk**):
- indexing: **204 s**, limited by the disk;
- searches: **19 to 40 ms**.

The first measure, on 2026-09-24, used files still in the cache, so it was optimistic:
- indexing: **6.0 s**;
- searches: **18 to 41 ms**. The slowest is `contrat OR contrato OR عقد` (75k files match); fuzzy `paiemant` takes 39 ms.

The goal.md target (< 200 ms) is met by a factor of 5.

That corpus has few distinct words. A real site of code (20,875 files, millions of distinct words) shows other costs: dictionary walks (typos, `ab` off) and highlighting. `real_index_timings` measures them on a copy of a real data folder:
`PROSPECTOR_BENCH_DATA=<copy> cargo test -p prospector-core --release --test perf real_index_timings -- --ignored --nocapture`
On 2026-09-28 (BUG-037): 0 to 198 ms; the slowest is `Import` with `Aa` (15,703 candidates checked). Times seen in `pnpm tauri dev` are debug times, 10 to 20 times slower.

Test temporary files go to `target/tmp` (`CARGO_TARGET_TMPDIR`), never to the system TEMP folder.

**Driving the development app.** In debug builds only (`#[cfg(debug_assertions)]` in `src-tauri/src/lib.rs`), the window opens the Chrome DevTools Protocol on `127.0.0.1:9222`. `scripts/drive.py` connects to it with Playwright (`connect_over_cdp`): search, click, read and screenshot the real app with its real indexes (screenshots in `target/tmp/drive/`). Release builds, installed or portable, never open that port.

## Packaging and distribution (Étape 4)

- **Bundle** (`src-tauri/tauri.conf.json` → `bundle`):
  - targets `nsis` (per-user `.exe`, no admin rights, language selector EN/FR/ES/AR) and `msi` (one per language: en-US, fr-FR, es-ES, ar-SA);
  - publisher "Faycal Azib";
  - licence files in `licenses/` next to the executable;
  - WebView2 through the download bootstrapper;
  - `useLocalToolsDir` keeps the NSIS/WiX downloads in `target/` (not on C:).
- **Windows integration in the installers** (lot 5.7):
  - `bundle.fileAssociations`: `.prospector` → the app (`"exe" "%1"`, handled by `launch.rs`);
  - NSIS `installerHooks` (`src-tauri/windows/hooks.nsh`): `NSIS_HOOK_POSTUNINSTALL` deletes the Explorer menu keys, except with `/UPDATE`;
  - WiX `fragmentPaths` + `componentRefs` (`src-tauri/windows/explorer-menu.wxs`): component `ExplorerMenuCleanup` with `RemoveRegistryKey … removeOnUninstall`.
- **Release profile** (workspace `Cargo.toml`): LTO, one codegen unit, stripped. `panic = "unwind"` is **required**: PDF extraction catches pdf-extract panics, and a compile-time guard enforces this (BUG-027).
- **Updates**:
  - `tauri-plugin-updater` + `tauri-plugin-process`. The plugin is only registered when `plugins.updater` exists in the config (public key + GitHub `latest.json` address). Without it the app starts normally, and `AppInfo.updatesEnabled` hides the UI.
  - UI side: `stores/updates.svelte.ts` (check 8 s after start unless turned off, download progress, relaunch), `UpdateBanner.svelte`, and a Settings section.
- **Third-party notices**: `scripts/third-party.py` walks the resolved runtime dependency graph (`cargo metadata`, Windows target) and the npm dependencies, then writes `src-tauri/licenses/THIRD-PARTY-NOTICES.txt` with the full UnRAR licence.
- **Site** (`site/`, static, GitHub Pages): download page and user guide in 4 languages.
  - `i18n.js`: texts, `?lang=`, then the browser language; RTL for Arabic.
  - `download.js`: asks the GitHub API for the latest release and points at its `…-setup.exe`.
  - Fonts are copied locally, as in the app.
- **Automation**: `.github/workflows/release.yml` (tag `v*`: checks, tests, tauri-action → draft release with `latest.json`) and `site.yml` (Pages). The procedure is in [RELEASE.md](RELEASE.md).

## Third-party licences

- Everything is MIT / Apache-2.0 / BSD / OFL, except **UnRAR** (through the `unrar` crate). UnRAR is free to use and redistribute for **extracting** RAR archives, but its source may not be used to create a RAR-compatible archiver. The installer (Étape 4) must ship its licence text (`unrar_sys` → `vendor/unrar/license.txt`).
- Test fixtures copied from crates: `.msg` (msg_parser, MIT), `Empty.pst` (outlook-pst, MIT), `verrouille.rar` (unrar, MIT/Apache).

**Index migration (lot 5.8).** An index written with an older schema (`SchemaError`) is rewritten by `index::migrate` from what it stores: `path`, `name`, `kind`, `lang`, `size`, `modified` and `body` are stored; `file`, `name_raw` and the stemmed fields are derived; `created` is read on the disk (the modification date when the file is gone). The new index is written in `<id>.migrating`, then swapped in (`<id>.previous` removed). No file is read again, no OCR redone; the manifest `VERSION` stays the same. If the migration fails, the old behaviour applies (empty index, rebuilt by the next update). `index::document(DocParts)` is the single builder, used by indexing and migration.

**Alerts and background (lot 6.1).**
- Engine: every successful update (`apply`, so watcher batches **and** full updates, startup catch-up included) records its changed / removed files per site. `check_alerts(site)` takes them and, for each alert on that site, runs its request limited to those files (`SearchRequest.inFiles`, a `TermSetQuery` on `file`; above 10,000 files the whole site). `enable_alert` runs the request once to fill `seen` (up to 20,000 documents).
- Tauri: `watch.rs` (after a batch) and `run_indexing` (after an update) call `check_alerts`, then `background::announce`: one Windows notification per saved search (`tauri-plugin-notification`, title in the UI language, first 3 file names) and `alerts://news` for the badges. The UI (`saved.svelte.ts`) marks the announced documents "new" when the saved search is opened (`search.alertNew`), then `mark_alert_read`.
- `background.rs`: a tray icon (Tauri `tray-icon`; click = open, menu Open / Quit), texts from the UI (`sync_shell_texts`). The window is created **hidden** (`visible: false`) and shown in `setup`, except when started with Windows (`tauri-plugin-autostart`, argument `--background`). `CloseRequested` hides the window instead of quitting when `settings.background` is on, or, by default, as soon as an alert exists.

**Detectors in the UI (lot 6.2).** `Filters.detectors` (saved with searches and results files); "Audit" = all of them, a search without words. The banner above the list shows `search.detections` (the engine's totals in indexed mode, else counted from the hits); a click sets `detectorFocus` (client-side filter). The PDF report adds a summary table and masks, in highlighted passages with at least 8 digits, all but the last 4 characters (`filters.ts` → `maskSensitive`).

**Counts in the UI (lot 6.3).** `search.facets`: the engine's counts in indexed mode, else counted on the hits (live scan, opened results, browser demo). Shown on the kind chips (masthead), the language chips and a "Years" row (filter panel: a click sets a custom period on that calendar year, `toggleYear`), and next to each site of the rail.

**Exports and copy in the UI (lot 6.4).** `TransferDialog.svelte` (`ui.transfer` = `export` | `copy`). `exporting.ts` gathers the hits (shown, or `api.search` with `limit: 10000` in indexed mode), the keyword report (`keyword_report`), then writes through the save dialog, or starts `copy_files` (events `copy://progress` / `copy://finished`, listeners registered before the copy starts). `export.ts` builds the formats: Excel CSV (UTF-8 BOM, `;` in French / Spanish, `,` otherwise, `YYYY-MM-DD HH:MM` dates, one column per term and per detected kind, a totals row), a self-contained HTML page (`dir` of the interface, `<mark>` passages), JSON. The PDF report gets the keyword table when printed from the menu (`ui.reportKeywords`).

**Duplicates in the UI (lot 6.5).** `DuplicatesDialog.svelte` (`ui.duplicates`, rail button). `find_duplicates` runs in a thread (`dup://progress`, `dup://finished`). Files are sent to the Windows Recycle Bin by `trash_files` (crate `trash`, never a permanent delete; inner documents refused); the dialog never lets every file of a group be chosen and always asks first. The folder watcher then removes them from the index.

**Images in the UI (lot 6.6).** `thumb` returns a `data:image/jpeg;base64` URL (allowed by the CSP). `thumbs.ts`: session cache, at most 3 thumbnails made at a time; list cards get theirs when shown (BUG-033: not through an IntersectionObserver). The preview of an image shows a 1,200 px thumbnail with `image_matches` boxes overlaid (percentages, `direction: ltr` inside the frame), then the OCR text as before.

**Meaning search (lot 8.3).** `SearchRequest.meaning`; the app's `search` command computes the question's vector (`sense::question_text`: operators, excluded words, regexes and quotes removed; a model kept for questions, `SenseRuntime.questions`) and calls `Engine::search_meaning`: the words (the usual search, if there is text), then the 400 closest passages grouped by document (best passage each), kept within 0.05 of the best score (scores are close together) and at most 60; those documents go through the usual filters by a search by name (`*`, or the name criterion) restricted to them (`within_paths`); both lists are fused by reciprocal ranks (k = 60). `Hit.meaning` (`score`, passage `start`/`end`, `only` = no word of the search in it); a document found by meaning only gets its passage as snippet. `Engine::preview_passage` marks that passage when no word is found (the preview opens on it). UI: the "≈ Meaning" chip (module installed and a site understood; off in live scan), "≈ By meaning" + passage and a ≈ circle on the cards, a ≈ mark on the others, "N by meaning" in the readout. `prospector-cli --meaning` (the module of the user's personal folder), `meaning` in JSON and CSV. Mockups `docs/mockups/meaning-results-*.png`.

**Meaning index in the app (lot 8.2).** `src-tauri/src/sense.rs`: `SenseRuntime` (models loaded when first needed at the chosen pace, speed measured then), a thread woken by `sense::wake` (end of an indexing or of a folder-watch update, a box ticked, the module installed; otherwise every 5 minutes) that computes the ticked sites one after the other while this PC holds the lease; `sense://site` progress (≤ 1 per 0.7 s), `AppState.sense_progress` in `SiteView`, and `sensePassages`; `sense_estimate` (passages + seconds at this PC's speed), `set_site_sense`, `get_sense_pace` / `set_sense_pace` (`Settings.sense_pace`). UI: in the rail, "≈ Meaning · 42 % · ≈ 1 h 10 left" with a bar (time left from the pace seen), then "≈ Meaning up to date · N passages"; the ≈ action of a site (when the module is installed) asks with its estimate; `SenseSitesDialog.svelte` after installing the module (sites of an hour or less pre-ticked); Settings: normal / economy pace. Mockups `docs/mockups/sense-rail-*.png`, `sense-dialog-*.png`.

**Meaning module in the app (lot 8.1).** `src-tauri/src/sense.rs`: `sense_status`, `install_sense_module` (a chosen ZIP), `download_sense_module` (reqwest with rustls + ring, as the updater: nothing new to ship; `sense://progress`, cancel flag; the ZIP SHA-256 checked), `cancel_sense_download`, `remove_sense_module`; `cleanup` at startup. UI: `stores/sense.svelte.ts`; Settings → "Meaning search (local AI)" after the OCR section (not installed / downloading / installed; download disabled while `MODULE_URL` is unset). Mockups `docs/mockups/sense-settings-*.png`.

**Term lists in the UI (lot 7.4).** `Filters.termList` (`{ name, terms, skipped, all }`, kept by value), so tabs, history, saved searches, results files and the automatic re-run carry it; `buildRequest` sends `termList` / `termListAll`. `TermList.svelte` in the row of criteria: "+ Term list" (`pickTermFile`, `read_term_list`), then a chip that opens a panel (mode, preview of 12 terms, keyword report = the export dialog, replace, remove), placed towards the side with room. `hasTerms` counts a list, so the export gets a column per term; the PDF report lists it in its criteria. `prospector-cli --terms-file` / `--terms-all`. Mockups `docs/mockups/terms-*.png`.

**Shared index in the app (lot 7.3).** `src-tauri/src/share.rs`: at startup `share::start` claims the lease (only the holder runs `watch::start_all`), then a thread every 30 s renews it; a role change starts or stops the watchers (`cancel_all` for running indexings), and emits `share://status` / `sites://changed` (also when the other PC changed the catalog). `set_data_dir` → `share::restart` (the old lease is released). `RunEvent::Exit` releases the lease. `add_site` and `set_data_dir` store mapped drives as UNC. UI: `sitesStore.share` / `readOnly`; the rail shows who keeps the index and hides add / reindex / remove for a reader; adding a local-disk site to a shared index shows a warning. `prospector-cli`: `index` claims the lease (refused, code 3, if another PC holds it) and releases it; other commands open read-only when someone else holds it; `--data` keeps the personal files in that folder too.

**Site groups in the UI (lot 7.2).** `stores/groups.svelte.ts` (list from `list_site_groups`; `active` = `'all'` or the group whose sites are exactly the ticked ones; `apply` ticks them in the current tab, `applyIndex` for Ctrl+0…9 read from `event.code`, so AZERTY works). `SiteGroups.svelte` at the top of the sites panel: chips (pencil `GROUP_COLORS[color]`, name, digit), "+ Group" (name editor, saves the ticked sites), right-click menu (rename, use the ticked sites, remove); numbered chips only in the compact rail. Each site shows the pencils of its groups under its check box. Mockups: `docs/mockups/groups-*.png`.

**Command line (lot 7.1).** Crate `cli/` → `prospector-cli.exe` (clap). Same data as the app (`locate::Location`, `--data` to force it). `search` → `Engine::search`; `scan` → `live_scan` (hits printed from the scan threads through a `Mutex<Printer<Stdout>>`); `index` → `index_site` with the site's last exclusions, refused while the app is open (its single-instance mutex `<identifier>-sim` exists, `OpenMutexW`); `sites`. `Criteria` maps every filter to `SearchRequest`. `output.rs`: text (`[match]`), JSON (one document), JSON Lines (streamed), CSV (header, first snippet); marks removed outside text; ISO 8601 dates. Exit codes 0 found / 1 nothing / 2 usage or query / 3 data. Hidden `uninstall-cleanup` removes its folder from the PATH (NSIS `NSIS_HOOK_PREUNINSTALL`, WiX custom action before `RemoveFiles`). The app offers Settings → "Command line in the PATH" (`set_cli_path`, when `prospector-cli.exe` is next to it and not portable). Packaging: `src-tauri/tauri.release.conf.json` (merged with `--config`) builds it first and adds it as a resource, so `pnpm tauri dev` does not need it; the portable ZIP includes it.

**Portable mode (lot 6.8).** `src-tauri/src/portable.rs`: a `portable` file next to `prospector.exe` → everything in `ProspectorData\` beside it (`settings.json`, `data\` = catalog + indexes + `opened`, `webview\` = WebView2 data, i.e. the interface's `localStorage`). The main window is made in `setup` (`"create": false` in `tauri.conf.json`, `WebviewWindowBuilder::from_config` + `data_directory`), and the identifier gets `.portable` (single instance: never hands over to an installed Prospector). `Settings.portable_drive` records the drive letter; at start, a new letter runs `portable::move_drive` before `Engine::open`. Nothing is written in the registry: Explorer menu and start with Windows refused and hidden (`IntegrationStatus.portable`, `BackgroundStatus.portable`); "Index folder" replaced by the portable folder (`AppInfo.portableDir`); updates are only announced, "Download" opens the releases page (`open_releases_page`, derived from the updater endpoint). `scripts/portable.py` makes the ZIP; `release.yml` uploads it to the draft release; the download page links it (`download.js`: asset `*-portable.zip`).

**Extraction in the UI (lot 6.7).** `hit.innerKind` (from the engine) marks a file inside an archive or an e-mail: the preview shows **Extract to…** (`pickSaveAs` with the file's own name, then `extract_to`), and **Open** opens the file itself. An image inside a container (`isImageHit`) gets its picture and boxes like a file on disk (`thumbnail` and `image_matches` extract it first); in the list, not inside PST / OST / mbox (`thumbInList`: each picture would read the whole mailbox again). The copy dialog has "Extract the files from archives and e-mails" (ticked by default).
