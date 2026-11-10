# Localization

English is the default. Select Turkish with EN/TR or `rustcalc --lang tr`; the
choice is persisted. CLI output, scripts and developer documentation are English.

- `locales/en.json`: English source strings as keys and values.
- `locales/tr.json`: Turkish values with identical keys.
- `src/i18n.rs`: lookup and dynamic diagnostic templates.

Add each message to both catalogues and route it through `Language::text` or the
GPUI view's `tr` helper. Translate values before composing formatted labels.
Preserve mathematical identifiers, shortcuts and `{name}`/`{c}`/`{count}` placeholders.
Never use a mutable process-global locale; app/test instances must be isolated.

Labels, dialogs, units, tooltips, calculation errors and status messages are
localized. Unknown external OS/network diagnostics fall back to their original
English text. Add stable app-owned errors to both catalogues.

Tests cover key parity, nonempty values, diagnostic templates and interface
literals. Run `scripts/screenshots.ps1` on Windows to inspect the Turkish
workspace and settings screenshots.
`every_catalogue_message_is_still_used` fails when a message is no longer
referenced, so remove keys together with the code that used them. The Turkish
user guide is `docs/KULLANIM_KILAVUZU.md`; keep it in step with
`docs/USER_GUIDE.md`. Check compact/wide widths and both themes when changing translations.
