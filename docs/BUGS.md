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
