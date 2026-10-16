# Bug log

> Single file for all bugs, so the same problem is never hit twice.
> Format: ID · date · step · context · symptom · cause · fix · prevention.

---

## BUG-001: TypeScript 7 incompatible with svelte-check

- **Date:** 2026-09-24 · **Step:** Étape 0 (scaffolding)
- **Context:** `pnpm add -D typescript` installed TypeScript **7.0.2**, the new native Go compiler.
- **Symptom:** pnpm warned `unmet peer typescript@"^5.0.0 || ^6.0.0": found 7.0.2` for `svelte-check@4.7.6`.
- **Cause:** svelte-check 4.x relies on the JS TypeScript API, which is not exposed in the same way in TypeScript 7.
- **Fix:** pinned `typescript@^6` in `devDependencies`.
- **Prevention:** stay on TypeScript 6 until svelte-check officially supports version 7. Check the peer dependencies before bumping TypeScript.

## BUG-002: stateful regex in `normalize()` (caught before it ever ran)

- **Date:** 2026-09-24 · **Step:** Étape 0 (`ui/src/lib/text.ts`)
- **Context:** normalizing text by removing French and Spanish accents and Arabic tashkeel.
- **Symptom (latent):** a regex with the `g` flag, reused through `.test()` inside a `.replace()` callback, keeps its `lastIndex` between calls. One character in two could slip through, so results would be missed at random.
- **Fix:** the pipeline is now fully declarative:
  - `NFD`
  - `.replace(/[̀-ͯ]/g, '')`
  - `.replace(ARABIC_MARKS, '')`
  - alef, ya and ta marbuta unification
  - `NFC`
- **Prevention:** never call `.test()` or `.exec()` on a `g` regex shared between calls. Use `.replace()` or `.match()`, or a regex without `g`.

## BUG-003: progress bar margin not mirrored in RTL

- **Date:** 2026-09-24 · **Step:** Étape 0 (`IndexRail.svelte`)
- **Symptom (caught in review):** `margin: … 22px` (physical left) plus an RTL override `margin-inline: 0 22px` put the indent on the **wrong** side in Arabic.
- **Cause:** mixing physical properties with a hand-written RTL override.
- **Fix:** `margin-block` plus `margin-inline-start: 22px`, with no RTL override.
- **Prevention:** rule in docs/I18N.md: **logical properties only**, and never write an RTL override for a margin or padding.

## BUG-004: svelte-check a11y warning on the preview

- **Date:** 2026-09-24 · **Step:** Étape 0 (`PreviewPanel.svelte`)
- **Symptom:** `a11y_no_noninteractive_element_interactions`: `onkeydown` on a `div role="document"`.
- **Fix:** Enter / Shift+Enter (next or previous match) is handled by the `svelte:window` handler, but only when the document area has focus. The area becomes `role="region"`, focusable so it can be scrolled with the keyboard.
- **Prevention:** put global shortcuts on `svelte:window` and never attach them to non-interactive containers.

## BUG-005: light native title bar on the dark UI (Windows)

- **Date:** 2026-09-24 · **Step:** Étape 0 (first `pnpm tauri dev` screenshot)
- **Symptom:** the native Windows title bar stays white above the Void Navy interface.
- **Cause:** the Tauri window follows the OS theme by default (light).
- **Fix:** `"theme": "Dark"` on the `main` window in `src-tauri/tauri.conf.json`.
- **Prevention:** once the light theme exists (goal.md §5), sync the window theme with the in-app theme through `window.setTheme()`.

## BUG-006: the dev variant switcher hides the last result

- **Date:** 2026-09-24 · **Step:** Étape 0 (screenshot)
- **Symptom:** the fixed switcher at the bottom of the screen covers the last result row (`facture-2024-117.xlsx`).
- **Fix:** in dev mode, the `.dev-bar` class on `.app` adds `padding-block-end: 64px` to the results list.
- **Prevention:** any fixed overlay must reserve its space in the scrollable content underneath.

## BUG-007: physical `padding` shorthands in the redesign (caught before running)

- **Date:** 2026-09-24 · **Step:** Étape 0, redesign
- **Symptom (latent):** asymmetric 4-value `padding` in `ResultsList` (readout, rows, ledger header) and `PreviewPanel` (document). The larger indent would have landed on the wrong side in Arabic.
- **Cause:** the same mistake as BUG-003. A 4-value shorthand is always physical (top, right, bottom, left).
- **Fix:** switched to `padding-block` plus `padding-inline: <start> <end>`.
- **Prevention:** before every build, run `grep -rnE "(padding|margin):\s*\S+\s+\S+\s+\S+\s+\S+;" ui/src`. A 4-value shorthand is only allowed when start equals end.

## BUG-008: "Refine" button reopening the panel it just closed (caught before running)

- **Date:** 2026-09-24 · **Step:** Étape 0, redesign (`actions.ts`)
- **Symptom (latent):** with outside-click closing, a click on "Refine" fires `pointerdown` (close) then `click` (toggle → reopen), so the panel could never be closed with its own button.
- **Fix:** `dismissable` ignores clicks on an element with `aria-controls="<panel id>"`.
- **Prevention:** a toggle that controls a dismissable panel must declare `aria-controls`. That also improves accessibility.

## BUG-009: removing an unused component breaks the check

- **Date:** 2026-09-24 · **Step:** Étape 0 (the variants became a user setting)
- **Symptom:** `pnpm check` reported 2 errors in `VariantSwitcher.svelte`, which still used the removed `dev.variant` keys.
- **Cause:** typed i18n keys (`MessageKey`) catch every orphan reference, which is what we want. The file could not be deleted because the local hook blocks deletions.
- **Fix:** the file was moved out of the project (to the session scratchpad).
- **Prevention:** when a feature is removed, grep the removed keys (`dev.`) before `pnpm check`.

## BUG-010: ambiguous digits in the handwritten font (Architects Daughter)

- **Date:** 2026-09-24 · **Step:** Étape 0, "Colored pencils" theme
- **Context:** the mockup (`g-crayons.html`) used Architects Daughter for body text.
- **Symptom:** in the headless-Chrome screenshot of the real app, "ARTICLE 11" read "ARTICLE ll", "annexe 1" read "annexe l" and "48 213" read "48 2l3". The "1" looks like a lowercase "l". For a search tool (dates, amounts, references), that is an unacceptable legibility bug.
- **Cause:** a stylistic choice of the font (a "1" with no flag or base).
- **Fix:** body text switched to **Patrick Hand**, whose "1" has a flag and which matches the Patrick Hand SC titles. Architects Daughter was removed from the dependencies.
- **Prevention:** before adopting a font, render the string `Il1 O0 5S 8B 2024-11` in it and check that each character is unambiguous.

## BUG-011: svelte-check cannot type a component without a `<script>` block

- **Date:** 2026-09-24 · **Step:** Étape 0 (`ThemeDefs.svelte`)
- **Symptom:** `Could not find a declaration file for module './lib/components/ThemeDefs.svelte' … implicitly has an 'any' type` in `App.svelte`.
- **Cause:** a markup-only component (SVG filters) with no `<script lang="ts">` is not picked up as a TS component by svelte-check 4.
- **Fix:** added a `<script lang="ts">` block holding only a comment.
- **Prevention:** every `.svelte` component starts with a `<script lang="ts">` block, even an empty one.

## BUG-012: masthead chips wrap onto two lines

- **Date:** 2026-09-24 · **Step:** Étape 0, "Colored pencils" theme
- **Symptom:** the mode chips (Indexed / Live scan) dropped to a second line, eating ~35 px of height and cutting off the "Saved searches" panel in the rail.
- **Cause:** `max-inline-size: 1200px` on `.chips`, plus chips that were a bit too wide (16 px, 12 px padding).
- **Fix:** removed the cap; chips set to 15 px with 10 px padding; the rail tightened (smaller panel paddings, metadata on one line with ellipsis).
- **Prevention:** check at 1600×1000 in FR **and** ES (the longest labels, e.g. "ARCHIVOS COMPRIMIDOS") with a headless screenshot.

## BUG-013: pixel font where "C" looks like "O" (Pixelify Sans)

- **Date:** 2026-09-24 · **Step:** Étape 0, "Sonar" theme
- **Symptom:** the title read "PROSPEOTOR" and the ribbon "MON OARNET DE FOUILLE". The same font already had the "3"/"8" ambiguity ("30" read as "80" in mockup C).
- **Cause:** Pixelify Sans draws "C" almost closed.
- **Fix:** replaced by **Tiny5** (open "C"). The pixel font is confined to `--font-brand` (title, panel headers, never digits); everything else uses Chakra Petch.
- **Prevention:** the legibility test in THEMES.md ("Adding a theme"), rendered in headless Chrome, for every new font.

## BUG-014: overlapping Ledger headers and overflowing preview bar (Sonar theme)

- **Date:** 2026-09-24 · **Step:** Étape 0, "Sonar" theme
- **Symptom:** "CORRESP." and "TAILLE" overlapped ("CORRESPTAILLE"); in the preview, "1 / 7" was pushed out of the panel.
- **Cause:** Chakra Petch is wider than Patrick Hand. The fixed widths (52 px column, action bar without wrapping) had been tuned for the Crayons theme only.
- **Fix:** Matches column set to 72 px, headers at 15 px with an ellipsis, and a `flex-wrap` on the preview action bar.
- **Prevention:** every layout change is screenshotted in **both** themes (the widest labels are in Sonar and in ES).

## BUG-015: `filter: … none` trap when a theme has no wobble

- **Date:** 2026-09-24 · **Step:** Étape 0, theme switch (caught before running)
- **Symptom (latent):** the title uses `filter: var(--title-shadow) var(--wobble)`. With `--wobble: none`, the declaration becomes `drop-shadow(...) none`, which is invalid, so the whole filter would have been dropped (no glow at all).
- **Fix:** a theme without the wobble effect sets `--wobble: opacity(1)`, a valid filter that does nothing.
- **Prevention:** rule written in THEMES.md: a variable combined inside a list of values must never be `none`.

## BUG-016: API changes in quick-xml 0.42 and chardetng 1.0

