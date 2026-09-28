"""Builds the portable version (lot 6.8): a ZIP to unpack on a USB drive.

Run after `pnpm tauri build --config src-tauri/tauri.release.conf.json`
(the release workflow does it too):
    python scripts/portable.py

Writes target/release/bundle/portable/Prospector_<version>_x64-portable.zip:

    Prospector/prospector.exe
    Prospector/prospector-cli.exe  the command line (lot 7.1)
    Prospector/portable          the file that turns the portable mode on
    Prospector/README.txt        in English, French, Spanish and Arabic
    Prospector/licenses/…        third-party licences (UnRAR's is required)

On the first start, Prospector creates Prospector/ProspectorData next to it
(settings, indexes, interface data): nothing is written on the PC.
"""
import io
import json
import os
import sys
import zipfile

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
EXE = os.path.join(ROOT, 'target', 'release', 'prospector.exe')
CLI = os.path.join(ROOT, 'target', 'release', 'prospector-cli.exe')
LICENSES = os.path.join(ROOT, 'src-tauri', 'licenses')
OUT_DIR = os.path.join(ROOT, 'target', 'release', 'bundle', 'portable')
FOLDER = 'Prospector'

MARKER = (
    'This file turns on the portable mode of Prospector: its data stays in the '
    'ProspectorData folder next to it. Remove it to use the data of this PC instead.\n'
)

README = """Prospector — portable version
===============================

EN  Unpack this folder anywhere (a USB drive, for instance) and run
    prospector.exe. Settings and indexes stay in ProspectorData, next to
    it: nothing is installed or written on the PC. If the drive gets
    another letter on another PC, Prospector follows. Needs Windows 10 or
    11 with Microsoft Edge WebView2 (already there on up-to-date PCs).

FR  Décompressez ce dossier où vous voulez (une clé USB, par exemple) et
    lancez prospector.exe. Réglages et index restent dans ProspectorData, à
    côté : rien n'est installé ni écrit sur le PC. Si la clé change de
    lettre sur un autre PC, Prospector suit. Nécessite Windows 10 ou 11 avec
    Microsoft Edge WebView2 (déjà présent sur un PC à jour).

ES  Descomprima esta carpeta donde quiera (una memoria USB, por ejemplo) y
    ejecute prospector.exe. Los ajustes y los índices se quedan en
    ProspectorData, a su lado: no se instala ni se escribe nada en el PC.
    Si la unidad cambia de letra en otro PC, Prospector la sigue. Requiere
    Windows 10 u 11 con Microsoft Edge WebView2 (ya presente en un PC al día).

AR  فُكّ ضغط هذا المجلد حيث تشاء (على ذاكرة USB مثلًا) ثم شغّل
    prospector.exe. تبقى الإعدادات والفهارس في ProspectorData بجواره:
    لا يُثبَّت شيء ولا يُكتب على الحاسوب. إذا تغيّر حرف الوحدة على حاسوب
    آخر، يتبعه Prospector. يتطلب Windows 10 أو 11 مع Microsoft Edge
    WebView2 (موجود مسبقًا على الحواسيب المحدَّثة).
"""


def version():
    with io.open(os.path.join(ROOT, 'src-tauri', 'tauri.conf.json'), encoding='utf-8') as f:
        return json.load(f)['version']


def build(exe=EXE, cli=CLI, out_dir=OUT_DIR):
    for program in (exe, cli):
        if not os.path.isfile(program):
            sys.exit(f'{program} not found: run `pnpm tauri build --config src-tauri/tauri.release.conf.json` first.')
    os.makedirs(out_dir, exist_ok=True)
    out = os.path.join(out_dir, f'Prospector_{version()}_x64-portable.zip')
    with zipfile.ZipFile(out, 'w', zipfile.ZIP_DEFLATED) as z:
        z.write(exe, f'{FOLDER}/prospector.exe')
        z.write(cli, f'{FOLDER}/prospector-cli.exe')
        z.writestr(f'{FOLDER}/portable', MARKER)
        z.writestr(f'{FOLDER}/README.txt', README.replace('\n', '\r\n').encode('utf-8-sig'))
        for name in sorted(os.listdir(LICENSES)):
            z.write(os.path.join(LICENSES, name), f'{FOLDER}/licenses/{name}')
    return out


if __name__ == '__main__':
    path = build()
    print(f'{path} ({os.path.getsize(path) / 1024 / 1024:.1f} MB)')
