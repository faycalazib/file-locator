/**
 * Automatic updates (Étape 4, tauri-plugin-updater). Prospector checks the
 * release server a few seconds after starting (unless turned off in
 * Settings); if a newer version exists, a banner offers to install it and
 * restart. Offline or without a configured update key, nothing is shown.
 */
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { api, inTauri } from '../api';
import { t } from '../i18n/index.svelte';
import { notices } from './notices.svelte';

export type UpdateState = 'idle' | 'checking' | 'available' | 'downloading' | 'upToDate';

const AUTO_KEY = 'prospector.updates.auto';
const FIRST_CHECK_MS = 8000;

function readAuto(): boolean {
  try {
    return localStorage.getItem(AUTO_KEY) !== 'off';
  } catch {
    return true;
  }
}

class UpdatesStore {
  /** Updates are configured in this build (public key + server). */
  enabled = $state(false);
  /** Portable mode (lot 6.8): a new version is announced, never installed. */
  portable = $state(false);
  state = $state<UpdateState>('idle');
  /** Version offered by the server. */
  version = $state<string | null>(null);
  /** 0..1 while downloading, when the size is known. */
  progress = $state<number | null>(null);
  auto = $state(readAuto());
  /** The banner was closed with "Later" (until the next start). */
  dismissed = $state(false);
  #update: Update | null = null;

  async init() {
    if (!inTauri) return;
    try {
      const info = await api.appInfo();
      this.enabled = info.updatesEnabled;
      this.portable = Boolean(info.portableDir);
    } catch {
      this.enabled = false;
    }
    if (this.enabled && this.auto) setTimeout(() => void this.check(false), FIRST_CHECK_MS);
  }

  setAuto(on: boolean) {
    this.auto = on;
    try {
      localStorage.setItem(AUTO_KEY, on ? 'on' : 'off');
    } catch {
      /* not persisted */
    }
  }

  /** `manual`: from Settings, so "up to date" and errors are reported. */
  async check(manual: boolean) {
    if (!this.enabled || this.state === 'checking' || this.state === 'downloading') return;
    this.state = 'checking';
    try {
      this.#update = await check();
      if (this.#update) {
        this.version = this.#update.version;
        this.state = 'available';
        this.dismissed = false;
      } else {
        this.state = 'upToDate';
        if (manual) notices.push(t('updates.upToDate'));
      }
    } catch {
      this.state = 'idle';
      // Offline at startup is normal: only a manual check reports it.
      if (manual) notices.push(t('updates.checkFailed'), 'error');
    }
  }

  /** Downloads, installs, then restarts Prospector on the new version.
   * Portable: opens the page of the new version (nothing is installed). */
  async install() {
    if (this.portable) {
      await api.openReleasesPage().catch((e) => notices.error(e));
      this.dismissed = true;
      return;
    }
    const update = this.#update;
    if (!update) return;
    this.state = 'downloading';
    this.progress = null;
    let total = 0;
    let received = 0;
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === 'Started') total = event.data.contentLength ?? 0;
        if (event.event === 'Progress') {
          received += event.data.chunkLength;
          this.progress = total > 0 ? Math.min(received / total, 1) : null;
        }
      });
      await relaunch();
    } catch {
      this.state = 'available';
      notices.push(t('updates.installFailed'), 'error');
    }
  }
}

export const updates = new UpdatesStore();
