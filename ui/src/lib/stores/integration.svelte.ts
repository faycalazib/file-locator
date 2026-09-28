/**
 * Windows integration (lot 5.7): "Search with Prospector" in the Explorer
 * menu, and the code editor that opens a file at a line. Outside Tauri a
 * simulated state lets the Settings section and the preview button be seen.
 */
import { api, inTauri, type IntegrationDto } from '../api';
import { notices } from './notices.svelte';

const DEMO: IntegrationDto = {
  explorerMenu: true,
  editors: [
    { id: 'vscode', name: 'Visual Studio Code' },
    { id: 'notepadpp', name: 'Notepad++' },
  ],
  editor: '',
  editorCommand: '',
  editorReady: true,
};

class IntegrationStore {
  status = $state<IntegrationDto | null>(inTauri ? null : DEMO);

  /** At startup and on a language change: the menu follows the program and the language. */
  async sync(label: string) {
    if (!inTauri) return;
    try {
      this.status = await api.syncExplorerMenu(label);
    } catch (e) {
      notices.error(e);
    }
  }

  async setExplorerMenu(enabled: boolean, label: string) {
    if (!inTauri) {
      if (this.status) this.status = { ...this.status, explorerMenu: enabled };
      return;
    }
    try {
      this.status = await api.setExplorerMenu(enabled, label);
    } catch (e) {
      notices.error(e);
      this.status = await api.getIntegration().catch(() => this.status);
    }
  }

  /** Lot 7.1: `prospector-cli` in any new terminal. */
  async setCliPath(enabled: boolean) {
    if (!inTauri) {
      if (this.status) this.status = { ...this.status, cliInPath: enabled };
      return;
    }
    try {
      this.status = await api.setCliPath(enabled);
    } catch (e) {
      notices.error(e);
      this.status = await api.getIntegration().catch(() => this.status);
    }
  }

  async setEditor(editor: string, command: string) {
    if (!inTauri) {
      if (this.status) this.status = { ...this.status, editor, editorCommand: command, editorReady: editor !== 'custom' || command.trim() !== '' };
      return;
    }
    try {
      this.status = await api.setEditor(editor, command);
    } catch (e) {
      notices.error(e);
    }
  }

  async openInEditor(path: string, line: number) {
    if (!inTauri) return;
    await api.openInEditor(path, line).catch((e) => notices.error(e));
  }
}

export const integration = new IntegrationStore();