- **Date:** 2026-09-24 · **Step:** Étape 1 (`core/src/extract/`)
- **Symptom:** compile errors. `e.local_name().as_ref()` now returns `&str`, not `&[u8]`. `EncodingDetector::new()` and `guess()` take enums (`Iso2022JpDetection`, `Utf8Detection`) instead of booleans.
- **Fix:** match element names as strings (`"t"`, `"p"`…). Pass `Iso2022JpDetection::Deny` and `Utf8Detection::Allow`.
- **Prevention:** before using a crate, read its real API in `~/.cargo/registry/src/…` (what I did for Tantivy), never from memory.

## BUG-017: double search on the first run (Svelte effect)

- **Date:** 2026-09-24 · **Step:** Étape 1 (`App.svelte`, caught before running)
- **Symptom (latent):** the effect that re-runs the search when filters change read `search.hasRun`. The first search flips `hasRun` to `true`, which re-fired the effect, so the same search ran a second time 250 ms later.
- **Fix:** read `hasRun` through `untrack()`; the effect only depends on the filters, the options and the scope.
- **Prevention:** in an effect that triggers an action, read the action's own state **untracked**.

## BUG-018: demo site with no indexing phase

- **Date:** 2026-09-24 · **Step:** Étape 1 (browser mode, mocks)
- **Symptom:** in the headless screenshot, the demo site "Mail 2019–2024" read "Looking for files… 0 found" instead of its 62 % bar.
- **Cause:** the new rail picks what to show from the `phase` (`scanning` / `reading` / `saving`), and the Étape 0 mock only had `progress`.
- **Fix:** `phase: 'reading'` added to the mock.
- **Prevention:** when a type gains a field, update the mocks in the same commit, then take a browser-mode screenshot.

## BUG-019: "alger" finds "alter" (typo tolerance too permissive and hidden)

- **Date:** 2026-09-24 · **Step:** Étape 1, first real test by the user (Tauri window, Sonar theme)
- **Symptom:** a search for "alger" with typo tolerance on returned 14 matches of "alter" in code (n8n JSON workflows, Python). They were highlighted like exact matches, with nothing telling them apart.
- **Causes:**
  1. `fuzzy_distance` allowed 1 error from **4 letters**. On a short word, a single different letter usually gives another real word ("alger"/"alter", "prix"/"pris"), not a typo.
  2. Exact and approximate matches were not distinguished anywhere: same `⟦ ⟧` marker, same score, same count.
- **Fix:**
  - **No tolerance up to 5 letters**, 1 error from 6 to 9, 2 from 10 (`highlight.rs`, used by both the query and the highlighting).
  - `Match { range, fuzzy }`: approximate matches are marked `⟪ ⟫` and shown with a wavy underline, no highlighter.
  - `Hit.exactCount`: a file with no exact match gets score ×0.2 (shown after the others) and a dashed count circle.
  - `SearchResponse.exactFiles`, plus a banner "No exact match for « … »: these files contain close words" when every result is approximate.
  - Tests `short_words_get_no_typo_tolerance` and `approximate_matches_are_flagged_and_marked_apart`.
- **Prevention:** every approximate feature (fuzzy, stems, synonyms later) must be **visible** as such in the UI. Test typo tolerance with real near-miss word pairs (alger/alter, prix/pris), not only with real typos.

## BUG-020: "alger" finds "Algérie" (stemming grouped two different words)

- **Date:** 2026-09-24 · **Step:** Étape 1, user test in the Tauri window (right after BUG-019)
- **Symptom:** "alger" returned `cvData.ts` with "Algérie" highlighted as an **exact** match, next to "Alger".
- **Cause:** the French Snowball stemmer reduces "algérie" to "alger", the same stem as "alger". Stemming meant for plurals ("contrats" → "contrat") also grouped a derived word, and here a proper noun (country vs city).
- **Fix:**
  - Two words with the same stem only count as the same word if they differ by **one letter at most**: "contrat"/"contrats", "contrato"/"contratos", "signé"/"signée" pass; "Alger"/"Algérie" does not.
  - For Arabic, the lengths are compared **without the article** (ال, وال, بال, كال, فال, لل), so "عقد" still finds "العَقْدَ".
  - The highlighter becomes the final judge: a file returned by Tantivy (stemmed field) where it finds no occurrence, neither in the text nor in the name, is no longer a result.
  - Tests `stems_group_inflections_not_other_words` (Alger/Algérie, signé/signée/signés, عقد/العَقْدَ).
- **Accepted trade-off:** derived words are no longer grouped ("résilier" does not find "résiliation"). A precise search matters more here than a broad one.
- **Prevention:** test stemming on **pairs of different words that share a stem** (Alger/Algérie, général/généralité), not only on plurals.

**Side note (not a bug):** Rust test timings (0.5 s → 26 s) varied because `pnpm tauri dev` was running at the same time. It recompiles the engine on every edit and holds the build folder's lock. Take time measurements with `tauri dev` stopped.

## BUG-021: indexing a small site took 17 s (writer sized for big sites)

- **Date:** 2026-09-25 · **Step:** Étape 2 (diagnosed from slow tests)
- **Symptom:** indexing the fixtures (23 files) took 17 s, 16 of them in the "Saving" phase (commit + merge).
- **Measurements** (tests `extraction_timings` and `indexing_phase_timings`, `--ignored`):
  - reading all the files took < 200 ms;
  - "Saving" took **4.3 s on E:** and **16 s on H:\…\target** (see BUG-022).
- **Cause:** `index.writer(256 MB)` starts one indexing thread per core. Each thread writes its own segment, and the segments then have to be merged, even for 20 files.
- **Fix:** 1 thread per ~1000 files (max 8), 64 MB per thread (`writer_with_num_threads`). Fixtures: **5.3 s → 0.9 s** (E:).
- **Prevention:** size resources by volume. Every step has timing diagnostics in `core/tests/formats.rs`.

## BUG-022: "Access is denied" writing an index (antivirus)

- **Date:** 2026-09-25 · **Step:** Étape 2 (end-to-end tests)
- **Symptom:** tests failed intermittently with `Failed to open file for write … Access is denied`, then passed on a rerun. Index writes were also 4× slower on H:\…\target than on E:.
- **Cause (probable):** Windows Defender scans each index file as it is created, and briefly locks it. `pnpm tauri dev` recompiling in parallel adds to the contention.
- **Status:** not fixed in code.
  - Fewer threads means fewer files and fewer conflicts (BUG-021).
  - A retry on the index commit, or advising a Defender exclusion for the index folder, will be studied at Étape 4 (packaging).
- **Prevention:** measure timings with `tauri dev` stopped. Do not treat a single "Access is denied" as a code regression.

## BUG-023: the preview would have reloaded on every keystroke

- **Date:** 2026-09-25 · **Step:** Étape 2 (`search.svelte.ts`, caught before running)
- **Symptom (latent):** the preview reloads whenever its request changes. The first version reused the full search request, which contains the date filter (`Date.now()`), so the request changed on every render.
- **Fix:** `previewRequest()` only returns the last query and the options (`Aa`, `ab`, `.*`, typos). It has no filter and no clock.
- **Prevention:** a "request" object used as a cache key must never contain `Date.now()` or random values.

## BUG-024: fixture generator wrote through C: and failed

- **Date:** 2026-09-25 · **Step:** Étape 2 (`scripts/make-fixtures.py`)
- **Symptom:** `ValueError: path is on mount 'C:', start on mount 'H:'` when building a Word file to put inside a ZIP.
- **Cause:** the helper wrote the .docx to the system temp folder (C:) to read it back, then computed a relative path from the project (H:).
- **Fix:** `docx_raw()` builds the .docx in memory. The helper no longer touches C:.
- **Prevention:** fixtures are built in memory (`zip_bytes`, `docx_raw`). Nothing is written outside the project.
- **Follow-up (2026-09-27, Étape 3):** `pdf_from_html()` still wrote its HTML source to the system temp folder (C:). It now writes to `target/tmp` in the project, and `tempfile` is no longer imported by the script.

## BUG-025: indexing 100,000 files never finished (random reads on a hard disk)

- **Date:** 2026-09-25 · **Step:** Étape 2 (100k-file perf test, re-run after BUG-021)
- **Symptom:** the Étape 1 perf test (6 s) no longer finished in 10 minutes. The process sat at ~0 % CPU, and "Reading" slowed down as it went (40k files in 65 s, 80k in 342 s).
- **Investigation:**
  - Per-file stages were measured on the same files: extraction 0.16 ms, detection 0.09 ms, tokenization 0.07 ms, so the code costs ~0.3 ms per file.
  - Windows reads one file in 0.57 ms (0.16 ms from cache).
  - The disks: **H: is an external USB hard disk** (WD Elements) and E: a SATA hard disk; only C: is an SSD.
- **Cause:** 16 threads read thousands of small cold files in random order, so the disk heads seek all the time. At Étape 1, the corpus had just been written and was still in the file cache, which is why the measure was 6 s (too optimistic).
- **Fix:** `core/src/pipeline.rs`.
  - **One thread reads** the files, sorted by path (nearly sequential on disk), with a read-ahead of 256 files.
  - The CPU work (extraction, detection, indexing) runs on the other cores (`par_bridge`).
  - Containers (ZIP, PST) are read by the workers, entry by entry.
  - Used by both indexing and live scan.
- **Result:** 100,000 **cold** files on the USB hard disk are indexed in **204 s** (~500 files/s, limited by the disk). Searches stay at 19–40 ms.
- **Prevention:**
  - Always measure performance on **cold** data (files not just written) and on the slowest disk available.
  - `pipeline_stage_timings` / `indexing_phase_timings` (`--ignored`) separate CPU cost from disk cost.

## BUG-026: a live-watch update could be lost when a file is briefly locked

- **Date:** 2026-09-27 · **Step:** Étape 3 (incremental indexing, saved searches)
- **Symptom:** once in about fifteen runs, `a_second_update_reads_only_what_changed` failed on `index_site(...).unwrap()`, then passed on every rerun (not reproduced in 12 more runs).
- **Context:** this is the same family as BUG-022: Windows Defender scans each new file and briefly refuses access to it. Live watching makes writes much more frequent: every save leads to a commit, a manifest rewrite and a `sites.json` rewrite. In the app, the watcher worker **dropped** a batch that failed, so the change was only caught up at the next startup.
- **Fix:**
  - `core/src/fsutil.rs` → `write_atomic`: temporary file, then rename, **retried for up to 1 s** while Windows answers "access denied". Used by `sites.json`, the manifests and `saved-searches.json`.
  - `src-tauri/src/watch.rs`: a batch that fails is **retried 3 times** (3 s apart). After that, the next update of the site catches it up: every run compares against the manifest.
