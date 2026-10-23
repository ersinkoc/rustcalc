//! Keyboard shortcuts the user can change in Settings. Only the changes are
//! saved, as `command id -> "Ctrl+Shift+C"`; an empty text removes a shortcut.
use eframe::egui::{Key, KeyboardShortcut, Modifiers};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Calculator,
    Converter,
    Programmer,
    Graph,
    Formulas,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Settings,
    Help,
    SelectExpression,
    CopyResult,
}

impl Command {
    pub const ALL: [Command; 12] = [
        Command::Calculator,
        Command::Converter,
        Command::Programmer,
        Command::Graph,
        Command::Formulas,
        Command::ZoomIn,
        Command::ZoomOut,
        Command::ZoomReset,
        Command::Settings,
        Command::Help,
        Command::SelectExpression,
        Command::CopyResult,
    ];
    /// The saved name; it never changes, unlike the title.
    pub fn id(self) -> &'static str {
        match self {
            Command::Calculator => "calculator",
            Command::Converter => "converter",
            Command::Programmer => "programmer",
            Command::Graph => "graph",
            Command::Formulas => "formulas",
            Command::ZoomIn => "zoom_in",
            Command::ZoomOut => "zoom_out",
            Command::ZoomReset => "zoom_reset",
            Command::Settings => "settings",
            Command::Help => "help",
            Command::SelectExpression => "select_expression",
            Command::CopyResult => "copy_result",
        }
    }
    /// The English catalogue text shown in Settings and Help.
    pub fn title(self) -> &'static str {
        match self {
            Command::Calculator => "Calculator workspace",
            Command::Converter => "Converter workspace",
            Command::Programmer => "Programmer workspace",
            Command::Graph => "Graph workspace",
            Command::Formulas => "Formulas workspace",
            Command::ZoomIn => "Enlarge the text",
            Command::ZoomOut => "Shrink the text",
            Command::ZoomReset => "Reset the text size",
            Command::Settings => "Open settings",
            Command::Help => "Open help",
            Command::SelectExpression => "Select and edit the expression",
            Command::CopyResult => "Copy result",
        }
    }
    pub fn default_shortcut(self) -> KeyboardShortcut {
        let command = |key| KeyboardShortcut::new(Modifiers::COMMAND, key);
        match self {
            Command::Calculator => command(Key::Num1),
            Command::Converter => command(Key::Num2),
            Command::Programmer => command(Key::Num3),
            Command::Graph => command(Key::Num4),
            Command::Formulas => command(Key::Num5),
            Command::ZoomIn => command(Key::Plus),
            Command::ZoomOut => command(Key::Minus),
            Command::ZoomReset => command(Key::Num0),
            Command::Settings => command(Key::Comma),
            Command::Help => KeyboardShortcut::new(Modifiers::NONE, Key::F1),
            Command::SelectExpression => command(Key::L),
            Command::CopyResult => {
                KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::C)
            }
        }
    }
}

/// The shortcut in effect: the saved change, or else the default.
pub fn binding(saved: &BTreeMap<String, String>, command: Command) -> Option<KeyboardShortcut> {
    match saved.get(command.id()) {
        Some(text) => parse(text),
        None => Some(command.default_shortcut()),
    }
}

/// Assigns `shortcut` (or removes the shortcut with `None`), taking it away
/// from any other command. Returns the command that lost it.
pub fn assign(
    saved: &mut BTreeMap<String, String>,
    command: Command,
    shortcut: Option<KeyboardShortcut>,
) -> Option<Command> {
    let mut taken = None;
    if let Some(shortcut) = shortcut {
        for other in Command::ALL {
            if other != command && binding(saved, other) == Some(shortcut) {
                set(saved, other, None);
                taken = Some(other);
            }
        }
    }
    set(saved, command, shortcut);
    taken
}

fn set(saved: &mut BTreeMap<String, String>, command: Command, shortcut: Option<KeyboardShortcut>) {
    if shortcut == Some(command.default_shortcut()) {
        saved.remove(command.id());
    } else {
        saved.insert(
            command.id().into(),
            shortcut.map(format).unwrap_or_default(),
        );
    }
}

/// The shortcut as people read it: egui's platform names, with the symbol
/// for punctuation (`Ctrl+,` rather than `Ctrl+Comma`). Arrows keep their names
/// because the interface font has no arrow symbols.
pub fn display(shortcut: KeyboardShortcut, formatted: String) -> String {
    let key = shortcut.logical_key;
    let symbol = key.symbol_or_name();
    let arrow = matches!(
        key,
        Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp
    );
    match formatted.strip_suffix(key.name()) {
        Some(prefix) if !arrow => format!("{prefix}{symbol}"),
        _ => formatted,
    }
}

