"""Select the target library for libsodium-sys 0.2.7's host-dependent MSVC build."""
import json
from pathlib import Path
import subprocess
import tomllib


TARGET = 'i686-pc-windows-msvc'


def configure(root):
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--locked', '--format-version', '1', '--filter-platform', TARGET],
        cwd=root, text=True,
    ))
    packages = [p for p in metadata['packages'] if p['name'] == 'libsodium-sys']
    if len(packages) != 1 or packages[0]['version'] != '0.2.7' or packages[0]['links'] != 'sodium':
        raise SystemExit('Review the libsodium-sys target configuration for the new dependency version')
    package = packages[0]
    if not (package.get('source') or '').startswith('registry+'):
        raise SystemExit('Expected the locked registry package for libsodium-sys')
    library = Path(package['manifest_path']).parent / 'msvc/Win32/Release/v142'
    if not (library / 'libsodium.lib').is_file():
        raise SystemExit('The locked libsodium-sys package is missing its x86 release library')
    expected = {'rustc-link-lib': ['static=libsodium'], 'rustc-link-search': [library.as_posix()]}
    path = root / '.cargo/config.toml'
    source = path.read_text(encoding='utf-8') if path.exists() else ''
    current = tomllib.loads(source).get('target', {}).get(TARGET, {}).get('sodium')
    if current is not None and current != expected:
        raise SystemExit('Refusing to replace an existing libsodium target configuration')
    if current is None:
        path.parent.mkdir(exist_ok=True)
        source += f'\n[target.{TARGET}.sodium]\n'
        source += ''.join(f'{key} = {json.dumps(value)}\n' for key, value in expected.items())
        path.write_text(source, encoding='utf-8', newline='\n')
    print('Configured the locked libsodium x86 release library for the i686 Cargo target')


if __name__ == '__main__':
    configure(Path(__file__).resolve().parents[1])
