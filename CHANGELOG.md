# Changelog

Versions follow semantic versioning. During 0.x, product and library interfaces
may evolve before the first stable contract.

## Unreleased

### Fixed

- On Windows, a program put in place by self-update or rollback kept the
  temporary-file attribute of its staging file; it is now an ordinary file.

### Changed

- `sha2` 0.11, and the CI and release workflows use `actions/checkout` v7,
  `actions/upload-artifact` v7 and `actions/download-artifact` v8, which fails a
  release whose downloaded artifact digests do not match.

## 0.5.0 — Linux, macOS, everyday percent and clearer guidance

### Added

- **Linux and macOS.** Official binaries for Linux x64 and macOS (Apple silicon
  and Intel), each with a tar.gz archive; the macOS archive includes
  `RustCalc.app`. `install.sh` installs a SHA-256- and version-verified binary to
  `~/.local/opt/rustcalc`, links `~/.local/bin/rustcalc`, adds a menu entry with
  an icon on Linux or a launcher in `~/Applications` on macOS, and never edits
  shell startup files. `uninstall.sh` removes only what the installer recorded.
- Self-update and `rustcalc update --rollback` work on all three systems: each
  build updates from its own asset, checks the native executable header, makes
  the staged file executable and follows the command symlink to the installed
  file.
- CI tests, lints and packages on Windows, Linux (Ubuntu 22.04 for a glibc 2.35
  baseline) and macOS, and runs installer integration tests on each. Releases
  publish one `SHA256SUMS` for all platforms.
- RustCalc reopens where you left it: the workspace, the standard or scientific
  keypad, the converter's units and the selected formula are remembered.
- The unit converter accepts expressions such as `1500*12`.
- `round(x; -2)` rounds to hundreds (and other negative digits to tens,
  thousands …).
- A rewritten README with a quick start, the calculator rules, a function table
  and FAQ, plus a full [user guide](docs/USER_GUIDE.md) and a Turkish
  [kullanım kılavuzu](docs/KULLANIM_KILAVUZU.md).
- Tests that every Turkish screen (including help, About, the formula editor and
  the update prompt) shows no untranslated text, that no catalogue message is
  unused, and that every example in the README and both guides matches the
  engine.

### Changed

- **Percent now follows shop-calculator rules.** `100 + 10%` is 110 and
  `100 - 10%` is 90: adding or subtracting a percentage changes a number by that
  share of itself. `100 * 10%` is 10, `100 / 10%` is 1000 and `10%` alone is
  0.1, as before. Previously `100 + 10%` gave 100.1. Saved history keeps its
  stored results; formulas that relied on `a + b%` meaning `a + b/100` should
  be reviewed.
- **`round` rounds halves up, as taught at school:** `round(2.5)` is 3 and
  `round(0.125; 2)` is 0.13. It used to round halves to the even digit.
- **Divisions that never end are marked ≈.** `1/3` shows `≈ 0.333333333333333`,
  and `1/3*3`, `floor(1/3*3)` and `1/3*3 == 1` give 1. Approximate results show
  the 15 significant digits they can vouch for, so `sqrt(2)^2` reads `≈ 2`;
  `ans` and memory keep every digit for later calculations.
- Results that are provably exact no longer show ≈: `sqrt(144)`, `cbrt(-27)`,
  `log(1000)`, `log(8; 2)`, `ln(1)`, `exp(0)`, `acosh(1)`, and in degrees
  `sin(30)`, `cos(180)`, `tan(45)`, `asin(0.5)`, `atan(1)`. Rounding an
  approximate value to digits it can vouch for, such as `round(10/7; 2)`, is
  exact.
- `x` is an unknown name outside the graph, so `3x + 1` reports it instead of
  silently giving 1.
- F1 opens **How to use RustCalc**, a guide with worked examples for
  calculating, percent, exact and approximate results, functions, dates, memory
  and history, every workspace, the keyboard and privacy. The sidebar button is
  now **? Help**. The calculator keeps working while Help or About is open.
- The calculator explains percent under the result whenever the expression
  contains `%`, and the keypad note shows `100 + 10% = 110`.
- Clearer messages for `max(1, 2)` (separate arguments with `;`) and for
  thousands separators such as `1.000.000`; formula inputs such as `250.000`
  warn that they are read as 250.
