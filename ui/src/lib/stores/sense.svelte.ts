/**
 * Meaning-search module (Étape 8): installed or not, its download (progress,
 * cancel), installing it from a file, removing it. Kept by the engine in this
 * PC's personal folder.
 */
import { api, inTauri, type SenseEstimateDto, type SenseStatusDto } from '../api';
import { formatNumber, t } from '../i18n/index.svelte';
import { notices } from './notices.svelte';

/** "≈ 55 min", "≈ 3 h 10" (null: unknown). */
export function formatDuration(seconds: number | null): string {
  if (seconds === null) return '';
  const minutes = Math.max(1, Math.round(seconds / 60));
  if (minutes < 60) return t('sense.minutes', { n: formatNumber(minutes) });
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return t('sense.hours', { h: formatNumber(h), m: String(m).padStart(2, '0') });
}

/** Browser demo estimates (screenshots). */
const DEMO_ESTIMATES: Record<string, SenseEstimateDto> = {
  clients: { passages: 186_400, seconds: 3300, done: 0 },
  mail: { passages: 64_800, seconds: 1200, done: 0 },
  code: { passages: 610_000, seconds: 11_400, done: 0 },
  archives: { passages: 12_900, seconds: 240, done: 0 },
};

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
  /** Lot 8.2: "Which sites to understand?" (after the module is installed). */
  askSites = $state(demo === 'dialog');
  /** Computation pace. */
  pace = $state<'normal' | 'economy'>('normal');

  /** What ticking "Meaning" on a site costs (a few seconds: text read, models measured). */
  async estimate(siteId: string): Promise<SenseEstimateDto | null> {
    if (!inTauri) return DEMO_ESTIMATES[siteId] ?? { passages: 1000, seconds: 60, done: 0 };
    try {
      return await api.senseEstimate(siteId);
    } catch (e) {
      notices.error(e);
      return null;
    }
  }

  async loadPace() {
    if (inTauri) this.pace = await api.getSensePace().catch(() => this.pace);
  }

  async setPace(pace: 'normal' | 'economy') {
    this.pace = inTauri ? await api.setSensePace(pace).catch(() => this.pace) : pace;
  }
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
      this.askSites = true;
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
      this.askSites = true;
      return;
    }
    const path = await api.pickOpenPath('zip', t('sense.zipKind'), t('sense.pickZip')).catch(() => null);
    if (!path) return;
    this.busy = true;
    try {
      this.status = await api.installSenseModule(path);
      notices.push(t('sense.installedNotice'));
      this.askSites = true;
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
