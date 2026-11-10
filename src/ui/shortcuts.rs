use super::app::{Calc, Page, Panel};
use gpui::{Context, Focusable, KeyDownEvent, Keystroke, Window};
use gpui_component::input::SelectAll;
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Command {
    Page(Page),
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Settings,
    Help,
    SelectExpression,
    CopyResult,
}

impl Command {
    pub const ALL: [Self; 12] = [
        Self::Page(Page::Calculator),
        Self::Page(Page::Converter),
        Self::Page(Page::Programmer),
        Self::Page(Page::Graph),
        Self::Page(Page::Formulas),
        Self::ZoomIn,
        Self::ZoomOut,
        Self::ZoomReset,
        Self::Settings,
        Self::Help,
        Self::SelectExpression,
        Self::CopyResult,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Page(page) => page.key(),
            Self::ZoomIn => "zoom_in",
            Self::ZoomOut => "zoom_out",
            Self::ZoomReset => "zoom_reset",
            Self::Settings => "settings",
            Self::Help => "help",
            Self::SelectExpression => "select_expression",
            Self::CopyResult => "copy_result",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Page(Page::Calculator) => "Calculator workspace",
            Self::Page(Page::Converter) => "Converter workspace",
            Self::Page(Page::Programmer) => "Programmer workspace",
            Self::Page(Page::Graph) => "Graph workspace",
            Self::Page(Page::Formulas) => "Formulas workspace",
            Self::ZoomIn => "Enlarge the text",
            Self::ZoomOut => "Shrink the text",
            Self::ZoomReset => "Reset the text size",
            Self::Settings => "Open settings",
            Self::Help => "Open help",
            Self::SelectExpression => "Select and edit the expression",
            Self::CopyResult => "Copy result",
        }
    }

    fn default(self) -> &'static str {
        match self {
            Self::Page(Page::Calculator) => "Ctrl+1",
            Self::Page(Page::Converter) => "Ctrl+2",
            Self::Page(Page::Programmer) => "Ctrl+3",
            Self::Page(Page::Graph) => "Ctrl+4",
            Self::Page(Page::Formulas) => "Ctrl+5",
            Self::ZoomIn => "Ctrl+Plus",
            Self::ZoomOut => "Ctrl+Minus",
            Self::ZoomReset => "Ctrl+0",
            Self::Settings => "Ctrl+Comma",
            Self::Help => "F1",
            Self::SelectExpression => "Ctrl+L",
            Self::CopyResult => "Ctrl+Shift+C",
        }
    }
}

pub(super) fn binding(saved: &BTreeMap<String, String>, command: Command) -> Option<&str> {
    let value = saved
        .get(command.id())
        .map_or(command.default(), String::as_str);
    (!value.is_empty()).then_some(value)
}

fn set(saved: &mut BTreeMap<String, String>, command: Command, key: Option<&str>) {
    if key == Some(command.default()) {
        saved.remove(command.id());
    } else {
        saved.insert(command.id().into(), key.unwrap_or_default().into());
    }
}

pub(super) fn assign(saved: &mut BTreeMap<String, String>, command: Command, key: Option<&str>) {
    if key.is_some() {
        for other in Command::ALL {
            if other != command && binding(saved, other) == key {
                set(saved, other, None);
            }
        }
    }
    set(saved, command, key);
}

pub(super) fn display(key: &str) -> String {
    let mut text = if cfg!(target_os = "macos") {
        key.replace("Ctrl+", "Cmd+")
    } else {
        key.to_owned()
    };
    for (name, symbol) in [
        ("Comma", ","),
        ("Period", "."),
        ("Plus", "+"),
        ("Minus", "−"),
    ] {
        if let Some(prefix) = text.strip_suffix(name) {
            text = format!("{prefix}{symbol}");
        }
    }
    text
}

fn key_name(keystroke: &Keystroke) -> String {
    let m = keystroke.modifiers;
    let key = match keystroke.key.as_str() {
        "=" | "+" if m.platform || m.control => "Plus".into(),
        "-" => "Minus".into(),
        "," => "Comma".into(),
        "." => "Period".into(),
        "left" => "ArrowLeft".into(),
        "right" => "ArrowRight".into(),
        "up" => "ArrowUp".into(),
        "down" => "ArrowDown".into(),
        "pageup" => "PageUp".into(),
        "pagedown" => "PageDown".into(),
        "backspace" => "Backspace".into(),
        "delete" => "Delete".into(),
        "home" => "Home".into(),
        "end" => "End".into(),
        "space" => "Space".into(),
        key => key.to_ascii_uppercase(),
    };
    let mut prefix = String::new();
    if m.platform || m.control {
        prefix.push_str("Ctrl+");
    }
    if m.alt {
        prefix.push_str("Alt+");
    }
    if m.shift && key != "Plus" {
        prefix.push_str("Shift+");
    }
    prefix + &key
}

