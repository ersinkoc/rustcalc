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

Expected assets:

- `rustcalc-windows-x86_64.exe`
- `rustcalc-v<VERSION>-windows-x86_64.zip`
- `install.ps1`
- `uninstall.ps1`
- `SHA256SUMS`

Do not rename assets without changing installer/updater/tests/docs together.
After publication verify names, manifests, digests and annotated tag identity.
The live proof below downloads all assets from public URLs, checks their GitHub
digests and complete manifest, checks the ZIP, installs into a temporary folder,
and invokes the canonical command outside the checkout. It never changes the
persistent user PATH. The second form exercises the real self-update command
between two published versions, then checks the final executable hash and removal.

```powershell
pwsh -NoProfile -File scripts/test-live-release.ps1 -Version 0.1.0
pwsh -NoProfile -File scripts/test-live-release.ps1 -Version 0.1.1 -UpgradeFrom 0.1.0
```

JSON proof reports are saved in `artifacts/live-release-v<VERSION>.json`. The
tested target must be the latest stable release. Local fixtures do not replace
this published-release proof.

Leave a failed release as a draft. Never silently replace an already published
version; publish a new patch version for corrections.