- **Prevention:**
  - Every JSON store goes through `write_atomic`, never `write` + `rename` by hand.
  - Watcher errors are retried, not ignored.
  - As with BUG-022, an isolated failure that passes on rerun points first at the antivirus.

## BUG-027: in the release build, one damaged PDF would have closed the whole app

- **Date:** 2026-09-27 · **Step:** Étape 4 (packaging, found while reviewing the release profile, before any installed build)
- **Symptom (latent):** none in tests. In an installed build, indexing a folder with a malformed PDF would have made Prospector disappear without a message.
- **Cause:** the workspace `[profile.release]` had `panic = "abort"` (a common size optimization, set at Étape 0). `core/src/extract/pdf.rs` protects against pdf-extract panics with `catch_unwind`, which only works when panics unwind. Tests always run with unwinding, so they could not see it.
- **Fix:**
  - `Cargo.toml`: `panic = "unwind"`, with a comment explaining why.
  - `core/src/extract/pdf.rs`: `#[cfg(panic = "abort")] compile_error!(…)`, so the build fails if someone switches back.
- **Prevention:** any `catch_unwind` in the code comes with this compile-time guard. Review the release profile before packaging: it is not covered by tests.

## BUG-028: `tauri build` refused to start (Tauri Rust and JS versions out of step)

- **Date:** 2026-09-27 · **Step:** Étape 4 (first installer build)
- **Symptom:** `Found version mismatched Tauri packages … tauri (v2.12.0) : @tauri-apps/api (v2.11.1)`. `pnpm tauri dev` would have failed the same way.
- **Cause:** `cargo add tauri-plugin-updater` updated the lock file and pulled the `tauri` crate to 2.12.0. The npm packages stayed at 2.11.
- **Fix:** `@tauri-apps/api` and `@tauri-apps/cli` moved to ^2.12.0.
- **Prevention:** after adding or updating a Tauri plugin on one side (Rust or npm), align the other side on the same minor version, then run `pnpm tauri build` (it checks this; `cargo check` does not).

## BUG-029: OCR silently returned nothing when several files were read at once

- **Date:** 2026-09-27 · **Step:** Étape 5, lot 5.2 (OCR), found by the tests before delivery
- **Symptom:** the scanned images were found through the index but not by the live scan run at the same time (`tests/ocr.rs`: "index and live scan disagree").
- **Cause:**
  - A Windows `OcrEngine` does not accept several recognitions at once: concurrent `RecognizeAsync` calls fail, and the failure became an empty text.
  - The engines were shared by every thread, since they are marked thread-safe (agile WinRT objects), while the indexing pipeline reads files on all cores.
- **Fix:** `core/src/extract/ocr.rs`, **one recognizer per thread** (`thread_local!` + `OnceCell`). The list of installed languages is read once for the whole process. Parallelism is kept.
- **Prevention:**
  - A WinRT object that is "thread-safe" is not necessarily usable by two threads **at the same time**; test it under load (here, index and live scan in parallel).
  - The OCR tests compare the index and the live scan on every word.

## BUG-030: generating the Office test files with Word hung, and turned off the user's spell check

- **Date:** 2026-09-27 · **Step:** Étape 5, lot 5.3 (test fixtures), found before delivery
- **Symptom:** the script driving Word and PowerPoint (COM automation) to write real `.doc`/`.ppt` samples never finished. An invisible `WINWORD.EXE` stayed blocked, writing nothing.
- **Investigation:**
  - A minimal script (save "test" as `.doc`) worked.
  - The same save with full French sentences hung. Word's live spelling and grammar check blocks `SaveAs` in automation.
  - Turning the check off fixed the minimal case. **These two options are stored in the user's Word profile**, so turning them off changed the user's Word.
  - They were put back at once, and after each killed instance: `CheckSpellingAsYouType` / `CheckGrammarAsYouType` = True, checked.
  - Even then, the full script still hung when run from the shell or as a job, for a cause not found.
- **Fix:** Office is no longer driven.
  - `.doc` and `.ppt`: **real files from the Apache POI test corpus** (Apache License 2.0), whose text is asserted by POI's own tests: `test2.doc`, `HeaderFooterUnicode.doc`, `rasp.doc`, `basic_test_ppt_file.ppt`, `with_textbox.ppt`.
  - RTF, ODT and ODP: written by `scripts/make-fixtures.py`.
  - The Office script was moved out of the project.
- **Prevention:**
  - Never drive the user's Office applications from a script: they share the user's profile and their dialogs are invisible.
  - Prefer public test corpora (POI, Tika) for real files.

## BUG-031: in live scan, the name criterion let whole messages through

- **Date:** 2026-09-27 · **Step:** Étape 5, lot 5.4 (e-mail attachments), found by the tests before delivery
- **Symptom:** `*.docx` as file name found only the attached Word file through the index, but in live scan it also listed every `.msg` and `.eml`: "index and live scan disagree".
- **Cause:** until then a container (ZIP, PST) produced only inner documents, and the live scan checked the name criterion only on those. A `.msg` or `.eml` is now **also a document of its own** (the message, without inner path), which slipped past the check.
- **Fix:** `core/src/scan.rs`: a document without inner path is checked against its file name, an inner document against its own name or its container's (the index's rule, `names.rs`).
- **Prevention:** every new document kind is tested with `both()` (index **and** live scan), with and without a name criterion.

## BUG-032: a folder spelled in another case gave nothing through the index

- **Date:** 2026-09-27 · **Step:** Étape 5, lot 5.7 (Explorer right-click, "search in this folder"), found by the tests before delivery
- **Symptom:** limited to `…\TEST_FIXTURES\FR` (upper case), the live scan found 4 files, the index none: "index and live scan disagree".
- **Cause:** Windows ignores case, the index does not. The folder was compared to the site's root without case (`scope::inside`), and the root part was rewritten with the root's spelling. But the part **below** the root kept the spelling it was given (`FR`), while the index stores `fr`: the path prefix matched no document.
- **Fix:** `core/src/scope.rs`, `respell`: each folder below the root takes the name its parent folder lists on disk (case-insensitive comparison). A name that cannot be listed is kept as given.
- **Prevention:**
  - Any path that comes from outside (Explorer, command line, a typed path) is spelled as on the disk before it is compared with indexed paths.
  - The test `a_folder_inside_the_site_limits_both_modes_whatever_the_case` checks the same folder in upper case, through the index and the live scan.

## BUG-033: list thumbnails never loaded (IntersectionObserver silent)

- **Date:** 2026-09-28 · **Step:** Étape 6, lot 6.6 (image thumbnails), found by the screenshots before delivery
- **Symptom:** image results kept their doodle icon instead of their thumbnail; the preview's big image did show.
- **Investigation:** the Svelte action did run (console trace), but the `IntersectionObserver` callback never came, not even the first one it always sends. A minimal page with the same observer, in the same headless Chrome, did get it. The cause inside the app page was not found (constant timers of the results animation? the scroll container?).
- **Fix:** no dependency on the observer: the thumbnail is asked when the card is shown (`thumbs.ts`, `lazyThumb`). The list holds at most a few hundred results, thumbnails are made 3 at a time and cached for the session, so the cost stays small.
- **Prevention:** check a lazy-loading effect in the DOM (`--dump-dom`), not only on a screenshot; prefer a bounded queue to visibility tricks when the list is bounded.

## BUG-034: `prospector-cli scan` panicked (two arguments named `folders`)

- **Date:** 2026-09-28 · **Step:** Étape 7, lot 7.1 (command line), found by the end-to-end tests before delivery
- **Symptom:** every `scan` command ended with exit code 101 (a panic) instead of its results; `search`, `index` and `sites` worked.
- **Cause:** the positional folders of `scan` (field `folders`) and the shared `--folders` option of the criteria (look for folders) had the same clap id. clap only checks that ids are unique when the subcommand is parsed, in debug builds: nothing showed at compile time, nor for the other commands. A release build would have mixed the two up silently.
- **Fix:** the positional argument is named `paths` (still shown as `<FOLDER>` in the help).
- **Prevention:** a unit test runs `Cli::command().debug_assert()`, which checks every subcommand at once (`cli/src/main.rs`).

## BUG-035: a text got another meaning vector when batched with longer ones

- **Date:** 2026-09-28 · **Step:** Étape 8, lot 8.1 (meaning module), found by the test on the real model before delivery
- **Symptom:** "عقد إيجار شقة" computed alone and computed in a batch of six texts gave vectors only 0.966 similar (0.980 for a French sentence), where the same text must give the same vector. The gaps that matter are of the same size (0.87 for a translation, 0.68 for an unrelated text): a passage indexed in one batch and a query computed alone would not have matched reliably.
- **Cause:** the tokenizer padded the shorter texts of a batch to the longest one. With the quantized model (int8, scales computed on the whole input), the padding tokens change the result even though the attention mask hides them. Texts of the same length gave exactly the same vector (1.00000).
- **Fix:** no padding (`with_padding(None)`); `SenseModel::embed` groups the texts by exact token length and runs each group as one batch (`core/src/sense/mod.rs`).
- **Prevention:** `core/tests/sense.rs` checks that every text of a mixed batch gets the same vector as alone (> 0.9999).

## BUG-036: computing the meaning of 100,000 files stopped on "Access is denied"

- **Date:** 2026-09-28 · **Step:** Étape 8, lot 8.2 (meaning index), found by the 100,000-file measurement before delivery
- **Symptom:** after 6 minutes (13,700 passages), `sense_update` failed: Tantivy could not create a new file of the meaning index (`Failed to open file for write … Access is denied`).
- **Cause:** the same family as BUG-022 and BUG-026: the antivirus briefly locks a file that has just been created. The meaning index commits every 32 files: over 100,000 files, that is about 3,000 commits, each creating new files. The JSON stores already retried (`write_atomic`), the Tantivy commit of the meaning index did not.
- **Fix:** `core/src/engine/meaning.rs` → a batch is computed first (its vectors kept in memory), then written and committed; on "access denied" the writer is dropped, and the batch is written again with a new writer, up to 5 times, 1 s apart. The vectors are never computed twice. In the app, a batch that still fails is only lost until the next wake-up (the computation resumes from the last commit).
- **Prevention:** any index written often (live watching, meaning index) must survive a brief lock of a new file; the long measurements (`meaning_of_100_000_files`) are the ones that show it.

