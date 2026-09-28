/**
 * "Find copies" (lot 5.8): the files identical to a result, in a new tab.
 * The engine only hashes the files of the same size, found in the index or
 * during the walk, so it stays fast on big sites.
 */
import { api, inTauri } from './api';
import { showInTab } from './results-io';
import { notices } from './stores/notices.svelte';
import { search } from './stores/search.svelte';
import { splitPath } from './text';
import type { SearchHit } from './types';

/** Browser demo: a fixed digest, so the tab and its chip can be seen. */
const DEMO_SHA256 = '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08';

export async function findCopies(hit: SearchHit) {
  let digest: { sha256: string; size: number };
  try {
    digest = inTauri ? await api.fileDigest(hit.path) : { sha256: DEMO_SHA256, size: hit.sizeBytes };
  } catch (e) {
    notices.error(e);
    return;
  }
  const base = search.blank();
  showInTab({
    ...base,
    filters: { ...base.filters, hash: digest.sha256 },
    copiesOf: { name: splitPath(hit.path).name, sha256: digest.sha256, size: digest.size },
  });
  void search.run();
}
