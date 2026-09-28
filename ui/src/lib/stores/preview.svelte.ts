/**
 * Preview of the selected hit. In Tauri the text comes from the index (with
 * the last query's matches marked); outside Tauri from the Étape 0 mocks.
 */
import { api, inTauri } from '../api';
import { previewFor } from '../mock/data';
import type { SearchRequestDto } from '../api';
import type { PreviewDoc, SearchHit } from '../types';
import { notices } from './notices.svelte';

class PreviewStore {
  doc = $state<PreviewDoc | null>(null);
  loading = $state(false);
  truncated = $state(false);
  #generation = 0;
  #key = '';

  async load(hit: SearchHit | null, request: SearchRequestDto) {
    const key = hit ? `${hit.id}\u0000${JSON.stringify(request)}` : '';
    if (key === this.#key) return;
    this.#key = key;
    const generation = ++this.#generation;

    if (!hit) {
      this.doc = null;
      return;
    }
    // A folder has no text: the panel offers to open it.
    if (hit.kind === 'folder') {
      this.doc = { hitId: hit.id, layout: 'prose', lines: [] };
      this.truncated = false;
      return;
    }
    if (!inTauri) {
      this.doc = previewFor(hit);
      this.truncated = false;
      return;
    }
    this.loading = true;
    try {
      const dto = await api.preview(hit.siteId, hit.path, request);
      if (generation !== this.#generation) return;
      this.doc = { hitId: hit.id, layout: dto.layout, lines: dto.lines };
      this.truncated = dto.truncated;
    } catch (e) {
      if (generation !== this.#generation) return;
      this.doc = null;
      notices.error(e);
    } finally {
      if (generation === this.#generation) this.loading = false;
    }
  }
}

export const preview = new PreviewStore();
