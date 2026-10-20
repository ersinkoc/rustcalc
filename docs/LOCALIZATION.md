# Localization

English is the default. Select Turkish with EN/TR or `rustcalc --lang tr`; the
choice is persisted. CLI output, scripts and developer documentation are English.

- `locales/en.json`: English source strings as keys and values.
- `locales/tr.json`: Turkish values with identical keys.
- `src/i18n.rs`: lookup and dynamic diagnostic templates.

Add each message to both catalogues and route it through `Language::text` or the
UI palette's `tr` helper. Translate values before composing formatted labels.
Preserve mathematical identifiers, shortcuts and `{name}`/`{c}`/`{count}` placeholders.
Never use a mutable process-global locale; app/test instances must be isolated.

Labels, dialogs, units, tooltips, calculation errors and status messages are
localized. Unknown external OS/network diagnostics fall back to their original
English text. Add stable app-owned errors to both catalogues.

Tests cover key parity, nonempty values, templates, persistence and live GUI
switching. Check compact/wide widths and both themes when changing translations.