impl Calc {
    pub fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.handle_key(&event.keystroke, window, cx) {
            window.prevent_default();
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn handle_key(
        &mut self,
        keystroke: &Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = keystroke.key.as_str();
        let m = keystroke.modifiers;
        if key == "tab" && !m.control && !m.platform && !m.alt && self.shortcut_capture.is_none() {
            if m.shift {
                window.focus_prev();
            } else {
                window.focus_next();
            }
            return true;
        }
        let name = key_name(keystroke);
        if let Some(command) = self.shortcut_capture {
            if key == "escape" {
                self.shortcut_capture = None;
            } else if key == "delete" && m == gpui::Modifiers::default() {
                assign(&mut self.notebook.shortcuts, command, None);
                self.shortcut_capture = None;
                self.save();
            } else if !matches!(
                key,
                "tab" | "enter" | "shift" | "control" | "alt" | "cmd" | "win"
            ) && (m.control
                || m.platform
                || m.alt
                || key
                    .strip_prefix('f')
                    .is_some_and(|s| s.parse::<u8>().is_ok()))
            {
                assign(&mut self.notebook.shortcuts, command, Some(&name));
                self.shortcut_capture = None;
                self.save();
            }
            return true;
        }
        if let Some(command) = Command::ALL
            .into_iter()
            .find(|&command| binding(&self.notebook.shortcuts, command) == Some(&name))
        {
            match command {
                Command::Page(page) => self.open_page(page, window, cx),
                Command::Settings => self.open_panel(Panel::Settings, window, cx),
                Command::Help => self.open_panel(Panel::Help, window, cx),
                Command::ZoomIn => self.notebook.step_zoom(1),
                Command::ZoomOut => self.notebook.step_zoom(-1),
                Command::ZoomReset => self.notebook.zoom = 1.,
                Command::SelectExpression => {
                    self.open_page(Page::Calculator, window, cx);
                    self.committed = None;
                    window.dispatch_action(Box::new(SelectAll), cx);
                }
                Command::CopyResult => self.copy_result(cx),
            }
            return true;
        }
        if key == "escape" {
            if self.pending_history.take().is_some() {
                return true;
            }
            if self.search.read(cx).focus_handle(cx).is_focused(window)
                && !self.search.read(cx).value().is_empty()
            {
                Self::set(&self.search, "", window, cx);
                return true;
            }
            if self.panel != Panel::Workspace {
                self.open_panel(self.panel, window, cx);
                return true;
            }
            if self.draft_form.take().is_some() || self.unit_menu.take().is_some() {
                return true;
            }
            if self.page == Page::Calculator {
                self.clear_expression(window, cx);
                return true;
            }
        }
        if self.panel == Panel::Workspace
            && self.page == Page::Graph
            && self.focus.is_focused(window)
            && !m.control
            && !m.platform
            && !m.alt
        {
            match key {
                "home" => {
                    self.view = rustcalc::plot::Viewport::default();
                    return true;
                }
                "=" | "+" => {
                    self.zoom_graph(0.8);
                    return true;
                }
                "-" => {
                    self.zoom_graph(1.25);
                    return true;
                }
                _ => {}
            }
            let pan = match key {
                "left" => Some((-0.1, 0.)),
                "right" => Some((0.1, 0.)),
                "up" => Some((0., 0.1)),
                "down" => Some((0., -0.1)),
                _ => None,
            };
            if let Some((dx, dy)) = pan {
                self.view.pan(dx, dy);
                return true;
            }
        }
        if self.panel != Panel::Workspace
            || self.page != Page::Calculator
            || self.search.read(cx).focus_handle(cx).is_focused(window)
        {
            return false;
        }
        let editor = self.expression.read(cx).focus_handle(cx).is_focused(window);
        if !editor && !self.focus.is_focused(window) {
            return false;
        }
        if (m.control || m.platform) && !m.shift && !m.alt && key == "v" && self.committed.is_some()
        {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                self.insert(text.lines().next().unwrap_or_default(), window, cx);
            }
            return true;
        }
        if m.control || m.platform || m.alt {
            return false;
        }
        if key == "enter" && !editor {
            self.calculate(window, cx);
            return true;
        }
        let text = keystroke
            .key_char
            .as_deref()
            .or_else(|| (key.len() == 1).then_some(key));
        if text == Some("=") {
            let expression = self.expression.read(cx).value();
            let comparison = expression.trim_end().ends_with(['<', '>', '!', '=']);
            let assignment = rustcalc::model::check_variable_name(expression.trim()).is_ok();
            if self.committed.is_some() || !(comparison || assignment) {
                self.calculate(window, cx);
                return true;
            }
        }
        if let Some(text) = text.filter(|text| !text.chars().any(char::is_control))
            && (self.committed.is_some() || !editor)
        {
            self.insert(text, window, cx);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigning_an_existing_shortcut_unbinds_the_previous_command() {
        let mut saved = BTreeMap::new();
        assign(&mut saved, Command::Help, Some("Ctrl+1"));
        assert_eq!(binding(&saved, Command::Page(Page::Calculator)), None);
        assert_eq!(binding(&saved, Command::Help), Some("Ctrl+1"));
        assign(&mut saved, Command::Help, Some("F1"));
        assert!(!saved.contains_key("help"));
    }
}
