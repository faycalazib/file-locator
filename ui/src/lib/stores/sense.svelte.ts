/**
 * Meaning-search module (Étape 8): installed or not, its download (progress,
 * cancel), installing it from a file, removing it. Kept by the engine in this
 * PC's personal folder.
 */
import { api, inTauri, type SenseStatusDto } from '../api';
import { t } from '../i18n/index.svelte';
import { notices } from './notices.svelte';

/** Browser demo: `?sense=installed` or `?sense=downloading` (screenshots). */
const demo = inTauri ? null : new URLSearchParams(location.search).get('sense');
const DEMO_STATUS: SenseStatusDto = {
  installed: demo === 'installed',
  model: demo === 'installed' ? 'granite-embedding-97m-multilingual-r2' : null,
  size: demo === 'installed' ? 139_600_000 : 0,
  downloadable: false,
  downloadSize: 80_437_000,
};

class SenseStore {
  status = $state<SenseStatusDto | null>(inTauri ? null : DEMO_STATUS);
  /** Bytes received / expected while downloading. */
  progress = $state<{ done: number; total: number } | null>(demo === 'downloading' ? { done: 35_700_000, total: 80_437_000 } : null);
  busy = $state(false);
  #listening = false;

  async load() {
    if (!inTauri) return;
    if (!this.#listening) {
      this.#listening = true;
      await api.onSenseProgress((p) => (this.progress = p));
    }
    try {
      this.status = await api.senseStatus();
    } catch (e) {
      notices.error(e);
    }
  }

  async download() {
    if (!inTauri || this.busy) return;
    this.busy = true;
    this.progress = { done: 0, total: this.status?.downloadSize ?? 0 };
    try {
      this.status = await api.downloadSenseModule();
      notices.push(t('sense.installedNotice'));
    } catch (e) {
      notices.error(e);
    } finally {
      this.busy = false;
      this.progress = null;
    }
  }

  cancel() {
    if (inTauri) void api.cancelSenseDownload().catch(() => {});
    else this.progress = null;
  }

  async installFromFile() {
    if (!inTauri) {
      this.status = { ...DEMO_STATUS, installed: true, model: 'granite-embedding-97m-multilingual-r2', size: 139_600_000 };
      return;
    }
    const path = await api.pickOpenPath('zip', t('sense.zipKind'), t('sense.pickZip')).catch(() => null);
    if (!path) return;
    this.busy = true;
    try {
      this.status = await api.installSenseModule(path);
      notices.push(t('sense.installedNotice'));
    } catch (e) {
      notices.error(e);
    } finally {
      this.busy = false;
    }
  }

  async remove() {
    if (!inTauri) {
      this.status = DEMO_STATUS.installed ? { ...DEMO_STATUS, installed: false, model: null, size: 0 } : DEMO_STATUS;
      return;
    }
    const ok = await api.confirm(t('sense.removeConfirm'), t('sense.remove')).catch(() => false);
    if (!ok) return;
    try {
      this.status = await api.removeSenseModule();
    } catch (e) {
      notices.error(e);
    }
  }
}

export const sense = new SenseStore();
