# History backups and undo

Open **History → Files…** in either the sidebar or compact history window.
Save backup writes JSON; Export as text writes a readable report; Import backup
opens a JSON backup and asks for confirmation before merging it. Native file
dialogs and disk I/O run outside the UI thread. Cancelling changes nothing.
Exports use a snapshot captured when the command is selected.

## Backup format

```json
{
  "format": "rustcalc-history",
  "version": 1,
  "entries": [
    {
      "expression": "2+3",
      "result": "5",
      "approximate": false,
      "angle": "Degrees",
      "pinned": true,
      "timestamp": 0
    }
  ]
}
```

Timestamps are Unix seconds. Angle is `Degrees` or `Radians`; results are decimal
strings, preserving precision. Backups store history only, not current answer,
memory, language or appearance. Text reports cannot be imported as backups.
UTF-8 JSON with or without a byte-order mark is accepted.

Import validates the format/version, file size (4 MiB), record count (500),
expressions (nonempty, at most 4096 characters, no control characters) and decimal
results. It never evaluates stored expressions, including `ans` or `mem`.
Malformed/unsupported files and merges over 500 records leave existing history,
pins and undo unchanged. Saving stages and syncs a temporary file beside the
destination before atomically replacing it; a failed save retains the original.

## Merging

Matching uses expression, stored result, approximation marker, angle and timestamp.
Each existing occurrence is matched once: repeated calculations in the same
second keep their original multiplicity. Importing the same backup again does
not grow history. A pin from either copy is kept; imports never unpin a record.
Existing records stay in order and new occurrences are appended in backup order.
No existing record is silently pruned to make room for an import.

## Deletion undo

Individual deletion and **Clear → Clear history** can be reversed with
**Undo deletion**. Clear keeps pinned entries. Undo restores removed records at
their previous positions while keeping calculations created afterward; it does
not restore an old answer or memory value. The last 20 deletion actions are kept
only for the current session. Restarting clears this undo stack.

An import that changes history starts a new undo baseline, preventing older
deletions from restoring duplicates already present in the backup. An unchanged
import keeps undo. If restoration would exceed 500 records, undo reports the
limit and keeps both history and the pending undo action unchanged.
