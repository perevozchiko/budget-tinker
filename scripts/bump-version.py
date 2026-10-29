#!/usr/bin/env python3
"""Update the BudgetTinker package version in Cargo.toml and Cargo.lock together."""

import re
import sys
from pathlib import Path

from importlib.util import module_from_spec, spec_from_file_location

ROOT = Path(__file__).resolve().parent.parent
spec = spec_from_file_location("release_version", ROOT / "scripts/release-version.py")
release_version = module_from_spec(spec)
spec.loader.exec_module(release_version)


def update_version(root, new_version):
    if not release_version.SEMVER.fullmatch(new_version):
        raise ValueError(f"Invalid version: {new_version}")

    old, locked = release_version.package_versions(root)
    if old != locked:
        raise ValueError(f"Existing Cargo.toml/Cargo.lock mismatch: {old} != {locked}")

    manifest_path = root / "Cargo.toml"
    lock_path = root / "Cargo.lock"
    manifest = manifest_path.read_text(encoding="utf-8")
    lock = lock_path.read_text(encoding="utf-8")
    manifest_pattern = re.compile(r'(?m)^(version\s*=\s*)"' + re.escape(old) + r'"$')
    lock_pattern = re.compile(
        r'(?m)(^\[\[package\]\]\nname = "budgettinker"\nversion = ")' +
        re.escape(old) + r'("$)'
    )
    new_manifest, n_manifest = manifest_pattern.subn(lambda m: m.group(1) + '"' + new_version + '"', manifest)
    new_lock, n_lock = lock_pattern.subn(lambda m: m.group(1) + new_version + m.group(2), lock)
    if n_manifest != 1 or n_lock != 1:
        raise ValueError("Could not uniquely locate the two package version fields")
    if old == new_version:
        return False

    manifest_path.write_text(new_manifest, encoding="utf-8")
    lock_path.write_text(new_lock, encoding="utf-8")
    return True


def main():
    if len(sys.argv) != 2:
        raise SystemExit("Usage: python3 scripts/bump-version.py 0.1.0-alpha.3")
    changed = update_version(ROOT, sys.argv[1])
    print(f"Version {'updated to' if changed else 'already'} {sys.argv[1]}")
    print(f"After merging into master and passing CI: git tag v{sys.argv[1]} && git push origin v{sys.argv[1]}")


if __name__ == "__main__":
    main()
