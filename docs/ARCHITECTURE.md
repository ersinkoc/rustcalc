# Architecture

RustCalc renders native [GPUI](https://gpui.rs/) elements with Direct3D on
Windows, Vulkan on Linux, and Metal on macOS. GPUI Component provides the text
editor and RustCalc owns the workspaces, controls and palettes. The calculation
engine and notebook can also run independently of a window.

GPUI 0.2.2 requires exactly Taffy 0.9.0. The local `compat/taffy` facade
forwards its public API and features to upstream Taffy 0.9.2 at a pinned
commit. This selects grid 1.0.1 with the upstream integer-overflow fix.
Remove the facade when GPUI accepts the maintained Taffy dependency.

| Module | Responsibility |
| --- | --- |
| `main.rs` | Entry point, native window and restart after shutdown |
| `cli.rs` | Version/help, arguments and explicit update commands |
| `engine.rs` | Bounded Pratt parser to an expression tree, variables, decimals, functions |
| `dates.rs` | Proleptic Gregorian day numbers, month/workday arithmetic, parsing |
| `formulas.rs` | Formula schema, bundled templates, validation, evaluation, display, files |
| `model.rs` | History, pins, answer, memory and persistent preferences |
| `history.rs` | Versioned backups, bounded validation and atomic file writes |
| `units.rs` | Defined unit factors, offsets and conversion validation |
| `plot.rs` | Plot viewport, pan/zoom, ticks, sampling, clipping and roots |
| `programmer.rs` | Fixed-width signed/unsigned parsing, formatting and wrapping ops |
| `i18n.rs` | Per-instance language, catalogues and diagnostic templates |
| `updater.rs` | Release selection, bounded fetching, integrity and replacement |
| `ui/app.rs` | GPUI window, navigation, adaptive layout and input entities |
| `ui/calculator.rs`, `ui/workspaces.rs` | Calculator, history, conversion, programmer and graph |
| `ui/formulas.rs` | Formulas workspace, editor and formula file worker |
| `ui/settings.rs`, `ui/shortcuts.rs` | Appearance, key bindings and background update jobs |
| `ui/storage.rs` | Atomic reads and writes of the existing app notebook |

`Expression::parse` builds a tree once; `eval` runs it against a context and a
variable map. Operators handled by one parser level are stored as a flat chain,
so a 1000-term sum is one node with 999 steps rather than a deep tree, and
evaluation depth is bounded by the 64-level nesting limit. When `+` or `-` is
directly followed by an operand ending in `%` (optionally negated), the evaluator
adds or subtracts that share of the left value, so `100 + 10%` is 110; any other
`%` divides by 100. Scientific results carry ≈ unless `proven_exact` verifies them
in decimal arithmetic (perfect roots, powers of ten, whole-degree angles with
sines of 0, ±½ or ±1). `if`, `and` and `or`
evaluate lazily. Dates are `i64` day numbers since 1970-01-01, so ordinary
arithmetic works on them; `today()` reads the context (tests pin it) or the
local date.

Formulas are data: `templates/formulas.json` holds the bundled templates in the
same schema as user formulas, which are stored in the notebook. Validation
checks identifiers, reserved names, duplicates, lengths and that each result
only references inputs or earlier results. Evaluation records failed values so
dependent results report a dependency error instead of an unknown name.

The engine has no GUI/network dependency. It uses bounded decimal precision and
marks scientific approximations. The model retains approximation state through
answer/memory operations. The library API is experimental during 0.x.

Preview does not write history. A committed result is stable rather than
reevaluating `ans` against its own new answer. Keypad operations use the editor's
cursor/selection. History is limited to 500 entries, preserving pins preferentially;
if all entries are pinned, the oldest leaves when another is added.

The notebook is stored in RON under `notebook-v1` in the existing `app.ron`.
The GPUI application retains the other keys so a previous version can read
the file after a rollback. Demo and screenshot sessions use a temporary
in-memory notebook. Updates replace the executable and keep the data file.

Deletion undo records removed entries and their original positions, rather than
restoring an old notebook snapshot. New calculations, answer and memory therefore
survive undo. The 20-action stack is session-only. Imports validate and merge a
clone before committing; changed imports reset deletion undo to avoid restoring
duplicates from an older baseline.

History file dialogs and I/O run on GPUI's background executor. Backup imports
return validated entries for an explicit UI confirmation;
they never evaluate stored expressions. JSON format version 1 contains only
history, uses a 4 MiB read limit and requires at most 500 valid entries. Exports
stage and sync a temporary file in the destination directory before replacement.

Plotting keeps math out of the GUI. `plot::sample` evaluates one point per pixel
and bisects jumps larger than half the view span: a continuous curve's jump
shrinks with each halving, while a pole or step keeps its size and splits the
curve. Segments are clipped to the view in f64 before conversion to screen
coordinates, so very large values never reach f32 painting.

The programmer workspace stores a raw bit pattern and a `Word` (width and
signedness). DEC parsing checks the word's numeric range; other bases describe
patterns that must fit. Operations compute in i128 and wrap to the word.

GUI updates use one worker and an mpsc channel, with checking/installing/result
states. Automatic checks reuse that worker through an injectable runner, gated by
`updater::auto_check_due`. Fetching and final replacement are injectable boundaries so tests can
exercise integrity failures without network traffic or mutating their executable.

`scripts/package.ps1` writes portable ZIPs and `SHA256SUMS`; installer/updater
consume the same binary name. Installer tests use real local release fixtures.