/// The saved form: `Ctrl` stands for Cmd on a Mac, as it does in egui.
pub fn format(shortcut: KeyboardShortcut) -> String {
    let m = shortcut.modifiers;
    let mut text = String::new();
    for (on, name) in [
        (m.command || m.ctrl || m.mac_cmd, "Ctrl"),
        (m.alt, "Alt"),
        (m.shift, "Shift"),
    ] {
        if on {
            text += name;
            text += "+";
        }
    }
    text + shortcut.logical_key.name()
}

pub fn parse(text: &str) -> Option<KeyboardShortcut> {
    let mut parts: Vec<&str> = text.split('+').map(str::trim).collect();
    // "Ctrl++" splits into an empty part before the key.
    if text.ends_with("++") {
        parts.pop();
        parts.pop();
        parts.push("Plus");
    }
    let key = Key::from_name(parts.pop()?)?;
    let mut modifiers = Modifiers::NONE;
    for part in parts {
        match part {
            "Ctrl" => modifiers |= Modifiers::COMMAND,
            "Alt" => modifiers |= Modifiers::ALT,
            "Shift" => modifiers |= Modifiers::SHIFT,
            _ => return None,
        }
    }
    Some(KeyboardShortcut::new(modifiers, key))
}

/// A key press that may become a shortcut: it needs Ctrl or Alt, or a function
/// key, so typing a plain letter or digit never triggers a command.
pub fn acceptable(shortcut: KeyboardShortcut) -> bool {
    let m = shortcut.modifiers;
    let function_key = shortcut
        .logical_key
        .name()
        .strip_prefix('F')
        .is_some_and(|n| n.parse::<u8>().is_ok());
    (m.command || m.ctrl || m.mac_cmd || m.alt || function_key)
        && !matches!(shortcut.logical_key, Key::Escape | Key::Tab | Key::Enter)
}

/// One form for the same keys: Ctrl arrives as both `ctrl` and `command`
/// (Cmd on a Mac), `=` is the unshifted `+` on many keyboards, and the Shift
/// that makes `+` is no choice.
pub fn normalize(shortcut: KeyboardShortcut) -> KeyboardShortcut {
    let m = shortcut.modifiers;
    let mut modifiers = Modifiers::NONE;
    if m.command || m.ctrl || m.mac_cmd {
        modifiers |= Modifiers::COMMAND;
    }
    if m.alt {
        modifiers |= Modifiers::ALT;
    }
    let key = match shortcut.logical_key {
        Key::Equals => Key::Plus,
        key => key,
    };
    if m.shift && key != Key::Plus {
        modifiers |= Modifiers::SHIFT;
    }
    KeyboardShortcut::new(modifiers, key)
}

/// The command a key press triggers: the most specific matching shortcut.
pub fn command_for(
    saved: &BTreeMap<String, String>,
    key: Key,
    modifiers: Modifiers,
) -> Option<Command> {
    in_match_order(saved)
        .into_iter()
        .find(|(_, s)| {
            let same_key =
                s.logical_key == key || (s.logical_key == Key::Plus && key == Key::Equals);
            same_key && modifiers.matches_logically(s.modifiers)
        })
        .map(|(command, _)| command)
}

