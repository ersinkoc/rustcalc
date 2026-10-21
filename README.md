<div align="center">

# RustCalc

**Room to think. A native calculator built in Rust.**

[![Windows checks](https://github.com/ersinkoc/rustcalc/actions/workflows/check.yml/badge.svg)](https://github.com/ersinkoc/rustcalc/actions/workflows/check.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-99c995.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.88%2B-orange.svg)](Cargo.toml)

Decimal arithmetic · Formulas & templates · Date math · Unit conversion · Programmer tools · Function plots

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
| Programmer | 8/16/32/64-bit signed or unsigned HEX/DEC/OCT/BIN, interactive bit map, wrapping arithmetic, bitwise operations and shifts |
| Formulas | 21 ready-made templates (finance, dates, everyday, health) and your own formulas with named inputs, live results, JSON sharing |
| Function plot | Plot up to three expressions in `x`; drag to pan, scroll to zoom, find roots and intersections |
| Appearance | Light/dark themes, English/Turkish UI, compact layout, resizable window and native DPI scaling |
| Updates | Manual or optional daily automatic checks; SHA-256-verified GitHub Releases downloads |

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
**Install update**. Restart afterward.

Automatic updates are off until you choose otherwise. On first launch the status
bar asks once; change the choice later under **About & updates → Automatic
updates**:

| Mode | Behavior |
| --- | --- |
| Off | Never connects unless you check manually |
| Notify me | Checks at most once a day and shows **Update to x.y.z** in the status bar |
| Install automatically | Checks daily, then downloads, verifies and installs; **Restart to update** applies it |

Automatic installs use the same verification as manual ones and never restart
the app for you. Each update keeps the replaced executable as
`rustcalc.previous.exe`; `rustcalc update --rollback` restores it, and running it
again returns to the newer version. A failed automatic check is retried the next day; the reason
is shown in About.

```powershell
rustcalc --version
rustcalc update --check  # Check only; does not change the executable.
rustcalc update          # Download, verify and replace this executable.
rustcalc update --rollback  # Restore the version the last update replaced.
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
`log`, `exp`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`, `abs`, `floor`,
`ceil`, `round`. Hyperbolic functions ignore DEG/RAD. Multi-argument functions
use `;` because the comma is a decimal separator: `min`, `max`, `sum`, `avg`,
`gcd`, `lcm`, `ncr`, `npr`, `mod`, `round(x; digits)`, `log(x; base)`, `sign`.
Comparisons (`<`, `>`, `<=`, `>=`, `==`, `!=`) give 1 or 0, and `if(c; a; b)`,
`and`, `or`, `not` evaluate only what they need. Dates are day numbers:
`date(2026; 12; 31) - today()` is the number of days until New Year's Eve; see
the [formula guide](docs/FORMULAS.md) for every date function. Constants: `pi`, `e`, `ans`, `mem`.
`round` uses ties-to-even. Factorials accept 0–27, integer exponents are bounded
to ±10000, and parsing is limited to 4096 bytes / 64 nesting levels. Plots use
radians and equal numeric axis ranges; physical scales depend on aspect ratio.

## Formulas

Open **05 Formulas** for ready-made calculations: loans, compound interest,
savings, VAT, margins, pay, dates, ages, discounts, bills, fuel, unit prices,
electricity and BMI. Type values and the results update immediately; inputs
accept expressions such as `1500*12` or `today() + 30`.

**+ New formula** builds your own: name the inputs, write result expressions
that can use the inputs and earlier results, and pick how each result is shown
(number, money, percent, date or weekday). The editor validates and previews as
you type. **Customize a copy** turns any template into an editable formula.
**Export…**/**Import…** share formulas as JSON files. See the
[formula guide](docs/FORMULAS.md).

## Function plots

Enter up to three functions of `x` as `f`, `g` and `h`; leave a row empty to hide
it. Drag the plot to pan, scroll (or pinch) to zoom around the pointer and
double-click to reset. While the pointer is over the plot and no text field has
focus, arrow keys pan, `+`/`-` zoom and Home resets. Poles and jumps split a
curve instead of being joined by a vertical line; a function that cannot be drawn
shows the reason below the plot.

**Roots and intersections** (on by default) marks where each curve crosses zero
and where curves cross each other, and lists up to eight x values per equation.
Only sign changes are found, so a curve that touches zero without crossing it,
such as `x^2` at 0, is not listed.

## Programmer

Choose 8, 16, 32 or 64 bits and **Signed** for two's complement. DEC input
accepts the word's range, such as -128 to 127 for signed 8-bit; HEX, OCT and BIN
input is a bit pattern that must fit the word. Arithmetic wraps like hardware
integers, division truncates toward zero, and `>>` keeps the sign for signed
words. Switching to a smaller word keeps the low bits.

## History backups and undo

Open **History → Files…** to **Save backup…** (JSON), **Export as text…** or
**Import backup…**. A backup preserves expressions, stored results, approximate
markers, DEG/RAD, pins and timestamps. File dialogs run outside the UI thread.
Exports capture history when the command is selected.

Import previews the record count and requires confirmation. It merges with your
history, skips duplicates and preserves pins. It does not reevaluate expressions
or change the current answer, memory, language or theme. Invalid files and merges
that would exceed 500 entries are rejected without changing history. Backups are
limited to 4 MiB. Text exports are readable reports; import uses JSON backups.

**Undo deletion** restores a deleted calculation or a confirmed clear without
losing calculations added afterward. Up to 20 deletion actions are kept for the
current session. Undo is not saved across restarts. Importing changed history
starts a new undo session; an unchanged duplicate import keeps the undo stack.
Undo that would exceed the 500-entry limit is rejected without dropping entries.
See the [backup format and merge/undo rules](docs/HISTORY.md).

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
account, advertisement or analytics service. Only update checks you request or
enable connect to GitHub; calculations are never included. Project/release hyperlinks open your
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
[architecture](docs/ARCHITECTURE.md), [formulas](docs/FORMULAS.md), [localization](docs/LOCALIZATION.md) and
[releasing](docs/RELEASING.md).

## Status and limitations

This is an early, working desktop application. Official binaries are Windows x64
and unsigned. Screen-reader accessibility needs validation. There is no symbolic
algebra, arbitrary precision or currency service.
Checksums provide integrity, not an independent publisher signature. Planned
work is in [the roadmap](docs/ROADMAP.md).

## License

[MIT](LICENSE) © 2026 [Ersin Koç](https://github.com/ersinkoc).
