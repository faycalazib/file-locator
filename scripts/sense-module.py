"""Builds the meaning-search module (Étape 8): one ZIP to attach to a GitHub
release, and the same files unpacked for the tests.

Run: python scripts/sense-module.py

Downloads (once, into target/tmp/sense-src):
- the model Granite Embedding 97M multilingual r2 (IBM, Apache 2.0),
  quantized ONNX (`onnx/model_quint8_avx2.onnx`) and `tokenizer.json`;
- ONNX Runtime for Windows x64 (Microsoft, MIT): `onnxruntime.dll`.

Writes:
- target/sense-module/granite-97m-r2/      the module, unpacked (tests);
- target/sense-module/prospector-sense-<version>.zip, reproducible (fixed dates,
  fixed order): its SHA-256 is printed, to be written in the app
  (core/src/sense/module.rs → MODULE_SHA256) with the release address.
"""
import hashlib
import io
import json
import os
import shutil
import sys
import urllib.request
import zipfile

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
SRC = os.path.join(ROOT, 'target', 'tmp', 'sense-src')
OUT = os.path.join(ROOT, 'target', 'sense-module')

MODULE_VERSION = 1
MODEL_ID = 'granite-embedding-97m-multilingual-r2'
HF = 'https://huggingface.co/ibm-granite/granite-embedding-97m-multilingual-r2/resolve/main'
ORT_VERSION = '1.28.0'
ORT_ZIP = f'onnxruntime-win-x64-{ORT_VERSION}.zip'
ORT_URL = f'https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/{ORT_ZIP}'
APACHE = 'https://www.apache.org/licenses/LICENSE-2.0.txt'

FIXED_DATE = (2026, 9, 28, 0, 0, 0)

NOTICE = f"""Prospector — meaning-search module {MODULE_VERSION}

model.onnx, tokenizer.json
  Granite Embedding 97M multilingual r2, by IBM
  https://huggingface.co/ibm-granite/{MODEL_ID}
  Licence: Apache License 2.0 (LICENSE-model.txt). Quantized ONNX as published by IBM.

onnxruntime.dll
  ONNX Runtime {ORT_VERSION}, by Microsoft
  https://github.com/microsoft/onnxruntime
  Licence: MIT (LICENSE-onnxruntime.txt); third parties: ThirdPartyNotices-onnxruntime.txt.
"""


def fetch(url, name):
    os.makedirs(SRC, exist_ok=True)
    path = os.path.join(SRC, name)
    if not os.path.exists(path):
        print(f'downloading {url}')
        tmp = path + '.part'
        with urllib.request.urlopen(url) as r, open(tmp, 'wb') as f:
            shutil.copyfileobj(r, f)
        os.replace(tmp, path)
    return path


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def main():
    files = {}
    with open(fetch(f'{HF}/onnx/model_quint8_avx2.onnx', 'model_quint8_avx2.onnx'), 'rb') as f:
        files['model.onnx'] = f.read()
    with open(fetch(f'{HF}/tokenizer.json', 'tokenizer.json'), 'rb') as f:
        files['tokenizer.json'] = f.read()
    with open(fetch(APACHE, 'LICENSE-2.0.txt'), 'rb') as f:
        files['LICENSE-model.txt'] = f.read()
    with zipfile.ZipFile(fetch(ORT_URL, ORT_ZIP)) as z:
        prefix = f'onnxruntime-win-x64-{ORT_VERSION}/'
        files['onnxruntime.dll'] = z.read(prefix + 'lib/onnxruntime.dll')
        files['LICENSE-onnxruntime.txt'] = z.read(prefix + 'LICENSE')
        files['ThirdPartyNotices-onnxruntime.txt'] = z.read(prefix + 'ThirdPartyNotices.txt')
    files['NOTICE.txt'] = NOTICE.encode('utf-8')
    manifest = {
        'module': 'sense',
        'version': MODULE_VERSION,
        'model': MODEL_ID,
        'dimensions': 384,
        'onnxruntime': ORT_VERSION,
        'files': {name: sha256(data) for name, data in sorted(files.items())},
    }
    files['manifest.json'] = (json.dumps(manifest, indent=2) + '\n').encode('utf-8')

    # Unpacked, for the tests.
    unpacked = os.path.join(OUT, 'granite-97m-r2')
    os.makedirs(unpacked, exist_ok=True)
    for name, data in files.items():
        with open(os.path.join(unpacked, name), 'wb') as f:
            f.write(data)

    # The ZIP, reproducible: same files → same bytes → same SHA-256.
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, 'w', zipfile.ZIP_DEFLATED) as z:
        for name in sorted(files):
            info = zipfile.ZipInfo(name, FIXED_DATE)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            z.writestr(info, files[name])
    data = buffer.getvalue()
    out = os.path.join(OUT, f'prospector-sense-{MODULE_VERSION}.zip')
    with open(out, 'wb') as f:
        f.write(data)
    print(f'{out}\n  {len(data) / 1024 / 1024:.1f} MB, SHA-256 {sha256(data)}')
    print(f'{unpacked}\n  ' + ', '.join(f'{n} {len(d) / 1024 / 1024:.1f} MB' for n, d in sorted(files.items()) if len(d) > 1024 * 1024))


if __name__ == '__main__':
    sys.exit(main())
