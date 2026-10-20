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
belong to `main`. It runs checks, packages the build, creates a draft release,
verifies uploaded asset digests and then publishes.

Expected assets:

- `rustcalc-windows-x86_64.exe`
- `rustcalc-v<VERSION>-windows-x86_64.zip`
- `install.ps1`
- `uninstall.ps1`
- `SHA256SUMS`

Do not rename assets without changing installer/updater/tests/docs together.
After publication verify names, manifests, digests and annotated tag identity.
Test the public installer and canonical command outside the checkout. Exercise
an actual upgrade between two published versions before claiming the live
update path is certified; local fixtures do not prove that path.

Leave a failed release as a draft. Never silently replace an already published
version; publish a new patch version for corrections.
