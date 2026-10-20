use eframe::egui::{self, Color32, FontId, RichText, Stroke};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rustcalc::{
    engine::{self, Angle, Context, Value},
    history,
    i18n::Language,
    model::{Entry, Notebook},
    units,
    updater::{self, UpdatePlan},
};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

enum UpdateStatus {
    Idle,
    Checking,
    Available(Box<UpdatePlan>),
    Installing,
    Current,
    Installed,
    Failed(String),
}
enum UpdateReply {
    Checked(Result<Option<UpdatePlan>, String>),
    Installed(Result<(), String>),
}
#[derive(Clone, Copy)]
enum HistoryFileAction {
    Backup,
    Text,
    Import,
}
enum HistoryFileReply {
    Saved(Result<bool, String>),
    Loaded(Result<Option<Vec<Entry>>, String>),
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Page {
    Calculator,
    Converter,
    Programmer,
    Graph,
}

#[derive(Clone, Copy)]
struct Palette {
    language: Language,
    bg: Color32,
    side: Color32,
    card: Color32,
    key: Color32,
    border: Color32,
    text: Color32,
    muted: Color32,
    accent: Color32,
    ink: Color32,
    danger: Color32,
}
impl Palette {
    fn tr(self, text: impl AsRef<str>) -> String {
        self.language.text(text)
    }
    fn new(light: bool, language: Language) -> Self {
        if light {
            Self {
                language,
                bg: rgb(244, 246, 243),
                side: rgb(234, 238, 233),
                card: Color32::WHITE,
                key: rgb(255, 255, 255),
                border: rgb(209, 217, 208),
                text: rgb(24, 40, 33),
                muted: rgb(85, 103, 91),
                accent: rgb(42, 108, 76),
                ink: Color32::WHITE,
                danger: rgb(176, 45, 53),
            }
        } else {
            Self {
                language,
                bg: rgb(21, 25, 25),
                side: rgb(17, 21, 21),
                card: rgb(28, 33, 33),
                key: rgb(38, 44, 44),
                border: rgb(48, 58, 56),
                text: rgb(237, 241, 233),
                muted: rgb(151, 167, 158),
                accent: rgb(183, 230, 180),
                ink: rgb(23, 49, 32),
                danger: rgb(255, 155, 148),
            }
        }
    }
}
fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}
fn label(text: impl Into<String>, size: f32, color: Color32, language: Language) -> RichText {
    RichText::new(language.text(text.into()))
        .size(size)
        .color(color)
}
fn card(p: Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0_f32, p.border))
        .corner_radius(12)
        .inner_margin(20)
}

