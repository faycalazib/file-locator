/** Global shortcut: the choices offered in Settings and their display. */
import { t, type MessageKey } from './i18n/index.svelte';

/** Tauri accelerators. Alt+Space is left out: Windows opens the window menu with it. */
export const SHORTCUT_CHOICES = ['CmdOrCtrl+Shift+Space', 'CmdOrCtrl+Alt+Space', 'CmdOrCtrl+Alt+F', ''] as const;

const isMac = typeof navigator !== 'undefined' && /Mac/i.test(navigator.platform);

const KEYS: Record<string, MessageKey> = {
  cmdorctrl: isMac ? 'keys.cmd' : 'keys.ctrl',
  commandorcontrol: isMac ? 'keys.cmd' : 'keys.ctrl',
  ctrl: 'keys.ctrl',
  control: 'keys.ctrl',
  shift: 'keys.shift',
  alt: 'keys.alt',
  option: 'keys.alt',
  space: 'keys.space',
};

/** `CmdOrCtrl+Shift+Space` → `Ctrl + Maj + Espace` (translated key names). */
export function formatShortcut(accelerator: string): string {
  if (!accelerator) return t('settings.shortcutNone');
  return accelerator
    .split('+')
    .map((part) => {
      const key = KEYS[part.toLowerCase()];
      return key ? t(key) : part;
    })
    .join(' + ');
}
