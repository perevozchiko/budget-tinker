#!/usr/bin/env python3
"""Calculate version for one-click releases from Cargo and existing Git tags."""

import os
import re
import subprocess
import sys
from pathlib import Path
from importlib.util import module_from_spec, spec_from_file_location

ROOT = Path(__file__).resolve().parent.parent
spec = spec_from_file_location("release_version", ROOT / "scripts/release-version.py")
release_version = module_from_spec(spec)
spec.loader.exec_module(release_version)

CURRENT_VERSION = re.compile(r"^(\d+)\.(\d+)\.(\d+)(?:-(alpha|beta|rc)\.(\d+))?$")
STAGES = ("alpha", "beta", "rc", "stable")
BUMPS = ("current", "patch", "minor", "major")


def next_version(current, bump, stage, tags):
    """Choose a previously unused version without going backwards within a series."""
    if bump not in BUMPS or stage not in STAGES:
        raise ValueError("Invalid release bump or stage")
    match = CURRENT_VERSION.fullmatch(current)
    if not match:
        raise ValueError(f"Unsupported Cargo version: {current}")
    major, minor, patch = map(int, match.group(1, 2, 3))
    current_stage, current_n = match.group(4), match.group(5)
    if bump == "patch":
        patch += 1
    elif bump == "minor":
        minor, patch = minor + 1, 0
    elif bump == "major":
        major, minor, patch = major + 1, 0, 0
    base = f"{major}.{minor}.{patch}"
    existing = set(tags)
    if f"v{base}" in existing:
        raise ValueError(f"v{base} is already stable; choose newer Patch, Minor or Major")

    highest = None
    stage_counts = {name: [] for name in STAGES[:-1]}
    for tag in existing:
        tag_match = re.fullmatch(r"v" + re.escape(base) + r"-(alpha|beta|rc)\.(\d+)", tag)
        if tag_match:
            name, number = tag_match.group(1), int(tag_match.group(2))
            stage_counts[name].append(number)
            if highest is None or STAGES.index(name) > STAGES.index(highest):
                highest = name

    if bump == "current" and current_stage:
        if highest is None or STAGES.index(current_stage) > STAGES.index(highest):
            highest = current_stage
    if bump == "current" and not current_stage and stage != "stable":
        raise ValueError("Current Cargo version is stable; use Patch, Minor or Major for a prerelease")
    if highest and STAGES.index(stage) < STAGES.index(highest):
        raise ValueError(f"Cannot move backwards from {highest} to {stage} in {base}")

    if stage == "stable":
        candidate = base
    else:
        numbers = stage_counts[stage]
        # Reuse a manually bumped but not yet tagged Cargo version.
        if bump == "current" and current_stage == stage and current_n:
            numbers.append(int(current_n) - 1)
        candidate = f"{base}-{stage}.{max(numbers, default=0) + 1}"
    if f"v{candidate}" in existing:
        raise ValueError(f"Release tag v{candidate} already exists")
    return candidate


def main():
    if len(sys.argv) != 3:
        raise SystemExit("Usage: python3 scripts/next-release-version.py current|patch|minor|major alpha|beta|rc|stable")
    current, locked = release_version.package_versions(ROOT)
    if current != locked:
        raise ValueError("Cargo.toml and Cargo.lock must agree")
    tags = subprocess.check_output(["git", "tag", "--list", "v*"], cwd=ROOT, text=True).splitlines()
    candidate = next_version(current, sys.argv[1], sys.argv[2], tags)
    result = {"version": candidate, "tag": "v" + candidate, "prerelease": str("-" in candidate).lower()}
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        for key, value in result.items():
            output.write(f"{key}={value}\n")
    print(f"Planned release: {result['tag']} (from {current}; {sys.argv[1]} / {sys.argv[2]})")


if __name__ == "__main__":
    main()