## BUG-037: with `Aa`, counts wrong and files missed; searches slow on a site full of code

- **Date:** 2026-09-28 · **Step:** manual tests of Étape 2 (`Aa`, `ab` chips), on the user's "GIT Repositories" site (20,875 files of code)
- **Symptom:**
  - `Contrat` with `Aa`: 1 file shown, but the type chips still counted TEXT 43, SOURCE CODE 31… (the counts of `contrat` in any case).
  - Searches took 1 to 1.75 s in the app (`pnpm tauri dev`) with typo tolerance on (the default), `Aa` on or `ab` off; 48 ms with the tolerance off.
- **Cause:**
  - `Aa` was only applied by the highlighter: the index gave its first 200 candidates in any case (with word forms and typos), the highlighter dropped the others. The counts came from the index, before that check. A capitalized word ranked after the first 200 candidates was never found, and the total was wrong.
  - Highlighting a file stemmed every word of its text and computed an edit distance for every word, even words of a very different length. Code repeats the same identifiers: the same work, thousands of times per file (217 ms for 200 files in release).
  - `ab` off: `RegexQuery .*word.*` walks the whole dictionary (millions of words in code). The counts per criterion ran the query 3 more times: 4 walks per search.
  - In the app, `pnpm tauri dev` runs a debug build of the Rust code: 10 to 20 times slower than the installed version. The same searches, in release, took 36 to 223 ms.
- **Fix:**
  - `core/src/search.rs`: `SearchRequest::verified` (regex, NEAR, LINES, detectors **or `Aa`**): every candidate is checked on its text, the counts come from the accepted files. With `Aa`, the index looks for the exact word only (no stems, no typos: `WordMode.forms`), and `NOT` is decided on the text (`NOT Brouillon` keeps "brouillon").
  - `core/src/search.rs`: candidates are read and checked in parallel batches of 256 (`rayon`), in score order; the loop still stops once the list is full when nothing is counted.
  - `core/src/search.rs`: `Contains::expand` lists the indexed words containing each word once per site and search (one walk of the dictionary), reused by the counts (`TermSetQuery`).
  - `core/src/highlight.rs`: `Matcher::judge` + a per-text memory of each spelling; a word is stemmed only when its length is within one letter of a searched word (BUG-020 rule); with `Aa`, a text without the typed spelling is skipped without being split into words.
  - Measured on a copy of the user's index (release): `contrat` with tolerance 217 → 25 ms; `ab` off 143 → 37 ms; `Contrat` + `Aa` 86 ms (wrong counts) → 0 ms; `Import` + `Aa` (15,703 candidates, all checked) 198 ms.
- **Prevention:**
  - `core/tests/facets.rs` → `case_and_partial_words_counted_like_the_list` (capital word after the first 200, counts = list, `NOT` with `Aa`, `ab` off).
  - `core/tests/perf.rs` → `real_index_timings` (ignored): timings on a copy of a real data folder (`PROSPECTOR_BENCH_DATA`). The 100k-file corpus has few distinct words: it cannot show dictionary or highlighting costs. Search times seen in `pnpm tauri dev` are debug times: measure in release before concluding.

## BUG-038: first push — Pages deploy failed and the push went to the wrong account

- **Date:** 2026-09-28 · **Step:** publishing (first push to GitHub)
- **Symptom:**
  - The `deploy` job (`site.yml`) stopped at `actions/configure-pages@v5` with "Get Pages site failed … HttpError: Not Found".
  - Then `git push` in the project folder: "No such remote 'origin'", and after adding it, "Repository not found".
- **Cause:**
  - Pages was not enabled on the repository, so `configure-pages` found no site. Its `enablement` option cannot help: the default `GITHUB_TOKEN` has no admin right.
  - The project folder had no remote. Two GitHub accounts share the same Git Credential Manager, which picks one login for `github.com` by default.
  - The remote was typed with the wrong name (`my-file-locator` instead of the real `file-locator`).
- **Fix:**
  - Settings → Pages → Source: *GitHub Actions*, then re-run the job.
  - `git remote add origin https://faycalazib@github.com/faycalazib/file-locator.git`, plus `credential.username faycalazib` and `credential.useHttpPath true` in the repository's config.
  - `site/config.js`: `PROSPECTOR_REPO = 'faycalazib/file-locator'`.
- **Prevention:** docs/RELEASE.md, one-time setup: enable Pages before the first push; the account goes in the remote URL.

## BUG-038: dark theme — open lists unreadable, and the Settings outline scrolled away

- **Date:** 2026-09-28 · **Step:** manual tests of Étape 3, "Sonar" (dark) theme
- **Symptom:**
  - An open drop-down list (Size, Date…) showed pale text on a white background: unreadable.
  - Settings panel scrolled: its neon outline stopped in the middle of the panel and crossed a line of text ("Command line in the PATH"); the lower sections had no outline.
- **Cause:**
  - The open list of a `<select>` is drawn by Windows. `index.html` declared `color-scheme: light` only, and the options had no colors: they kept the light list background under the dark theme's light text.
  - The outline of the Sonar theme is `.sketch::before` (`position: absolute; inset: 0`). On an element that scrolls itself (`overflow-y: auto`), that layer is part of the scrolled content: it moves with it. Same flaw in the "Understand the meaning" and "Copy / Export" dialogs when their content is taller than the window.
- **Fix:**
  - `ui/src/styles/base.css`: `option` takes the theme's `--surface` and `--text`; `ui/index.html`: `color-scheme` "light dark".
  - `SettingsMenu.svelte`, `SenseSitesDialog.svelte`, `TransferDialog.svelte`: the frame (`.sketch`, outline) no longer scrolls; an inner `.scroll` / `.body` does (flex column, `min-block-size: 0`).
- **Prevention:** an element with the `sketch` class must never scroll itself: put an inner element that scrolls.

## BUG-039: "Registre" layout — saved and recent searches shown nowhere

- **Date:** 2026-09-28 · **Step:** manual tests of Étape 3, lot 3.2 (first test driven by script, `scripts/drive.py`)
- **Symptom:** ★ saved the search (message "Search « contrat » saved", full star, entry written in `saved-searches.json`), but no "Saved searches" section appeared in the rail. Recent searches were missing too. Saved searches could not be run, renamed, removed or given an alert in that layout.
- **Cause:** `IndexRail.svelte` showed either the filters (`withFilters`, "Registre" layout: filters in the rail) or the recent and saved sections (`{#if withFilters} … {:else if !compact} …`), never both.
- **Fix:** `IndexRail.svelte`: the two sections are snippets (`recentSection`, `savedSection`). With the filters in the rail, they follow them: filters, saved searches, recent. The other layouts are unchanged.
- **Prevention:** a feature reached from the rail must be checked in the three layouts (Journal, Registre, Strates).

## BUG-040: Arabic interface — line numbers of a code preview overlapped the code

- **Date:** 2026-09-28 · **Step:** manual tests of Étape 3 (driven by script), interface in Arabic
- **Symptom:** in the preview of a code file, the line numbers went to the right and were drawn over the end of the lines (`'IT Setup'42`).
- **Cause:** only the text of each line was `dir="ltr"`. The line itself (number + text, a flex row) followed the page direction, right to left: the number went to the right, and the long code lines, wider than the preview, ran under it.
- **Fix:** `PreviewPanel.svelte`: the whole preview is `dir="ltr"` for code (numbers on the left, horizontal scroll from the left). Prose keeps the page direction, each line `dir="auto"`.
- **Prevention:** code, paths and regexes always read left to right, whatever the interface language: set the direction on the block, not only on the text inside.

## BUG-041: Excel dates shown and indexed as numbers (45293 instead of 2024-01-02)

- **Date:** 2026-09-28, fixed 2026-10-13 · **Step:** manual tests of Étape 3, lot 3.4 (driven by script), on a generated workbook of 600 rows (`target/tmp/drive/make_xlsx.py`)
- **Symptom:** in the preview of a workbook, a date column showed `45293`, `45294`… Searching a date in a workbook could not find it.
- **Cause:** Excel stores a date as a number (days since 1899-12-30, or 1904-01-01 on old Macs) with a date format. calamine reads it as `Data::DateTime`, but its `to_string()` writes the number. `extract/office.rs` used `to_string()` for every cell.
- **Fix:**
  - `core/src/extract/office.rs` → `excel_date`: ISO 8601, the same in every language and unambiguous: `2024-01-02`, `2024-01-02 14:30` (seconds only when not zero), a time alone `14:30`, a duration `36:00:00`. Choice made with the user (option A, ISO).
  - `core/src/manifest.rs`: `VERSION` 8. A version-7 manifest is kept and only its spreadsheets (`FileKind::Excel`) are read again at the next update (`Manifest::reread`), instead of rebuilding every site.
  - Numbers stay raw (`137.5`): calamine does not give the number format.
- **Prevention:** `office::tests::dates_read_as_dates` (date, date + time, time, duration, 1904 system), `manifest::tests::version_7_rereads_only_spreadsheets`. Known limit: a workbook inside an archive or an e-mail keeps its old text until the archive changes.

## BUG-042: portable test failed when the temporary folder started with `h:`

- **Date:** 2026-10-13 · **Step:** full test run after BUG-041
- **Symptom:** `a_new_drive_letter_moves_sites_indexes_and_saved_searches` failed in `cargo test --workspace`, passed alone: `H:\…\devis.txt` found, `h:\…\devis.txt` expected.
- **Cause:** the test, not the code. `drive_of` / `move_drive` write the drive letter in upper case (same path on Windows). `CARGO_TARGET_TMPDIR` follows the current folder of the terminal, which may start with `h:`: after a move to another letter and back, the paths came back as `H:`.
- **Fix:** `core/tests/portable.rs` compares the paths with the drive letter in upper case (`upper_drive`), and nothing else.
- **Prevention:** tests on Windows paths must not depend on the case of the drive letter.