pub struct RustCalc {
    notebook: Notebook,
    page: Page,
    expression: String,
    committed: bool,
    error: Option<String>,
    search: String,
    only_pinned: bool,
    history: bool,
    history_popup: bool,
    history_file_worker: Option<Receiver<HistoryFileReply>>,
    pending_import: Option<Vec<Entry>>,
    history_error: Option<String>,
    scientific: bool,
    help: bool,
    about: bool,
    update_status: UpdateStatus,
    update_worker: Option<Receiver<UpdateReply>>,
    confirm_clear: bool,
    focus: bool,
    reset_cursor: bool,
    insertion: Option<String>,
    notice: Option<(String, Instant)>,
    category: usize,
    from: usize,
    to: usize,
    amount: String,
    integer: String,
    radix: u32,
    operand: String,
    graph: String,
    graph_range: f64,
    screenshot: Option<String>,
    screenshot_frame: u32,
    initial_size: Option<egui::Vec2>,
    #[cfg(test)]
    targets: std::collections::HashMap<String, egui::Rect>,
}
impl Default for RustCalc {
    fn default() -> Self {
        Self {
            notebook: Notebook::default(),
            page: Page::Calculator,
            expression: String::new(),
            committed: false,
            error: None,
            search: String::new(),
            only_pinned: false,
            history: true,
            history_popup: false,
            history_file_worker: None,
            pending_import: None,
            history_error: None,
            scientific: true,
            help: false,
            about: false,
            update_status: UpdateStatus::Idle,
            update_worker: None,
            confirm_clear: false,
            focus: true,
            reset_cursor: false,
            insertion: None,
            notice: None,
            category: 0,
            from: 0,
            to: 1,
            amount: "1000".into(),
            integer: "255".into(),
            radix: 10,
            operand: "15".into(),
            graph: "sin(x)".into(),
            graph_range: 10.0,
            screenshot: None,
            screenshot_frame: 0,
            initial_size: None,
            #[cfg(test)]
            targets: Default::default(),
        }
    }
}
impl RustCalc {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        // Use the OS typeface where available, retaining bundled fallback fonts.
        if let Ok(bytes) = std::fs::read("C:/Windows/Fonts/segoeui.ttf") {
            fonts
                .font_data
                .insert("Segoe UI".into(), egui::FontData::from_owned(bytes).into());
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "Segoe UI".into());
        }
        cc.egui_ctx.set_fonts(fonts);
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 8.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(15.0));
        cc.egui_ctx.set_style(style);
        let args: Vec<String> = std::env::args().collect();
        let screenshot = args
            .windows(2)
            .find(|w| w[0] == "--screenshot")
            .map(|w| w[1].clone());
        let demo = args.iter().any(|s| s == "--demo");
        let initial_size = if args.iter().any(|s| s == "--compact") {
            Some(egui::vec2(760.0, 760.0))
        } else if screenshot.is_some() {
            Some(egui::vec2(1180.0, 820.0))
        } else {
            None
        };
        if screenshot.is_some() {
            cc.egui_ctx.memory_mut(|m| *m = egui::Memory::default());
        }
        let mut notebook: Notebook = if screenshot.is_some() {
            Notebook::default()
        } else {
            cc.storage
                .and_then(|s| eframe::get_value(s, "notebook-v1"))
                .unwrap_or_default()
        };
        if demo {
            for expr in [
                "1280 * 0.18",
                "sqrt(144) + 8^2",
                "sin(30)",
                "(2400 + 1800) * 1.2",
            ] {
                let _ = notebook.calculate(expr);
            }
            if let Some(e) = notebook.entries.first_mut() {
                e.pinned = true;
            }
        }
        let page = if args.iter().any(|s| s == "--converter") {
            Page::Converter
        } else if args.iter().any(|s| s == "--programmer") {
            Page::Programmer
        } else if args.iter().any(|s| s == "--graph") {
            Page::Graph
        } else {
            Page::Calculator
        };
        if args.iter().any(|s| s == "--light") {
            notebook.light = true;
        }
        if let Some(code) = args.windows(2).find(|w| w[0] == "--lang") {
            notebook.language = Language::parse(&code[1]).unwrap_or_default();
        }
        Self {
            notebook,
            page,
            expression: if demo {
                "(2400 + 1800) * 1.2".into()
            } else {
                String::new()
            },
            committed: demo,
            screenshot,
            initial_size,
            ..Self::default()
        }
    }
    fn palette(&self) -> Palette {
        Palette::new(self.notebook.light, self.notebook.language)
    }
    fn preview(&self) -> Result<Value, engine::CalcError> {
        if self.committed {
            return Ok(Value {
                number: self.notebook.context().ans,
                approximate: self.notebook.answer_approximate,
            });
        }
        engine::evaluate(&self.expression, self.notebook.context())
    }
    fn toast(&mut self, text: impl Into<String>) {
        self.notice = Some((text.into(), Instant::now()));
    }
    fn calculate(&mut self) {
        match self.notebook.calculate(&self.expression) {
            Ok(_) => {
                self.committed = true;
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e.message);
                self.committed = false;
            }
        }
        self.focus = true;
    }
    fn insert(&mut self, text: &str) {
        let continuation = matches!(text, "+" | "-" | "*" | "/" | "^" | "%" | "!" | "^2");
        if self.committed {
            self.expression = if continuation {
                "ans".into()
            } else {
                String::new()
            };
            self.committed = false;
            self.reset_cursor = true;
        }
        self.insertion = Some(text.into());
        self.error = None;
        self.focus = true;
    }
    fn navigation(&mut self, ctx: &egui::Context, p: Palette) {
        egui::SidePanel::left("navigation")
            .exact_width(178.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(p.side).inner_margin(18))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let (r, _) =
                        ui.allocate_exact_size(egui::vec2(29.0, 32.0), egui::Sense::hover());
                    ui.painter().rect_filled(r.shrink(1.0), 7, p.accent);
                    ui.painter().text(
                        r.center(),
                        egui::Align2::CENTER_CENTER,
                        "=",
                        FontId::proportional(27.0),
                        p.ink,
                    );
                    ui.label(label("rustcalc", 24.0, p.text, p.language).strong());
                });
                ui.add_space(4.0);
                ui.label(label("Room to think.", 12.0, p.muted, p.language));
                ui.add_space(43.0);
                ui.label(label("WORKSPACE", 10.0, p.muted, p.language).strong());
                ui.add_space(10.0);
                for (page, icon, text) in [
                    (Page::Calculator, "01", "Calculate"),
                    (Page::Converter, "02", "Convert units"),
                    (Page::Programmer, "03", "Programmer"),
                    (Page::Graph, "04", "Plot a graph"),
                ] {
                    let active = self.page == page;
                    let button = egui::Button::new(label(
                        format!("{icon}    {}", p.tr(text)),
                        15.0,
                        if active { p.accent } else { p.muted },
                        p.language,
                    ))
                    .fill(if active { p.card } else { Color32::TRANSPARENT })
                    .stroke(if active {
                        Stroke::new(1.0_f32, p.border)
                    } else {
                        Stroke::NONE
                    })
                    .corner_radius(7);
                    if ui.add_sized([142.0, 44.0], button).clicked() {
                        self.page = page;
                        self.error = None;
                        self.focus = true;
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(label(
                        format!(
                            "v{}  /  {}",
                            env!("CARGO_PKG_VERSION"),
                            p.tr("BUILT WITH RUST")
                        ),
                        10.0,
                        p.muted,
                        p.language,
                    ));
                    ui.horizontal(|ui| {
                        let en = ui.selectable_value(
                            &mut self.notebook.language,
                            Language::English,
                            "EN",
                        );
                        let tr = ui.selectable_value(
                            &mut self.notebook.language,
                            Language::Turkish,
                            "TR",
                        );
                        #[cfg(test)]
                        {
                            self.targets.insert("language_en".into(), en.rect);
                            self.targets.insert("language_tr".into(), tr.rect);
                        }
                        #[cfg(not(test))]
                        let _ = (en, tr);
                        ui.label(label("Language", 11.0, p.muted, p.language));
                    });
                    if ui
                        .add_sized(
                            [142.0, 30.0],
                            egui::Button::new(label("About & updates", 12.0, p.muted, p.language))
                                .frame(false),
                        )
                        .clicked()
                    {
                        self.about = true;
                    }
                    ui.add_space(12.0);
                    if ui
                        .add_sized(
                            [142.0, 35.0],
                            egui::Button::new(label("?    Shortcuts", 13.0, p.muted, p.language))
                                .fill(Color32::TRANSPARENT),
                        )
                        .clicked()
                    {
                        self.help = true;
                    }
                    if ui
                        .add_sized(
                            [142.0, 35.0],
                            egui::Button::new(label(
                                if self.notebook.light {
                                    "Dark appearance"
                                } else {
                                    "Light appearance"
                                },
                                13.0,
                                p.muted,
                                p.language,
                            ))
                            .fill(Color32::TRANSPARENT),
                        )
                        .clicked()
                    {
                        self.notebook.light = !self.notebook.light;
                    }
                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(5.0, 12.0), egui::Sense::hover());
                        ui.painter().circle_filled(rect.center(), 2.0, p.accent);
                        ui.label(label("Offline calculations", 11.0, p.accent, p.language));
                    });
                });
            });
    }
    fn history_ui(&mut self, ui: &mut egui::Ui, p: Palette) {
        ui.horizontal(|ui| {
            ui.label(label("History", 19.0, p.text, p.language).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(label(
                    self.notebook.entries.len().to_string(),
                    12.0,
                    p.muted,
                    p.language,
                ));
            });
        });
        ui.add_space(10.0);
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .id(egui::Id::new("history_search"))
                .hint_text(p.tr("Search expressions or results…"))
                .desired_width(f32::INFINITY)
                .margin(egui::vec2(10.0, 9.0)),
        );
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.only_pinned, false, p.tr("All"));
            ui.selectable_value(&mut self.only_pinned, true, p.tr("Pinned"));
        });
        ui.add_space(8.0);
        let mut reuse = None;
        let mut copy = None;
        let mut pin = None;
        let mut delete = None;
        let query = self.search.to_lowercase();
        egui::ScrollArea::vertical()
            .id_salt("history_scroll")
            .max_height((ui.available_height() - 106.0).max(100.0))
            .show(ui, |ui| {
                let entries: Vec<_> = self
                    .notebook
                    .entries
                    .iter()
                    .enumerate()
                    .rev()
                    .filter(|(_, e)| {
                        (!self.only_pinned || e.pinned)
                            && (query.is_empty()
                                || e.expression.to_lowercase().contains(&query)
                                || e.result.contains(&query))
                    })
                    .collect();
                if entries.is_empty() {
                    ui.add_space(30.0);
                    ui.label(label(
                        if self.notebook.entries.is_empty() {
                            "A fresh page."
                        } else {
                            "No results found."
                        },
                        18.0,
                        p.text,
                        p.language,
                    ));
                    ui.label(label(
                        "Your calculations live here.\nClick a result to keep going.",
                        13.0,
                        p.muted,
                        p.language,
                    ));
                }
                for (index, entry) in entries {
                    egui::Frame::new()
                        .fill(p.card)
                        .corner_radius(8)
                        .inner_margin(12)
                        .show(ui, |ui| {
                            ui.set_min_width((ui.available_width() - 1.0).max(0.0));
                            ui.label(label(&entry.expression, 13.0, p.muted, p.language));
                            let prefix = if entry.approximate { "≈ " } else { "" };
                            if ui
                                .add(
                                    egui::Button::new(label(
                                        format!("{prefix}{}", entry.result),
                                        22.0,
                                        p.text,
                                        p.language,
                                    ))
                                    .frame(false),
                                )
                                .on_hover_text(p.tr("Start a calculation with this result"))
                                .clicked()
                            {
                                reuse = Some(entry.result.clone());
                            }
                            ui.horizontal(|ui| {
                                ui.label(label(
                                    if entry.angle == Angle::Degrees {
                                        "DEG"
                                    } else {
                                        "RAD"
                                    },
                                    10.0,
                                    p.muted,
                                    p.language,
                                ));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let delete_button = ui
                                            .small_button(p.tr("×"))
                                            .on_hover_text(p.tr("Delete this calculation"));
                                        #[cfg(test)]
                                        self.targets.insert(
                                            format!("delete_history_{index}"),
                                            delete_button.rect,
                                        );
                                        if delete_button.clicked() {
                                            delete = Some(index);
                                        }
                                        if ui.small_button(p.tr("Copy")).clicked() {
                                            copy = Some(entry.result.clone());
                                        }
                                        if ui
                                            .small_button(p.tr(if entry.pinned {
                                                "★"
                                            } else {
                                                "☆"
                                            }))
                                            .on_hover_text(p.tr("Pin or unpin"))
                                            .clicked()
                                        {
                                            pin = Some(index);
                                        }
                                        if ui
                                            .small_button(p.tr("↗"))
                                            .on_hover_text(p.tr("Edit this expression"))
                                            .clicked()
                                        {
                                            reuse = Some(entry.expression.clone());
                                        }
                                    },
                                );
                            });
                        });
                    ui.add_space(7.0);
                }
            });
        if let Some(text) = reuse {
            self.expression = text;
            self.committed = false;
            self.focus = true;
            self.error = None;
            self.reset_cursor = true;
        }
        if let Some(text) = copy {
            ui.ctx().copy_text(text);
            self.toast("Result copied");
        }
        if let Some(i) = pin {
            self.notebook.entries[i].pinned = !self.notebook.entries[i].pinned;
        }
        if let Some(i) = delete {
            self.notebook.delete_history(i);
            self.toast("Calculation deleted");
        }
        ui.add_space(10.0);
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button(p.tr("Copy all")).clicked() {
                ui.ctx().copy_text(self.notebook.export());
                self.toast("History copied as text");
            }
            let mut action = None;
            ui.add_enabled_ui(self.history_file_worker.is_none(), |ui| {
                let files = ui.menu_button(p.tr("Files…"), |ui| {
                    for (label, file_action) in [
                        ("Save backup…", HistoryFileAction::Backup),
                        ("Export as text…", HistoryFileAction::Text),
                        ("Import backup…", HistoryFileAction::Import),
                    ] {
                        if ui.button(p.tr(label)).clicked() {
                            action = Some(file_action);
                            ui.close();
                        }
                    }
                });
                #[cfg(test)]
                self.targets
                    .insert("history_files".into(), files.response.rect);
                #[cfg(not(test))]
                let _ = files;
            });
            if self.history_file_worker.is_some() {
                ui.spinner();
            }
            if let Some(action) = action {
                self.start_history_file(action, ui.ctx());
            }
        });
        ui.horizontal(|ui| {
            let undo = ui
                .add_enabled(
                    self.notebook.can_undo_history(),
                    egui::Button::new(p.tr("Undo deletion")),
                )
                .on_hover_text(
                    p.tr("Restore deleted history entries without losing new calculations"),
                );
            #[cfg(test)]
            self.targets.insert("undo_history".into(), undo.rect);
            if undo.clicked() {
                match self.notebook.undo_history() {
                    Ok(true) => self.toast("History restored"),
                    Ok(false) => {}
                    Err(error) => self.history_error = Some(error),
                }
            }
            let clear = ui.add_enabled(
                self.notebook.entries.iter().any(|e| !e.pinned),
                egui::Button::new(p.tr("Clear")),
            );
            #[cfg(test)]
            self.targets.insert("clear_history".into(), clear.rect);
            if clear.clicked() {
                self.confirm_clear = true;
            }
        });
    }
    fn header(&mut self, ui: &mut egui::Ui, p: Palette, title: &str, subtitle: &str) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(label("RUSTCALC  /  DESKTOP", 10.0, p.muted, p.language).strong());
                ui.add_space(5.0);
                ui.label(label(title, 28.0, p.text, p.language).strong());
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let wide = ui.ctx().content_rect().width() > 1010.0;
                if self.page == Page::Calculator
                    && ui
                        .selectable_label(
                            if wide {
                                self.history
                            } else {
                                self.history_popup
                            },
                            p.tr("History"),
                        )
                        .clicked()
                {
                    if wide {
                        self.history = !self.history;
                    } else {
                        self.history_popup = !self.history_popup;
                    }
                }
            });
        });
        ui.add_space(3.0);
        ui.label(label(subtitle, 13.0, p.muted, p.language));
        ui.add_space(22.0);
    }
    fn calculator(&mut self, ui: &mut egui::Ui, p: Palette, calculate_requested: bool) {
        self.header(
            ui,
            p,
            "Calculator",
            "From a small calculation to a big idea.",
        );
        let previous_angle = self.notebook.angle;
        ui.horizontal(|ui| {
            let standard = ui.selectable_value(&mut self.scientific, false, p.tr("Standard"));
            #[cfg(test)]
            self.targets.insert("standard".into(), standard.rect);
            #[cfg(not(test))]
            let _ = standard;
            ui.selectable_value(&mut self.scientific, true, p.tr("Scientific"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.selectable_value(&mut self.notebook.angle, Angle::Radians, p.tr("RAD"));
                ui.selectable_value(&mut self.notebook.angle, Angle::Degrees, p.tr("DEG"));
            });
        });
        if self.notebook.angle != previous_angle {
            self.committed = false;
            self.error = None;
        }
        ui.add_space(8.0);
        card(p).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    label(
                        if self.committed {
                            "RESULT"
                        } else {
                            "LIVE PREVIEW"
                        },
                        10.0,
                        p.muted,
                        p.language,
                    )
                    .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(label("DECIMAL · UP TO 28 PLACES", 9.0, p.muted, p.language));
                });
            });
            ui.add_space(10.0);
            let editor = ui.add(
                egui::TextEdit::singleline(&mut self.expression)
                    .id(egui::Id::new("expression"))
                    .font(FontId::monospace(19.0))
                    .text_color(p.text)
                    .hint_text(p.tr("Type an expression…"))
                    .desired_width(f32::INFINITY)
                    .frame(false)
                    .char_limit(4096),
            );
            #[cfg(test)]
            self.targets.insert("expression".into(), editor.rect);
            if editor.changed() {
                self.committed = false;
                self.error = None;
            }
            if self.focus {
                editor.request_focus();
                self.focus = false;
            }
            if self.reset_cursor {
                let mut state = egui::TextEdit::load_state(ui.ctx(), editor.id).unwrap_or_default();
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(
                        egui::text::CCursor::new(self.expression.chars().count()),
                    )));
                state.store(ui.ctx(), editor.id);
                self.reset_cursor = false;
            }
            if let Some(text) = self.insertion.take() {
                let mut state = egui::TextEdit::load_state(ui.ctx(), editor.id).unwrap_or_default();
                let range = state
                    .cursor
                    .char_range()
                    .unwrap_or(egui::text::CCursorRange::one(egui::text::CCursor::new(
                        self.expression.chars().count(),
                    )));
                let range = range.as_sorted_char_range();
                let mut chars: Vec<char> = self.expression.chars().collect();
                let start = range.start.min(chars.len());
                let end = range.end.min(chars.len());
                let backspace = text == "\u{8}";
                let start = if backspace && start == end {
                    start.saturating_sub(1)
                } else {
                    start
                };
                let insert = if backspace { "" } else { &text };
                chars.splice(start..end, insert.chars());
                self.expression = chars.into_iter().collect();
                let cursor = egui::text::CCursor::new(start + insert.chars().count());
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(cursor)));
                state.store(ui.ctx(), editor.id);
                editor.request_focus();
            }
            if calculate_requested {
                self.calculate();
            }
            ui.add_space(10.0);
            let preview = self.preview();
            let (result, approximate) = match &preview {
                Ok(v) => (v.text(), v.approximate),
                Err(_) => ("0".into(), false),
            };
            let size = if result.len() > 24 {
                28.0
            } else if result.len() > 16 {
                37.0
            } else {
                52.0
            };
            egui::ScrollArea::horizontal()
                .id_salt("result_scroll")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if approximate {
                            ui.label(label("≈", 28.0, p.muted, p.language));
                        }
                        ui.label(label(result, size, p.text, p.language));
                    });
                });
            ui.horizontal(|ui| {
                if ui.small_button(p.tr("Copy result")).clicked()
                    && let Ok(value) = &preview
                {
                    ui.ctx().copy_text(value.text());
                    self.toast("Result copied");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(label(
                        if approximate {
                            "≈ approximate result"
                        } else {
                            "Press Enter to calculate"
                        },
                        11.0,
                        p.muted,
                        p.language,
                    ));
                });
            });
            if let Some(error) = &self.error {
                ui.label(label(error, 13.0, p.danger, p.language));
            } else if !self.expression.is_empty()
                && let Err(error) = &preview
            {
                ui.label(label(&error.message, 12.0, p.muted, p.language));
            }
        });
        ui.add_space(9.0);
        ui.horizontal(|ui| {
            for (text, hint) in [
                ("MC", "Clear memory"),
                ("MR", "Insert memory"),
                ("MS", "Store result in memory"),
                ("M+", "Add to memory"),
                ("M−", "Subtract from memory"),
            ] {
                if ui
                    .add_sized(
                        [44.0, 28.0],
                        egui::Button::new(label(text, 12.0, p.muted, p.language)).frame(false),
                    )
                    .on_hover_text(p.tr(hint))
                    .clicked()
                {
                    match text {
                        "MC" => {
                            self.notebook.memory.clear();
                            self.notebook.memory_approximate = false;
                            self.toast("Memory cleared");
                        }
                        "MR" => {
                            self.insert("mem");
                        }
                        _ => {
                            if let Ok(v) = self.preview() {
                                let result = if text == "MS" {
                                    self.notebook.memory = v.text();
                                    self.notebook.memory_approximate = v.approximate;
                                    Ok(())
                                } else {
                                    self.notebook.memory_add(v, text == "M−")
                                };
                                match result {
                                    Ok(()) => self.toast("Memory updated"),
                                    Err(e) => self.toast(e),
                                }
                            } else {
                                self.toast("Enter a valid expression first");
                            }
                        }
                    }
                }
            }
            if !self.notebook.context().memory.is_zero() {
                ui.label(label("● M", 11.0, p.accent, p.language))
                    .on_hover_text(p.tr(&self.notebook.memory));
            }
        });
        let science_rows: &[&[(&str, &str)]] = &[
            &[
                ("sin", "sin("),
                ("cos", "cos("),
                ("tan", "tan("),
                ("(", "("),
                (")", ")"),
            ],
            &[
                ("ln", "ln("),
                ("log", "log("),
                ("√", "sqrt("),
                ("x²", "^2"),
                ("x^y", "^"),
            ],
        ];
        let key_height = if self.scientific { 45.0 } else { 58.0 };
        if self.scientific {
            for row in science_rows {
                self.key_row(ui, p, row, 34.0, true);
            }
            ui.add_space(4.0);
        }
        let rows: &[&[(&str, &str)]] = &[
            &[
                ("AC", "clear"),
                ("±", "negate"),
                ("%", "%"),
                ("Del", "backspace"),
            ],
            &[("7", "7"), ("8", "8"), ("9", "9"), ("÷", "/")],
            &[("4", "4"), ("5", "5"), ("6", "6"), ("×", "*")],
            &[("1", "1"), ("2", "2"), ("3", "3"), ("−", "-")],
            &[("0", "0"), (".", "."), ("=", "equals"), ("+", "+")],
        ];
        for row in rows {
            self.key_row(ui, p, row, key_height, false);
        }
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            for (title, value) in [
                ("π", "pi"),
                ("e", "e"),
                ("n!", "!"),
                ("1/x", "reciprocal"),
                ("Ans", "ans"),
            ] {
                if ui
                    .add(egui::Button::new(label(title, 12.0, p.muted, p.language)).frame(false))
                    .clicked()
                {
                    self.action(value);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(label("% = ÷ 100", 10.0, p.muted, p.language))
                    .on_hover_text(p.tr("200 + 10% = 200.1; percent divides a value by 100."));
            });
        });
    }
    fn key_row(
        &mut self,
        ui: &mut egui::Ui,
        p: Palette,
        keys: &[(&str, &str)],
        height: f32,
        science: bool,
    ) {
        let width = (ui.available_width() - (keys.len() - 1) as f32 * 8.0) / keys.len() as f32;
        ui.horizontal(|ui| {
            for &(text, action) in keys {
                let equals = action == "equals";
                let operation = matches!(action, "+" | "-" | "*" | "/");
                let color = if equals {
                    p.ink
                } else if operation {
                    p.accent
                } else if action == "clear" {
                    p.danger
                } else if science {
                    p.muted
                } else {
                    p.text
                };
                let fill = if equals {
                    p.accent
                } else if science {
                    p.card
                } else {
                    p.key
                };
                let response = ui.add_sized(
                    [width, height],
                    egui::Button::new(label(
                        text,
                        if science { 16.0 } else { 23.0 },
                        color,
                        p.language,
                    ))
                    .fill(fill)
                    .corner_radius(7),
                );
                #[cfg(test)]
                self.targets.insert(action.into(), response.rect);
                if response.clicked() {
                    self.action(action);
                }
            }
        });
    }
    fn action(&mut self, action: &str) {
        match action {
            "equals" => self.calculate(),
            "clear" => {
                self.expression.clear();
                self.error = None;
                self.committed = false;
                self.focus = true;
                self.insertion = None;
            }
            "backspace" => {
                // Delete the character/selection at the editor cursor, not the end.
                self.insertion = Some("\u{8}".into());
                self.focus = true;
                self.committed = false;
            }
            "negate" => {
                if !self.expression.is_empty() {
                    self.expression = format!("-({})", self.expression);
                    self.reset_cursor = true;
                    self.committed = false;
                    self.focus = true;
                }
            }
            "reciprocal" => {
                if !self.expression.is_empty() {
                    self.expression = format!("1/({})", self.expression);
                    self.reset_cursor = true;
                    self.committed = false;
                    self.focus = true;
                }
            }
            _ => self.insert(action),
        }
    }
    fn converter(&mut self, ui: &mut egui::Ui, p: Palette) {
        self.header(
            ui,
            p,
            "Unit converter",
            "The same value. A different language.",
        );
        ui.horizontal_wrapped(|ui| {
            for (i, category) in units::CATEGORIES.iter().enumerate() {
                if ui
                    .selectable_label(self.category == i, p.tr(category.name))
                    .clicked()
                {
                    self.category = i;
                    self.from = 0;
                    self.to = 1;
                }
            }
        });
        ui.add_space(22.0);
        let category = &units::CATEGORIES[self.category];
        card(p).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(label("VALUE TO CONVERT", 10.0, p.muted, p.language));
            ui.add_space(14.0);
            ui.add(
                egui::TextEdit::singleline(&mut self.amount)
                    .font(FontId::proportional(38.0))
                    .desired_width(f32::INFINITY)
                    .frame(false)
                    .char_limit(128),
            );
            ui.add_space(12.0);
            unit_combo(ui, "from", &mut self.from, category, p);
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button(p.tr("Swap units")).clicked() {
                std::mem::swap(&mut self.from, &mut self.to);
            }
        });
        ui.add_space(12.0);
        let converted = Decimal::from_str_exact(&self.amount.replace(',', "."))
            .map_err(|_| "Enter a valid decimal number".to_string())
            .and_then(|v| units::convert(v, self.category, self.from, self.to));
        card(p).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(label("CONVERTED VALUE", 10.0, p.muted, p.language));
            ui.add_space(14.0);
            match &converted {
                Ok(v) => {
                    ui.add(
                        egui::Label::new(label(
                            v.normalize().to_string(),
                            38.0,
                            p.accent,
                            p.language,
                        ))
                        .wrap(),
                    );
                }
                Err(e) => {
                    ui.label(label(e, 15.0, p.danger, p.language));
                }
            }
            ui.add_space(12.0);
            unit_combo(ui, "to", &mut self.to, category, p);
        });
        ui.add_space(16.0);
        if ui
            .add_enabled(converted.is_ok(), egui::Button::new("Copy result"))
            .clicked()
            && let Ok(v) = converted
        {
            ui.ctx().copy_text(v.normalize().to_string());
            self.toast("Conversion copied");
        }
        ui.add_space(24.0);
        ui.label(label(
            "8 categories  ·  SI and binary data units  ·  Offline",
            12.0,
            p.muted,
            p.language,
        ));
        ui.label(label(
            "Decimal results may round to 28 places.",
            12.0,
            p.muted,
            p.language,
        ));
    }
    fn programmer(&mut self, ui: &mut egui::Ui, p: Palette) {
        self.header(
            ui,
            p,
            "Programmer",
            "Clarity at the bit level. Unsigned 64-bit integers.",
        );
        ui.horizontal(|ui| {
            for (radix, title) in [(16, "HEX"), (10, "DEC"), (8, "OCT"), (2, "BIN")] {
                if ui
                    .selectable_label(self.radix == radix, p.tr(title))
                    .clicked()
                {
                    if let Ok(n) = engine::parse_integer(&self.integer, self.radix) {
                        self.integer = integer_string(n, radix);
                    }
                    self.radix = radix;
                }
            }
        });
        ui.add_space(12.0);
        card(p).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(label("INPUT", 10.0, p.muted, p.language));
            ui.add(
                egui::TextEdit::singleline(&mut self.integer)
                    .font(FontId::monospace(24.0))
                    .desired_width(f32::INFINITY)
                    .frame(false)
                    .char_limit(80),
            );
        });
        ui.add_space(14.0);
        match engine::parse_integer(&self.integer, self.radix) {
            Err(e) => {
                ui.label(label(e, 14.0, p.danger, p.language));
            }
            Ok(n) => {
                card(p).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    for (base, title) in [(16, "HEX"), (10, "DEC"), (8, "OCT"), (2, "BIN")] {
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [40.0, 22.0],
                                egui::Label::new(label(title, 11.0, p.muted, p.language)),
                            );
                            let text = integer_string(n, base);
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(&text)
                                            .monospace()
                                            .size(if base == 2 { 12.0 } else { 19.0 })
                                            .color(if base == self.radix {
                                                p.accent
                                            } else {
                                                p.text
                                            }),
                                    )
                                    .frame(false)
                                    .wrap(),
                                )
                                .on_hover_text(p.tr("Copy"))
                                .clicked()
                            {
                                ui.ctx().copy_text(text);
                                self.toast("Number copied");
                            }
                        });
                    }
                });
                ui.add_space(18.0);
                ui.label(label(
                    "BIT MAP  /  CLICK TO TOGGLE",
                    10.0,
                    p.muted,
                    p.language,
                ));
                ui.add_space(8.0);
                let width = ((ui.available_width() - 15.0 * 4.0) / 16.0).clamp(15.0, 30.0);
                egui::Grid::new("bits")
                    .spacing(egui::vec2(4.0, 5.0))
                    .show(ui, |ui| {
                        for bit in (0..64).rev() {
                            let set = n & (1u64 << bit) != 0;
                            if ui
                                .add_sized(
                                    [width, 28.0],
                                    egui::Button::new(label(
                                        if set { "1" } else { "0" },
                                        12.0,
                                        if set { p.ink } else { p.muted },
                                        p.language,
                                    ))
                                    .fill(if set { p.accent } else { p.card })
                                    .corner_radius(3),
                                )
                                .on_hover_text(p.tr(format!("Bit {bit} · 2^{bit}")))
                                .clicked()
                            {
                                self.integer = integer_string(n ^ (1u64 << bit), self.radix);
                            }
                            if bit % 16 == 0 {
                                ui.end_row();
                            }
                        }
                    });
                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.label(label("Second operand (DEC)", 12.0, p.muted, p.language));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.operand)
                            .desired_width(130.0)
                            .char_limit(30),
                    );
                });
                let operand = engine::parse_integer(&self.operand, 10);
                ui.horizontal_wrapped(|ui| {
                    for op in ["AND", "OR", "XOR", "NOT", "<< 1", ">> 1"] {
                        if ui
                            .add_enabled(
                                operand.is_ok() || matches!(op, "NOT" | "<< 1" | ">> 1"),
                                egui::Button::new(op),
                            )
                            .clicked()
                        {
                            let rhs = operand.clone().unwrap_or_default();
                            let result = match op {
                                "AND" => n & rhs,
                                "OR" => n | rhs,
                                "XOR" => n ^ rhs,
                                "NOT" => !n,
                                "<< 1" => n.wrapping_shl(1),
                                _ => n >> 1,
                            };
                            self.integer = integer_string(result, self.radix);
                        }
                    }
                });
                if let Err(e) = operand {
                    ui.label(label(e, 12.0, p.danger, p.language));
                }
                ui.add_space(10.0);
                ui.label(label(
                    "Left shifts discard overflowing bits. NOT inverts all 64 bits.",
                    12.0,
                    p.muted,
                    p.language,
                ));
            }
        }
    }
    fn graph(&mut self, ui: &mut egui::Ui, p: Palette) {
        self.header(ui, p, "Plot a graph", "Explore the shape of a function.");
        ui.horizontal(|ui| {
            ui.label(label("f(x) =", 20.0, p.accent, p.language));
            ui.add(
                egui::TextEdit::singleline(&mut self.graph)
                    .font(FontId::monospace(19.0))
                    .desired_width(f32::INFINITY)
                    .char_limit(512),
            );
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            for f in ["sin(x)", "x^2", "1/x", "sqrt(x)"] {
                if ui.button(p.tr(f)).clicked() {
                    self.graph = f.into();
                }
            }
            ui.label(label("RAD", 11.0, p.muted, p.language));
        });
        ui.add(
            egui::Slider::new(&mut self.graph_range, 1.0..=100.0)
                .logarithmic(true)
                .text(p.tr("View ±")),
        );
        ui.add_space(12.0);
        let size = egui::vec2(
            ui.available_width(),
            (ui.available_height() - 45.0).clamp(240.0, 540.0),
        );
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 10, p.card);
        let range = self.graph_range;
        let map = |x: f64, y: f64| {
            egui::pos2(
                rect.center().x + (x / range) as f32 * rect.width() / 2.0,
                rect.center().y - (y / range) as f32 * rect.height() / 2.0,
            )
        };
        for i in -5..=5 {
            let v = range * i as f64 / 5.0;
            painter.line_segment(
                [map(v, -range), map(v, range)],
                Stroke::new(1.0_f32, if i == 0 { p.muted } else { p.border }),
            );
            painter.line_segment(
                [map(-range, v), map(range, v)],
                Stroke::new(1.0_f32, if i == 0 { p.muted } else { p.border }),
            );
            if i != 0 && i % 2 == 0 {
                painter.text(
                    map(v, 0.0) + egui::vec2(3.0, 4.0),
                    egui::Align2::LEFT_TOP,
                    format!("{v:.1}"),
                    FontId::monospace(10.0),
                    p.muted,
                );
            }
        }
        let eval = |x: f64| -> Option<f64> {
            let context = Context {
                angle: Angle::Radians,
                x: Decimal::from_f64_retain(x)?,
                ..self.notebook.context()
            };
            engine::evaluate(&self.graph, context)
                .ok()?
                .number
                .to_f64()
                .filter(|v| v.is_finite())
        };
        let mut last: Option<(f64, f64)> = None;
        let mut valid = 0;
        for i in 0..=600 {
            let x = -range + 2.0 * range * i as f64 / 600.0;
            if let Some(y) = eval(x).filter(|y| y.abs() <= range * 2.0) {
                valid += 1;
                if let Some((px, py)) = last
                    && (y - py).abs() < range / 2.0
                {
                    painter.line_segment([map(px, py), map(x, y)], Stroke::new(2.5_f32, p.accent));
                }
                last = Some((x, y));
            } else {
                last = None;
            }
        }
        if let Some(pos) = response.hover_pos() {
            let x = (pos.x - rect.center().x) as f64 / (rect.width() / 2.0) as f64 * range;
            if let Some(y) = eval(x) {
                painter.line_segment(
                    [
                        egui::pos2(pos.x, rect.top()),
                        egui::pos2(pos.x, rect.bottom()),
                    ],
                    Stroke::new(1.0_f32, p.muted),
                );
                if y.abs() <= range {
                    painter.circle_filled(map(x, y), 5.0, p.accent);
                }
                painter.text(
                    rect.left_top() + egui::vec2(15.0, 15.0),
                    egui::Align2::LEFT_TOP,
                    format!("x = {x:.4}    f(x) = {y:.4}"),
                    FontId::monospace(13.0),
                    p.text,
                );
            }
        }
        ui.add_space(8.0);
        if valid == 0 {
            ui.label(label(
                "No plottable values in this range. Change the expression or view.",
                12.0,
                p.danger,
                p.language,
            ));
        } else {
            ui.label(label(
                "Hover to read values. Both axes show the same range; angles are in radians.",
                12.0,
                p.muted,
                p.language,
            ));
        }
    }
    fn dialogs(&mut self, ctx: &egui::Context, p: Palette) {
        if let Some(entries) = &self.pending_import {
            let mut import = false;
            let mut cancel = false;
            let modal = egui::Modal::new(egui::Id::new("history_import_confirmation")).show(ctx, |ui| {
                    ui.set_width(390.0);
                    ui.heading(p.tr("Import history?"));
                    ui.label(p.tr("Calculations in backup: {count}").replace("{count}", &entries.len().to_string()));
                    ui.label(p.tr("Merge this backup with existing history. Duplicate entries are skipped and pinned entries stay pinned."));
                    ui.label(p.tr("Import starts a new history undo session. Current answer, memory and preferences are kept."));
                    ui.horizontal(|ui| {
                        cancel = ui.button(p.tr("Cancel")).clicked();
                        let button = ui.button(p.tr("Import history"));
                        #[cfg(test)]
                        self.targets.insert("confirm_import".into(), button.rect);
                        import = button.clicked();
                    });
                });
            cancel |= modal.should_close();
            if import {
                let entries = self.pending_import.take().unwrap();
                match self.notebook.import_history(&entries) {
                    Ok(_) => self.toast("History imported"),
                    Err(error) => self.history_error = Some(error),
                }
            } else if cancel {
                self.pending_import = None;
            }
        }
        if let Some(error) = &self.history_error {
            let mut close = false;
            let modal = egui::Modal::new(egui::Id::new("history_file_error")).show(ctx, |ui| {
                ui.set_width(390.0);
                ui.heading(p.tr("History file"));
                ui.label(egui::RichText::new(p.tr(error)).color(p.danger));
                close = ui.button(p.tr("Close")).clicked();
            });
            if close || modal.should_close() {
                self.history_error = None;
            }
        }
        egui::Window::new(p.tr("Shortcuts and calculation")).open(&mut self.help).collapsible(false).resizable(false).default_width(430.0).show(ctx, |ui| {
            for (key, action) in [("Enter", "Calculate and save to history"), ("Esc", "Clear the expression"), ("Ctrl + L", "Select and edit the expression"), ("Ctrl + Shift + C", "Copy result"), ("F1", "Open this help window")] {
                ui.horizontal(|ui| { ui.add_sized([140.0, 24.0], egui::Label::new(label(key, 13.0, p.accent, p.language))); ui.label(p.tr(action)); });
            }
            ui.separator();
            ui.label(p.tr("Precedence: parentheses, factorial/percent, powers, multiplication/division, addition/subtraction. 2^3^2 = 512; -2^2 = -4."));
            ui.label(p.tr("Commas and periods are decimal separators. Do not use thousands separators. 2pi and 2(3+4) are supported."));
            ui.label(p.tr("Percent always divides by 100: 200 + 10% = 200.1. For an increase, use 200 * (1+10%)."));
            ui.label(p.tr("sin, cos, tan, asin, acos, atan, sqrt, cbrt, ln, log, exp, abs, floor, ceil, round. Constants: pi, e, ans, mem."));
            ui.label(p.tr("Basic arithmetic uses 96-bit decimals with up to 28 fractional places. Division may round. Scientific functions show approximately 15 significant digits with ≈."));
            ui.label(p.tr("History (up to 500 entries), memory, angle mode, language and theme are saved locally. Only manual update checks connect to GitHub."));
        });
        if self.confirm_clear {
            let modal =
                egui::Modal::new(egui::Id::new("clear_history_confirmation")).show(ctx, |ui| {
                    ui.set_width(390.0);
                    ui.heading(p.tr("Clear history?"));
                    ui.label(
                        p.tr("All unpinned calculations will be deleted. Pinned entries are kept."),
                    );
                    ui.label(p.tr("Use Undo deletion to restore them during this session."));
                    ui.horizontal(|ui| {
                        if ui.button(p.tr("Cancel")).clicked() {
                            self.confirm_clear = false;
                        }
                        let clear = ui.button(p.tr("Clear history"));
                        #[cfg(test)]
                        self.targets
                            .insert("confirm_clear_history".into(), clear.rect);
                        if clear.clicked() {
                            self.notebook.clear_unpinned_history();
                            self.toast("Unpinned history cleared");
                            self.confirm_clear = false;
                        }
                    });
                });
            if modal.should_close() {
                self.confirm_clear = false;
            }
        }
        if self.about {
            self.about_dialog(ctx, p);
        }
    }
    fn start_history_file(&mut self, action: HistoryFileAction, ctx: &egui::Context) {
        if self.history_file_worker.is_some() {
            return;
        }
        let (send, receive) = mpsc::channel();
        self.history_file_worker = Some(receive);
        let entries = self.notebook.entries.clone();
        let text = self.notebook.export();
        let language = self.notebook.language;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let reply = match action {
                HistoryFileAction::Import => {
                    let path = rfd::FileDialog::new()
                        .set_title(language.text("Import backup…"))
                        .add_filter(language.text("RustCalc history backup"), &["json"])
                        .pick_file();
                    HistoryFileReply::Loaded(
                        path.map(|path| history::read_backup(&path)).transpose(),
                    )
                }
                HistoryFileAction::Backup | HistoryFileAction::Text => {
                    let backup = matches!(action, HistoryFileAction::Backup);
                    let bytes = if backup {
                        history::encode(&entries)
                    } else {
                        Ok(text.into_bytes())
                    };
                    HistoryFileReply::Saved(bytes.and_then(|bytes| {
                        let path = rfd::FileDialog::new()
                            .set_title(language.text(if backup {
                                "Save backup…"
                            } else {
                                "Export as text…"
                            }))
                            .set_file_name(if backup {
                                "rustcalc-history.json"
                            } else {
                                "rustcalc-history.txt"
                            })
                            .add_filter(
                                language.text(if backup {
                                    "RustCalc history backup"
                                } else {
                                    "Text file"
                                }),
                                &[if backup { "json" } else { "txt" }],
                            )
                            .save_file();
                        if let Some(path) = path {
                            history::write_file(&path, &bytes)?;
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    }))
                }
            };
            let _ = send.send(reply);
            ctx.request_repaint();
        });
    }
    fn poll_history_file(&mut self) {
        let Some(receiver) = &self.history_file_worker else {
            return;
        };
        match receiver.try_recv() {
            Ok(reply) => {
                self.history_file_worker = None;
                match reply {
                    HistoryFileReply::Saved(Ok(true)) => self.toast("History file saved"),
                    HistoryFileReply::Loaded(Ok(Some(entries))) => {
                        self.confirm_clear = false;
                        self.pending_import = Some(entries)
                    }
                    HistoryFileReply::Saved(Err(error)) | HistoryFileReply::Loaded(Err(error)) => {
                        self.confirm_clear = false;
                        self.history_error = Some(error)
                    }
                    _ => {}
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.history_file_worker = None;
                self.confirm_clear = false;
                self.history_error =
                    Some("The history file operation could not finish. Please try again.".into());
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
    fn start_update_check(&mut self, ctx: &egui::Context) {
        let (send, receive) = mpsc::channel();
        self.update_worker = Some(receive);
        self.update_status = UpdateStatus::Checking;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = send.send(UpdateReply::Checked(updater::check()));
            ctx.request_repaint();
        });
    }
    fn start_update_install(&mut self, plan: UpdatePlan, ctx: &egui::Context) {
        let (send, receive) = mpsc::channel();
        self.update_worker = Some(receive);
        self.update_status = UpdateStatus::Installing;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = send.send(UpdateReply::Installed(updater::install(&plan)));
            ctx.request_repaint();
        });
    }
    fn poll_update(&mut self, ctx: &egui::Context) {
        if let Some(receiver) = &self.update_worker {
            match receiver.try_recv() {
                Ok(reply) => {
                    self.update_worker = None;
                    self.update_status = match reply {
                        UpdateReply::Checked(Ok(Some(plan))) => {
                            UpdateStatus::Available(Box::new(plan))
                        }
                        UpdateReply::Checked(Ok(None)) => UpdateStatus::Current,
                        UpdateReply::Installed(Ok(())) => UpdateStatus::Installed,
                        UpdateReply::Checked(Err(e)) | UpdateReply::Installed(Err(e)) => {
                            UpdateStatus::Failed(e)
                        }
                    };
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.update_worker = None;
                    self.update_status =
                        UpdateStatus::Failed("The update worker stopped unexpectedly.".into());
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(100));
                }
            }
        }
    }
    fn about_dialog(&mut self, ctx: &egui::Context, p: Palette) {
        let mut open = self.about;
        egui::Window::new(p.tr("About & updates")).open(&mut open).collapsible(false).resizable(false).default_width(440.0).show(ctx, |ui| {
            ui.label(label(format!("RustCalc {}", env!("CARGO_PKG_VERSION")), 27.0, p.text, p.language));
            ui.label(p.tr("A portable, open-source calculator."));
            ui.hyperlink_to("ersinkoc/rustcalc", "https://github.com/ersinkoc/rustcalc");
            ui.label("MIT License · © 2026 Ersin Koç");
            ui.separator();
            ui.label(p.tr("Connects to GitHub only when you request it. Your calculations are never sent."));
            ui.add_space(12.0);
            match &self.update_status {
                UpdateStatus::Idle | UpdateStatus::Current | UpdateStatus::Failed(_) => {
                    if let UpdateStatus::Current = self.update_status { ui.label(label("You are using the latest version.", 14.0, p.accent, p.language)); }
                    if let UpdateStatus::Failed(e) = &self.update_status { ui.label(label(e, 13.0, p.danger, p.language)); }
                    if ui.button(p.tr("Check for updates")).clicked() { self.start_update_check(ctx); }
                }
                UpdateStatus::Checking => { ui.spinner(); ui.label(p.tr("Checking…")); },
                UpdateStatus::Available(plan) => {
                    ui.label(format!("{} → {}", env!("CARGO_PKG_VERSION"), plan.version));
                    ui.hyperlink_to(p.tr("Release notes"), format!("{}/tag/{}", updater::RELEASES_URL, plan.tag));
                    let plan = (**plan).clone();
                    if ui.button(p.tr("Install update")).clicked() { self.start_update_install(plan, ctx); }
                }
                UpdateStatus::Installing => { ui.spinner(); ui.label(p.tr("Downloading and verifying…")); },
                UpdateStatus::Installed => {
                    ui.label(label("Update installed. Restart RustCalc to use it.", 14.0, p.accent, p.language));
                    if ui.button(p.tr("Restart")).clicked() { crate::RESTART.store(true, std::sync::atomic::Ordering::SeqCst); ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
                }
            }
        });
        self.about = open;
    }
}

