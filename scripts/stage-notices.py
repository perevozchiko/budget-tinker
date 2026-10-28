#!/usr/bin/env python3
"""Stage verified legal material identically for each platform; no publication."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil

ROOT = Path(__file__).resolve().parent.parent
ROOT_NOTICES = (
    'LICENSE-MIT', 'LICENSE-APACHE', 'LICENSE-STATUS.md', 'NOTICE', 'ATTRIBUTION.md',
)


def verify_inventory():
    folder = ROOT / 'docs/third-party'
    inventory = json.loads((folder / 'inventory.json').read_text())
    identities = set()
    for row in inventory:
        identity = (row['name'], row['version'])
        if identity in identities or not row['notices']:
            raise ValueError(f'Duplicate or missing notice: {identity}')
        identities.add(identity)
        base = folder / f"{row['name']}-{row['version']}"
        for name in row['notices']:
            path = base / name
            if path.is_symlink() or path.resolve().parent != base.resolve():
                raise ValueError(f'Invalid notice path: {path}')
            actual = hashlib.sha256(path.read_bytes()).hexdigest()
            if actual != row['notice_sha256'][name]:
                raise ValueError(f'Notice hash mismatch: {path}')
    return len(inventory)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('destination', type=Path, help='Notices directory beneath target/ or dist/')
    args = parser.parse_args()
    destination = args.destination.resolve()
    if destination.name != 'Notices' or not any(
        destination.is_relative_to(ROOT / parent) for parent in ('target', 'dist')
    ):
        parser.error('Destination must be a Notices directory inside project target/ or dist/')
    count = verify_inventory()
    for name in ROOT_NOTICES:
        if not (ROOT / name).is_file():
            raise ValueError(f'Required legal file missing: {name}')
    if destination.exists():
        shutil.rmtree(destination)
    destination.mkdir(parents=True)
    for name in ROOT_NOTICES:
        shutil.copy2(ROOT / name, destination / name)
    shutil.copytree(ROOT / 'docs/third-party', destination / 'third-party')
    shutil.copy2(ROOT / 'assets/LICENSE.md', destination / 'BRANDING-LICENSE.md')
    shutil.copy2(ROOT / 'docs/ICON-EDIT.md', destination / 'ICON-EDIT.md')
    # Cargo archives can timestamp license files before ZIP's 1980 lower bound.
    # Normalize staged copies only; do not change source bytes or source mtimes.
    for path in sorted(destination.rglob('*')):
        os.utime(path, (946684800, 946684800))  # 2000-01-01 UTC, valid in every ZIP timezone.
    os.utime(destination, (946684800, 946684800))
    print(f'Staged {count} dependency records and project/branding license status in {destination.relative_to(ROOT)}')


if __name__ == '__main__':
    main()
