/**
 * What a launch asks Prospector to open (lot 5.7): the folder of the
 * Explorer's "Search with Prospector", or a double-clicked `.prospector`
 * file. One instance only: a second launch hands its request over to this
 * one (`app://launch`), which takes it here.
 */
import { api, inTauri, type LaunchRequestDto } from './api';
import { openResultsAt, showInTab } from './results-io';
import { search } from './stores/search.svelte';

/** The search box, focused and selected (global shortcut, Explorer). */
export function focusSearch() {
  setTimeout(() => {
    const input = document.getElementById('query') as HTMLInputElement | null;
    input?.focus();
    input?.select();
  });
}

/**
 * A search limited to the folder, in a new tab. Inside an indexed site it
 * uses the index (instant); otherwise the folder is scanned now.
 */
function openFolder(path: string, siteId: string | null) {
  const base = search.blank();
  const scope = siteId && !base.scope.includes(siteId) ? [...base.scope, siteId] : base.scope;
  showInTab({ ...base, folder: path, mode: siteId ? 'indexed' : 'live', scope });
  focusSearch();
}

async function take() {
  const requests: LaunchRequestDto[] = await api.takeLaunchRequests().catch(() => []);
  for (const request of requests) {
    if (request.kind === 'folder') openFolder(request.path, request.siteId);
    else await openResultsAt(request.path);
  }
}

/** Listens for later launches, then takes the requests of this one. */
export async function initLaunches() {
  if (!inTauri) return;
  await api.onLaunch(() => void take());
  await take();
}
