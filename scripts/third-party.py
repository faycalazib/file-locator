"""Writes src-tauri/licenses/THIRD-PARTY-NOTICES.txt, shipped by the installer.

Run: python scripts/third-party.py   (before a release; see docs/RELEASE.md)

Lists every Rust crate compiled into the Windows app (normal dependencies of
the `prospector` and `prospector-cli` binaries, resolved by `cargo metadata`) and the npm packages
bundled in the interface, with their licences. The UnRAR licence is quoted in
full, as its paragraph 2 requires.
"""
import io
import json
import os
import subprocess

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
OUT = os.path.join(ROOT, 'src-tauri', 'licenses', 'THIRD-PARTY-NOTICES.txt')
TARGET = 'x86_64-pc-windows-msvc'


def rust_crates():
    meta = json.loads(subprocess.run(
        ['cargo', 'metadata', '--format-version', '1', '--filter-platform', TARGET],
        cwd=ROOT, check=True, capture_output=True, text=True, encoding='utf-8').stdout)
    packages = {p['id']: p for p in meta['packages']}
    nodes = {n['id']: n for n in meta['resolve']['nodes']}
    roots = [p['id'] for p in meta['packages'] if p['name'] in ('prospector', 'prospector-cli')]
    seen, stack = set(), list(roots)
    while stack:
        node = nodes[stack.pop()]
        for dep in node['deps']:
            # Runtime code only: not build scripts, not dev dependencies.
            if any(k['kind'] is None for k in dep['dep_kinds']) and dep['pkg'] not in seen:
                seen.add(dep['pkg'])
                stack.append(dep['pkg'])
    crates = [packages[i] for i in seen if packages[i]['source']]  # workspace crates excluded
    return sorted(crates, key=lambda p: (p['name'], p['version']))


def npm_packages():
    with io.open(os.path.join(ROOT, 'package.json'), encoding='utf-8') as f:
        deps = json.load(f).get('dependencies', {})
    out = []
    for name in sorted(deps):
        with io.open(os.path.join(ROOT, 'node_modules', *name.split('/'), 'package.json'), encoding='utf-8') as f:
            pkg = json.load(f)
        out.append((name, pkg.get('version', '?'), pkg.get('license', '?')))
    return out


def main():
    lines = [
        'Prospector — third-party notices',
        '=' * 32,
        '',
        'Prospector is free software (MIT licence). It includes the components',
        'below, each under its own licence. Full licence texts are available',
        'from each project (crates.io / npmjs.com).',
        'Components under MPL-2.0 are used unmodified; their source code is',
        'available on crates.io under the name and version listed below.',
        '',
        'UnRAR',
        '-----',
        'RAR archives are read with the UnRAR library by Alexander Roshal',
        '(through the Rust crates `unrar` and `unrar_sys`). Its licence follows',
        'in full; see also licenses/UnRAR-license.txt.',
        '',
    ]
    with io.open(os.path.join(ROOT, 'src-tauri', 'licenses', 'UnRAR-license.txt'), encoding='utf-8', errors='replace') as f:
        lines += [line.rstrip() for line in f.read().splitlines()]
    lines += [
        '',
        'Fonts and icons',
        '---------------',
        'Fonts (SIL Open Font License 1.1, bundled through @fontsource): Patrick Hand,',
        'Patrick Hand SC, Courier Prime, Baloo Bhaijaan 2, Tiny5, Chakra Petch,',
        'JetBrains Mono, IBM Plex Sans, IBM Plex Sans Arabic, IBM Plex Mono.',
        'Doodles: Microsoft Fluent Emoji "Color" (MIT), see licenses/FluentEmoji-MIT.txt.',
        '',
        'npm packages (interface)',
        '------------------------',
    ]
    lines += [f'{name} {version} — {license}' for name, version, license in npm_packages()]
    crates = rust_crates()
    lines += ['', f'Rust crates ({len(crates)})', '-' * 20]
    lines += [f"{p['name']} {p['version']} — {p.get('license') or 'see ' + (p.get('license_file') or 'crate')}" for p in crates]
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with io.open(OUT, 'w', encoding='utf-8', newline='\n') as f:
        f.write('\n'.join(lines) + '\n')
    print(f'{OUT}: {len(crates)} crates')


if __name__ == '__main__':
    main()