/// The commands ordered so that a shortcut with more modifiers is matched
/// first: egui ignores extra Shift and Alt, so Ctrl+C would also catch Ctrl+Shift+C.
pub fn in_match_order(saved: &BTreeMap<String, String>) -> Vec<(Command, KeyboardShortcut)> {
    let mut all: Vec<_> = Command::ALL
        .into_iter()
        .filter_map(|c| Some((c, binding(saved, c)?)))
        .collect();
    all.sort_by_key(|(_, s)| std::cmp::Reverse(s.modifiers.shift as u8 + s.modifiers.alt as u8));
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_round_trip_through_their_saved_text() {
        for command in Command::ALL {
            let shortcut = command.default_shortcut();
            assert_eq!(parse(&format(shortcut)), Some(shortcut), "{command:?}");
        }
        assert_eq!(
            format(Command::CopyResult.default_shortcut()),
            "Ctrl+Shift+C"
        );
        assert_eq!(parse("Ctrl++"), Some(Command::ZoomIn.default_shortcut()));
        assert_eq!(
            parse("Alt+F7"),
            Some(KeyboardShortcut::new(Modifiers::ALT, Key::F7))
        );
        assert_eq!(parse("Hyper+K"), None);
        assert_eq!(parse(""), None);
    }
    #[test]
    fn assigning_a_taken_shortcut_moves_it_and_defaults_are_not_saved() {
        let mut saved = BTreeMap::new();
        let ctrl_l = Command::SelectExpression.default_shortcut();
        assert_eq!(
            assign(&mut saved, Command::Help, Some(ctrl_l)),
            Some(Command::SelectExpression)
        );
        assert_eq!(binding(&saved, Command::Help), Some(ctrl_l));
        assert_eq!(binding(&saved, Command::SelectExpression), None);
        assert_eq!(saved["select_expression"], "");
        assign(
            &mut saved,
            Command::Help,
            Some(Command::Help.default_shortcut()),
        );
        assign(&mut saved, Command::SelectExpression, Some(ctrl_l));
        assert!(saved.is_empty(), "{saved:?}");
    }
    #[test]
    fn punctuation_is_shown_as_its_symbol() {
        let show = |s: KeyboardShortcut| display(s, format(s));
        assert_eq!(show(Command::Settings.default_shortcut()), "Ctrl+,");
        assert_eq!(show(Command::ZoomIn.default_shortcut()), "Ctrl++");
        assert_eq!(show(Command::CopyResult.default_shortcut()), "Ctrl+Shift+C");
        assert_eq!(
            show(KeyboardShortcut::new(Modifiers::ALT, Key::ArrowUp)),
            "Alt+Up"
        );
    }
    #[test]
    fn plain_keys_cannot_become_shortcuts() {
        let plain = |key| KeyboardShortcut::new(Modifiers::NONE, key);
        assert!(!acceptable(plain(Key::A)));
        assert!(!acceptable(plain(Key::Num5)));
        assert!(!acceptable(KeyboardShortcut::new(Modifiers::SHIFT, Key::A)));
        assert!(!acceptable(KeyboardShortcut::new(
            Modifiers::COMMAND,
            Key::Escape
        )));
        assert!(acceptable(plain(Key::F5)));
        assert!(acceptable(KeyboardShortcut::new(Modifiers::ALT, Key::A)));
        assert!(acceptable(KeyboardShortcut::new(
            Modifiers::COMMAND,
            Key::K
        )));
    }
    #[test]
    fn shortcuts_with_shift_are_matched_first() {
        let mut saved = BTreeMap::new();
        assign(
            &mut saved,
            Command::Help,
            Some(KeyboardShortcut::new(Modifiers::COMMAND, Key::C)),
        );
        let order = in_match_order(&saved);
        let copy = order
            .iter()
            .position(|(c, _)| *c == Command::CopyResult)
            .unwrap();
        let help = order.iter().position(|(c, _)| *c == Command::Help).unwrap();
        assert!(copy < help);
    }
    #[test]
    fn key_presses_find_the_most_specific_command() {
        let saved = BTreeMap::new();
        let ctrl = Modifiers::COMMAND;
        assert_eq!(
            command_for(&saved, Key::C, ctrl | Modifiers::SHIFT),
            Some(Command::CopyResult)
        );
        assert_eq!(command_for(&saved, Key::C, ctrl), None);
        assert_eq!(
            command_for(&saved, Key::Equals, ctrl),
            Some(Command::ZoomIn)
        );
        assert_eq!(
            command_for(&saved, Key::Plus, ctrl | Modifiers::SHIFT),
            Some(Command::ZoomIn)
        );
        assert_eq!(command_for(&saved, Key::Num3, Modifiers::NONE), None);
        assert_eq!(
            normalize(KeyboardShortcut::new(ctrl | Modifiers::SHIFT, Key::Equals)),
            Command::ZoomIn.default_shortcut()
        );
        assert_eq!(
            normalize(KeyboardShortcut::new(Modifiers::CTRL | ctrl, Key::L)),
            Command::SelectExpression.default_shortcut()
        );
    }
    #[test]
    fn every_title_has_a_catalogue_entry() {
        let en: BTreeMap<String, String> =
            serde_json::from_str(include_str!("../../locales/en.json")).unwrap();
        for command in Command::ALL {
            assert!(en.contains_key(command.title()), "{}", command.title());
        }
    }
}
