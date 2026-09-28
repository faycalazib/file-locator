<script lang="ts">
  import Icon from './Icon.svelte';
  import { formatBytes, formatDate, t, type MessageKey } from '../i18n/index.svelte';
  import { api, inTauri } from '../api';
  import { notices } from '../stores/notices.svelte';
  import { preview } from '../stores/preview.svelte';
  import { integration } from '../stores/integration.svelte';
  import { findCopies } from '../copies';
  import { DEMO_BOXES, isImageHit, thumbnail } from '../thumbs';
  import { search } from '../stores/search.svelte';
  import { segments, splitPath } from '../text';
  import { colorSegments } from '../syntax';
  import { isLong, isNumeric, sheets, slides, type Seg } from '../rich';

  const hit = $derived(search.selected);
  // Only the document of the selected hit (a stale one is never shown).
  const doc = $derived(hit && preview.doc?.hitId === hit.id ? preview.doc : null);

  // Load the selected document, highlighted with the last query run.
  $effect(() => {
    void preview.load(hit, search.previewRequest());
  });

  function openFile() {
    if (hit && inTauri) api.openFile(hit.path).catch((e) => notices.error(e));
  }

  function revealFile() {
    if (hit && inTauri) api.revealFile(hit.path).catch((e) => notices.error(e));
  }

  /** Lot 6.7: a file inside an archive or an e-mail (not a message) can be taken out. */
  const extractable = $derived(hit !== null && hit.innerKind !== undefined);
  let extracting = $state(false);

  async function extractFile() {
    if (!hit || !extractable || extracting) return;
    // The dialog's default name: the file's own, without what Windows refuses.
    const name = splitPath(hit.path).name.replace(/[<>:"/\\|?*\u0000-\u001f]/g, '_');
    if (!inTauri) {
      notices.push(t('preview.extracted', { path: `D:\\Export\\${name}` }));
      return;
    }
    const path = hit.path;
    const target = await api.pickSaveAs(name);
    if (!target) return;
    extracting = true;
    try {
      await api.extractTo(path, target);
      notices.push(t('preview.extracted', { path: target }));
    } catch (e) {
      notices.error(e);
    } finally {
      extracting = false;
    }
  }

  /** Lines split into segments; every match gets a document-wide index. */
  const lines = $derived.by(() => {
    let occ = 0;
    return (doc?.lines ?? []).map((line) => ({
      n: line.n,
      // Code: syntax colors on the plain parts (a match keeps its highlight).
      segs: (doc?.layout === 'code' ? colorSegments(segments(line.text)) : segments(line.text).map((s) => ({ ...s, parts: [] }))).map(
        (s) => ({ ...s, occ: s.hit ? occ++ : -1 }),
      ),
    }));
  });
  // Rich preview: Excel sheets as tables, PowerPoint slides as cards.
  const sections = $derived(doc?.layout === 'sheet' ? sheets(lines) : doc?.layout === 'slides' ? slides(lines) : []);
  const total = $derived(lines.reduce((sum, l) => sum + l.segs.filter((s) => s.hit).length, 0));

  let current = $state(0);
  let copied = $state(false);
  let bodyEl: HTMLElement | undefined = $state();

  // New document → back to the first occurrence.
  $effect(() => {
    void doc?.hitId;
    current = 0;
  });

  // Bring the active occurrence into view.
  $effect(() => {
    const index = current;
    if (!bodyEl || total === 0) return;
    const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
    bodyEl
      .querySelector(`[data-occ="${index}"]`)
      ?.scrollIntoView({ block: 'center', behavior: reduce ? 'auto' : 'smooth' });
  });

  function go(delta: number) {
    if (total === 0) return;
    current = (current + delta + total) % total;
  }

  /** F3 / Shift+F3 anywhere, Enter / Shift+Enter while the document has focus. */
  function onWindowKey(event: KeyboardEvent) {
    const inDoc = event.key === 'Enter' && bodyEl !== undefined && bodyEl === document.activeElement;
    if (event.key === 'F3' || inDoc) {
      event.preventDefault();
      go(event.shiftKey ? -1 : 1);
    }
  }

  async function copyPath() {
    if (!hit) return;
    try {
      await navigator.clipboard.writeText(hit.path);
      copied = true;
      setTimeout(() => (copied = false), 1500);
    } catch {
      /* clipboard refused: nothing to do in the mock */
    }
  }

  /**
   * Text and code files open in the code editor at a line (lot 5.7). Not a
   * document inside an archive or a mailbox, nor PDF / Word, whose lines are
   * the extractor's, not the file's.
   */
  const editable = $derived(
    hit !== null &&
      (hit.kind === 'text' || hit.kind === 'code') &&
      !hit.path.includes(' › ') &&
      integration.status?.editorReady === true,
  );
  /** Line of the active occurrence (else the first one shown). */
  const activeLine = $derived(lines.find((l) => l.segs.some((s) => s.occ === current))?.n ?? lines[0]?.n ?? 1);

  // Lot 6.6: an image is shown, the words found boxed on it (the OCR reads it
  // again for their position), its OCR text below. Lot 6.7: also an image
  // inside an archive or attached to an e-mail.
  const isImage = $derived(hit !== null && isImageHit(hit));
  let picture = $state<string | null>(null);
  let boxes = $state<{ x: number; y: number; w: number; h: number; fuzzy: boolean }[]>([]);
  let locating = $state(false);
  $effect(() => {
    const h = hit;
    const request = search.previewRequest();
    picture = null;
    boxes = [];
    if (!h || !isImage) return;
    let stale = false;
    void thumbnail(h.path, 1200).then((url) => {
      if (!stale) picture = url;
    });
    if (!inTauri) {
      boxes = DEMO_BOXES;
    } else if (request.query.trim() || request.detectors) {
      locating = true;
      api
        .imageMatches(h.path, request)
        .then((found) => {
          if (!stale) boxes = found;
        })
        .catch(() => {})
        .finally(() => {
          if (!stale) locating = false;
        });
    }
    return () => {
      stale = true;
      locating = false;
    };
  });

  /** Files on disk only: a document inside an archive has no digest of its own. */
  const copiable = $derived(hit !== null && hit.kind !== 'folder' && !hit.path.includes(' › '));
  let hashing = $state(false);

  async function copies() {
    if (!hit || hashing) return;
    hashing = true;
    try {
      await findCopies(hit);
    } finally {
      hashing = false;
    }
  }

  function openInEditor(line: number) {
    if (hit && editable) void integration.openInEditor(hit.path, line);
  }

  const kindKey = $derived(hit ? (`filters.kinds.${hit.kind}` as MessageKey) : null);
  const langKey = $derived(hit ? (`languages.${hit.lang}` as MessageKey) : null);
</script>

<svelte:window onkeydown={onWindowKey} />

<aside class="preview sketch" aria-label={t('preview.label')}>
  <span class="tag sketch hatch">{t('preview.label')}</span>
  {#if hit && doc}
    {@const p = splitPath(hit.path)}
    <header>
      <h2 title={p.name}><span class="ltr-isolate" dir="ltr">{p.name}</span></h2>
      <div class="path mono" title={hit.path}><span class="ltr-isolate" dir="ltr">{p.folder}</span></div>
      <p class="meta">
        {#if hit.kind === 'folder'}
          {kindKey ? t(kindKey) : ''} · {formatDate(hit.modified)}
        {:else}
          {kindKey ? t(kindKey) : ''} · <span class="mono">{formatBytes(hit.sizeBytes)}</span> · <span title={t('preview.created', { date: formatDate(hit.created) })}>{formatDate(hit.modified)}</span> · {langKey ? t(langKey) : ''}
        {/if}
      </p>
    </header>

    {#if preview.truncated}
      <p class="note">{t('preview.truncated')}</p>
    {/if}

    <!-- Match marks of the rich layouts (tables, slides). -->
    {#snippet text(segs: Seg[])}{#each segs as seg, i (i)}{#if seg.hit}<mark data-occ={seg.occ} class:active={seg.occ === current} class:fuzzy={seg.fuzzy} title={seg.fuzzy ? t('results.approximate') : undefined}>{seg.text}</mark>{:else}{seg.text}{/if}{/each}{/snippet}

    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="doc {doc.layout}" bind:this={bodyEl} tabindex="0" role="region" aria-label={p.name}>
      {#if isImage}
        <figure class="picture">
          {#if picture}
            <div class="frame">
              <img src={picture} alt={p.name} />
              {#each boxes as b, i (i)}
                <span
                  class="box"
                  class:fuzzy={b.fuzzy}
                  class:active={i === current % Math.max(boxes.length, 1)}
                  style:inset-inline-start="{b.x * 100}%"
                  style:inset-block-start="{b.y * 100}%"
                  style:inline-size="{b.w * 100}%"
                  style:block-size="{b.h * 100}%"
                ></span>
              {/each}
            </div>
          {:else}
            <p class="picture-note">{t('preview.loading')}</p>
          {/if}
          <figcaption>
            {#if locating}{t('preview.imageLocating')}{:else if boxes.length > 0}{t('preview.imageBoxes', { count: boxes.length })}{:else if search.lastQuery.trim()}{t('preview.imageNoBoxes')}{/if}
          </figcaption>
        </figure>
      {/if}
      {#if hit.kind === 'folder'}
        <p class="folder-note">{t('preview.folder')}</p>
      {:else if doc.layout === 'sheet'}
        {#each sections as sheet (sheet.n)}
          <section class="sheet">
            {#if sheet.title}<h3 class="section-title" dir="auto">{@render text(sheet.title)}</h3>{/if}
            <!-- The table follows the document's direction, not the interface's. -->
            <div class="table-wrap" dir={hit.lang === 'ar' ? 'rtl' : 'ltr'}>
              <table>
                <tbody>
                  {#each sheet.rows as row, index (row.n)}
                    {#if row.kind === 'gap'}
                      <tr class="gap"><td colspan={sheet.columns} aria-label={t('preview.skipped')}>⋯</td></tr>
                    {:else}
                      <tr class:first={index === 0}>
                        {#each { length: sheet.columns } as _, col (col)}
                          {@const cell = row.cells[col] ?? []}
                          <td class:num={isNumeric(cell)} class:long={isLong(cell)} dir="auto">{@render text(cell)}</td>
                        {/each}
                      </tr>
                    {/if}
                  {/each}
                </tbody>
              </table>
            </div>
          </section>
        {/each}
      {:else if doc.layout === 'slides'}
        {#each sections as slide (slide.n)}
          <section class="slide sketch">
            {#if slide.title}<span class="slide-no mono">{@render text(slide.title)}</span>{/if}
            {#each slide.rows as row, index (row.n)}
              {#if row.kind === 'gap'}
                <p class="slide-gap" aria-label={t('preview.skipped')}>⋯</p>
              {:else if index === 0}
                <h3 class="slide-title" dir="auto">{@render text(row.cells[0] ?? [])}</h3>
              {:else}
                <p class="slide-line" dir="auto">{@render text(row.cells[0] ?? [])}</p>
              {/if}
            {/each}
          </section>
        {/each}
      {:else}
      {#each lines as line (line.n)}
        <div class="line">
          {#if editable}
            <!-- Mouse shortcut; from the keyboard, the footer button does the same. -->
            <button type="button" class="ln mono link" tabindex="-1" title={t('preview.lineLink', { line: line.n })} aria-label={t('preview.lineLink', { line: line.n })} onclick={() => openInEditor(line.n)}>{line.n}</button>
          {:else}
            <span class="ln mono" aria-hidden="true">{line.n}</span>
          {/if}
          <span class="txt" dir={doc.layout === 'code' ? 'ltr' : 'auto'}
            >{#each line.segs as seg, i (i)}{#if seg.hit}<mark data-occ={seg.occ} class:active={seg.occ === current} class:fuzzy={seg.fuzzy} title={seg.fuzzy ? t('results.approximate') : undefined}>{seg.text}</mark>{:else if doc.layout === 'code'}{#each seg.parts as part, j (j)}{#if part.cls}<span class="syn-{part.cls}">{part.text}</span>{:else}{part.text}{/if}{/each}{:else}{seg.text}{/if}{/each}</span
          >
        </div>
      {/each}
      {/if}
    </div>

    <footer class="bar">
      <button type="button" class="btn sketch hatch primary" title={extractable ? t('preview.openInnerHint') : undefined} onclick={openFile}>
        <Icon name="external" size={15} /><span>{t('preview.open')}</span>
      </button>
      {#if extractable}
        <button type="button" class="btn sketch" title={t('preview.extractHint')} onclick={extractFile} disabled={extracting}>
          <Icon name="download" size={15} /><span>{t('preview.extract')}</span>
        </button>
      {/if}
      <button type="button" class="btn sketch" title={t('preview.reveal')} onclick={revealFile}>
        <Icon name="folder" size={15} /><span>{t('preview.reveal')}</span>
      </button>
      {#if editable}
        <button type="button" class="btn sketch" title={t('preview.openAtLineHint')} onclick={() => openInEditor(activeLine)}>
          <Icon name="code" size={15} /><span>{t('preview.openAtLine', { line: activeLine })}</span>
        </button>
      {/if}
      {#if copiable}
        <button type="button" class="btn sketch" title={t('preview.copiesHint')} onclick={copies} disabled={hashing}>
          <Icon name="twins" size={15} /><span>{hashing ? t('preview.hashing') : t('preview.copies')}</span>
        </button>
      {/if}
      <button type="button" class="btn sketch icon" onclick={copyPath} aria-label={copied ? t('preview.copied') : t('preview.copyPath')} title={copied ? t('preview.copied') : t('preview.copyPath')}>
        <Icon name="copy" size={15} />
      </button>

      {#if hit.kind !== 'folder'}
      <div class="occ-nav" role="group" aria-label={t('results.matches', { count: total })}>
        <button type="button" class="arrow" onclick={() => go(-1)} disabled={total === 0} aria-label={t('preview.prev')} title="{t('preview.prev')} (Shift+F3)">↑</button>
        <span class="occ" aria-live="polite">{t('preview.occurrence', { current: total ? current + 1 : 0, total })}</span>
        <button type="button" class="arrow" onclick={() => go(1)} disabled={total === 0} aria-label={t('preview.next')} title="{t('preview.next')} (F3)">↓</button>
      </div>
      {/if}
    </footer>
  {:else if hit && preview.loading}
    <p class="none">{t('preview.loading')}</p>
  {:else}
    <p class="none">{t('preview.none')}</p>
  {/if}
</aside>

<style>
  .preview {
    --k: var(--ok);
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
    min-block-size: 0;
    margin-block-start: var(--sp-3);
    padding: var(--sp-6) var(--sp-6) var(--sp-4);
    background: var(--surface);
  }

  .tag {
    --k: var(--ok);
    --h: var(--fill-ok);
    --sw: 2px;
    position: absolute;
    inset-block-start: -16px;
    inset-inline-start: var(--sp-6);
    z-index: 1;
    padding: 0 var(--sp-4);
    background-color: var(--bg);
    font: var(--fs-h2) / var(--lh-h2) var(--font-brand);
    text-transform: uppercase;
  }

  header {
    flex: none;
    padding-block-end: var(--sp-3);
  }

  h2 {
    margin: var(--sp-1) 0 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: var(--fs-title) / var(--lh-title) var(--font-display);
  }

  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-caption);
    line-height: var(--lh-caption);
    color: var(--text-dim);
  }

  .meta {
    margin: var(--sp-1) 0 0;
    font: 16px var(--font-display);
    text-transform: uppercase;
    color: var(--text-dim);
  }

  /* ── Document ──────────────────────────────────────────── */
  .doc {
    flex: 1 1 auto;
    min-block-size: 0;
    overflow: auto;
    padding-block: var(--sp-2) var(--sp-4);
    border-block-start: 2px dashed var(--text-faint);
  }

  .doc:focus-visible {
    outline-offset: -3px;
  }

  .line {
    display: flex;
    gap: var(--sp-3);
    max-inline-size: 72ch;
  }

  .prose .line + .line {
    margin-block-start: var(--sp-2);
  }

  .ln {
    flex: none;
    min-inline-size: 28px;
    font-size: var(--fs-caption);
    line-height: var(--lh-body);
    color: var(--text-faint);
    text-align: end;
    user-select: none;
  }

  button.ln {
    padding: 0;
    background: none;
    border: 0;
    font-family: var(--font-mono);
    cursor: pointer;
  }

  button.ln:hover {
    color: var(--accent);
    text-decoration: underline;
  }

  .txt {
    min-inline-size: 0;
    flex: 1 1 auto;
    unicode-bidi: plaintext;
  }

  .code .txt {
    font-family: var(--font-mono);
    font-size: 15px;
    white-space: pre;
    text-align: left;
  }

  mark {
    background: var(--mark-bg);
    color: var(--mark-text);
    padding-inline: 3px;
    font-weight: 700;
  }

  mark.fuzzy {
    background: none;
    font-weight: 400;
    text-decoration: underline wavy var(--accent) 1.5px;
    text-underline-offset: 3px;
  }

  mark.active {
    background: var(--mark-active-bg);
    color: var(--mark-active-text);
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .folder-note {
    margin: var(--sp-3) 0;
    color: var(--text-dim);
  }

  /* ── Rich preview: sheets (Excel) ──────────────────────── */
  .sheet + .sheet,
  .slide + .slide {
    margin-block-start: var(--sp-5);
  }

  .section-title {
    margin: 0 0 var(--sp-2);
    font: 16px/24px var(--font-display);
    text-transform: uppercase;
    color: var(--secondary-ink);
  }

  .table-wrap {
    overflow-x: auto;
  }

  table {
    border-collapse: collapse;
    font-size: 14px;
    line-height: 20px;
  }

  td {
    padding: var(--sp-1) var(--sp-3);
    border: 1px solid var(--text-faint);
    vertical-align: top;
    white-space: nowrap;
  }

  td.long {
    min-inline-size: 18ch;
    max-inline-size: 36ch;
    white-space: normal;
    overflow-wrap: anywhere;
  }

  td.num {
    font-family: var(--font-mono);
    font-size: 13px;
    text-align: end;
  }

  tr.first td {
    font-weight: 700;
    border-block-end: 2px solid var(--text-dim);
  }

  tr:nth-child(even):not(.gap) td {
    background: color-mix(in srgb, var(--text-faint) 14%, transparent);
  }

  tr.gap td {
    text-align: center;
    color: var(--text-faint);
    border-style: dashed;
  }

  /* ── Rich preview: slides (PowerPoint) ─────────────────── */
  .slide {
    --k: var(--secondary);
    position: relative;
    max-inline-size: 72ch;
    padding-block: var(--sp-4);
    padding-inline: calc(var(--sp-5) + 28px) var(--sp-5);
  }

  .slide-no {
    position: absolute;
    inset-block-start: var(--sp-4);
    inset-inline-start: var(--sp-3);
    display: grid;
    place-items: center;
    min-inline-size: 28px;
    block-size: 28px;
    padding-inline: 4px;
    border: 2px solid var(--secondary);
    border-radius: 999px;
    font-size: 13px;
    color: var(--secondary-ink);
  }

  .slide-title {
    margin: 0 0 var(--sp-2);
    font: 20px/28px var(--font-display);
  }

  .slide-line,
  .slide-gap {
    margin: var(--sp-1) 0 0;
  }

  .slide-gap {
    color: var(--text-faint);
  }

  /* ── Footer bar ────────────────────────────────────────── */
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--sp-2) var(--sp-3);
    flex: none;
    padding-block-start: var(--sp-3);
  }

  .btn {
    --sw: 2px;
    display: flex;
    align-items: center;
    gap: var(--sp-2);
    padding: var(--sp-1) var(--sp-3);
    background-color: transparent;
    border: 0;
    font: 17px/24px var(--font-display);
    text-transform: uppercase;
    white-space: nowrap;
  }

  .btn.primary {
    --k: var(--secondary);
    --h: var(--fill-secondary);
  }

  .btn.icon {
    padding-inline: var(--sp-2);
  }

  .btn:hover {
    --k: var(--secondary);
  }

  .occ-nav {
    display: flex;
    align-items: center;
    gap: var(--sp-1);
    margin-inline-start: auto;
  }

  .arrow {
    display: grid;
    place-items: center;
    inline-size: 30px;
    block-size: 30px;
    padding: 0;
    background: transparent;
    border: 0;
    font: 24px/1 var(--font-display);
  }

  .arrow:hover:not(:disabled) {
    color: var(--secondary-ink);
  }

  .arrow:disabled {
    opacity: 0.35;
  }

  .occ {
    min-inline-size: 56px;
    text-align: center;
    font: 24px/1 var(--font-display);
    font-variant-numeric: tabular-nums;
  }

  .note {
    margin: 0 0 var(--sp-2);
    font-size: var(--fs-caption);
    line-height: var(--lh-caption);
    color: var(--text-dim);
  }

  /* Code syntax colors: defined by the theme. */
  .syn-kw {
    color: var(--syn-kw);
    font-weight: 700;
  }

  .syn-str {
    color: var(--syn-str);
  }

  .syn-com {
    color: var(--syn-com);
    font-style: italic;
  }

  .syn-num {
    color: var(--syn-num);
  }

  .none {
    margin: auto;
    padding: var(--sp-4);
    max-inline-size: 300px;
    text-align: center;
    color: var(--text-dim);
  }

  /* Lot 6.6: the image, the words found boxed on it. */
  .picture {
    margin: 0 0 var(--sp-4);
  }

  .frame {
    position: relative;
    display: inline-block;
    max-inline-size: 100%;
    /* Boxes are placed in the image's own direction, whatever the interface. */
    direction: ltr;
  }

  .frame img {
    display: block;
    max-inline-size: 100%;
    max-block-size: 60vh;
    border-radius: 4px;
    box-shadow: 0 0 0 1.5px var(--text-faint);
  }

  .box {
    position: absolute;
    border: 2px solid var(--accent);
    border-radius: 3px;
    background: color-mix(in srgb, var(--mark-bg, #ffe58a) 35%, transparent);
    pointer-events: none;
  }

  .box.fuzzy {
    border-style: dashed;
  }

  .box.active {
    border-width: 3px;
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 35%, transparent);
  }

  figcaption,
  .picture-note {
    margin-block-start: var(--sp-1);
    font-size: var(--fs-caption);
    color: var(--text-dim);
  }
</style>
