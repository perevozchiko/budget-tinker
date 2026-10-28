#!/usr/bin/env python3
"""Run cargo-deny on the active graph and every crate in the locked inventory."""
import argparse
import datetime
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / 'target/release-audit'


ADVISORY_REVIEW_DEADLINE = datetime.date(2027, 1, 28)


def verify_advisory_scope(metadata, today=None):
    """Fail closed when the approved font-stack scope changes or its review is due."""
    today = today or datetime.datetime.now(datetime.timezone.utc).date()
    if today >= ADVISORY_REVIEW_DEADLINE:
        raise ValueError('RUSTSEC-2026-0192 exception requires review by 2027-01-28; approval has expired')
    packages = {p['id']: (p['name'], p['version']) for p in metadata['packages']}
    expected = {
        'ttf-parser': '0.25.1', 'owned_ttf_parser': '0.25.1', 'ab_glyph': '0.2.32',
        'epaint': '0.32.3', 'egui': '0.32.3', 'eframe': '0.32.3',
    }
    for name, version in expected.items():
        versions = {v for n, v in packages.values() if n == name}
        if versions != {version}:
            raise ValueError(f'RUSTSEC-2026-0192 approved dependency scope changed: {name} {versions}')
    ttf_ids = {identity for identity, (name, _) in packages.items() if name == 'ttf-parser'}
    parents = {
        packages[node['id']] for node in metadata['resolve']['nodes']
        if ttf_ids.intersection(node['dependencies'])
    }
    if parents != {('owned_ttf_parser', '0.25.1')}:
        raise ValueError(f'RUSTSEC-2026-0192 has unapproved direct dependers: {parents}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--offline', action='store_true', help='Reuse the already fetched advisory database')
    args = parser.parse_args()
    OUTPUT.mkdir(parents=True, exist_ok=True)
    results_path = OUTPUT / "dependency-results.json"
    results_path.unlink(missing_ok=True)  # Never retain a stale passing report after a policy failure.
    deny = ROOT / 'target/audit-tools/bin/cargo-deny'
    if sys.platform == 'win32':
        deny = deny.with_suffix('.exe')
    if not deny.is_file():
        parser.error('Install cargo-deny 0.20.2 locally; see docs/DEPENDENCY-AUDIT.md')
    actual_version = subprocess.check_output([str(deny), '--version'], text=True).strip()
    if actual_version != 'cargo-deny 0.20.2':
        parser.error(f'Expected cargo-deny 0.20.2, got {actual_version}')
    raw = subprocess.check_output(
        ['cargo', 'metadata', '--locked', '--all-features', '--format-version', '1'], cwd=ROOT,
    )
    metadata = json.loads(raw)
    try:
        verify_advisory_scope(metadata)
    except ValueError as error:
        results_path.write_text(json.dumps({
            'tool': actual_version,
            'checked_at_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
            'policy_error': str(error), 'checks': {},
        }, indent=2) + '\n')
        parser.error(str(error))
    (OUTPUT / 'metadata.json').write_bytes(raw)
    expected = {(r['name'], r['version']) for r in json.loads((ROOT / 'docs/third-party/inventory.json').read_text())}
    observed = {(p['name'], p['version']) for p in metadata['packages'] if p.get('source')}
    if expected != observed:
        parser.error(f'Inventory differs from locked metadata: {expected ^ observed}')
    # A separate exported metadata view treats every locked package as a root. This checks
    # licenses/advisories in inactive optional dependencies too. It does not edit Cargo.toml,
    # Cargo.lock, real workspace membership or activated application features.
    inventory = dict(metadata)
    inventory['workspace_members'] = [p['id'] for p in metadata['packages']]
    inventory['workspace_default_members'] = inventory['workspace_members']
    inventory_path = OUTPUT / 'inventory-metadata.json'
    inventory_path.write_text(json.dumps(inventory) + '\n')
    results = {'tool': actual_version, 'checked_at_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'inventory_crates': len(observed), 'exception_review_deadline': ADVISORY_REVIEW_DEADLINE.isoformat(), 'exception_scope_checked': True, 'checks': {}}
    for name, metadata_args in [
        ('active', []),
        ('inventory', ['--metadata-path', str(inventory_path), '--workspace']),
    ]:
        for check in ('licenses', 'advisories'):
            command = [str(deny), *metadata_args, '--locked']
            # Fetch once for the active advisory check; inventory reuses exactly that DB.
            if args.offline or name == 'inventory':
                command.append('--offline')
            command += ['check', check, '--show-stats']
            log = OUTPUT / f'deny-{name}-{check}.log'
            with log.open('w') as output:
                status = subprocess.run(command, cwd=ROOT, stdout=output, stderr=subprocess.STDOUT).returncode
            results['checks'][f'{name}-{check}'] = {'exit_code': status, 'log': str(log.relative_to(ROOT))}
            print(f'{name} {check}: exit {status}; {log.relative_to(ROOT)}')
    # Record exact advisory cache revision through read-only git commands. This is only
    # cargo-deny's upstream data cache, never a repository for the BudgetTinker project.
    results['advisory_databases'] = []
    for database in sorted((OUTPUT / 'advisory-dbs').glob('*/.git')):
        def read_git(*args):
            return subprocess.check_output(['git', '-C', str(database.parent), *args], text=True).strip()
        results['advisory_databases'].append({
            'source': read_git('remote', 'get-url', 'origin'),
            'revision': read_git('rev-parse', 'HEAD'),
            'commit_date': read_git('show', '-s', '--format=%cI', 'HEAD'),
        })
    results_path.write_text(json.dumps(results, indent=2) + '\n')
    return int(any(c['exit_code'] for c in results['checks'].values()))


if __name__ == '__main__':
    sys.exit(main())
