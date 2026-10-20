# Update design

Updates are opt-in. **About & updates → Check for updates** and
`rustcalc update --check` only check. **Install update** and `rustcalc update`
explicitly install. No worker starts at app launch; calculations need no network.

## Validation

1. Request `https://api.github.com/repos/ersinkoc/rustcalc/releases/latest`.
2. Require a non-draft, non-prerelease stable semantic version prefixed with `v`.
3. Reject the current version and downgrades.
4. Require exactly one binary and manifest with exact filenames and URLs under
   that tag in `ersinkoc/rustcalc`.
5. Download the pinned tag's files with size/time limits.
6. Require one matching SHA-256 manifest entry, matching size and a Windows
   executable header. Enforce GitHub's asset digest when present.
7. Stage and sync a temporary file before `self-replace` handles Windows binary
   replacement. Report success and ask for a restart.

The GUI runs work on a background thread and keeps the window open during
installation. Timeouts: 90 seconds per request, 15 seconds to connect. Binary
limit: 64 MiB. Invalid metadata, corrupt downloads and replacement errors do not
silently report success. Preferences/history are separate from the executable.

## Trust and recovery

Checksums detect corruption/mismatched assets, not an independently authenticated
publisher: someone controlling the release can replace both binary and manifest.
Signing is planned, not implemented. See [SECURITY.md](../SECURITY.md).

If replacement is blocked, close other app windows and retry. A verified portable
download or installer can also recover the application after it is closed.
Back up the state file before changing application data manually.

There is no silent background updater, arbitrary feed URL, package-manager
self-update, forced downgrade, telemetry or forced restart.
