<div align="center">

# RustCalc

**Room to think. A native calculator built in Rust.**

[![Windows checks](https://github.com/ersinkoc/rustcalc/actions/workflows/check.yml/badge.svg)](https://github.com/ersinkoc/rustcalc/actions/workflows/check.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-99c995.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.88%2B-orange.svg)](Cargo.toml)

Decimal arithmetic · Scientific functions · Unit conversion · Programmer tools · Function plots

[Download](https://github.com/ersinkoc/rustcalc/releases) · [Installation](#installation) · [Contributing](CONTRIBUTING.md) · [Roadmap](docs/ROADMAP.md)

</div>

![RustCalc in English, dark appearance](docs/images/calculator-en-dark.png)

RustCalc is an open-source Windows desktop calculator with editable expressions,
live results, persistent history, and a charcoal-and-sage interface. It runs
locally, requires no account or browser runtime, and supports **English and
Turkish** throughout the interface. English is the default; switch with the
**EN / TR** sidebar controls. Your choice is saved.

> Early release: Windows x64 is the supported binary target. Installation and
> self-update use published stable GitHub Releases. Source builds are also
> available; read the calculation limits and update trust model before use.

## Features

| Workspace | What you can do |
| --- | --- |
| Calculator | Standard/scientific keypads, editable expressions, live preview, parentheses, powers, factorials, percentages, DEG/RAD, constants and memory |
| History | Search, pin, copy, edit previous expressions, and continue from a result; up to 500 entries |
| Unit converter | Length, mass, temperature, area, volume, speed, time and SI/binary data units |
| Programmer | Exact unsigned 64-bit HEX/DEC/OCT/BIN conversion, interactive bit map, AND/OR/XOR/NOT and shifts |
| Function plot | Plot expressions in `x`, adjust the visible range and inspect values under the pointer |
| Appearance | Light/dark themes, English/Turkish UI, compact layout, resizable window and native DPI scaling |
| Updates | Explicit update checks in the app or CLI; SHA-256-verified GitHub Releases downloads |

<details>
<summary>Light appearance and Turkish interface</summary>

![English light appearance](docs/images/calculator-en-light.png)
![Turkish dark appearance](docs/images/calculator-tr-dark.png)

</details>

## Installation

### PowerShell installer

To install a published stable release, download and inspect the installer, then
run it. Administrator access is not required.

```powershell
Invoke-WebRequest https://raw.githubusercontent.com/ersinkoc/rustcalc/main/install.ps1 -OutFile install-rustcalc.ps1
# Review install-rustcalc.ps1 before running it.
& .\install-rustcalc.ps1
rustcalc
```

The installer uses `%LOCALAPPDATA%\Programs\RustCalc`, creates a Start menu
shortcut, and adds its `bin` directory to the user PATH and current PowerShell
process. Open a new terminal if you run it in a separate PowerShell process.
Downloads are staged, checked against `SHA256SUMS`, and version-verified before
the installed executable changes.

```powershell
& .\install-rustcalc.ps1 -Version 0.1.0
& .\install-rustcalc.ps1 -InstallDir "$env:LOCALAPPDATA\Tools\RustCalc"
& .\install-rustcalc.ps1 -NoPath -NoShortcut
```

See [installation details](docs/INSTALLATION.md) for offline installs, removal
and PowerShell troubleshooting.

### Portable executable

Download `rustcalc-windows-x86_64.exe` or the ZIP from
[GitHub Releases](https://github.com/ersinkoc/rustcalc/releases). Verify it using
the release's `SHA256SUMS`, then run it from any directory. You can rename it
to `rustcalc.exe`. No installation is required; preferences and history still
use the current user's application-data folder.

### Build from source

Requires Rust 1.88+ and Visual Studio C++ build tools with a Windows SDK.

```powershell
git clone https://github.com/ersinkoc/rustcalc.git
cd rustcalc
cargo run --release --locked
```

The executable is `target\release\rustcalc.exe`. Linux/macOS source builds are
not currently part of the supported or validated distribution.

## Updates

Open **About & updates → Check for updates**, review the version, and select
**Install update**. Restart afterward. The app never checks without your action.

```powershell
rustcalc --version
rustcalc update --check  # Check only; does not change the executable.
rustcalc update          # Download, verify and replace this executable.
```

Only newer stable versions from `ersinkoc/rustcalc` are accepted. Downloads must
match the release manifest and GitHub asset digest when present. Since 0.1.1,
the updater also runs the verified candidate's `--version` command and requires
the expected version before replacement. Errors are reported; updates retain
history/preferences. Read the [update design and trust model](docs/UPDATES.md).

## Calculation behavior

Basic arithmetic uses 96-bit decimals, so **`0.1 + 0.2 = 0.3`**. The range is
approximately ±7.9 × 10²⁸ with up to 28 fractional places. This is bounded
precision; division and other arithmetic may round. Scientific functions and
non-integer powers use `f64` with approximately 15 significant digits and show
**≈**, preserved through `ans` and memory. Invalid domains, overflow, division by
zero and multiplication/division underflowing to zero produce diagnostics.

| Expression | Result / meaning |
| --- | --- |
| `2 + 3 * 4` | `14` |
| `2^3^2` | `512` — powers associate to the right |
| `-2^2` / `(-2)^2` | `-4` / `4` |
| `2(3+4)` / `2pi` | Implicit multiplication |
| `sin(30)` in DEG | `≈ 0.5` |
| `200 * (1+10%)` | `220` |
| `200 + 10%` | `200.1` — `%` always divides by 100 |
| `ans * 2` / `mem + 1` | Previous answer / saved memory |

Comma and period are decimal separators. Do not use thousands separators.
Functions: `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sqrt`, `cbrt`, `ln`,
`log`, `exp`, `abs`, `floor`, `ceil`, `round`. Constants: `pi`, `e`, `ans`, `mem`.
`round` uses ties-to-even. Factorials accept 0–27, integer exponents are bounded
to ±10000, and parsing is limited to 4096 bytes / 64 nesting levels. Plots use
radians and equal numeric axis ranges; physical scales depend on aspect ratio.

## Keyboard and language

In the calculator workspace, type numbers and `+`, `-`, `*`, `/` directly and
press Enter. There is no need to click the expression field first, even after
clicking a mode button. When history search has focus, typing stays in search.

| Shortcut | Action |
| --- | --- |
| Enter or `=` | Calculate and save to history |
| Esc | Clear the expression |
| Ctrl+L | Select the expression |
| Ctrl+Shift+C | Copy the result |
| Ctrl+C / Ctrl+V / Ctrl+Z | Standard text editing |
| F1 | Show help |

```powershell
rustcalc --lang en
rustcalc --lang tr --compact
rustcalc --light
rustcalc --help
```

## Privacy and local storage

Calculation, conversion, history and plots work offline. There is no telemetry,
account, advertisement or analytics service. Only manual updates connect to
GitHub; calculations are never included. Project/release hyperlinks open your
browser. eframe stores state in the current user's RustCalc data directory
(normally `%APPDATA%\RustCalc\data\app.ron` on Windows), every five seconds and on
normal exit. Uninstall preserves this data. The expression being edited is not
persisted. Back up the state file before manually editing it.

## Development

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
pwsh -NoProfile -File scripts/test-install.ps1
pwsh -NoProfile -File scripts/package.ps1
```

Tests cover arithmetic, conversion, history, locales, persistence, updater
integrity errors and egui keyboard/pointer interactions. Installer tests use
local fixtures and do not change persistent PATH. See [contributing](CONTRIBUTING.md),
[architecture](docs/ARCHITECTURE.md), [localization](docs/LOCALIZATION.md) and
[releasing](docs/RELEASING.md).

## Status and limitations

This is an early, working desktop application. Official binaries are Windows x64
and unsigned. Screen-reader accessibility needs validation. There is no symbolic
algebra, arbitrary precision, signed programmer arithmetic or currency service.
Checksums provide integrity, not an independent publisher signature. Planned
work is in [the roadmap](docs/ROADMAP.md).

## License

[MIT](LICENSE) © 2026 [Ersin Koç](https://github.com/ersinkoc).
