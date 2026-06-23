# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| latest release | ✅ |
| older releases | ❌ (upgrade recommended) |

Security fixes are released as **patch** versions (e.g. `v0.1.1`) on the latest minor line.

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Instead, report privately via [GitHub Security Advisories](https://github.com/carvalhosauro/local-ticker-wallet/security/advisories/new) or email the maintainer through their GitHub profile.

Include:

- Description of the vulnerability and impact
- Steps to reproduce
- Affected version(s)
- Suggested fix (if any)

You should receive an acknowledgment within **7 days**. We will coordinate disclosure and credit reporters who wish to be named.

## Scope

In scope:

- `ltw` binary (daemon, CLI, TUI)
- IPC socket handling
- SQLite storage and migrations
- Network requests to market-data providers
- Import/export parsers

Out of scope:

- Third-party APIs (Yahoo Finance, brapi.dev) — report to those providers directly
- User machine configuration (file permissions on `wallet.db`, socket path)
- Social engineering

## Hardening notes

- `ltw` binds a Unix domain socket on the local machine — protect `$XDG_RUNTIME_DIR`
- The database contains your financial ledger — treat `wallet.db` like sensitive data
- Market-data tokens (`brapi_token` in config) should not be committed to version control