impl RustCalc {
    fn render(&mut self, ctx: &egui::Context) {
        self.poll_update(ctx);
        self.poll_history_file();
        if matches!(self.update_status, UpdateStatus::Installing)
            && ctx.input(|i| i.viewport().close_requested())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "RustCalc — {}",
            self.notebook.language.text("Room to think.")
        )));
        if let Some(size) = self.initial_size.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
        }
        let p = self.palette();
        let mut visuals = if self.notebook.light {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };
        visuals.override_text_color = Some(p.text);
        visuals.panel_fill = p.bg;
        visuals.window_fill = p.card;
        visuals.extreme_bg_color = p.side;
        visuals.selection.bg_fill = p.accent.gamma_multiply(0.25);
        visuals.selection.stroke = Stroke::new(1.0_f32, p.text);
        visuals.widgets.inactive.bg_fill = p.key;
        visuals.widgets.inactive.weak_bg_fill = p.card;
        visuals.widgets.hovered.bg_fill = p.border;
        visuals.widgets.hovered.weak_bg_fill = p.border;
        visuals.widgets.active.bg_fill = p.border;
        visuals.widgets.active.weak_bg_fill = p.border;
        ctx.set_visuals(visuals);
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.help = !self.help;
        }
        let mut calculate_requested = false;
        if self.page == Page::Calculator
            && !self.help
            && !self.confirm_clear
            && !self.about
            && self.pending_import.is_none()
            && self.history_error.is_none()
        {
            // Calculator input should work after clicking a button or empty space,
            // while typing in history search must remain with that field.
            let searching = ctx.memory(|m| m.has_focus(egui::Id::new("history_search")));
            let calculator_input = ctx.input(|i| {
                i.events.iter().any(|event| match event {
                    egui::Event::Text(text) | egui::Event::Paste(text) => !text.is_empty(),
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => {
                        !modifiers.command
                            && !modifiers.ctrl
                            && !modifiers.alt
                            && matches!(
                                key,
                                egui::Key::Enter | egui::Key::Backspace | egui::Key::Delete
                            )
                    }
                    _ => false,
                })
            });
            if !searching && calculator_input {
                // Set focus before TextEdit processes this frame's events so the
                // first character (or Enter) is not discarded.
                ctx.memory_mut(|m| m.request_focus(egui::Id::new("expression")));
            }
            if !searching {
                // Consume before widgets handle Enter, but evaluate after the
                // expression editor has applied any text from the same frame.
                calculate_requested =
                    ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
            }
            let editor_focused = ctx.memory(|m| m.has_focus(egui::Id::new("expression")));
            if editor_focused {
                let typing = ctx.input(|i| {
                    i.events.iter().find_map(|e| {
                        if let egui::Event::Text(text) = e {
                            Some(text.clone())
                        } else {
                            None
                        }
                    })
                });
                if let Some(text) = typing {
                    if text == "=" {
                        ctx.input_mut(|i| {
                            i.events
                                .retain(|e| !matches!(e, egui::Event::Text(t) if t == "="))
                        });
                        self.calculate();
                    } else if self.committed {
                        let continuation = text.starts_with(['+', '-', '*', '/', '^', '%', '!']);
                        self.expression = if continuation {
                            "ans".into()
                        } else {
                            String::new()
                        };
                        self.committed = false;
                        let id = egui::Id::new("expression");
                        let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::one(
                                egui::text::CCursor::new(self.expression.chars().count()),
                            )));
                        state.store(ctx, id);
                    }
                }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.action("clear");
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::L)) {
                self.focus = true;
                let id = egui::Id::new("expression");
                let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(self.expression.chars().count()),
                    )));
                state.store(ctx, id);
            }
            if ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::C)
            }) && let Ok(v) = self.preview()
            {
                ctx.copy_text(v.text());
                self.toast("Result copied");
            }
        }
        self.navigation(ctx, p);
        let wide = ctx.content_rect().width() > 1010.0;
        if self.page == Page::Calculator && self.history && wide {
            egui::SidePanel::right("history")
                .exact_width(284.0)
                .resizable(false)
                .frame(egui::Frame::new().fill(p.side).inner_margin(20))
                .show(ctx, |ui| {
                    ui.add_space(18.0);
                    self.history_ui(ui, p);
                });
        }
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(p.side)
                    .inner_margin(egui::Margin::symmetric(20, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let notice = self
                        .notice
                        .as_ref()
                        .filter(|(_, at)| at.elapsed() < Duration::from_secs(4))
                        .map(|(s, _)| s.as_str());
                    ui.label(label(
                        notice.unwrap_or("Runs locally. Your data stays yours."),
                        11.0,
                        if notice.is_some() { p.accent } else { p.muted },
                        p.language,
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(label("F1  HELP", 10.0, p.muted, p.language));
                    });
                });
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .inner_margin(egui::Margin::symmetric(28, 22)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("workspace")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        match self.page {
                            Page::Calculator => self.calculator(ui, p, calculate_requested),
                            Page::Converter => self.converter(ui, p),
                            Page::Programmer => self.programmer(ui, p),
                            Page::Graph => self.graph(ui, p),
                        }
                    });
            });
        if !wide && self.history_popup && self.page == Page::Calculator {
            let mut open = self.history_popup;
            egui::Window::new(p.tr("Calculation history"))
                .open(&mut open)
                .default_width(320.0)
                .default_height(500.0)
                .show(ctx, |ui| {
                    self.history_ui(ui, p);
                });
            self.history_popup = open;
        }
        self.dialogs(ctx, p);
        if self.notice.is_some() {
            ctx.request_repaint_after(Duration::from_secs(4));
        }
        if let Some(path) = &self.screenshot {
            self.screenshot_frame += 1;
            if self.screenshot_frame == 5 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                    path.clone(),
                )));
            }
            for event in ctx.input(|i| i.events.clone()) {
                if let egui::Event::Screenshot { image, .. } = event {
                    let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    if let Some(parent) = std::path::Path::new(path).parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    match image::save_buffer(
                        path,
                        &bytes,
                        image.width() as u32,
                        image.height() as u32,
                        image::ColorType::Rgba8,
                    ) {
                        Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                        Err(error) => {
                            eprintln!("Screenshot failed: {error}");
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }
                }
            }
            if self.screenshot_frame > 300 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint_after(Duration::from_millis(30));
        }
    }
}
impl eframe::App for RustCalc {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render(ctx);
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.screenshot.is_none() {
            eframe::set_value(storage, "notebook-v1", &self.notebook);
        }
    }
    fn auto_save_interval(&self) -> Duration {
        Duration::from_secs(5)
    }
    fn persist_egui_memory(&self) -> bool {
        self.screenshot.is_none()
    }
}

