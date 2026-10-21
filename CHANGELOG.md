# Changelog

Versions follow semantic versioning. During 0.x, product and library interfaces
may evolve before the first stable contract.

## 0.3.0 — Explorable plots and automatic updates

### Added

- Optional automatic updates. On first launch a one-time, non-blocking question
  in the status bar offers **Notify me**, **Install automatically** or **Not
  now**; the choice can be changed in About & updates. Enabled modes check
  GitHub at most once a day while RustCalc is open. Install mode uses the same
  verified installer as a manual update and never restarts by itself: the
  status bar offers **Restart to update**. Nothing connects until you opt in,
  and automatic failures stay quiet, with the reason shown in About.
- Programmer word sizes of 8, 16, 32 and 64 bits with signed (two's
  complement) or unsigned interpretation. Narrowing keeps the low bits; the
  signed toggle keeps the bit pattern. New wrapping `+`, `−`, `×`, `÷`, `MOD`,
  `NEG` and shift-by-operand operations; `>>` is arithmetic for signed words.
  Binary output is grouped by nibble and padded to the word size.
- Plot roots and intersections: sign changes are refined by bisection, marked
  on the plot and listed below it. Poles and jumps are not reported as roots.
- Plot up to three functions, `f`, `g` and `h`, at once, each in its own color
  with a combined hover readout.
- Drag to pan, scroll or pinch to zoom around the pointer, and double-click to
  reset. Zoom buttons, a reset button and a wider view slider are available,
  and arrow keys, `+`/`-` and Home work while the pointer is over the plot.
- Hyperbolic functions: `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`.
  They take plain numbers and ignore DEG/RAD.

### Changed

- Plots sample one point per pixel and bisect large jumps: poles such as
  `1/x` and `tan(x)` and step discontinuities split the curve, while steep but
  continuous curves stay connected. Curves leaving the view are clipped at the
  edge with their true slope instead of disappearing early.
- Grid lines use 1-2-5 spacing with labels that follow the axes, or stay at the
  edge when an axis is off-screen.
- A curve that cannot be drawn shows why, such as a syntax error, in the current
  language.
- `pi` and `e` keep 28 digits instead of about 15.

### Fixed

- Scientific results smaller than about 1e-14, such as `exp(-40)` or
  `sqrt(2e-20)`, were rejected as outside the number range. They now keep as
  many digits as the 28 decimal places allow.
- `cos(90)`, `sin(180)`, `tan(180)` and `sin(pi)` in radians returned an error
  instead of zero because of floating-point residue near 1e-16.
- The programmer bit map no longer runs past the right edge of compact windows.

## 0.2.0 — Portable history and undo

### Added

- History → Files: native save/open dialogs for versioned JSON backups,
  readable text exports and backup import, in English and Turkish.
- Import confirmation, duplicate detection and pin preservation. Stored results
  are restored without reevaluating expressions or changing answer/memory/settings.
- Undo deletion for individual entries and clearing unpinned history, keeping
  calculations made after deletion. Up to 20 deletion actions are retained per
  session; changed imports start a new undo baseline.
- Atomic file replacement, a 4 MiB backup limit, entry validation and rejection
  of imports that would exceed 500 entries. Failed imports keep existing data.
- File and UI tests for round trips, malformed input, capacity, cancellation,
  confirmation, undo and English/Turkish compact layouts.

### Changed

- File operations run outside the UI thread; cancellation is silent and errors
  remain visible until dismissed.
- History import, error and clear dialogs block background input and close with
  Escape without modifying history.

## 0.1.2 — Direct keyboard input

### Fixed

- Numbers, arithmetic operators and Enter now work directly in the calculator
  after clicking a mode button or leaving the expression field.
- Enter is handled once by the calculator, after pending text input is applied.
- Typing after a result starts a new calculation; an operator continues from
  the answer, including when the expression field has lost focus.
- History search keeps its own keyboard input. Calculator shortcuts do not
  evaluate expressions behind help, confirmation or About dialogs.

### Added

- UI regressions for lost focus, all four arithmetic operations, direct Enter,
  result continuation, English/Turkish mode clicks and search/dialog isolation.

## 0.1.1 — Verified update candidates

### Fixed

- Self-update now runs the hash-verified candidate with `--version` before
  replacing the installed executable. A correctly hashed but wrong-version or
  non-executable release asset is rejected without changing the installation.
- Candidate probes run without a console window, with bounded output and a
  ten-second timeout.
- Release publication now reuses the exact package that passed CI and installer
  checks instead of rebuilding a separate executable in the publishing job.
- Annotated release tags are re-fetched from the authoritative remote after
  checkout; a peeled local tag no longer blocks a valid publication.
- Draft lookup uses GitHub's release ID endpoint, and retries safely reuse
  matching draft assets without overwriting published files.

### Added

- Regression tests using the actual built application as an update candidate.
- A reproducible public-release proof for asset digests, annotated tags, online
  installation, canonical command lookup, real self-update and safe removal.

## 0.1.0 — Initial public release

### Added

- Native Rust/egui calculator with dark/light appearance and English/Turkish UI.
- Decimal arithmetic, scientific functions, live results, DEG/RAD and memory.
- Searchable/pinnable history and readable clipboard export.
- Eight unit-conversion categories, 64-bit programmer tools and function plots.
- Opt-in GitHub Releases checks, checksum verification, self-update and restart.
- Per-user PowerShell install/uninstall, offline installation and portable ZIP.
- CI, release packaging, contributor documentation and regression tests.

### Known limitations

- Official binaries target Windows x64 and are unsigned.
- Decimal precision is bounded; scientific functions are approximate.
- Accessibility and non-Windows builds need further validation.
