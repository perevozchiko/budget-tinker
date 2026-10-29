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
artifact_names = load('ci-artifact-name')
release_version = load('release-version')
bump_version = load('bump-version')
next_release = load('next-release-version')


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

    def test_ci_fetches_rustsec_before_offline_audit(self):
        workflow = (ROOT / '.github/workflows/rust.yml').read_text()
        self.assertIn('repository: RustSec/advisory-db', workflow)
        self.assertIn('ref: main', workflow)
        self.assertIn('path: target/release-audit/advisory-dbs/advisory-db-3157b0e258782691', workflow)
        self.assertIn('persist-credentials: false', workflow)
        self.assertIn('python3 scripts/audit-dependencies.py --offline', workflow)

    def test_ci_auto_builds_only_master_and_supports_manual_platforms(self):
        workflow = (ROOT / '.github/workflows/rust.yml').read_text()
        self.assertRegex(
            workflow,
            r"(?m)^on:\n  push:\n    branches:\n      - master\n    tags:\n      - 'v\*'\n  workflow_dispatch:\n",
        )
        self.assertNotIn('  pull_request:', workflow)
        self.assertIn('        default: all', workflow)
        for platform in ('all', 'macos', 'linux', 'windows'):
            self.assertIn(f'          - {platform}\n', workflow)
        for platform in ('macos', 'linux', 'windows'):
            self.assertIn(
                "if: ${{ github.event_name == 'push' || inputs.platform == 'all' || inputs.platform == '" +
                platform + "' }}",
                workflow,
            )

    def test_ci_archive_naming_and_node24_actions(self):
        workflow = (ROOT / '.github/workflows/rust.yml').read_text()
        self.assertNotIn('actions/checkout@v4', workflow)
        self.assertNotIn('actions/upload-artifact@v4', workflow)
        self.assertEqual(workflow.count('actions/checkout@v6'), 6)
        self.assertEqual(workflow.count('actions/upload-artifact@v6'), 4)
        self.assertEqual(workflow.count('scripts/ci-artifact-name.py'), 3)
        self.assertEqual(workflow.count('name: ${{ steps.package.outputs.stem }}'), 3)
        self.assertEqual(workflow.count('path: ${{ steps.package.outputs.stem }}.zip'), 2)
        self.assertIn('path: ${{ steps.package.outputs.stem }}.tar.gz', workflow)
        for platform, archive in (
            ('macos-arm64', 'BudgetTinker-0.1.0-alpha.2-macos-arm64.zip'),
            ('windows-x64', 'BudgetTinker-0.1.0-alpha.2-windows-x64.zip'),
            ('linux-x64', 'BudgetTinker-0.1.0-alpha.2-linux-x64.tar.gz'),
        ):
            stem = artifact_names.artifact_stem('0.1.0-alpha.2', platform)
            self.assertEqual(stem, archive.removesuffix('.tar.gz').removesuffix('.zip'))
            self.assertIn(f'scripts/ci-artifact-name.py {platform}', workflow)
        self.assertIn('test "$(uname -m)" = "arm64"', workflow)
        self.assertIn('test "$(uname -m)" = "x86_64"', workflow)
        self.assertIn('PROCESSOR_ARCHITECTURE -ne "AMD64"', workflow)
        with self.assertRaises(ValueError):
            artifact_names.artifact_stem('invalid/version', 'macos-arm64')
        with self.assertRaises(ValueError):
            artifact_names.artifact_stem('0.1.0-alpha.2', 'unknown')

    def test_tag_release_is_gated_by_successful_platform_builds(self):
        workflow = (ROOT / '.github/workflows/rust.yml').read_text()
        self.assertIn('needs: [validate, build, linux, windows]', workflow)
        self.assertIn("startsWith(github.ref, 'refs/tags/v')", workflow)
        self.assertIn('actions/download-artifact@v8', workflow)
        self.assertIn('git merge-base --is-ancestor HEAD origin/master', workflow)
        self.assertIn('python3 scripts/release-version.py', workflow)
        self.assertIn('gh release create "$RELEASE_TAG"', workflow)
        self.assertIn('gh release upload "$RELEASE_TAG"', workflow)
        self.assertIn('IS_PRERELEASE:', workflow)
        self.assertEqual(
            release_version.check_tag('v0.1.0-alpha.2', '0.1.0-alpha.2', '0.1.0-alpha.2'),
            'true',
        )
        self.assertEqual(
            release_version.check_tag('v1.2.3', '1.2.3', '1.2.3'),
            'false',
        )
        with self.assertRaisesRegex(ValueError, 'must match'):
            release_version.check_tag('v0.1.0-alpha.3', '0.1.0-alpha.2', '0.1.0-alpha.2')
        with self.assertRaisesRegex(ValueError, 'differs'):
            release_version.check_tag('v0.1.0-alpha.2', '0.1.0-alpha.2', '0.1.0-alpha.1')

    def test_bump_version_updates_both_cargo_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'Cargo.toml').write_text(
                '[package]\nname = "budgettinker"\nversion = "0.1.0-alpha.2"\n\n[dependencies]\n'
                'serde = "1"\n', encoding='utf-8'
            )
            (root / 'Cargo.lock').write_text(
                'version = 4\n\n[[package]]\nname = "budgettinker"\n'
                'version = "0.1.0-alpha.2"\n', encoding='utf-8'
            )
            self.assertTrue(bump_version.update_version(root, '0.1.0-alpha.3'))
            self.assertEqual(
                release_version.package_versions(root),
                ('0.1.0-alpha.3', '0.1.0-alpha.3'),
            )
            self.assertFalse(bump_version.update_version(root, '0.1.0-alpha.3'))
            with self.assertRaises(ValueError):
                bump_version.update_version(root, 'invalid version')

    def test_generated_release_notes_categories(self):
        config = (ROOT / '.github/release.yml').read_text(encoding='utf-8')
        self.assertTrue(config.startswith('changelog:\n') or 'changelog:\n' in config)
        self.assertIn('  exclude:\n', config)
        self.assertIn('      - skip-changelog\n', config)
        self.assertIn('      - no-release-notes\n', config)
        for category in (
            'Breaking Changes', 'New Features', 'Bug Fixes', 'Improvements',
            'Documentation', 'Maintenance & CI', 'Other Changes',
        ):
            self.assertIn(category, config)
        for label in ('breaking-change', 'feature', 'enhancement', 'bug',
                      'improvement', 'performance', 'documentation', 'ci',
                      'dependencies'):
            self.assertIn(f'        - {label}\n', config)
        self.assertIn('    - title: "Other Changes"\n      labels:\n        - "*"\n', config)
        workflow = (ROOT / '.github/workflows/rust.yml').read_text()
        self.assertIn('--generate-notes', workflow)

    def test_one_click_release_version_selection(self):
        tags = ['v0.1.0-alpha.1', 'v0.1.0-alpha.2', 'v0.1.0-alpha.3']
        choose = next_release.next_version
        self.assertEqual(choose('0.1.0-alpha.3', 'current', 'alpha', tags), '0.1.0-alpha.4')
        self.assertEqual(choose('0.1.0-alpha.3', 'current', 'beta', tags), '0.1.0-beta.1')
        self.assertEqual(choose('0.1.0-alpha.3', 'current', 'rc', tags), '0.1.0-rc.1')
        self.assertEqual(choose('0.1.0-alpha.3', 'current', 'stable', tags), '0.1.0')
        self.assertEqual(choose('0.1.0-alpha.3', 'patch', 'alpha', tags), '0.1.1-alpha.1')
        self.assertEqual(choose('0.1.0-alpha.3', 'minor', 'alpha', tags), '0.2.0-alpha.1')
        self.assertEqual(choose('0.1.0-alpha.3', 'major', 'stable', tags), '1.0.0')
        self.assertEqual(choose('0.1.0-beta.1', 'current', 'beta', tags + ['v0.1.0-beta.1']), '0.1.0-beta.2')
        self.assertEqual(choose('0.1.0-rc.1', 'current', 'rc', tags + ['v0.1.0-rc.1']), '0.1.0-rc.2')
        self.assertEqual(choose('0.1.1-alpha.1', 'current', 'alpha', tags), '0.1.1-alpha.1')
        with self.assertRaisesRegex(ValueError, 'backwards'):
            choose('0.1.0-beta.1', 'current', 'alpha', tags + ['v0.1.0-beta.1'])
        with self.assertRaisesRegex(ValueError, 'already stable'):
            choose('0.1.0', 'current', 'stable', tags + ['v0.1.0'])
        with self.assertRaisesRegex(ValueError, 'use Patch'):
            choose('0.1.0', 'current', 'alpha', tags)
        with self.assertRaises(ValueError):
            choose('0.1.0-alpha.3', 'patch', 'unknown', tags)

    def test_manual_one_click_release_workflow(self):
        workflow = (ROOT / '.github/workflows/release.yml').read_text()
        rust = (ROOT / '.github/workflows/rust.yml').read_text()
        self.assertIn('  workflow_dispatch:', workflow)
        self.assertIn('  workflow_call:', rust)
        self.assertIn('      release_ref:', rust)
        self.assertIn('ref: ${{ inputs.release_ref || github.sha }}', rust)
        self.assertIn("github.ref == 'refs/heads/master'", workflow)
        for option in ('current', 'patch', 'minor', 'major', 'alpha', 'beta', 'rc', 'stable'):
            self.assertIn(f'          - {option}\n', workflow)
        self.assertIn('python3 scripts/next-release-version.py "$BUMP" "$STAGE"', workflow)
        self.assertIn('python3 scripts/bump-version.py "$VERSION"', workflow)
        self.assertIn('git push origin HEAD:refs/heads/master', workflow)
        self.assertIn('uses: ./.github/workflows/rust.yml', workflow)
        self.assertIn('needs: [prepare, desktop]', workflow)
        self.assertIn('actions/download-artifact@v8', workflow)
        self.assertIn('git push origin "refs/tags/$RELEASE_TAG"', workflow)
        self.assertIn('--generate-notes', workflow)
        self.assertIn('gh release create "$RELEASE_TAG"', workflow)

    def test_dotfiles_are_public_but_generated_data_and_mac_metadata_are_excluded(self):
        for name in ['.github/workflows/rust.yml', '.github/workflows/release.yml', '.github/release.yml', '.gitignore', '.gitattributes']:
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
            original = b'Original license line 1\nOriginal license line 2\n'
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
