# Contributing to RustCalc

Keep code, documentation, issues and pull requests in English. User-facing
messages belong in both locale catalogues. Contributions use the MIT license.

## Setup and checks

Install Rust 1.88+, Windows MSVC C++ tools and a Windows SDK. Fork the repository,
create a focused branch from `main`, and use `cargo run --locked` to develop.

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
pwsh -NoProfile -File scripts/test-install.ps1
```

## Expectations

- Explain the user-visible problem and resulting behavior; include UI screenshots.
- Discuss large features in an issue first and keep changes focused.
- Reproduce bugs before fixing them and retain behavioral regression tests.
- Add messages to both `locales/en.json` and `locales/tr.json` with matching keys.
- Check both themes/languages and compact/wide sizes for UI changes.
- Preserve old history compatibility; new settings need serde defaults.
- Keep the engine independent of egui. Keep bounded network work in `updater.rs`.
- Never add telemetry, unsolicited requests, credentials or local state.
- Commit `Cargo.lock`; do not commit `target`, `dist` or local artifacts.
- Do not loosen tests or accuracy limits to hide failures.

Report exact expressions/input sequences, version, Windows version and sanitized
logs. Report security issues privately per [SECURITY.md](SECURITY.md). Follow the
[Code of Conduct](CODE_OF_CONDUCT.md).
