# Security Policy

## Supported versions

Security fixes target the latest stable release and `main`. Older releases are
not maintained separately. This early-stage project has no security response SLA.

## Reporting

Do not post exploit details, credentials or personal data in public issues. Use
[GitHub private vulnerability reporting](https://github.com/ersinkoc/rustcalc/security/advisories/new)
when available; otherwise use a private contact method on
[the maintainer's profile](https://github.com/ersinkoc). Do not assume private
reporting is enabled until GitHub confirms it. Include the version, reproduction,
impact and sanitized logs. Allow investigation before public disclosure.

## Distribution trust

Official binaries for Windows, Linux and macOS come from `ersinkoc/rustcalc` GitHub Releases. The installer
and updater require manifest SHA-256 matches; the app also checks GitHub's asset
digest when provided. Checks are user-initiated or, if the user enables
automatic updates, at most daily. HTTPS/checksums protect transport
and integrity, not an independent signature if the GitHub account/release is
compromised. Binaries/scripts are currently unsigned. Default installation needs
no elevation; prefer a directory writable only by the intended user.
