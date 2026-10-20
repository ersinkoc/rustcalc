# Changelog

Versions follow semantic versioning. During 0.x, product and library interfaces
may evolve before the first stable contract.

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