## BUG-043: Arabic — in "Word · 1 KB", the number left its unit

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 3, lot 3.5 (PDF report generated through CDP `Page.printToPDF`, interface in Arabic)
- **Symptom:** in the PDF report and in the preview header, the details of a file whose kind has a Latin name read `Word · 1   كيلوبايت`: the size number stuck to "Word" and was cut from its unit. Arabic kind names (`نص · 226 بايت`) were fine.
- **Cause:** the parts of the line (kind, size, date, language, matches) were plain text joined by `·`, with no isolation. The Unicode bidirectional algorithm grouped "Word · 1" as one left-to-right run.
- **Fix:** `ReportView.svelte` and `PreviewPanel.svelte`: each part in its own `<bdi>`.
- **Prevention:** a line that joins translated parts with numbers or Latin names (kind, size, date) isolates each part (`<bdi>`, or `isolate()` in `text.ts` for plain strings), like the sites and terms of the report already did.

## BUG-044: the error of a search stayed on screen over the next search's results

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 5, lot 5.1 (driven by script)
- **Symptom:** an invalid name regex (`/[a/`) showed "This search could not be understood". A valid search right after (`*.rar`) showed its results, with the error message still at the bottom of the screen.
- **Cause:** errors stay 12 s (`notices.svelte.ts`), whatever happens next. A new search did not remove the message of the previous one.
- **Fix:** `Notice.topic` and `notices.clear(topic)`. The search errors have the topic `search` (`search.svelte.ts`: indexed, live scan, end of a live scan), and each new search clears them first.
- **Prevention:** a message about an action that can be redone gets a topic, cleared when the action runs again.

## BUG-045: a `.ppt` showed old versions of its slides (5 cards for 2 slides)

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 5, lot 5.3 (driven by script), `legacy/basic_test_ppt_file.ppt` from the Apache POI corpus
- **Symptom:** the preview showed 5 slide cards (1, 2 and 4 the same, 3 and 5 the same) for a presentation of 2 slides. Search results were right (same words), the preview and the slide numbers were not.
- **Cause:** PowerPoint 97-2003 "fast saves" append the changed records instead of rewriting the file: older versions of the document and of its slides stay in the `PowerPoint Document` stream. `extract/ppt.rs` read the stream from start to end and took every `SlideListWithText` and every `Slide` container, old ones included.
- **Fix:** `core/src/extract/ppt.rs` follows the edit chain of [MS-PPT]: `Current User` → `UserEditAtom` (`offsetLastEdit` back to the first edit) → persist directories, newer entries over older ones → the live document (`docPersistIdRef`) and each slide (`SlidePersistAtom.persistIdRef`). A file without that chain (damaged) is still read from start to end. `core/src/manifest.rs`: `VERSION` 9, only `.ppt` files are read again (a version-7 manifest goes through 8 then 9).
- **Prevention:** `ppt::tests::fast_saves_read_only_the_live_version` (two saved versions, only the live one read); `legacy.rs` → `ppt_reads_only_the_live_slides` (2 slides in the real POI file); `manifest::tests::old_versions_reread_only_what_changed`. Binary formats with an edit history (.ppt, .doc fast saves) must be read through their live pointers, never by scanning the whole stream.

## BUG-046: Alt+← brought back the old results with the new query (and kept the new filter)

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 5, lot 5.6 (driven by script)
- **Symptom:** after `contrat`, `devis`, `facture`, Alt+← showed "facture" in the box with the results of "devis"; a second Alt+← showed "devis" with the results of "contrat". After switching the PDF chip on, Alt+← left it on.
- **Cause:** `search.run()` recorded the step being left with `capture(false)`, the state at that moment. The box (`bind:value`) and the filters already held the new search: the step mixed the new criteria with the old results.
- **Fix:** `search.svelte.ts`: each run keeps the criteria it ran with (`#ranCriteria`: query, name pattern, folders, mode, options, filters, sites, within, folder, copies). The next run records `{ ...capture(false), ...ranCriteria }`: the results on screen with the criteria that gave them. `restore()` resets them from the snapshot shown (its `lastQuery` when it has run).
- **Prevention:** a history step pairs results with the request that produced them, never with the inputs as they are now.

## BUG-047: middle click did not close a tab

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 5, lot 5.6 (driven by script)
- **Symptom:** a middle click on a tab did nothing.
- **Cause:** `TabStrip.svelte` closed on `auxclick`. The page received the press and the release of the middle button, but no `auxclick` event.
- **Fix:** close on `mouseup` with `button === 1`; `mousedown` with the middle button is cancelled so Windows does not start auto-scrolling.
- **Prevention:** for a mouse gesture, listen to `mousedown` / `mouseup`, which every WebView sends.

## BUG-048: "No code editor found" with VS Code and Notepad++ installed on D:

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 5, lot 5.7 (driven by script)
- **Symptom:** Settings → Code editor said "No code editor found on this PC", and the "Open at line N" button never showed, although VS Code and Notepad++ were installed (in `D:\Programs\…`).
- **Cause:** `src-tauri/src/editor.rs` looked only in the default folders (`%LOCALAPPDATA%\Programs`, `%ProgramFiles%`, `%ProgramFiles(x86)%`). An editor installed elsewhere was invisible.
- **Fix:** after the default folders, `program()` looks where Windows registered the program (`App Paths\<exe>`, then `Classes\Applications\<exe>\shell\open\command`, user then machine), then for its launcher in the PATH (`bin\code.cmd` → `Code.exe`; `codium.cmd`, `cursor.cmd`, `subl.exe`). On the user's PC: VS Code and Notepad++ found in `D:\Programs`.
- **Prevention:** `editor::tests::an_editor_installed_elsewhere_is_found_through_its_launcher`; `editors_found_here` (ignored) prints what a PC has. Never assume a program is on C: or in its default folder.

## BUG-049: Arabic — the example text of the digest field was scrambled

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 5, lot 5.8 (driven by script), interface in Arabic
- **Symptom:** the empty digest field showed its Arabic example in a broken order ("إلى a أو 64 حرفًا من 0 إلى 9 ومن 32").
- **Cause:** the field is `dir="ltr"` for the hexadecimal digests, and its placeholder (a translated sentence) followed that direction.
- **Fix:** `FilterPanel.svelte`: `dir="ltr"` only once a digest is typed; empty, the field follows the page, and the Arabic example reads right to left.
- **Prevention:** a field forced left to right (paths, digests, regexes) keeps a placeholder in Latin characters, or drops `dir` while empty.

## BUG-050: the "New" tag of an alert was cut off with long file names

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 6, lot 6.1 (driven by script), "Registre" layout, seen first in Arabic
- **Symptom:** in the results of an alert, the "New" tag ("جديد" in Arabic) showed as a pink sliver, or not at all, next to a long file name.
- **Cause:** the tag sat inside `.name`, the element cut with `overflow: hidden` and an ellipsis: a long name pushed it past the edge of the column.
- **Fix:** `ResultsList.svelte`: `.name` is a flex row; only the name itself (`.ltr-isolate`) is cut with "…", the tag (`flex: none`) always shows. Checked in French and Arabic with a 50-character name.
- **Prevention:** a tag or badge next to a text that can be cut sits outside the cut element.

## BUG-051: a phone number written in Arabic-Indic digits was not detected

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 6, lot 6.2 (driven by script)
- **Symptom:** the "Phone" detector did not find `٠٦ ١٢ ٣٤ ٥٦ ٧٨`, although Arabic-Indic digits are announced as supported (the same number in Latin digits was found).
- **Cause:** `core/src/detect.rs`: the national form of the phone pattern began with `\b0[1-9]`, in Latin digits only; the rest of the pattern used `{d}`, the three digit sets.
- **Fix:** `zero` (`[0\u{660}\u{6f0}]`) and `nonzero` (1-9 in the three sets) for the start of a national number. Detectors run on the stored text at search time: nothing to index again.
- **Prevention:** `detect::tests::contact_data` checks the number in Arabic-Indic and in Persian digits. A pattern that accepts other digit sets uses them everywhere, first digit included.

## BUG-052: Arabic — the example text of the search box was scrambled and aligned left

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 6 (driven by script), interface in Arabic
- **Symptom:** the empty search box showed "…AND / OR / NOT / NEAR ،«كلمات، «عبارة مطابقة" aligned left, in a broken order.
- **Cause:** the box is `dir="auto"` (it follows what is typed). Empty, there is no letter to decide: the browser takes left to right, and the Arabic example followed. Same family as BUG-049.
- **Fix:** `SearchBar.svelte`: `dir="auto"` only once something is typed; empty, the box follows the page. Checked: Arabic interface → example right to left; a Latin word → left to right; an Arabic word → right to left; French interface → left to right.
- **Prevention:** a field with `dir="auto"` or `dir="ltr"` and a translated placeholder drops `dir` while empty.

## BUG-053: Escape did not close the dialogs (export / copy, duplicates, meaning)

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 6, lot 6.4 (driven by script)
- **Symptom:** Escape left the "Export" / "Copy" dialog open; its backdrop kept blocking every click behind it. Same for the "Duplicates" and "Understand the meaning" dialogs.
- **Cause:** the three modal dialogs closed only through their buttons or a click on the backdrop; none listened to the keyboard.
- **Fix:** `TransferDialog.svelte`, `DuplicatesDialog.svelte`, `SenseSitesDialog.svelte`: `<svelte:window onkeydown>` closes on Escape while open, through the same `close()` (a copy in progress is not interrupted by Escape: `close()` keeps the dialog while it runs).
- **Prevention:** every modal dialog closes on Escape (WCAG, and what a Windows user expects).

## BUG-054: the duplicates list exported as CSV opened in one column in French Excel

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 6, lot 6.5 (driven by script, the save dialog answered by the script)
- **Symptom:** "Export the list" (Duplicates) wrote `Groupe,Type,Ressemblance,…` with commas. In French or Spanish, Excel uses `;` as list separator: every row stays in one column. Dates were ISO with a time zone (`2026-09-27T14:28:55.000Z`), not read as dates.
- **Cause:** `DuplicatesDialog.svelte` had its own CSV code, with `,` hard-coded and `toISOString()`, instead of the rules of the results export (`csvSeparator`, `excelDate`, lot 6.4).
- **Fix:** `export.ts` → `toCsv(rows)`: BOM, the language's separator, quoting; `toExcelCsv` and the duplicates export both use it, with `excelDate` (`2026-10-13 12:15`).
- **Prevention:** every CSV of the app goes through `toCsv`; never a separator written by hand.

