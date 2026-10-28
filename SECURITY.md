# Security policy

BudgetTinker is an early local desktop budgeting application. Only the current development
version is maintained. There is no guaranteed response or fix deadline. Keep independent
backups; budgets and backups are not encrypted by the application.

## Report privately

The [BudgetTinker repository](https://github.com/perevozchiko/budget-tinker) is private.
Private vulnerability reporting has not been verified as configured. If the maintainer
enables it, use **Security → Advisories → Report a vulnerability** there.

Until a private channel is enabled, contact the maintainer through an already established
private channel. If none is available, open only a minimal request for a private contact
if appropriate; do not include exploit instructions, secrets, real budgets or
malicious attachments in a public issue.

Include affected version/commit, OS/architecture, affected importer/operation, impact,
reproduction steps using synthetic data and any workaround. Redact credentials and personal
paths from logs. Coordinate disclosure with the maintainer before publishing exploit details.
Imported files are untrusted inputs; minimize and document any accepted regression fixture.

## Maintainer setup before release

Before public release, an owner/admin should verify private reporting is enabled under **Settings →
Security and quality → Advanced Security → Private vulnerability reporting → Enable**.
Verify the **Report a vulnerability** button appears under Security/Advisories, enable
notifications and confirm who will triage reports. A SECURITY.md file does not enable
this setting by itself. No setting was changed by this local task.

See [GitHub's repository setup instructions](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/configure-vulnerability-reporting/configure-for-a-repository)
and [private reporting instructions](https://docs.github.com/en/code-security/how-tos/report-and-fix-vulnerabilities/report-privately).

## Dependency checks

Run the pinned cargo-deny license and RustSec advisory checks documented in
[the dependency audit](docs/DEPENDENCY-AUDIT.md). Record the database revision/date and
actual diagnostics. Do not silently ignore advisories or treat an audit as a security guarantee.
