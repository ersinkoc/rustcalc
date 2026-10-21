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

CI builds and tests each platform: the Windows job packages the EXE and ZIP,
the Linux job (Ubuntu 22.04) the Linux binary and tar.gz, and the macOS job the
Apple silicon and Intel binaries and tar.gz files. The publish job downloads
those exact artifacts, writes one `SHA256SUMS` with
`scripts/assemble-release.ps1`, and publishes them. `scripts/release-assets.ps1`
is the single list of asset names for packaging, publishing and verification.

On Linux or macOS, verify a published release with
`sh scripts/test-live-release.sh 0.5.1 0.5.0` (install, self-update, rollback,
redo and uninstall in a temporary HOME); omit the second version for an
install-only proof.

Expected assets (0.5.0 and later):

- `rustcalc-windows-x86_64.exe`, `rustcalc-v<VERSION>-windows-x86_64.zip`
- `rustcalc-linux-x86_64`, `rustcalc-v<VERSION>-linux-x86_64.tar.gz`
- `rustcalc-macos-aarch64`, `rustcalc-v<VERSION>-macos-aarch64.tar.gz`
- `rustcalc-macos-x86_64`, `rustcalc-v<VERSION>-macos-x86_64.tar.gz`
- `install.ps1`, `uninstall.ps1`, `install.sh`, `uninstall.sh`
- `SHA256SUMS` covering all twelve files above

Releases before 0.5.0 contain only the four Windows files and `SHA256SUMS`.

Do not rename assets without changing installer/updater/tests/docs together.
After publication verify names, manifests, digests and annotated tag identity.
The live proof below downloads all assets from public URLs, checks their GitHub
digests and complete manifest, checks the ZIP, installs into a temporary folder,
and invokes the canonical command outside the checkout. It never changes the
persistent user PATH. The second form exercises the real self-update command
between two published versions, then checks the final executable hash and removal.

```powershell
pwsh -NoProfile -File scripts/test-live-release.ps1 -Version 0.4.0
pwsh -NoProfile -File scripts/test-live-release.ps1 -Version 0.4.0 -UpgradeFrom 0.3.0
```

JSON proof reports are saved in `artifacts/live-release-v<VERSION>.json`. The
tested target must be the latest stable release. Local fixtures do not replace
this published-release proof.

Close running calculator windows before the proof: GUI auto-save or concurrent
edits legitimately change the state file and invalidate its preservation check.
If GitHub returns an API rate-limit error, wait for the quota to reset and rerun;
a partial proof is not a pass. The published `0.3.0 -> 0.4.0` scenario passed all
26 checks, including the final up-to-date response and state preservation, and
the fresh 0.4.0 installation proof passed all 16 checks. The proof resolves the
canonical command to the first PATH match, so an existing personal installation
does not interfere.

Leave a failed release as a draft. Never silently replace an already published
version; publish a new patch version for corrections.