## BUG-055: "Extract to…" — a write error spoke of the index folder

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 6, lot 6.7 (driven by script)
- **Symptom:** when the chosen file could not be written (folder that does not exist, read-only, disk full), the message was "Impossible d'écrire dans le dossier des index", about another folder.
- **Cause:** `extract_to` (core) read the document and wrote the target in one call; any `io::Error` became `CoreError::Io`, shown as the generic storage error of the index folder.
- **Fix:** `src-tauri/src/commands.rs` → `extract_to`: reading (`unpack::read`, errors of its own: "could not be taken out of its archive") and writing apart; the write runs in a blocking task and fails with `WriteFailed { path }`: "Impossible d'enregistrer <file>". Checked: `Q:\dossier\inexistant\devis.txt` → "Impossible d'enregistrer Q:\dossier\inexistant\devis.txt".
- **Prevention:** an error about a file the user chose names that file.

## BUG-056: the same folder could be added twice as two dig sites

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 7, lot 7.2 (driven by script)
- **Symptom:** two more `test_fixtures` sites appeared, on exactly the same folder (a "+ Add a folder" clicked by mistake during the tests, the folder chosen in the Windows dialog). The app accepted them: every file indexed and found twice, "Replace with the 4 ticked sites" with 2 real folders.
- **Cause:** `Engine::add_site` checked nothing about the folders.
- **Fix:** `add_site` refuses a folder that is already the root of a site, compared without case and without a final separator (`H:\Dossier\` = `h:\dossier`): `CoreError::SiteExists { name }`, "Ce dossier est déjà le site « … »" (4 languages, `errors.siteExists`). A folder inside a site stays possible (a sub-folder kept apart). Checked in the app: adding `test_fixtures` again is refused with its name.
- **Prevention:** `engine.rs` → `the_same_folder_is_not_added_twice`. The two duplicate sites of the tests are left for the user to remove.

## BUG-057: "Delete" and "Replace with the ticked sites" did nothing on a site group

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 7, lot 7.2 (driven by script)
- **Symptom:** right-click on a group → "Supprimer" or "Remplacer par les N sites cochés": the menu closed, nothing changed, no message.
- **Cause:** `SiteGroups.svelte`: `{@const group = groups.list.find((g) => g.id === menuFor)}`; the handlers did `menuFor = null;` then `group.id`. In Svelte 5 a `{@const}` is derived: once `menuFor` is null, `group` is undefined and `group.id` threw inside the click handler, before any call to the engine. "Rename" worked: it received the group as an argument before the menu closed.
- **Fix:** read the id first (`const id = group.id; menuFor = null; …`). A scan of every component found no other handler of this kind.
- **Prevention:** in a handler, read what a `{@const}` gives before changing the state it is derived from.

## BUG-058: the terms of a list were not highlighted in the preview

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 7, lot 7.4 (driven by script)
- **Symptom:** with a term list (and no word typed), the results counted their matches, but the preview highlighted nothing (no mark, ↑ / ↓ without effect).
- **Cause:** `search.svelte.ts` → `previewRequest()` (what the preview highlights) sent the query and the detectors, not the term list.
- **Fix:** `previewRequest()` adds `termList` / `termListAll`. Checked: "Martin" in an e-mail, "García" in a presentation, the multi-word "contrat de maintenance" in a ZIP and in an Excel table.
- **Prevention:** a new criterion that finds text is added to both requests: the search (`buildRequest`) and the preview (`previewRequest`).

## BUG-059: a saved search lost its term list, and could keep filters of another search

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 7, lot 7.4 (driven by script)
- **Symptom:** a ★ saved with a term list came back without it. And applying a saved search without detectors (or without a list) kept the detectors / list left on screen: wrong results.
- **Cause:** `saved.svelte.ts` → `snapshot()` did not store the term list; `apply()` (and the alert request) started from the filters on screen and only reset the disk filters and attributes, so detectors and lists absent from the saved search stayed.
- **Fix:** `SavedSettings.filters.termList` (the terms themselves: the file may move), stored only when set (searches saved before still count as the same); `OPTIONAL_FILTERS_OFF` (disk filters, attributes, detectors, term list) is applied before the saved filters, in `apply()` and for the alert request. Checked both ways in the app.
- **Prevention:** every optional filter has an "off" value in `OPTIONAL_FILTERS_OFF`, so that a saved search never inherits one.

## BUG-060: errors of the meaning module were shown half in English

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 8, lot 8.1 (driven by script)
- **Symptom:** "Installer depuis un fichier…" with a ZIP that is not the module: "Recherche par le sens : not a Prospector meaning module (no manifest)". Same for a damaged module, another version, and the download errors ("download cancelled"…).
- **Cause:** the engine sent English text in `CoreError::Sense { message }`, shown as is, against the rule "the engine sends codes, the UI translates" (goal.md §5bis-A).
- **Fix:** `CoreError::SenseNotModule`, `SenseDamaged`, `SenseVersion { found, expected }` (core, `sense/module.rs`; a manifest of another module is "not a module", no longer "wrong version"); `AppError::SenseNoDownload`, `SenseDownloadFailed`, `SenseDownloadCancelled` (app, `sense.rs`); 6 messages in 4 languages (`errors.sense…`, 532 keys). Checked in the app: "Ce fichier n'est pas un module de recherche par le sens de Prospector."
- **Prevention:** `sense::module::tests::install_check_and_remove` checks the codes (damaged, not a module, another version). `CoreError::Sense { message }` stays for internal errors only.

## BUG-061: right after "Understand", a site read "Meaning up to date · 0 passages"

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 8, lot 8.2 (driven by script)
- **Symptom:** after "Comprendre" on a site, the rail showed "≈ Sens à jour · 0 passages" until the first batch was computed (the models take seconds to load), then the progress.
- **Cause:** the progress was published only from inside `sense_update`; before it, the site had `sense` on and no progress, which the UI shows as "up to date".
- **Fix:** `src-tauri/src/sense.rs`: `set_site_sense` puts "0 %" (0 of 1) at once when something is pending; `run_round` publishes "0 %" when it takes the site in hand, before loading the models (removed if the models cannot load). Checked: "≈ Sens · 0 %" → "52 %" → "100 %" → "à jour · 61 passages".
- **Prevention:** a state shown as "up to date" is only shown when nothing is pending.

## BUG-062: a saved search lost "≈ Sens"

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 8, lot 8.3 (driven by script)
- **Symptom:** a ★ saved with "≈ Sens" on came back with it off.
- **Cause:** `saved.svelte.ts` → `snapshot()` listed the options one by one (case, whole word, regex, fuzzy), without `meaning` (added at lot 8.3).
- **Fix:** `meaning: true` stored when on (absent otherwise: searches saved before still count as the same). Checked in the app.
- **Prevention:** an option added to `SearchOptions` is added to the saved settings in the same change (same family as BUG-059).

## BUG-063: the list stopped at 200 files without saying so

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 6, lot 6.3 (driven by script); fixed after the user's choice
- **Symptom:** `import` on the "GIT Repositories" site: "1 263 trouvailles dans 200 fichiers", although 15 703 files matched (the kind chips showed it). Nothing said that only the first 200 (the most relevant) were listed.
- **Cause:** the engine returns `totalFiles`, but the interface kept only the hits and counted them.
- **Fix:** `search.svelte.ts` keeps `totalFiles` (reset at each search, kept by tabs and history); `ResultsList.svelte` shows "1 263 trouvailles dans 200 premiers fichiers sur 15 703" when more files were found than listed, with a tooltip "narrow the search (kind, date, folder) to see the others" (`results.firstOf`, `results.firstOfHint`, 4 languages).
- **Prevention:** a list cut at a limit says it is, with the real total.

## BUG-064: the PDF report masked phones too

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 6, lot 6.2 (driven by script); fixed after the user's choice ("mask by type")
- **Symptom:** in the PDF report of an audit, a phone came out as `+••-••-••••-3452`, and a large amount (`12 500 000,00 €`) would have been masked too. The report's note, however, says "account, card and identity numbers", and amounts must stay readable.
- **Cause:** `filters.ts` → `maskSensitive` guessed from the text: any highlighted passage with at least 8 digits was masked, whatever the detector that found it.
- **Fix:** the engine says what is sensitive. `Detector::is_sensitive()` (IBAN, RIB, card, social security, DNI, passport MRZ); `Highlighter::sensitive(text)` lists what those detectors found (deduplicated, at most 100); `Hit.sensitive` (indexed search and live scan, omitted when empty). `maskSensitive(marked, sensitive)` masks only the highlighted passages among them (spaces ignored), last 4 characters kept. Results files keep `sensitive`, so a report printed from reopened results is masked too. Test `sensitive_data_is_only_account_card_and_identity_numbers`; checked in the app (IBAN `•••• … •••0 189`, card `•••• •••• •••• 1111`; phone, amounts and e-mails readable).
- **Prevention:** what a detector found is decided by the engine, not guessed again from the text in the interface.

## BUG-065: the header line of a CSV term list became a term

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 7, lot 7.4 (driven by script); fixed after the user's choice
- **Symptom:** a list `fournisseurs.csv` (`nom;ville`, then `Martin;Lyon`…) gave 3 terms: "nom" was one, and it is found almost everywhere.
- **Cause:** `terms.rs` → `parse_list` read every line of a CSV / TSV the same way.
- **Fix:** for a `.csv` / `.tsv`, the first line that is not empty nor a `#` comment is a header: left out and counted in "lines ignored". A `.txt` list is unchanged. Test `lines_columns_comments_repeats`; checked in the app ("fournisseurs.csv · 2 termes · 1 ligne ignorée").
- **Prevention:** a CSV is read as a table (header first); a CSV without a header loses its first term, which the panel shows (preview and "1 ligne ignorée").

## BUG-066: a new tab kept the term list

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 7, lot 7.4 (driven by script); fixed after the user's choice
- **Symptom:** Ctrl+T opened an empty tab that still had "Liste : fournisseurs.txt", so its first search was filtered without the user asking.
- **Cause:** `search.svelte.ts` → `blank()` copied every filter of the current tab (it left out only the digest).
- **Fix:** `blank()` also sets `termList: null`: a list is a criterion of one search, like the digest; sites and options are still copied. Checked in the app (new tab without list, first tab keeps it).
- **Prevention:** what belongs to one search (digest, term list) is not carried to a new tab.

## BUG-067: a refused global shortcut stayed checked in Settings

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 3, lot 3.3 (by the user)
- **Symptom:** Settings → Global shortcut → "Ctrl + Alt + Space", already held by another application on this PC: the error message appeared and Ctrl + Alt + F stayed active, but the radio button stayed on Ctrl + Alt + Space.
- **Cause:** `SettingsMenu.svelte` → `changeShortcut` reloads the real shortcut after a refusal, but it is the same value as before the click: Svelte sees no change and leaves the radios as the click left them.
- **Fix:** after a refusal, the radios of the fieldset are set again from the active shortcut (`shortcutBox`).
- **Prevention:** a choice the back end can refuse puts the controls back explicitly (state unchanged = DOM untouched). The editor and "meaning pace" radios have the same pattern; they are refused only when the settings cannot be written.

## BUG-068: indexing "stuck" at 88 %, and another site stuck at 100 %

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 3, lot 3.6 (by the user)
- **Symptom:** the "Religion" site (293 files, 3.8 GB) stayed at "Indexing… 88 %" for a long time, and the "Youtube" site added next (30 files) stayed at "Indexing… 100 %" without finishing. Prospector was not frozen: it kept one core busy.
- **Cause:** three faults together.
  1. `engine.rs` reported the reading progress every 64 files: 256 / 293 = 88 %, then nothing until the end.
  2. One file of the site, `alalbani_pdf.zip` (1.9 GB, 282 books in PDF, many scanned so read by OCR), counts as one file and takes hours, with nothing saying so.
  3. `pipeline.rs` used the shared rayon pool for every site: the end of Youtube's run waited for a worker busy with that ZIP.
- **Fix:**
  1. A report at least every 1 % (`step = total / 100`).
  2. `process_files_watched` watches the files being extracted; one read for more than 3 s (`SLOW_FILE`) is sent in `IndexProgress.current`, and the rail shows "Lecture de « alalbani_pdf.zip »…" under the bar (full path in the tooltip, `rail.readingFile`, 4 languages).
  3. Each run has its own rayon pool (`prospector-read-N` threads); the live scan too.

  Tests in `core/tests/pipeline.rs`: `the_bar_moves_at_least_every_percent` (250 files, ≥ 100 reports), `a_busy_shared_pool_does_not_hold_an_indexing` (every worker of the shared pool blocked: the indexing still ends; it hangs without the fix).
- **Prevention:** a site never waits for another one; a long step always says what it is doing. Possible next step, not done: let "Stop indexing" interrupt a large archive between two of its documents (today it waits for the end of the file).

## BUG-069: a site whose first indexing was cut off stayed "Not indexed"

- **Date:** 2026-10-13 · **Step:** manual tests of Étape 4, lot 4.1 (by the user, installed version)
- **Symptom:** "Religion" was being indexed for the first time when the app was closed. At the next start it showed "Not indexed" and nothing happened: no indexing, no watching, no message.
- **Cause:** `watch.rs` → `start_all` kept only the sites with a `last_indexed` date: watched, then brought up to date. A site never finished has none, so it was left out for good.
- **Fix:** `start_all` splits the sites: the indexed ones are watched and updated first (as before), then the unfinished ones are indexed in full, one after the other; `run_indexing` watches them once done. Also when this PC becomes the maintainer of a shared index (lot 7.3).
- **Prevention:** at startup, every site ends up indexed or in error in the rail; none is silently left aside.

## BUG-070: Arabic text of some PDFs could not be found (backwards, shaped glyphs)

- **Date:** 2026-10-14 · **Step:** manual tests of Étape 5, lot 5.5 (by the user: `NEAR` in Arabic on their own PDF)
- **Symptom:** in the user's PDF "1000 أمر وبرومبت موك أب…", no Arabic word was found (`برومبت`, `تصميم`…), only `1000` and Latin words. The index held `ﻢﯿﻤﺼﺘﻟ ChatGPT تارﺎﺼﺘﺧا :ﻲﻋﺎﻨﻄﺻﻻا … ﺮﻣأ 1000`.
- **Cause:** pdf-extract returns the text as it is drawn. This PDF (like many Arabic PDFs) stores Arabic as **presentation forms** (shaped glyphs, U+FB50–FDFF / U+FE70–FEFC: `ﻟ` instead of `ل`) and in **display order** (left to right on the page): every line reads backwards. The search folds letters, not glyphs, and reads words in order.
- **Fix:** `extract/pdf.rs` → `arabic(pages)`, on the text layer only (OCR text is already right):
  1. glyphs back to letters with NFKC, only on presentation forms (`ﻻ` → `لا`); in an Arabic document, the Farsi yeh glyphs at the start or in the middle of a word (same drawing) become `ي`;
  2. the document is backwards when, turned around, it has clearly more words starting with the article `ال` (≥ 3 and more than twice as many as kept as is); then each line with Arabic is turned around, Latin words and numbers are read left to right again, brackets outside them mirrored.

  Manifest version 10: PDFs, archives and mailboxes (which may hold PDFs) are read again, nothing else. Tests `arabic_drawn_backwards_is_read_in_order` (the real title line gives back "1000 أمر وبرومبت موك أب للذكاء الاصطناعي: اختصارات ChatGPT لتصميم") and `arabic_in_reading_order_stays_as_it_is`; checked on the user's PDF with `prospector-cli scan` (`الاصطناعي`, `تصميم`, `برومبت`, `الذكاء NEAR الاصطناعي` found).
- **Prevention:** text taken from a drawing (PDF) is put back in reading order before indexing; a test file with real Arabic glyphs keeps it so. Limit: a line mostly in Latin with a few Arabic words may keep those words in an imperfect order (each word is still found).

## BUG-071: an Arabic word did not find its form with ل ("for") attached

- **Date:** 2026-10-14 · **Step:** manual tests of Étape 5, lot 5.5 (by the user: `برومبت NEAR تصميم` on their PDF found nothing, after BUG-070)
- **Symptom:** `تصميم` ("design") did not find `لتصميم` ("for design"), in a plain search as in `NEAR`, indexed or live scan. `وبرومبت`, `بتصميم`, `التصميم` were found.
- **Cause:** the Snowball Arabic stemmer removes the attached prefixes و, ب, ك, ال, لل… but **never ل alone** (`لتصميم` → `لتصميم`, `لكتاب` → `لكتاب`), on purpose: many words really start with ل (لعبة, لون). The one-letter rule of the highlighter needs the same stem, so the form was a different word.
- **Fix:** on the query side only, `lang::with_lam(word)` gives the form "ل + word" of an Arabic word typed (letters only, at least 2): `word_query` also looks for it in `body` and `stem_ar`, and `Matcher::new` adds it to its words and stems (so highlighting, NEAR, live scan and the preview follow). Not with `Aa` (exact spelling). The text is never cut: `لعبة` still finds only `لعبة`. No reindexing. Tests `an_arabic_word_finds_its_form_with_lam` (highlight) and `an_arabic_word_finds_its_form_with_lam_in_the_index` (core/tests/pipeline.rs); checked on the user's PDF (`برومبت NEAR تصميم` → "[وبرومبت] … [لتصميم]").
- **Prevention:** Arabic test texts include words with attached prepositions (ل, ب, و), not only the bare word.

## BUG-072: at startup, a long site held the update of the following ones

- **Date:** 2026-10-14 · **Step:** manual tests of Étape 5 (by the user, installed version, after the BUG-070 update)
- **Symptom:** after installing the version that reads PDFs again (manifest 10), "Youtube" (76 documents) still showed its old text: it was waiting behind "Religion", which was reading its PDFs and a 2 GB ZIP of scanned books again (hours). The user had to click "Update now" on Youtube.
- **Cause:** `watch.rs` → `start_all` brought the sites up to date one after the other, in the order of the rail, all disks together.
- **Fix:** `startup_plan` groups the sites by disk (`D:`, or `\\server\share` for a network folder, case ignored); each disk has its own thread, its sites one after the other (no seeking between two folders of the same disk), the smallest first (`size_bytes`), the sites whose first indexing was cut off last (BUG-069). The watching starts as before, in the background. Test `small_sites_first_one_disk_at_a_time`.
- **Prevention:** a long site never delays a small one, nor another disk.

## BUG-073: live scan of a whole disk: minutes without any result or counter

- **Date:** 2026-10-14 · **Step:** manual tests of Étape 5, lot 5.7 (by the user: Explorer right-click on an external hard disk not covered by any site)
- **Symptom:** "Search with Prospector" on the disk, then a word: several minutes with no result and no number moving, before the first results came.
- **Cause:** `scan.rs` first walked the whole disk (`crawl_with`, with an empty progress callback), then sorted the list by path, then only started reading. On a whole disk the walk alone takes minutes, and nothing was reported meanwhile.
- **Fix:**
  - **B.** `crawler::crawl_each` hands each file over as soon as it is found (`crawl_with` now builds on it, unchanged for indexing); the live scan sends them through a bounded channel (4,096) to `pipeline::process_stream`, which reads them in that order while the walk goes on (`process_files_watched` sorts, then uses the same stream). The filters (dates, attributes, kind, name, files, "within") are applied as files come; name-only answers are given at once.
  - **A.** `on_progress(read, found)`: the second number is now the files found so far by the walk; the results header shows "1 234 lus sur 56 789 trouvés" while scanning (`results.scannedOf`, 4 languages), then "N fichiers parcourus" as before.

  Test `the_live_scan_reads_while_walking_and_counts_both` (core/tests/pipeline.rs); the other live-scan tests (formats, legacy, mail, OCR, query, detectors, disk) unchanged and green.
- **Prevention:** a long step always shows a moving count; a stream is preferred to "list everything, then work". Note: in live scan the files are no longer read in path order but in walk order (still folder by folder).

## BUG-074: a list of 2,656 terms took 89 s with typo tolerance

- **Date:** 2026-10-15 · **Step:** manual tests of Étape 7, lot 7.4 (by the user: a list of thousands of terms on the 20,875-document site)
- **Symptom:** "Liste : liste-3000-termes.txt · 2 656 termes" on all sites, typo tolerance on: 89,381 ms. Measured with `prospector-cli`: 75 s with `--fuzzy`, 1.6 s without, for 20,790 files against 20,784 (6 more).
- **Cause:** typo tolerance applies to the whole request: each of the 2,656 terms became a `FuzzyTermQuery` (distance 1 or 2), each walking the dictionary of every site, plus the same check in the highlighter.
- **Fix:** `terms::TYPO_TOLERANT_TERMS = 50`. A list of more terms gives `Clause::Exact` words (new clause, the opposite of `LIKE`): looked for with their forms (stems, plural) but without typos, in the index (`word_query`, `fuzzy: false`) and in the highlighter. Lists up to 50 terms and the words typed in the box follow the option as before. The list panel says so when tolerance is on ("Au-delà de 50 termes, la liste est cherchée sans tolérance aux fautes", `terms.noTypos`, 4 languages). Tests `a_term_is_a_word_a_phrase_or_a_regex` and `a_long_list_is_searched_without_typos`.
- **Prevention:** an option that multiplies the cost per word (typos) is bounded when the number of words is large.

## BUG-075: a resumed meaning computation showed 0 %

- **Date:** 2026-10-15 · **Step:** manual tests of Étape 8, lot 8.2 (by the user: Prospector closed during the computation of "GIT Repositories", then opened again)
- **Symptom:** after the restart, "≈ Meaning · 1 % · ≈ 37 min left": the bar started again from 0 %, as if everything was lost.
- **Cause:** nothing was lost (each batch is committed as it goes, and `sense_todo` gives only the files left), but `sense_update` reported its progress from 0 out of the passages **left to compute** (`plan.passages`), not out of the whole site.
- **Fix:** the progress starts at the passages already in the meaning index (`plan.done`) and its total is done + left; the interface's time left was already computed from the pace since the start of this run, so it stays right. Test `a_resumed_computation_counts_what_was_done` (fake vectors: stopped during the second of three batches, started again: first report = passages kept, out of the whole site; last = 100 %).
- **Prevention:** a progress that can be resumed counts what was already done.

## BUG-076: the ≈ button forgot a computation in progress; "Estimating…" blocked "Understand"

- **Date:** 2026-10-15 · **Step:** manual tests of Étape 8, lot 8.2 (by the user, installed version)
- **Symptom:**
  1. After reinstalling, the user clicked ≈ on "GIT Repositories" (being computed) to restart it and confirmed the window: the site went back to "not understood", its meaning index deleted.
  2. Clicking ≈ on GIT again: "Estimating…" for a long time, and "Understand" could not be clicked meanwhile.
- **Cause:**
  1. `IndexRail.svelte` → `toggleSense`: the same ≈ button asked "Understand?" on a site not understood, and "Stop understanding?" (a native confirmation, deleting the index) on a site understood **or being computed**; the confirmation did not say what was lost.
  2. The "Understand" button was `disabled` until the estimate came (reading the 20,875 stored texts and loading the models).
- **Fix:**
  1. On a site being computed, ≈ opens a bubble "Computing the meaning of …" with the progress and time left, "It goes on in the background…", **Continue** (default) and **Stop and forget** (red). Forgetting a site asks "Delete the N passages already computed for …? Everything will have to be computed again…" (`sense.runningTitle`, `runningHint`, `keep`, `stopForget`, `forgetConfirm` with `{count}`, 4 languages).
  2. "Understand" can be clicked at once; the estimate only informs and shows when ready.
- **Prevention:** a button never does two opposite things without saying which; a destructive confirmation says what is lost; an estimate never blocks the action it describes.

## BUG-077: a file name typed in the search box was not found ("2004.m20")

- **Date:** 2026-10-15 · **Step:** manual tests of Étape 3, lot 3.6 (by the user: their own ZIP archives)
- **Symptom:** `2004.m20` in the search box: nothing, although a ZIP of the site holds `2004.m20.i406.n016.jpg` (found with "+ File name" `2004.m20*`). A single word (`facture`) does find files by their name.
- **Cause:** the tokenizer splits `2004.m20` in two words, so it is a phrase; `phrase_query` looked in `body` only, while `word_query` (a single word) looks in `body` and `name` (×2).
- **Fix:** `phrase_query` is a union of the phrase in `body` and in `name` (boost 2), like a single word. `name` already has positions: no reindexing. The highlighter's name check (`name_has_word`) already accepted phrases. Test `a_phrase_finds_a_file_by_its_name` (core/tests/pipeline.rs: a plain file, an entry of a ZIP, and a phrase of the text still found).
  Live scan (`scan.rs`): the query is now checked on the document's own name and its text (`"{name}
{text}"`), as the index looks in `body` and `name` for every word or phrase, `NOT` included; with `LINES`, on the text only (the index checks LINES on the text too). The parity test `attachments_of_an_eml_including_an_attached_zip` now expects the attachment as well (it answers to its own name).
- **Prevention:** a phrase and a word follow the same rules (fields, boosts); only the order of the words differs. The index and the live scan read names the same way.

## BUG-078: a folder search walked the whole disk at each search (and `copy-app` seemed missing)

- **Date:** 2026-10-16 · **Step:** manual tests before the v0.1.0 (by the user, installed version)
- **Status:** fixed on 2026-10-16 (fix approved by the user).
- **Symptom:** site "GIT Repositories" (`H:\GIT Repositories`, 20,875 documents, a 16 TB USB hard disk). "+ File name" → "✓ Folders", `copy-app`, Indexed mode: "0 folders · 99 ms", although `H:\GIT Repositories\copy-app` exists at the first level, is not hidden and is not a default-excluded name.
- **Investigation:**
  - `search_folders` (`core/src/engine.rs`) does not use the index: it walks every folder of every site in scope on disk (`scan::find_folders` → `crawler::crawl_folders`), at each search.
  - The same walk with the command line (`prospector-cli scan "" "H:\GIT Repositories" --folders --name copy-app`, empty data folder) was still running after 50 minutes (3.6 million file-system operations, about 650 per second on the USB disk), then stopped. A real walk of that site cannot take 99 ms: the app's answer did not come from a full walk. Cause of the 99 ms not found yet (the app's data are on C:, not read).
