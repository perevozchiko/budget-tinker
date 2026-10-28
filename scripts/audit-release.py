#!/usr/bin/env python3
"""Verify public source candidates without creating an archive."""
import hashlib
import json
from pathlib import Path
import re
import sys
import subprocess

import importlib.util

ROOT = Path(__file__).resolve().parent.parent
DIRECTORIES = {'src', 'tests', 'examples', 'locales', 'assets', 'scripts', 'docs', '.github'}
ROOT_FILES = {
    'Cargo.toml', 'Cargo.lock', 'build.rs', '.gitignore', '.gitattributes', 'deny.toml', 'AGENTS.md',
    'ATTRIBUTION.md', 'CONTRIBUTING.md', 'LICENSE-MIT', 'LICENSE-APACHE',
    'LICENSE-STATUS.md', 'NOTICE', 'README.md', 'SECURITY.md',
}
PRIVATE_PARTS = {'target', 'dist', '.reference', '.git', '__pycache__', '.venv', 'node_modules', '__MACOSX'}
SECRET_PATTERNS = {
    'private key': rb'-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----',
    'GitHub credential': rb'gh[pousr]_[A-Za-z0-9]{30,}',
    'GitHub fine-grained credential': rb'github_pat_[A-Za-z0-9_]{30,}',
    'AWS access key': rb'AKIA[A-Z0-9]{16}',
    'OpenAI credential': rb'sk-(?:proj-)?[A-Za-z0-9_-]{30,}',
}


def excluded(path):
    if PRIVATE_PARTS.intersection(path.parts):
        return True
    name = path.name.lower()
    return (
        name == '.ds_store' or name.startswith(('._', '.env'))
        or name.endswith(('.sqlite', '.sqlite3', '.budgettinker', '.db', '.pem', '.key', '.p12', '.pfx', '.log', '.zip', '.dmg', '.tar.gz', '.preferences.json'))
        or re.search(r'\.(?:sqlite3?|db|budgettinker)-(?:wal|shm|journal)$', name)
        or name in {'credentials.json', 'secrets.json'}
        or any(part.endswith(('.app', '.y' + 'nab4')) for part in path.parts)
    )


def main():
    errors = []
    spec = importlib.util.spec_from_file_location('stage_notices', ROOT / 'scripts/stage-notices.py')
    stage = importlib.util.module_from_spec(spec)
    sys.dont_write_bytecode = True
    spec.loader.exec_module(stage)
    count = stage.verify_inventory()
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--locked', '--format-version', '1'], cwd=ROOT,
    ))
    locked = {(p['name'], p['version']) for p in metadata['packages'] if p.get('source')}
    rows = json.loads((ROOT / 'docs/third-party/inventory.json').read_text())
    inventory = {(p['name'], p['version']) for p in rows}
    if locked != inventory:
        errors.append(f'Lock/inventory mismatch: {locked ^ inventory}')
    project = next(p for p in metadata['packages'] if Path(p['manifest_path']).resolve() == ROOT / 'Cargo.toml')
    if project.get('license') != 'MIT OR Apache-2.0' or project.get('publish') != []:
        errors.append('Expected the active dual license and disabled crate publishing')
    for name, heading in [('LICENSE-MIT', 'MIT License'), ('LICENSE-APACHE', 'Apache License')]:
        if not (ROOT / name).read_text().lstrip().startswith(heading):
            errors.append(f'Missing active standard license text: {name}')
    workflow = (ROOT / '.github/workflows/rust.yml').read_text()
    if not re.search(r'^on:\n  workflow_dispatch:\s*\n', workflow, re.M) or re.search(r'^  (?:push|pull_request|schedule):', workflow, re.M):
        errors.append('CI must have workflow_dispatch as its only trigger')
    if any(not (ROOT / name).is_file() for name in ('.gitignore', '.gitattributes', '.github/workflows/rust.yml')):
        errors.append('Required .gitignore, .gitattributes or workflow is missing from the actual source tree')
    candidates = []
    excluded_files = []
    # Walk only allowed roots; do not descend into tool caches or generated application bundles.
    paths = [ROOT / name for name in ROOT_FILES]
    for name in DIRECTORIES:
        paths.extend((ROOT / name).rglob('*'))
    for path in sorted(set(paths)):
        rel = path.relative_to(ROOT)
        if excluded(rel):
            if path.is_file():
                excluded_files.append(str(rel))
            continue
        if path.is_symlink():
            errors.append(f'Source symlink requires explicit review: {rel}')
            continue
        if not path.is_file():
            continue
        data = path.read_bytes()
        if path.stat().st_size > 2_000_000:
            errors.append(f'Unexpected large candidate: {rel}')
        for label, pattern in SECRET_PATTERNS.items():
            if re.search(pattern, data):
                errors.append(f'{label} pattern in {rel}; do not print its contents')
        # Upstream author addresses and copyrights are retained legal notices, not user data.
        if 'third-party' not in rel.parts and path.suffix not in {'.png', '.ico'}:
            try:
                text = data.decode('utf-8')
                # This file defines literal detection signatures, not private data/claims.
                is_scanner = rel == Path('scripts/audit-release.py')
                if not is_scanner and re.search(r'/Users/[^/\s]+/', text):
                    errors.append(f'Personal home path in public candidate: {rel}')
                if re.search(r'\b[\w.+-]+@[\w.-]+\.[A-Za-z]{2,}\b', text):
                    errors.append(f'Email address outside upstream notices requires review: {rel}')
                if not is_scanner and re.search(r'(?i)\b(?:clean-room (?:implementation|reimplementation)|guaranteed unique|registered trademark of BudgetTinker)\b', text):
                    errors.append(f'Unsupported ownership/clean-room claim requires review: {rel}')
            except UnicodeDecodeError:
                errors.append(f'Unexpected binary: {rel}')
        candidates.append({'path': str(rel), 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
    package_result = subprocess.run(
        ['cargo', 'package', '--list', '--allow-dirty', '--locked'], cwd=ROOT,
        text=True, capture_output=True,
    )
    package_files = set(package_result.stdout.splitlines())
    # Cargo generates these metadata files; they are not source archive candidates.
    extra_package_files = package_files - {r['path'] for r in candidates} - {'Cargo.toml.orig', '.cargo_vcs_info.json'}
    required_hidden = {'.gitignore', '.gitattributes', '.github/workflows/rust.yml'}
    if not required_hidden.issubset(package_files):
        errors.append('Cargo source list omitted required hidden configuration files')
    if package_result.returncode or extra_package_files:
        errors.append(f'Cargo source list failed or contains non-candidates: {sorted(extra_package_files)}')
    report = {
        'cargo_package_entries': len(package_files),
        'dependency_records': count, 'candidates': candidates,
        'excluded_present_files': excluded_files,
        'excluded_roots': sorted(PRIVATE_PARTS), 'errors': errors,
    }
    output = ROOT / 'target/release-audit/source-candidates.json'
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + '\n')
    print(f'{len(candidates)} source candidates; {count} dependency records; {len(errors)} errors')
    for error in errors:
        print(error, file=sys.stderr)
    return bool(errors)


if __name__ == '__main__':
    sys.exit(main())
