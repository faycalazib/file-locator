/** Short messages at the bottom of the screen (errors, confirmations). */
import { errorCode } from '../api';
import { t, type MessageKey } from '../i18n/index.svelte';

export interface Notice {
  id: number;
  kind: 'error' | 'info';
  text: string;
  /** What it is about: the next action on the same topic removes it (`clear`). */
  topic?: string;
}

const LIFETIME_MS = 6000;

class NoticesStore {
  items = $state<Notice[]>([]);
  #next = 1;

  push(text: string, kind: Notice['kind'] = 'info', topic?: string) {
    const id = this.#next++;
    this.items = [...this.items, { id, kind, text, topic }];
    setTimeout(() => this.dismiss(id), kind === 'error' ? LIFETIME_MS * 2 : LIFETIME_MS);
  }

  /** Translates an engine error `{ code, params }` and shows it. */
  error(e: unknown, topic?: string) {
    const { code, params } = errorCode(e);
    const key = `errors.${code}` as MessageKey;
    const text = t(key, params as Record<string, string | number>);
    this.push(text === key ? t('errors.unknown') : text, 'error', topic);
  }

  /** Removes the messages about `topic` (a new search: the last one's error is stale, BUG-044). */
  clear(topic: string) {
    this.items = this.items.filter((n) => n.topic !== topic);
  }

  dismiss(id: number) {
    this.items = this.items.filter((n) => n.id !== id);
  }
}

export const notices = new NoticesStore();