- **Fix:** an indexed site now answers a folder search from its manifest (`scan::folders_from_files`, called by `Engine::search_folders`): every folder between a root and an indexed file is a candidate, each one once; only the folders whose name matches are read on disk (date, attributes, gone ones left out). Exclusions, "limited to a folder" (Explorer right-click), date and attribute filters still apply. A site not indexed yet, a folder outside every site, and the live scan still walk the disk. The 99 ms answer no longer matters: the indexed path does not walk anymore.
  Visibility: the chip "+ File name" becomes "+ File or folder name" (`search.names.toggle`, 4 languages); the site guide gives the example of a project found by its folder name (`copy-app`).
  Test `scan::tests::folders_from_the_paths_of_indexed_files` (a first-level folder, a deeper one, root not listed, exclusion, limited to a folder, folder removed since); `folders_are_found_by_name_but_never_excluded_ones` (core/tests/query.rs) checks that the index and the live scan still give the same folders.
- **Real cause of the symptom (found after the fix, 2026-10-16):** the site "GIT Repositories" is `F:\GIT Repositories` (`prospector-cli sites`), not `H:\GIT Repositories`: the user has two folders with that name, and `copy-app` is only in the H: one. Prospector was right to find nothing. With the fix, `prospector-cli search "" --name accent-architect --folders` finds `F:\GIT Repositories\imeneemlouiv2\accent-architect` in 83 ms. The fix stays useful: a folder search no longer walks the disk at each search.
  Prevention for the UI: the name of a site does not show its drive; a site's folder is only visible on hover. To consider: show the root (`F:\…`) under the site name.
