# Update design

Updates are opt-in. **About & updates → Check for updates** and
`rustcalc update --check` only check. **Install update** and `rustcalc update`
explicitly install. Calculations need no network.

## Automatic mode

The persisted setting is `Off` (default), `Notify` or `Install`. A one-time
status-bar question asks for a choice; until it is answered, nothing connects.

- An automatic check starts when the setting is not `Off`, no update work is
  pending and at least 24 hours have passed since the last attempt (or the clock
  moved backwards). The attempt time is stored before the request, so failures
  retry the next day rather than repeatedly.
- `Notify` turns a newer release into an **Update to x.y.z** status-bar button
  that opens About & updates.
- `Install` passes the plan to the same installer as **Install update**, with
  every validation step below. A successful install shows **Restart to update**;
  the running app is never restarted or closed automatically.
- Automatic failures do not interrupt work; the reason appears in About.
- Screenshot mode never checks. Tests replace the network runner with a local one.

## Validation

1. Request `https://api.github.com/repos/ersinkoc/rustcalc/releases/latest`.
2. Require a non-draft, non-prerelease stable semantic version prefixed with `v`.
3. Reject the current version and downgrades.
4. Require exactly one binary and manifest with exact filenames and URLs under
   that tag in `ersinkoc/rustcalc`.
5. Download the pinned tag's files with size/time limits.
6. Require one matching SHA-256 manifest entry, matching size and a Windows
   executable header. Enforce GitHub's asset digest when present.
7. Stage and sync a temporary file, run its `--version` command without opening
   a window, and require the exact expected RustCalc version. A ten-second
   timeout and bounded output prevent a broken candidate from hanging the app.
8. Only then let `self-replace` handle Windows binary replacement. Report
   success and ask for a restart.

The GUI runs work on a background thread and keeps the window open during
installation. Timeouts: 90 seconds per request, 15 seconds to connect. Binary
limit: 64 MiB. Invalid metadata, corrupt downloads and replacement errors do not
silently report success. Preferences/history are separate from the executable.

## Platforms

Each build updates from its own release asset: `rustcalc-windows-x86_64.exe`,
`rustcalc-linux-x86_64`, `rustcalc-macos-aarch64` or `rustcalc-macos-x86_64`. The
download must start with the native executable header (PE, ELF or Mach-O).
On Linux and macOS the staged file is made executable before its `--version` is
checked, and a command reached through a symlink (`~/.local/bin/rustcalc`)
updates the installed file it points to. One `SHA256SUMS` covers every
platform's files; older updaters read only their own entry. Builds for other
platforms report that updates are unavailable.

## Rollback

Before replacing the executable, self-update copies it beside itself as
`rustcalc.previous.exe` on Windows or `rustcalc.previous` elsewhere (a failed
copy does not block the update).
`rustcalc update --rollback` runs that copy's `--version`, refuses to restore it
if it does not start, stages it beside the executable and replaces the running
file with it. The replaced build becomes the new previous copy, so running the
command again redoes the update. Uninstall removes the copy.

## Trust and recovery

Checksums detect corruption/mismatched assets, not an independently authenticated
publisher: someone controlling the release can replace both binary and manifest.
Signing is planned, not implemented. See [SECURITY.md](../SECURITY.md).

If replacement is blocked, close other app windows and retry. A verified portable
download or installer can also recover the application after it is closed.
Back up the state file before changing application data manually.

There is no background update without opt-in, arbitrary feed URL,
package-manager self-update, forced downgrade, telemetry or forced restart.
