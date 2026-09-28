/**
 * Save the results on screen to a `.prospector` file, and open one in a new
 * tab (lot 5.6). Tauri: native dialogs, read and written by Rust. Browser
 * demo: download, and a file input.
 */
import { api, inTauri } from './api';
import { download } from './export';
import { t } from './i18n/index.svelte';
import { parseResultsFile, RESULTS_EXTENSION, toResultsFile } from './results-file';
import { notices } from './stores/notices.svelte';
import { search, type SearchSnapshot } from './stores/search.svelte';
import { tabs } from './stores/tabs.svelte';

/** "contrat résiliation" → `contrat-resiliation.prospector`. */
function defaultName(): string {
  const words = (search.lastQuery || search.namePattern || 'prospector')
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/[^\p{L}\p{N}]+/gu, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 60);
  return `${words || 'prospector'}.${RESULTS_EXTENSION}`;
}

export async function saveResults() {
  if (!search.hasRun || search.hits.length === 0) return;
  const content = toResultsFile(search.capture(false));
  if (!inTauri) {
    download(defaultName(), content, 'application/json');
    return;
  }
  try {
    const path = await api.pickSavePath(defaultName(), RESULTS_EXTENSION, t('results.file.kind'));
    if (!path) return;
    await api.saveTextFile(path, content);
    notices.push(t('results.file.saved', { path }));
  } catch (e) {
    notices.error(e);
  }
}

/** Opens saved results in a new tab (or in this one, if it is empty). */
export async function openResults() {
  const picked = inTauri ? await pickInTauri() : await pickInBrowser();
  if (picked) show(picked);
}

/** A `.prospector` file double-clicked in the Explorer (lot 5.7). */
export async function openResultsAt(path: string) {
  try {
    show({ name: path, text: await api.readTextFile(path) });
  } catch (e) {
    notices.error(e);
  }
}

function show(picked: { name: string; text: string }) {
  const name = picked.name.split(/[\\/]/).pop() ?? picked.name;
  const snap = parseResultsFile(picked.text, name, search.blank());
  if (!snap) {
    notices.push(t('results.file.invalid', { name }));
    return;
  }
  showInTab(snap);
}

/** In this tab if nothing was searched in it yet, otherwise in a new one. */
export function showInTab(snap: SearchSnapshot) {
  if (!search.hasRun) search.restore(snap);
  else if (tabs.canAdd) tabs.add(snap);
  else search.restore(snap);
}

async function pickInTauri(): Promise<{ name: string; text: string } | null> {
  try {
    const path = await api.pickOpenPath(RESULTS_EXTENSION, t('results.file.kind'), t('results.file.open'));
    return path ? { name: path, text: await api.readTextFile(path) } : null;
  } catch (e) {
    notices.error(e);
    return null;
  }
}

function pickInBrowser(): Promise<{ name: string; text: string } | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = `.${RESULTS_EXTENSION},.json`;
    input.onchange = async () => {
      const file = input.files?.[0];
      resolve(file ? { name: file.name, text: await file.text() } : null);
    };
    input.click();
  });
}