- **Limit:** a folder with no indexed file (empty, or only programs and videos) is not found in Indexed mode; the live scan finds it.
- **Prevention:** a search in Indexed mode never walks a whole site on disk: what the index already knows (paths) answers. Test on a big external disk before calling a search "instant".

## BUG-079: `scripts/release.ps1` stopped at clippy on cargo's "Compiling" line

- **Date:** 2026-10-16 · **Step:** publishing the 1.0.0 (run by the user)
- **Symptom:**
  1. First run: `pnpm check` printed hundreds of `ERR_MODULE_NOT_FOUND` blocks (`@vitejs/plugin-react-swc`, `lovable-tagger`); the user stopped the script. Nothing was committed.
  2. Second run: `clippy... cargo.exe : Compiling prospector v1.0.0` then `NativeCommandError`, the script stopped.
- **Cause:**
  1. `svelte-check` loads the `vite.config.ts` of the React projects copied by an old test into `target/tmp/drive/out/grosse-copie/` (see "À surveiller" in SUIVI.md); it still ends with 0 errors.
  2. The script runs under `$ErrorActionPreference = 'Stop'`; in Windows PowerShell 5.1, a redirected stderr line of a native program (`*> $log`) becomes an error record, so cargo's progress on stderr stopped the script although cargo succeeded.
- **Fix:**
  1. Each check writes its whole output to `target/tmp/release-<name>.log` and prints one line (`ok`), or the last 30 lines and stops when it fails.
  2. `Check` and `Run` set `$ErrorActionPreference = 'Continue'` for the native command and judge it by `$LASTEXITCODE` only (git also writes its progress on stderr). Checked with a command writing on stderr with exit code 0 (ok) and 3 (stops).
- **Prevention:** in a PowerShell script, never judge a native program by its stderr: use the exit code. The user should delete `target/tmp/drive/out/grosse-copie/` (Claude cannot delete files).

## BUG-080: `scripts/release.ps1` failed on `git add -A` ("Missing an argument for parameter 'Args'")

- **Date:** 2026-10-16 · **Step:** publishing the 1.0.0 (run by the user, after BUG-079)
- **Symptom:** checks all `ok`, then `git : Missing an argument for parameter 'Args'` on `git add -A`. Nothing committed.
- **Cause:** in the user's PowerShell, `git` is not git.exe but a function or alias of the profile, with a parameter `-Args`: PowerShell took `-A` for an abbreviation of `-Args`. Second defect found while fixing: `if (git diff --cached --quiet)` tested the (always empty) output instead of the exit code, so "nothing to commit" could never be detected.
- **Fix:** the script calls git.exe itself (`Get-Command git -CommandType Application`) through `$git`, for every git command; `add --all` instead of `-A`; "something staged" is read from the exit code of `diff --cached --quiet`.
- **Prevention:** a script never calls a bare command name that a profile may wrap: resolve the application first. Judge native commands by their exit code (see BUG-079).

