# Architecture

RustCalc uses native eframe/egui with OpenGL. There is no WebView, JavaScript
runtime or server. Segoe UI is used on Windows with bundled egui font fallbacks.

| Module | Responsibility |
| --- | --- |
| `main.rs` | Entry point, native window and restart after shutdown |
| `cli.rs` | Version/help, arguments and explicit update commands |
| `engine.rs` | Bounded Pratt parser, decimals and scientific functions |
| `model.rs` | History, pins, answer, memory and persistent preferences |
| `history.rs` | Versioned backups, bounded validation and atomic file writes |
| `units.rs` | Defined unit factors, offsets and conversion validation |
| `i18n.rs` | Per-instance language, catalogues and diagnostic templates |
| `updater.rs` | Release selection, bounded fetching, integrity and replacement |
| `ui.rs` | Workspaces, input, dialogs, appearance and worker state |

The engine has no GUI/network dependency. It uses bounded decimal precision and
marks scientific approximations. The model retains approximation state through
answer/memory operations. The library API is experimental during 0.x.

Preview does not write history. A committed result is stable rather than
reevaluating `ans` against its own new answer. Keypad operations use the editor's
cursor/selection. History is limited to 500 entries, preserving pins preferentially;
if all entries are pinned, the oldest leaves when another is added.

eframe persists a serde/RON notebook under `notebook-v1`; new settings use defaults
for compatibility. Screenshot mode does not save notebook/window/egui state.
Updates replace the executable rather than the data file.

Deletion undo records removed entries and their original positions, rather than
restoring an old notebook snapshot. New calculations, answer and memory therefore
survive undo. The 20-action stack is session-only. Imports validate and merge a
clone before committing; changed imports reset deletion undo to avoid restoring
duplicates from an older baseline.

History file dialogs and I/O use a separate worker with an mpsc completion
channel. Backup imports return validated entries for an explicit UI confirmation;
they never evaluate stored expressions. JSON format version 1 contains only
history, uses a 4 MiB read limit and requires at most 500 valid entries. Exports
stage and sync a temporary file in the destination directory before replacement.

GUI updates use one worker and an mpsc channel, with checking/installing/result
states. Fetching and final replacement are injectable boundaries so tests can
exercise integrity failures without network traffic or mutating their executable.

`scripts/package.ps1` writes portable ZIPs and `SHA256SUMS`; installer/updater
consume the same binary name. Installer tests use real local release fixtures.
