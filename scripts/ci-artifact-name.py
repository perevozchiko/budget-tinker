#!/usr/bin/env python3
"""Create platform-specific distribution archive names from the Cargo package version."""

import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLATFORMS = {"macos-arm64", "windows-x64", "linux-x64"}


def artifact_stem(version, platform):
    if platform not in PLATFORMS:
        raise ValueError(f"Unsupported platform: {platform}")
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z][0-9A-Za-z.-]*)?(?:\+[0-9A-Za-z.-]+)?", version):
        raise ValueError(f"Invalid Cargo version: {version}")
    return f"BudgetTinker-{version}-{platform}"


def main():
    if len(sys.argv) != 2:
        raise SystemExit("Usage: ci-artifact-name.py macos-arm64|windows-x64|linux-x64")

    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
        cwd=ROOT,
        text=True,
    ))
    version = next(p["version"] for p in metadata["packages"] if p["name"] == "budgettinker")
    stem = artifact_stem(version, sys.argv[1])
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"stem={stem}\n")
    print(f"Artifact: {stem}")


if __name__ == "__main__":
    main()
