#!/usr/bin/env python3
"""Regression checks for advisory scope/deadline and private source exclusions."""
import datetime
import hashlib
import json
import subprocess
import tempfile
from types import SimpleNamespace
from unittest import mock
import zipfile
import importlib.util
from pathlib import Path
import sys
import unittest

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent.parent


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / (name + '.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


deps = load('audit-dependencies')
source = load('audit-release')
package = load('package-source')


class ReleasePolicyTests(unittest.TestCase):
    def metadata(self):
        versions = [('ttf-parser', '0.25.1'), ('owned_ttf_parser', '0.25.1'),
                    ('ab_glyph', '0.2.32'), ('epaint', '0.32.3'),
                    ('egui', '0.32.3'), ('eframe', '0.32.3')]
        return {'packages': [{'id': n, 'name': n, 'version': v} for n, v in versions],
                'resolve': {'nodes': [{'id': 'owned_ttf_parser', 'dependencies': ['ttf-parser']}]}}

    def test_approved_scope_passes_only_before_review_deadline(self):
        deps.verify_advisory_scope(self.metadata(), datetime.date(2027, 1, 27))
        with self.assertRaisesRegex(ValueError, 'expired'):
            deps.verify_advisory_scope(self.metadata(), datetime.date(2027, 1, 28))

    def test_different_parser_version_requires_review(self):
        metadata = self.metadata()
        metadata['packages'][0]['version'] = '0.25.2'
        with self.assertRaisesRegex(ValueError, 'scope changed'):
            deps.verify_advisory_scope(metadata, datetime.date(2026, 10, 28))

    def test_new_direct_font_parser_dependency_requires_review(self):
        metadata = self.metadata()
        metadata['resolve']['nodes'].append({'id': 'egui', 'dependencies': ['ttf-parser']})
        with self.assertRaisesRegex(ValueError, 'unapproved direct dependers'):
            deps.verify_advisory_scope(metadata, datetime.date(2026, 10, 28))

    def test_dotfiles_are_public_but_generated_data_and_mac_metadata_are_excluded(self):
        for name in ['.github/workflows/rust.yml', '.gitignore', '.gitattributes']:
            self.assertFalse(source.excluded(Path(name)), name)
        for name in ['target/audit.log', 'dist/BudgetTinker.app/budgettinker',
                     '.env', 'docs/budget.sqlite3', 'docs/budget.budgettinker-wal',
                     'docs/__MACOSX/icon', 'assets/._app-icon.png', 'docs/.DS_Store']:
            self.assertTrue(source.excluded(Path(name)), name)


    def test_notice_bytes_survive_windows_autocrlf_checkout(self):
        # A Git checkout with core.autocrlf=true must not change hashed legal notices.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / '.gitattributes').write_bytes((ROOT / '.gitattributes').read_bytes())
            rel = Path('docs/third-party/ab_glyph-0.2.32/LICENSE')
            notice = root / rel
            notice.parent.mkdir(parents=True)
            original = b'Original license line 1\\nOriginal license line 2\\n'
            notice.write_bytes(original)
            subprocess.run(['git', 'init', '-q'], cwd=root, check=True, capture_output=True)
            subprocess.run(
                ['git', '-c', 'core.autocrlf=true', 'add', '.gitattributes', rel.as_posix()],
                cwd=root, check=True, capture_output=True,
            )
            staged = subprocess.check_output(['git', 'show', ':' + rel.as_posix()], cwd=root)
            self.assertEqual(staged, original)
            notice.unlink()
            subprocess.run(
                ['git', '-c', 'core.autocrlf=true', 'checkout-index', '-f', '--', rel.as_posix()],
                cwd=root, check=True, capture_output=True,
            )
            self.assertEqual(notice.read_bytes(), original)


class SourceArchiveTests(unittest.TestCase):
    def test_git_checkout_audit_and_reproducible_archive(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            files = {
                '.git/config': 'private git metadata',
                'docs/.git/objects/object': 'nested git metadata',
                '.github/workflows/rust.yml': 'on:\n  workflow_dispatch:\n',
                '.gitignore': '/target/\n',
                '.gitattributes': 'docs/third-party/** -text\n',
                'Cargo.toml': '[package]\n',
                'LICENSE-MIT': 'MIT License\n',
                'LICENSE-APACHE': 'Apache License\n',
                'scripts/stage-notices.py': 'def verify_inventory(): return 0\n',
                'docs/third-party/inventory.json': '[]',
                'docs/example.sqlite3': 'private data',
                'docs/__pycache__/cache': 'cache',
                'docs/.DS_Store': 'metadata',
                'target/generated': 'build',
                'dist/generated': 'build',
            }
            for name, content in files.items():
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(content)
            metadata = {'packages': [{'manifest_path': str(root / 'Cargo.toml'),
                                      'license': 'MIT OR Apache-2.0', 'publish': []}]}
            cargo_files = [name for name in files if name not in {
                '.git/config', 'docs/.git/objects/object', 'docs/example.sqlite3',
                'docs/__pycache__/cache', 'docs/.DS_Store', 'target/generated', 'dist/generated'}]
            result = SimpleNamespace(stdout='\n'.join(cargo_files + ['.cargo_vcs_info.json', 'Cargo.toml.orig']), returncode=0)
            with mock.patch.object(source, 'ROOT', root), \
                 mock.patch.object(source.subprocess, 'check_output', return_value=json.dumps(metadata)), \
                 mock.patch.object(source.subprocess, 'run', return_value=result):
                self.assertFalse(source.main())
            report = json.loads((root / 'target/release-audit/source-candidates.json').read_text())
            names = {record['path'] for record in report['candidates']}
            self.assertEqual(names, set(cargo_files))
            with mock.patch.object(package, 'ROOT', root), \
                 mock.patch.object(package.subprocess, 'run'):
                package.main()
                archive_path = root / 'dist/BudgetTinker-source.zip'
                first = archive_path.read_bytes()
                with zipfile.ZipFile(archive_path) as archive:
                    self.assertEqual(set(archive.namelist()), {'budgettinker/' + name for name in names})
                    for record in report['candidates']:
                        self.assertEqual(hashlib.sha256(archive.read('budgettinker/' + record['path'])).hexdigest(), record['sha256'])
                package.main()
                self.assertEqual(first, archive_path.read_bytes())
                (root / '.gitignore').write_text('changed since audit')
                with self.assertRaisesRegex(ValueError, 'Source changed since audit'):
                    package.main()
                self.assertEqual(first, archive_path.read_bytes())


if __name__ == '__main__':
    unittest.main()
