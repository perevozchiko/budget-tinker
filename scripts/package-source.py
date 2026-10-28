#!/usr/bin/env python3
"""Create a local review ZIP from verified source candidates, including dotfiles."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import zipfile

ROOT = Path(__file__).resolve().parent.parent


def main():
    # Recheck the actual tree and Cargo include list. Never use shell '*' expansion,
    # which drops .github and .gitignore. This does not create or publish a repository.
    subprocess.run([sys.executable, str(ROOT / 'scripts/audit-release.py')], cwd=ROOT, check=True)
    report = json.loads((ROOT / 'target/release-audit/source-candidates.json').read_text())
    if report['errors']:
        raise ValueError('Public source checks failed')
    records = {r['path']: r for r in report['candidates']}
    required = {'.github/workflows/rust.yml', '.gitignore'}
    if not required.issubset(records):
        raise ValueError(f'Required hidden files missing: {required - records.keys()}')
    destination = ROOT / 'dist/BudgetTinker-source.zip'
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_suffix('.zip.tmp')
    try:
        with zipfile.ZipFile(temporary, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
            for name, record in sorted(records.items()):
                source = ROOT / name
                if source.is_symlink():
                    raise ValueError(f'Source changed to a symlink: {name}')
                data = source.read_bytes()
                if hashlib.sha256(data).hexdigest() != record['sha256']:
                    raise ValueError(f'Source changed since audit: {name}')
                # Stable ZIP timestamps also support old crate-license mtimes before 1980.
                info = zipfile.ZipInfo('budgettinker/' + name, date_time=(2000, 1, 1, 0, 0, 0))
                info.compress_type = zipfile.ZIP_DEFLATED
                info.external_attr = (0o100755 if source.stat().st_mode & 0o111 else 0o100644) << 16
                archive.writestr(info, data)
        with zipfile.ZipFile(temporary) as archive:
            expected = {'budgettinker/' + name for name in records}
            if set(archive.namelist()) != expected:
                raise ValueError('Archive file list differs from verified source candidates')
            for name, record in records.items():
                if hashlib.sha256(archive.read('budgettinker/' + name)).hexdigest() != record['sha256']:
                    raise ValueError(f'Archive content mismatch: {name}')
        temporary.replace(destination)
    finally:
        temporary.unlink(missing_ok=True)
    print(f'Created {destination.relative_to(ROOT)} with {len(records)} verified files, including .github and .gitignore. Local archive with active MIT/Apache-2.0 licenses; not published.')


if __name__ == '__main__':
    main()