fn unit_combo(
    ui: &mut egui::Ui,
    id: &str,
    selected: &mut usize,
    category: &units::Category,
    p: Palette,
) {
    let unit = &category.units[*selected];
    egui::ComboBox::from_id_salt(id)
        .selected_text(label(
            format!("{}  ·  {}", p.tr(unit.name), unit.symbol),
            16.0,
            p.text,
            p.language,
        ))
        .width(ui.available_width().min(360.0))
        .show_ui(ui, |ui| {
            for (i, u) in category.units.iter().enumerate() {
                ui.selectable_value(
                    selected,
                    i,
                    p.tr(format!("{}  ·  {}", p.tr(u.name), u.symbol)),
                );
            }
        });
}
fn integer_string(n: u64, radix: u32) -> String {
    match radix {
        16 => format!("{n:X}"),
        8 => format!("{n:o}"),
        2 => format!("{n:b}"),
        _ => n.to_string(),
    }
}

pub fn app_icon() -> egui::IconData {
    let size = 64;
    let mut rgba = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let border = !(6..58).contains(&x) || !(4..60).contains(&y);
            let display = (14..50).contains(&x) && (13..25).contains(&y);
            let key = ((14..28).contains(&x) || (36..50).contains(&x))
                && ((32..40).contains(&y) || (46..54).contains(&y));
            let color = if border {
                [21, 25, 25, 255]
            } else if display || key {
                [23, 49, 32, 255]
            } else {
                [183, 230, 180, 255]
            };
            rgba.extend(color);
        }
    }
    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        app: &mut RustCalc,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        width: f32,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 820.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| app.render(ctx),
        )
    }
    fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }
    fn click(app: &mut RustCalc, ctx: &egui::Context, target: &str) {
        let pos = app.targets[target].center();
        frame(
            app,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
            1180.0,
        );
        frame(
            app,
            ctx,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
            1180.0,
        );
        frame(app, ctx, vec![], 1180.0);
    }
    #[test]
    fn keyboard_calculation_saves_a_real_history_entry() {
        let mut app = RustCalc::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Text("0.1+0.2".into())],
            1180.0,
        );
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::Enter, Default::default())],
            1180.0,
        );
        assert_eq!(app.notebook.answer, "0.3");
        assert_eq!(app.notebook.entries.len(), 1);
    }
    #[test]
    fn direct_keyboard_arithmetic_recovers_expression_focus() {
        for (expression, answer) in [("12+3", "15"), ("12-3", "9"), ("12*3", "36"), ("12/3", "4")] {
            let mut app = RustCalc::default();
            let ctx = egui::Context::default();
            frame(&mut app, &ctx, vec![], 1180.0);
            ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("expression")));
            frame(
                &mut app,
                &ctx,
                vec![egui::Event::Text(expression.into())],
                1180.0,
            );
            assert_eq!(app.expression, expression, "direct input was lost");
            frame(
                &mut app,
                &ctx,
                vec![key(egui::Key::Enter, Default::default())],
                1180.0,
            );
            assert_eq!(app.notebook.answer, answer);
            assert_eq!(app.notebook.entries.len(), 1);
        }
    }
    #[test]
    fn direct_enter_calculates_after_the_expression_loses_focus() {
        let mut app = RustCalc {
            expression: "12*3".into(),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("expression")));
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::Enter, Default::default())],
            1180.0,
        );
        assert_eq!(app.notebook.answer, "36");
        assert_eq!(app.notebook.entries.len(), 1);
    }
    #[test]
    fn direct_typing_after_a_result_can_continue_or_start_over_without_focus() {
        for (text, expression, answer) in [("*4", "ans*4", "20"), ("9", "9", "9")] {
            let mut app = RustCalc {
                expression: "2+3".into(),
                ..Default::default()
            };
            let ctx = egui::Context::default();
            app.calculate();
            frame(&mut app, &ctx, vec![], 1180.0);
            ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("expression")));
            frame(&mut app, &ctx, vec![egui::Event::Text(text.into())], 1180.0);
            assert_eq!(app.expression, expression);
            frame(
                &mut app,
                &ctx,
                vec![key(egui::Key::Enter, Default::default())],
                1180.0,
            );
            assert_eq!(app.notebook.answer, answer);
        }
    }
    #[test]
    fn direct_keys_work_after_clicking_standard_mode_in_both_languages() {
        for language in [Language::English, Language::Turkish] {
            let mut app = RustCalc::default();
            app.notebook.language = language;
            let ctx = egui::Context::default();
            frame(&mut app, &ctx, vec![], 1180.0);
            click(&mut app, &ctx, "standard");
            for text in ["1", "2", "+", "3"] {
                frame(&mut app, &ctx, vec![egui::Event::Text(text.into())], 1180.0);
            }
            // Enter must evaluate even when a mode button has keyboard focus.
            click(&mut app, &ctx, "standard");
            frame(
                &mut app,
                &ctx,
                vec![key(egui::Key::Enter, Default::default())],
                1180.0,
            );
            assert_eq!(app.notebook.answer, "15");
            assert_eq!(app.notebook.entries.len(), 1);
        }
    }
    #[test]
    fn history_search_keeps_numbers_operators_and_enter() {
        let mut app = RustCalc {
            expression: "12*3".into(),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        ctx.memory_mut(|m| m.request_focus(egui::Id::new("history_search")));
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Text("12+3".into())],
            1180.0,
        );
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::Enter, Default::default())],
            1180.0,
        );
        assert_eq!(app.search, "12+3");
        assert_eq!(app.expression, "12*3");
        assert!(app.notebook.entries.is_empty());
    }
    #[test]
    fn enter_does_not_calculate_behind_dialogs_or_other_workspaces() {
        for (page, help, about, confirm_clear) in [
            (Page::Converter, false, false, false),
            (Page::Programmer, false, false, false),
            (Page::Graph, false, false, false),
            (Page::Calculator, true, false, false),
            (Page::Calculator, false, true, false),
            (Page::Calculator, false, false, true),
        ] {
            let mut app = RustCalc {
                expression: "12*3".into(),
                page,
                help,
                about,
                confirm_clear,
                ..Default::default()
            };
            let ctx = egui::Context::default();
            frame(&mut app, &ctx, vec![], 1180.0);
            frame(
                &mut app,
                &ctx,
                vec![key(egui::Key::Enter, Default::default())],
                1180.0,
            );
            assert!(app.notebook.entries.is_empty());
        }
    }
    #[test]
    fn keypad_clicks_evaluate_and_continue_from_the_result() {
        let mut app = RustCalc::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        for button in ["2", "+", "3", "equals"] {
            click(&mut app, &ctx, button);
        }
        assert_eq!(app.notebook.answer, "5");
        for button in ["*", "4", "equals"] {
            click(&mut app, &ctx, button);
        }
        assert_eq!(app.notebook.answer, "20");
    }
    #[test]
    fn history_delete_and_clear_can_be_undone_through_the_ui() {
        let mut app = RustCalc::default();
        for expression in ["2+3", "7+8", "9*9"] {
            app.notebook.calculate(expression).unwrap();
        }
        app.notebook.entries[0].pinned = true;
        let original = app.notebook.entries.clone();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        click(&mut app, &ctx, "delete_history_1");
        assert_eq!(app.notebook.entries.len(), 2);
        app.notebook.calculate("44").unwrap();
        frame(&mut app, &ctx, vec![], 1180.0);
        click(&mut app, &ctx, "undo_history");
        assert_eq!(&app.notebook.entries[..3], original);
        assert_eq!(app.notebook.entries[3].result, "44");
        click(&mut app, &ctx, "clear_history");
        assert!(app.confirm_clear);
        assert_eq!(app.notebook.entries.len(), 4);
        click(&mut app, &ctx, "confirm_clear_history");
        assert_eq!(app.notebook.entries.len(), 1);
        assert!(app.notebook.entries[0].pinned);
        click(&mut app, &ctx, "undo_history");
        assert_eq!(app.notebook.entries.len(), 4);
        assert_eq!(app.notebook.answer, "44");
    }
    #[test]
    fn import_requires_confirmation_and_preserves_the_current_answer() {
        let mut source = Notebook::default();
        source.calculate("sqrt(2)").unwrap();
        let entries = source.entries.clone();
        let mut app = RustCalc::default();
        app.notebook.calculate("100").unwrap();
        app.confirm_clear = true;
        let (send, receive) = mpsc::channel();
        app.history_file_worker = Some(receive);
        send.send(HistoryFileReply::Loaded(Ok(Some(entries.clone()))))
            .unwrap();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        assert_eq!(app.pending_import.as_ref().unwrap(), &entries);
        assert!(
            !app.confirm_clear,
            "only one history confirmation should be open"
        );
        assert_eq!(app.notebook.entries.len(), 1);
        frame(&mut app, &ctx, vec![], 1180.0);
        click(&mut app, &ctx, "confirm_import");
        assert!(app.pending_import.is_none());
        assert_eq!(app.notebook.entries[1], entries[0]);
        assert_eq!(app.notebook.answer, "100");
        assert_eq!(app.notice.as_ref().unwrap().0, "History imported");
    }
    #[test]
    fn history_file_cancellation_and_worker_failure_do_not_modify_history() {
        let mut app = RustCalc::default();
        app.notebook.calculate("100").unwrap();
        let original = app.notebook.entries.clone();
        let ctx = egui::Context::default();
        for reply in [
            HistoryFileReply::Saved(Ok(false)),
            HistoryFileReply::Loaded(Ok(None)),
        ] {
            let (send, receive) = mpsc::channel();
            app.history_file_worker = Some(receive);
            send.send(reply).unwrap();
            frame(&mut app, &ctx, vec![], 1180.0);
            assert!(app.pending_import.is_none());
            assert!(app.history_error.is_none());
            assert!(app.notice.is_none());
            assert!(app.history_file_worker.is_none());
        }
        let (send, receive) = mpsc::channel();
        app.history_file_worker = Some(receive);
        drop(send);
        frame(&mut app, &ctx, vec![], 1180.0);
        assert!(
            app.history_error
                .as_ref()
                .unwrap()
                .contains("could not finish")
        );
        assert_eq!(app.notebook.entries, original);
    }
    #[test]
    fn import_and_error_modals_block_background_typing_and_escape_cancels() {
        for failed in [false, true] {
            let mut app = RustCalc {
                expression: "2+3".into(),
                ..Default::default()
            };
            let ctx = egui::Context::default();
            frame(&mut app, &ctx, vec![], 1180.0);
            if failed {
                app.history_error = Some("The backup contains an invalid calculation.".into());
            } else {
                app.pending_import = Some(vec![]);
            }
            frame(&mut app, &ctx, vec![], 1180.0);
            frame(&mut app, &ctx, vec![], 1180.0);
            frame(
                &mut app,
                &ctx,
                vec![
                    egui::Event::Text("999".into()),
                    key(egui::Key::Enter, Default::default()),
                ],
                1180.0,
            );
            assert_eq!(app.expression, "2+3");
            assert!(app.notebook.entries.is_empty());
            frame(
                &mut app,
                &ctx,
                vec![key(egui::Key::Escape, Default::default())],
                1180.0,
            );
            assert!(app.pending_import.is_none());
            assert!(app.history_error.is_none());
            assert_eq!(app.expression, "2+3");
        }
    }
    #[test]
    fn history_files_menu_is_localized_and_compact_popup_keeps_its_controls() {
        for language in [Language::English, Language::Turkish] {
            let mut app = RustCalc::default();
            app.notebook.language = language;
            let ctx = egui::Context::default();
            frame(&mut app, &ctx, vec![], 1180.0);
            click(&mut app, &ctx, "history_files");
            let output = frame(&mut app, &ctx, vec![], 1180.0);
            let text = rendered_text(&output);
            for label in ["Save backup…", "Export as text…", "Import backup…"] {
                assert!(text.contains(&language.text(label)), "missing {label}");
            }
            let mut compact = RustCalc {
                history_popup: true,
                ..Default::default()
            };
            compact.notebook.language = language;
            let ctx = egui::Context::default();
            frame(&mut compact, &ctx, vec![], 760.0);
            let output = frame(&mut compact, &ctx, vec![], 760.0);
            let text = rendered_text(&output);
            assert!(text.contains(&language.text("Files…")));
            assert!(text.contains(&language.text("Undo deletion")));
        }
    }
    #[test]
    fn backspace_button_respects_the_text_cursor() {
        let mut app = RustCalc {
            expression: "1234".into(),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        let id = egui::Id::new("expression");
        let mut state = egui::TextEdit::load_state(&ctx, id).unwrap();
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(
                egui::text::CCursor::new(2),
            )));
        state.store(&ctx, id);
        click(&mut app, &ctx, "backspace");
        assert_eq!(app.expression, "134");
    }
    #[test]
    fn keypad_insertion_replaces_the_current_selection() {
        let mut app = RustCalc {
            expression: "1234".into(),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        let id = egui::Id::new("expression");
        let mut state = egui::TextEdit::load_state(&ctx, id).unwrap();
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(1),
                egui::text::CCursor::new(3),
            )));
        state.store(&ctx, id);
        click(&mut app, &ctx, "7");
        assert_eq!(app.expression, "174");
    }
    #[test]
    fn error_is_visible_and_clear_recovers_the_calculator() {
        let mut app = RustCalc {
            expression: "1/0".into(),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        click(&mut app, &ctx, "equals");
        assert!(app.error.as_deref().unwrap().contains("Division by zero"));
        assert!(app.notebook.entries.is_empty());
        click(&mut app, &ctx, "clear");
        assert!(app.error.is_none());
        assert!(app.expression.is_empty());
    }
    #[test]
    fn typing_after_a_result_starts_a_fresh_calculation() {
        let mut app = RustCalc::default();
        let ctx = egui::Context::default();
        app.expression = "2+3".into();
        app.calculate();
        frame(&mut app, &ctx, vec![], 1180.0);
        frame(&mut app, &ctx, vec![egui::Event::Text("9".into())], 1180.0);
        assert_eq!(app.expression, "9");
    }
    #[test]
    fn copy_shortcut_copies_the_evaluated_result() {
        let mut app = RustCalc {
            expression: "12*3".into(),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        let out = frame(
            &mut app,
            &ctx,
            vec![key(
                egui::Key::C,
                egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
            )],
            1180.0,
        );
        assert!(
            out.platform_output
                .commands
                .iter()
                .any(|c| matches!(c, egui::OutputCommand::CopyText(s) if s == "36"))
        );
    }
    #[test]
    fn all_workspaces_render_at_compact_and_wide_sizes_in_both_themes() {
        for language in [Language::English, Language::Turkish] {
            for width in [720.0, 1180.0] {
                for light in [false, true] {
                    for page in [
                        Page::Calculator,
                        Page::Converter,
                        Page::Programmer,
                        Page::Graph,
                    ] {
                        let mut app = RustCalc {
                            page,
                            ..Default::default()
                        };
                        app.notebook.light = light;
                        app.notebook.language = language;
                        let output = frame(&mut app, &egui::Context::default(), vec![], width);
                        assert!(!output.shapes.is_empty());
                        assert!(!app.history_popup);
                    }
                }
            }
        }
    }
    #[test]
    fn persistent_state_round_trips_through_the_app_storage_format() {
        #[derive(Default)]
        struct Storage(std::collections::HashMap<String, String>);
        impl eframe::Storage for Storage {
            fn get_string(&self, key: &str) -> Option<String> {
                self.0.get(key).cloned()
            }
            fn set_string(&mut self, key: &str, value: String) {
                self.0.insert(key.into(), value);
            }
            fn flush(&mut self) {}
        }
        let mut book = Notebook::default();
        book.calculate("0.1+0.2").unwrap();
        book.memory = "42".into();
        book.light = true;
        book.language = Language::Turkish;
        book.entries[0].pinned = true;
        let mut storage = Storage::default();
        eframe::set_value(&mut storage, "notebook-v1", &book);
        let restored: Notebook = eframe::get_value(&storage, "notebook-v1").unwrap();
        assert_eq!(restored.answer, "0.3");
        assert_eq!(restored.memory, "42");
        assert!(restored.light);
        assert_eq!(restored.language, Language::Turkish);
        assert!(restored.entries[0].pinned);
    }

    #[test]
    fn committed_answer_does_not_re_evaluate_ans_with_its_own_result() {
        let mut app = RustCalc {
            expression: "10".into(),
            ..Default::default()
        };
        app.calculate();
        app.expression = "ans+4".into();
        app.calculate();
        assert_eq!(app.notebook.answer, "14");
        assert_eq!(app.preview().unwrap().text(), "14");
    }

    #[test]
    fn continuation_after_a_short_expression_inserts_at_the_end() {
        let mut app = RustCalc::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        for button in ["1", "equals", "+", "2", "equals"] {
            click(&mut app, &ctx, button);
        }
        assert_eq!(app.notebook.answer, "3", "expression: {}", app.expression);
    }

    fn rendered_text(output: &egui::FullOutput) -> String {
        fn collect(shape: &egui::Shape, text: &mut String) {
            match shape {
                egui::Shape::Text(value) => {
                    text.push_str(&value.galley.job.text);
                    text.push('\n');
                }
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, text);
                    }
                }
                _ => {}
            }
        }
        let mut text = String::new();
        for clipped in &output.shapes {
            collect(&clipped.shape, &mut text);
        }
        text
    }
    #[test]
    fn language_buttons_switch_the_rendered_interface_without_changing_calculations() {
        let mut app = RustCalc {
            expression: "0.1+0.2".into(),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let en = frame(&mut app, &ctx, vec![], 1180.0);
        assert!(rendered_text(&en).contains("Calculator"));
        click(&mut app, &ctx, "language_tr");
        let tr = frame(&mut app, &ctx, vec![], 1180.0);
        let text = rendered_text(&tr);
        assert!(text.contains("Hesap makinesi"), "{text}");
        assert!(text.contains("Geçmiş"));
        assert_eq!(app.preview().unwrap().text(), "0.3");
        click(&mut app, &ctx, "language_en");
        assert_eq!(app.notebook.language, Language::English);
    }
    #[test]
    fn updates_are_idle_without_an_explicit_request_and_worker_failures_are_visible() {
        let mut app = RustCalc {
            about: true,
            ..Default::default()
        };
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        assert!(app.update_worker.is_none());
        assert!(matches!(app.update_status, UpdateStatus::Idle));
        let (send, receive) = mpsc::channel();
        app.update_worker = Some(receive);
        app.update_status = UpdateStatus::Checking;
        send.send(UpdateReply::Checked(Err(
            "No published stable release is available yet.".into(),
        )))
        .unwrap();
        let out = frame(&mut app, &ctx, vec![], 1180.0);
        assert!(matches!(app.update_status, UpdateStatus::Failed(_)));
        assert!(rendered_text(&out).contains("No published stable release"));
        assert!(app.update_worker.is_none());
    }
}
