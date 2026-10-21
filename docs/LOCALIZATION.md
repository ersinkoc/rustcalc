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
switching. `turkish_screens_contain_no_untranslated_catalogue_text` renders every
workspace, the help guide (all sections open), About, the formula editor and the
update prompt in Turkish and fails if any catalogue text appears untranslated.
`every_catalogue_message_is_still_used` fails when a message is no longer
referenced, so remove keys together with the code that used them. The Turkish
user guide is `docs/KULLANIM_KILAVUZU.md`; keep it in step with
`docs/USER_GUIDE.md`. Check compact/wide widths and both themes when changing translations.
