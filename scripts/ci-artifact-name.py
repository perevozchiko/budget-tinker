#!/usr/bin/env python3
"""Create reproducible version-and-revision names for CI desktop archives."""

import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PLATFORMS = {"macOS", "linux-x86_64", "windows-x86_64"}


def artifact_stem(version, commit, platform):
    if platform not in PLATFORMS:
        raise ValueError(f"Unsupported platform: {platform}")
    if not re.fullmatch(r"[0-9A-Za-z.+-]+", version):
        raise ValueError(f"Invalid Cargo version: {version}")
    if not re.fullmatch(r"[0-9a-fA-F]{40}", commit):
        raise ValueError("GITHUB_SHA must be a full 40-character Git commit SHA")
    return f"BudgetTinker-v{version}-{commit[:7]}-{platform}"


def main():
    if len(sys.argv) != 2:
        raise SystemExit("Usage: ci-artifact-name.py macOS|linux-x86_64|windows-x86_64")

    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
        cwd=ROOT,
        text=True,
    ))
    version = next(p["version"] for p in metadata["packages"] if p["name"] == "budgettinker")
    stem = artifact_stem(version, os.environ["GITHUB_SHA"], sys.argv[1])
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"stem={stem}\n")
    print(f"Artifact: {stem}")


if __name__ == "__main__":
    main()
