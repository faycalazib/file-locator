# Publishing a version of Prospector

> Windows first (Étape 4 choice). macOS and Linux: later, through GitHub Actions.
> Code signing: none for now. Windows SmartScreen shows "unknown publisher", and the download page explains how to get past it.

## One-time setup

1. **GitHub repository.** Create it, then:
   - put its `owner/name` in `site/config.js` (`PROSPECTOR_REPO`);
   - put the same `owner/name` in the updater address in `src-tauri/tauri.conf.json` (`plugins.updater.endpoints`).
2. **Update key.** Tauri signs every update with its own key, which is separate from a Windows certificate. The app refuses any update that is not signed with this key.

   ```bash
   pnpm tauri signer generate -w E:\keys\prospector\prospector-updater.key
   ```

   Run it yourself in a terminal: it asks for the password, which must never go through a chat or a file. The folder `E:\keys\prospector` was chosen on 2026-09-27.

   - The **private** key (`.key`) and its password never go in the repository. Keep a copy somewhere safe: losing it means installed copies can no longer be updated.
   - The **public** key (`.key.pub`) goes in `tauri.conf.json` → `plugins.updater.pubkey`.
   - In GitHub → Settings → Secrets and variables → Actions, add `TAURI_SIGNING_PRIVATE_KEY` (the content of the `.key` file) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
   - In `tauri.conf.json` → `bundle`, set `"createUpdaterArtifacts": true`.
3. **GitHub Pages.** Settings → Pages → Source: *GitHub Actions*. The `Site` workflow publishes `site/`.

Until step 2 is done, the app works normally: updates are simply off, and their section is hidden in Settings.

## Each version

1. Update the version number (semver `X.Y.Z`) in three places:
   - `src-tauri/tauri.conf.json` (`version`);
   - `Cargo.toml` (`[workspace.package] version`);
   - `package.json`.
2. Write the changes in `CHANGELOG.md`.
3. Refresh the licences: `python scripts/third-party.py`, then copy `src-tauri/licenses/THIRD-PARTY-NOTICES.txt` to `site/`.
4. Checks: `pnpm check`, `pnpm i18n:check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p prospector-core -p prospector-cli`.
5. Commit, then tag and push:

   ```bash
   git tag vX.Y.Z
   git push origin main --tags
   ```

6. The `Release` workflow builds the installers and creates a **draft** release. It contains:
   - `Prospector_X.Y.Z_x64-setup.exe` (NSIS, per user, 4 languages);
   - 4 `.msi` files (one per language);
   - `latest.json` and the `.sig` files for the updates;
   - `Prospector_X.Y.Z_x64-portable.zip`, the portable version (lot 6.8, `scripts/portable.py`).
7. Install the `.exe` on a clean Windows and run [TESTS-A-FAIRE.md](TESTS-A-FAIRE.md). Then **publish** the draft.

   - Installed copies see the new version at their next start (Settings → Updates).
   - The download page offers it automatically.

## Meaning-search module (Étape 8)

The module is not in the installers: users download it from Settings.

1. `python scripts/sense-module.py` builds `target/sense-module/prospector-sense-<version>.zip` (model, tokenizer, ONNX Runtime, licences, manifest) and prints its SHA-256. The ZIP is reproducible: the same inputs give the same bytes.
2. Attach the ZIP to the GitHub release.
3. In `core/src/sense/module.rs`, set `MODULE_URL` to its download address, and check `MODULE_SHA256` / `MODULE_BYTES` against what the script printed. A new model or runtime is a new `MODULE_VERSION`.

## App icon

The source is `src-tauri/icons/source.svg`: concept A, « Loupe crayonnée », chosen on 2026-09-27 from `docs/mockups/icons/concepts.png`. To regenerate every size (.ico, .icns, png):

1. Render it to a 1024 px PNG with a transparent background. Headless Chrome works: open `scripts/icon.html` with `--default-background-color=00000000 --window-size=1024,1024 --screenshot=…`.
2. Run `pnpm tauri icon target/tmp/icon-1024.png`.
3. Copy `src-tauri/icons/128x128.png` to `site/img/icon.png`.

## Build locally (without publishing)

```bash
pnpm tauri build --config src-tauri/tauri.release.conf.json
```

- `--config src-tauri/tauri.release.conf.json` builds `prospector-cli.exe` first and puts it in the installers (lot 7.1). Without it, the installers have no command line.

- Installers are written to `target/release/bundle/nsis/` and `target/release/bundle/msi/`.
- The NSIS and WiX tools are downloaded once into `target/` (`useLocalToolsDir`), not to C:.
- Without `TAURI_SIGNING_PRIVATE_KEY` in the environment, leave `createUpdaterArtifacts` off, otherwise the build stops.
- Portable version: `python scripts/portable.py` after the build → `target/release/bundle/portable/Prospector_X.Y.Z_x64-portable.zip` (the program, the `portable` file, a README in 4 languages, the licences).

## Later

- **Signing.** Add `bundle.windows.certificateThumbprint`, or a `signCommand` for Azure Trusted Signing. Nothing else changes.
- **macOS / Linux.** Add those runners to `release.yml` (tauri-action supports them). A notarized macOS build needs an Apple Developer account.
