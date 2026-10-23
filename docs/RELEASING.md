# Releasing RustCalc

Merging `main` does not publish a release. Tags trigger the release workflow.
Do not advertise installation as live until the first release assets exist.

## Prepare

Update the package version, lockfile and changelog. Run all checks and inspect
both languages/themes at compact/wide sizes. Review the diff for artifacts,
credentials and local state. Push to `main` and wait for CI.

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
pwsh -NoProfile -File scripts/test-install.ps1
pwsh -NoProfile -File scripts/package.ps1
git tag -a v0.1.0 -m "RustCalc v0.1.0"
git push origin v0.1.0
```

The workflow requires the tag version to match `Cargo.toml` and its commit to
belong to `main`. It runs checks and packaging, then passes that exact tested
package to the publishing job. The publisher checks the manifest, creates a
draft release, verifies uploaded asset digests and then publishes. It does not
rebuild a different executable after the installer checks have passed.

`scripts/verify-release-tag.ps1` re-fetches the authoritative annotated tag to
handle checkout implementations that peel local tag refs. Regression fixtures
exercise this behavior and reject truly lightweight remote tags.
`scripts/publish-release.ps1` checks the remote tag target, manifest and uploaded
digests. It resolves drafts through their REST release ID, resumes matching
partial uploads and refuses to overwrite conflicting assets. A rerun against an
identical published release performs verification without replacing anything.

CI builds and tests each platform on its own hardware: the Windows x64 and
ARM64 jobs package their EXE and ZIP, the Linux x64 and ARM64 jobs (Ubuntu
22.04) their binary and tar.gz, and the macOS job the Apple silicon and Intel
binaries and tar.gz files. `scripts/package.ps1` names a Windows package after
the machine type in the executable's own header. The publish job downloads
those exact artifacts, writes one `SHA256SUMS` with
`scripts/assemble-release.ps1`, and publishes them. `scripts/release-assets.ps1`
is the single list of asset names for packaging, publishing and verification.

On Linux or macOS, verify a published release with
`sh scripts/test-live-release.sh 0.8.0 0.7.0` (install, self-update, rollback,
redo and uninstall in a temporary HOME); omit the second version for an
install-only proof.

Expected assets (0.7.0 and later):

- `rustcalc-windows-x86_64.exe`, `rustcalc-v<VERSION>-windows-x86_64.zip`
- `rustcalc-windows-aarch64.exe`, `rustcalc-v<VERSION>-windows-aarch64.zip`
- `rustcalc-linux-x86_64`, `rustcalc-v<VERSION>-linux-x86_64.tar.gz`
- `rustcalc-linux-aarch64`, `rustcalc-v<VERSION>-linux-aarch64.tar.gz`
- `rustcalc-macos-aarch64`, `rustcalc-v<VERSION>-macos-aarch64.tar.gz`
- `rustcalc-macos-x86_64`, `rustcalc-v<VERSION>-macos-x86_64.tar.gz`
- `install.ps1`, `uninstall.ps1`, `install.sh`, `uninstall.sh`
- `SHA256SUMS` covering all sixteen files above

Releases 0.5.0 and 0.6.0 have no ARM64 Windows or Linux files (13 assets);
releases before 0.5.0 contain only the four Windows files and `SHA256SUMS`.

Do not rename assets without changing installer/updater/tests/docs together.
After publication verify names, manifests, digests and annotated tag identity.
The live proof below downloads all assets from public URLs, checks their GitHub
digests and complete manifest, checks the ZIP, installs into a temporary folder,
and invokes the canonical command outside the checkout. It never changes the
persistent user PATH. The second form exercises the real self-update command
between two published versions, then checks the final executable hash and removal.

```powershell
pwsh -NoProfile -File scripts/test-live-release.ps1 -Version 0.8.0
pwsh -NoProfile -File scripts/test-live-release.ps1 -Version 0.8.0 -UpgradeFrom 0.7.0
```

JSON proof reports are saved in `artifacts/live-release-v<VERSION>.json`. The
tested target must be the latest stable release. Local fixtures do not replace
this published-release proof.

Close running calculator windows before the proof: GUI auto-save or concurrent
edits legitimately change the state file and invalidate its preservation check.
If GitHub returns an API rate-limit error, wait for the quota to reset and rerun;
a partial proof is not a pass. The published `0.3.0 -> 0.4.0` scenario passed all
26 checks, including the final up-to-date response and state preservation, and
the fresh 0.4.0 installation proof passed all 16 checks. For 0.5.0, the
`0.4.0 -> 0.5.0` upgrade passed all 34 checks (including the digests of all 13
assets across Windows, Linux and macOS), the fresh installation passed 24, and
the Linux proof (`sh scripts/test-live-release.sh 0.5.0`) passed its 4 checks.
A manual Windows proof then installed 0.4.0 publicly, updated it to 0.5.0, ran
`rustcalc update --rollback` (back to 0.4.0) and again (0.5.0), each step with
exit code 0. For 0.6.0, the `0.5.0 -> 0.6.0` upgrade passed all 42 checks and
the fresh installation 24. The Linux proof
(`sh scripts/test-live-release.sh 0.6.0 0.5.0`) passed all 7 checks; this was the
first real Linux self-update, and it rolled back to 0.5.0 and forward again. The
manual Windows proof repeated `0.5.0 -> 0.6.0 -> 0.5.0 -> 0.6.0`, each step with
exit code 0. For 0.7.0, the first release with 17 assets, the `0.6.0 -> 0.7.0`
upgrade passed all 47 checks (digests of every asset, including the ARM64
files, and both Windows ZIPs) and the fresh installation 29; the Linux proof
`sh scripts/test-live-release.sh 0.7.0 0.6.0` passed all 7 checks and the
manual Windows rollback `0.6.0 -> 0.7.0 -> 0.6.0 -> 0.7.0` exited 0 at every
step. The ARM64 builds were tested on ARM runners in CI; a real ARM64
self-update can first be proven with 0.8.0. Read a GUI-subsystem build's output through redirected files
(`Start-Process -RedirectStandardOutput -Wait`): PowerShell does not wait for
such programs when capturing `& rustcalc …`, and the closed pipe fails with
os error 232. The proof resolves the
canonical command to the first PATH match, so an existing personal installation
does not interfere.

Leave a failed release as a draft. Never silently replace an already published
version; publish a new patch version for corrections.