- Dates must be written with a four-digit year; `21.10.26` is no longer read as
  the year 26.
- `ncr` and `npr` give 0 when choosing more items than there are, and `ncr`
  reaches every result that fits, such as `ncr(96; 48)`.
- `today()` uses the local date on Linux and macOS instead of the UTC date.
- New formulas start with labels in the interface language; an empty category
  is listed as Custom in either language.
- Turkish wording was reviewed throughout for grammar and consistent terms.
- Formula results copy as shown (988.86, not all 28 digits); history search
  accepts a decimal comma; Turkish searches match İ/ı either way.

### Fixed

- Pasting after a result appended to the old expression; it now starts a new
  calculation (or continues from `ans` when it begins with an operator), and a
  pasted `5+5=` calculates. `=` typed quickly together with other keys, or after
  `<`, `>` or `!`, is handled correctly; typing inside the previous expression
  edits it instead of clearing it.
- Esc in the history search cleared the expression; it now clears the search,
  and closes an open menu without clearing anything.
- Trigonometry on large degree inputs lost precision (`sin(360000000000030)`
  is 0.5), and an exact radian input could have its genuinely tiny sine
  rounded to 0.
- Unit conversions showed rounding residue (`36 km/h` is exactly 10 m/s;
  1 °F to °F stays 1).
- Graph tick labels could repeat the same value when zoomed in or far from 0.
- The travel time template could show "2 h 60 min"; unit prices of small
  amounts showed 0.00.
- Turkish category headings were upper-cased with English rules ("FINANS").
- One unreadable value in the saved state reset all history at the next
  autosave; the unreadable data is now kept in `app.ron` and the app says so.
- **Linux/macOS installer:** a CRLF `SHA256SUMS` made every install fail (the
  release manifest is now written with LF and the installer accepts both);
  uninstall left `RustCalc.app` behind when the install path contained `'`; a
  failed first install could not be retried; another program's `rustcalc` link
  was replaced and then deleted; reinstalling with `--no-link` forgot the earlier
  link; downloaded files stayed quarantined on macOS; and a `noexec` `/tmp`
  broke installs and updates, which now stage beside the program.
- On Linux, **Restart** after an in-app update, and a second update in the same
  session, failed because the replaced program's path was lost.
- On Windows, a mistyped command-line option now prints its error in the
  terminal; `--help` lists the workspace options; the window title no longer
  starts in Turkish for everyone.
- **Copy result** in the unit converter was not translated into Turkish.
- Help described history clicks incorrectly and still said only manual checks
  contact GitHub.

## 0.4.0 — Formulas and dates

### Added

- **Formulas** workspace with 21 ready-made templates in English and Turkish:
  loan payment, compound interest, savings plan, add/remove VAT, profit margin,
  pay, days between dates, add days, workday deadline, add months, age,
  discount, percentage change and share, bill split, fuel, travel time, unit
  prices, electricity cost and BMI. Results update as you type.
- Custom formulas: name inputs (numbers or dates) and results, write
  expressions that use inputs and earlier results, choose number, money,
  percent, date or weekday display, and see a validated live preview. Templates
  can be copied and customized. Formulas persist, and can be exported and
  imported as validated JSON files.
- Calculator engine: variables, `;`-separated arguments, `min`, `max`, `sum`,
  `avg`, `gcd`, `lcm`, `ncr`, `npr`, `mod`, `round(x; digits)`, `log(x; base)`,
  `sign`, comparisons (`< > <= >= == !=`), and short-circuit `if`, `and`, `or`,
  `not`.
- Date arithmetic on day numbers: `today`, `date`, `year`, `month`, `day`,
  `weekday`, `addmonths`, `addyears`, `addworkdays`, `months`, `years`,
  `workdays`, `endofmonth`.
- `rustcalc update --rollback` restores the executable an update replaced; run
  it again to undo. Self-update keeps the previous executable as
  `rustcalc.previous.exe`, which uninstall removes.
- Plot functions and the programmer word size are remembered across restarts.
- `--formulas` opens the new workspace.

### Changed

- Expressions are parsed once into a tree and then evaluated, so plots and
  formulas no longer re-parse text for every point. Long flat expressions stay
  flat, keeping evaluation depth bounded.
- Variable names may contain digits and `_` after the first letter.
- Help now lists the new functions and states the opt-in update behavior.

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
