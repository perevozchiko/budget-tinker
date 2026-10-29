#!/usr/bin/env python3
"""Validate a release tag against Cargo.toml and Cargo.lock before publication."""

import os
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SEMVER = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z][0-9A-Za-z.-]*)?(?:\+[0-9A-Za-z.-]+)?$")


def package_versions(root):
    manifest = (root / "Cargo.toml").read_text(encoding="utf-8")
    lock = (root / "Cargo.lock").read_text(encoding="utf-8")
    package = re.search(r"(?ms)^\[package\]\s*\n(.*?)(?=^\[|\Z)", manifest)
    if not package:
        raise ValueError("Missing [package] section in Cargo.toml")
    values = re.findall(r'^version\s*=\s*"([^"]+)"\s*$', package.group(1), re.MULTILINE)
    if len(values) != 1:
        raise ValueError("Expected one package version in Cargo.toml")
    locked = re.findall(r'(?m)^\[\[package\]\]\nname = "budgettinker"\nversion = "([^"]+)"$', lock)
    if len(locked) != 1:
        raise ValueError("Expected one budgettinker package in Cargo.lock")
    return values[0], locked[0]


def check_tag(tag, manifest_version, lock_version):
    if not SEMVER.fullmatch(manifest_version):
        raise ValueError(f"Invalid package version: {manifest_version}")
    if manifest_version != lock_version:
        raise ValueError(f"Cargo.toml version {manifest_version} differs from Cargo.lock {lock_version}")
    if tag != f"v{manifest_version}":
        raise ValueError(f"Tag {tag} must match Cargo.toml version v{manifest_version}")
    return "true" if "-" in manifest_version else "false"


def main():
    version, locked = package_versions(ROOT)
    prerelease = check_tag(os.environ["GITHUB_REF_NAME"], version, locked)
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"version={version}\nprerelease={prerelease}\n")
    print(f"Verified release: v{version} (prerelease={prerelease})")


if __name__ == "__main__":
    main()
