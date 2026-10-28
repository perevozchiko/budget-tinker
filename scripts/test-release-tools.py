#!/usr/bin/env python3
"""Regression checks for advisory scope/deadline and private source exclusions."""
import datetime
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
        for name in ['.github/workflows/rust.yml', '.gitignore']:
            self.assertFalse(source.excluded(Path(name)), name)
        for name in ['target/audit.log', 'dist/BudgetTinker.app/budgettinker',
                     '.env', 'docs/budget.sqlite3', 'docs/budget.budgettinker-wal',
                     'docs/__MACOSX/icon', 'assets/._app-icon.png', 'docs/.DS_Store']:
            self.assertTrue(source.excluded(Path(name)), name)


if __name__ == '__main__':
    unittest.main()
