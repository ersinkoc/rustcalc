use eframe::egui::{self, Color32, FontId, RichText, Stroke};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rustcalc::{
    engine::{self, Angle, Context, Value},
    history,
    i18n::Language,
    model::{Entry, Notebook},
    plot::{self, Viewport},
    programmer::{self, Op, Word},
    units,
    updater::{self, AutoUpdate, UpdatePlan},
};
use std::sync::mpsc::{self, Receiver};

mod formulas;
use formulas::{Draft, FormulaFileReply};
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
enum UpdateJob {
    Check,
    Install(Box<UpdatePlan>),
}
/// Network boundary for update workers; tests substitute a local runner.
type UpdateRunner = std::sync::Arc<dyn Fn(UpdateJob) -> UpdateReply + Send + Sync>;
fn github_update_runner() -> UpdateRunner {
    std::sync::Arc::new(|job| match job {
        UpdateJob::Check => UpdateReply::Checked(updater::check()),
        UpdateJob::Install(plan) => UpdateReply::Installed(updater::install(&plan)),
    })
}
fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
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
    Formulas,
}
impl Page {
    const ALL: [(Page, &'static str); 5] = [
        (Page::Calculator, "calculator"),
        (Page::Converter, "converter"),
        (Page::Programmer, "programmer"),
        (Page::Graph, "graph"),
        (Page::Formulas, "formulas"),
    ];
    fn name(self) -> &'static str {
        Self::ALL
            .iter()
            .find(|(page, _)| *page == self)
            .map_or("calculator", |(_, name)| name)
    }
    fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .find(|(_, n)| *n == name)
            .map(|(page, _)| *page)
    }
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
    curves: [Color32; 3],
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
                curves: [rgb(42, 108, 76), rgb(38, 94, 170), rgb(168, 92, 18)],
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
                curves: [rgb(183, 230, 180), rgb(138, 188, 255), rgb(244, 190, 118)],
            }
        }
    }
}
const CURVES: [&str; 3] = ["f", "g", "h"];
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
    search_had_focus: bool,
    only_pinned: bool,
    history: bool,
    history_popup: bool,
    history_file_worker: Option<Receiver<HistoryFileReply>>,
    pending_import: Option<Vec<Entry>>,
    history_error: Option<String>,
    scientific: bool,
    help: bool,
    about: bool,
    settings: bool,
    /// The operating system's appearance, for "Follow system".
    system_light: bool,
    update_status: UpdateStatus,
    update_worker: Option<Receiver<UpdateReply>>,
    update_automatic: bool,
    update_runner: UpdateRunner,
    confirm_clear: bool,
    focus: bool,
    reset_cursor: bool,
    insertion: Option<String>,
    notice: Option<(String, Instant)>,
    /// Saved data that could not be read; kept under a separate key so an
    /// autosave never destroys a user's history.
    unreadable_state: Option<String>,
    category: usize,
    from: usize,
    to: usize,
    amount: String,
    integer: String,
    radix: u32,
    operand: String,
    word: Word,
    programmer_error: Option<String>,
    graphs: [String; 3],
    view: Viewport,
    show_points: bool,
    formula_selected: String,
    formula_values: std::collections::HashMap<String, Vec<String>>,
    formula_search: String,
    formula_draft: Option<Draft>,
    formula_delete: bool,
    formula_file_worker: Option<Receiver<FormulaFileReply>>,
    formula_error: Option<String>,
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
            search_had_focus: false,
            only_pinned: false,
            history: true,
            history_popup: false,
            history_file_worker: None,
            pending_import: None,
            history_error: None,
            scientific: true,
            help: false,
            about: false,
            settings: false,
            system_light: false,
            update_status: UpdateStatus::Idle,
            update_worker: None,
            update_automatic: false,
            update_runner: github_update_runner(),
            confirm_clear: false,
            focus: true,
            reset_cursor: false,
            insertion: None,
            notice: None,
            unreadable_state: None,
            category: 0,
            from: 0,
            to: 1,
            amount: "1000".into(),
            integer: "255".into(),
            radix: 10,
            operand: "15".into(),
            word: Word::default(),
            programmer_error: None,
            graphs: ["sin(x)".into(), String::new(), String::new()],
            view: Viewport::default(),
            show_points: true,
            formula_selected: "loan".into(),
            formula_values: Default::default(),
            formula_search: String::new(),
            formula_draft: None,
            formula_delete: false,
            formula_file_worker: None,
            formula_error: None,
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
        // Use Segoe UI on Windows, retaining bundled fallback fonts. Linux and
        // macOS use egui's bundled typefaces.
        #[cfg_attr(not(windows), allow(unused_mut))]
        let mut fonts = egui::FontDefinitions::default();
        #[cfg(windows)]
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
        let mut unreadable_state = None;
        let mut notebook: Notebook = if screenshot.is_some() {
            Notebook::default()
        } else {
            let stored = cc.storage.and_then(|s| s.get_string("notebook-v1"));
            let loaded = cc.storage.and_then(|s| eframe::get_value(s, "notebook-v1"));
            if loaded.is_none() {
                unreadable_state = stored.filter(|raw| !raw.trim().is_empty());
            }
            loaded.unwrap_or_default()
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
        // A workspace flag wins; otherwise RustCalc reopens where it was left.
        let page = Page::ALL
            .iter()
            .find(|(_, name)| args.iter().any(|s| *s == format!("--{name}")))
            .map(|(page, _)| *page)
            .or_else(|| Page::from_name(&notebook.workspace))
            .unwrap_or(Page::Calculator);
        if args.iter().any(|s| s == "--light") {
            notebook.light = true;
        }
        if let Some(code) = args.windows(2).find(|w| w[0] == "--lang") {
            notebook.language = Language::parse(&code[1]).unwrap_or_default();
        }
        let mut app = Self {
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
        };
        app.settings = args.iter().any(|s| s == "--settings");
        if unreadable_state.is_some() {
            app.toast("Saved data could not be read. A copy was kept in app.ron.");
            app.unreadable_state = unreadable_state;
        }
        app.restore_workspace();
        app
    }
    /// Apply the workspace state saved with the notebook: keypad, converter
    /// units, selected formula, plot functions and the programmer word.
    fn restore_workspace(&mut self) {
        self.scientific = self.notebook.scientific;
        if let Some([category, from, to]) = self.notebook.converter
            && let Some(units) = units::CATEGORIES.get(category).map(|c| c.units.len())
            && from < units
            && to < units
        {
            (self.category, self.from, self.to) = (category, from, to);
        }
        if let Some(id) = self.notebook.formula_selected.clone()
            && (rustcalc::formulas::is_builtin(&id)
                || self.notebook.formulas.iter().any(|f| f.id == id))
        {
            self.formula_selected = id;
        }
        if let Some(plots) = self.notebook.plots.clone() {
            self.graphs = plots;
        }
        if let Some(bits) = self
            .notebook
            .programmer_bits
            .filter(|b| programmer::SIZES.contains(b))
        {
            self.word = Word {
                bits,
                signed: self.notebook.programmer_signed,
            };
            if self.word.parse(&self.integer, self.radix).is_err() {
                self.integer = "0".into();
            }
        }
    }
    /// Light or dark as chosen, or as the operating system is set.
    fn light(&self) -> bool {
        if self.notebook.follow_system_theme {
            self.system_light
        } else {
            self.notebook.light
        }
    }
    fn palette(&self) -> Palette {
        Palette::new(self.light(), self.notebook.language)
    }
    fn preview(&self) -> Result<Value, engine::CalcError> {
        if self.committed {
            return Ok(Value {
                number: self.notebook.context().ans,
                approximate: self.notebook.answer_approximate,
            });
        }
        self.notebook.evaluate(&self.expression).map(|(_, v)| v)
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
                    (Page::Formulas, "05", "Formulas"),
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
                    let settings = ui
                        .add_sized(
                            [142.0, 35.0],
                            egui::Button::new(label("⚙    Settings", 13.0, p.muted, p.language))
                                .fill(Color32::TRANSPARENT),
                        )
                        .on_hover_text("Ctrl+,");
                    #[cfg(test)]
                    self.targets.insert("settings".into(), settings.rect);
                    if settings.clicked() {
                        self.settings = true;
                    }
                    if ui
                        .add_sized(
                            [142.0, 35.0],
                            egui::Button::new(label("?    Help", 13.0, p.muted, p.language))
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
                                if self.light() {
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
                        self.notebook.light = !self.light();
                        self.notebook.follow_system_theme = false;
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
        let search = ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .id(egui::Id::new("history_search"))
                .hint_text(p.tr("Search expressions or results…"))
                .desired_width(f32::INFINITY)
                .margin(egui::vec2(10.0, 9.0)),
        );
        self.search_had_focus = search.has_focus();
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.only_pinned, false, p.tr("All"));
            ui.selectable_value(&mut self.only_pinned, true, p.tr("Pinned"));
        });
        ui.add_space(8.0);
        let mut reuse = None;
        let mut copy = None;
        let mut pin = None;
        let mut delete = None;
        // Results are stored with a decimal point; a typed comma finds them too.
        let query = rustcalc::i18n::fold(self.search.trim()).replace(',', ".");
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
                                || rustcalc::i18n::fold(&e.expression)
                                    .replace(',', ".")
                                    .contains(&query)
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
                                        format!("{prefix}{}", self.notebook.show(&entry.result)),
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
                                            copy = Some(self.notebook.copy_text(&entry.result));
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
                Ok(v) => (self.notebook.show(&v.text()), v.approximate),
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
                    ui.ctx().copy_text(self.notebook.copy_text(&value.text()));
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
            } else if self.expression.contains('%') {
                ui.label(label(
                    "%: 100 + 10% = 110 and 100 − 10% = 90 change a number by a share of itself; 100 × 10% = 10; 10% alone = 0.1.",
                    12.0,
                    p.muted,
                    p.language,
                ));
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
                                    self.notebook.memory = v.exact_text();
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
                    .on_hover_text(self.notebook.show(&self.notebook.memory));
            }
        });
        self.variables_strip(ui, p);
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
        // The decimal key matches the mark results are shown with.
        let mark = if self.notebook.numbers.comma(self.notebook.language) {
            ","
        } else {
            "."
        };
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
            &[("0", "0"), (mark, mark), ("=", "equals"), ("+", "+")],
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
                ui.label(label("100 + 10% = 110", 10.0, p.muted, p.language))
                    .on_hover_text(p.tr("Adding or subtracting a percentage changes a number by that share of itself. 100 × 10% = 10; 10% alone = 0.1."));
            });
        });
    }
    /// Named values as chips: click a name to insert it, × to delete it.
    fn variables_strip(&mut self, ui: &mut egui::Ui, p: Palette) {
        if self.notebook.variables.is_empty() {
            return;
        }
        let mut insert = None;
        let mut delete = None;
        ui.horizontal_wrapped(|ui| {
            ui.label(label("VARIABLES", 9.0, p.muted, p.language));
            for (name, variable) in &self.notebook.variables {
                let prefix = if variable.approximate { "≈ " } else { "" };
                let chip = ui
                    .add(
                        egui::Button::new(label(
                            format!("{name} = {prefix}{}", self.notebook.show(&variable.value)),
                            12.0,
                            p.text,
                            p.language,
                        ))
                        .fill(p.card),
                    )
                    .on_hover_text(p.tr("Insert this name"));
                #[cfg(test)]
                self.targets.insert(format!("variable_{name}"), chip.rect);
                if chip.clicked() {
                    insert = Some(name.clone());
                }
                let remove = ui
                    .small_button(p.tr("×"))
                    .on_hover_text(p.tr("Delete this variable"));
                #[cfg(test)]
                self.targets
                    .insert(format!("delete_variable_{name}"), remove.rect);
                if remove.clicked() {
                    delete = Some(name.clone());
                }
            }
        });
        if let Some(name) = insert {
            self.insert(&name);
        }
        if let Some(name) = delete {
            self.notebook.variables.remove(&name);
            self.toast("Variable deleted");
        }
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
        // The amount may be an expression (1500*12) and may carry stray spaces.
        let converted = self
            .notebook
            .evaluate(self.amount.trim())
            .map(|(_, v)| v.number)
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
                            self.notebook.show(&v.normalize().to_string()),
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
            .add_enabled(converted.is_ok(), egui::Button::new(p.tr("Copy result")))
            .clicked()
            && let Ok(v) = converted
        {
            ui.ctx()
                .copy_text(self.notebook.copy_text(&v.normalize().to_string()));
            self.toast("Conversion copied");
        }
        ui.add_space(24.0);
        ui.label(label(
            "11 categories  ·  SI, imperial and binary data units  ·  Offline",
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
            "Clarity at the bit level. 8 to 64-bit integers, signed or unsigned.",
        );
        let current = self.word.parse(&self.integer, self.radix);
        ui.horizontal_wrapped(|ui| {
            for (radix, title) in [(16, "HEX"), (10, "DEC"), (8, "OCT"), (2, "BIN")] {
                if ui
                    .selectable_label(self.radix == radix, p.tr(title))
                    .clicked()
                {
                    if let Ok(n) = current {
                        self.integer = self.word.format(n, radix);
                    }
                    self.radix = radix;
                }
            }
            ui.separator();
            for bits in programmer::SIZES {
                let button = ui.selectable_label(self.word.bits == bits, format!("{bits}-bit"));
                #[cfg(test)]
                self.targets.insert(format!("word_{bits}"), button.rect);
                if button.clicked() {
                    // Narrowing keeps the low bits, like casting in most languages.
                    let word = Word { bits, ..self.word };
                    if let Ok(n) = current {
                        self.integer = word.format(n, self.radix);
                    }
                    self.word = word;
                }
            }
            ui.separator();
            let mut signed = self.word.signed;
            let toggle = ui.checkbox(&mut signed, p.tr("Signed"));
            #[cfg(test)]
            self.targets.insert("signed".into(), toggle.rect);
            if toggle.changed() {
                // The bit pattern stays; only its decimal meaning changes.
                let word = Word {
                    signed,
                    ..self.word
                };
                if let Ok(n) = current {
                    self.integer = word.format(n, self.radix);
                }
                self.word = word;
            }
        });
        ui.add_space(12.0);
        card(p).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(label("INPUT", 10.0, p.muted, p.language));
            let input = ui.add(
                egui::TextEdit::singleline(&mut self.integer)
                    .id(egui::Id::new("programmer_input"))
                    .font(FontId::monospace(24.0))
                    .desired_width(f32::INFINITY)
                    .frame(false)
                    .char_limit(80),
            );
            if input.changed() {
                self.programmer_error = None;
            }
        });
        ui.add_space(14.0);
        let word = self.word;
        match word.parse(&self.integer, self.radix) {
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
                            let text = word.format(n, base);
                            let shown = if base == 2 {
                                word.grouped_binary(n)
                            } else {
                                text.clone()
                            };
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new(shown)
                                            .monospace()
                                            .size(if base == 2 { 13.0 } else { 19.0 })
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
                // Sixteen cells per row when they fit, otherwise eight, so the map
                // never runs past the right edge of compact windows.
                let available = ui.available_width();
                let per_row = if available >= 16.0 * 26.0 + 15.0 * 4.0 {
                    16
                } else {
                    8
                };
                let width =
                    ((available - (per_row - 1) as f32 * 4.0) / per_row as f32).clamp(18.0, 30.0);
                egui::Grid::new("bits")
                    .spacing(egui::vec2(4.0, 5.0))
                    .min_col_width(0.0)
                    .show(ui, |ui| {
                        ui.spacing_mut().button_padding = egui::Vec2::ZERO;
                        for bit in (0..word.bits).rev() {
                            let set = n & (1u64 << bit) != 0;
                            let button = ui
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
                                .on_hover_text(p.tr(format!("Bit {bit} · 2^{bit}")));
                            #[cfg(test)]
                            self.targets.insert(format!("bit_{bit}"), button.rect);
                            if button.clicked() {
                                self.integer = word.format(n ^ (1u64 << bit), self.radix);
                            }
                            if bit % per_row == 0 {
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
                let operand = word.parse(&self.operand, 10);
                let mut failure = None;
                ui.horizontal_wrapped(|ui| {
                    for (op, title) in [
                        (Op::Add, "+"),
                        (Op::Sub, "−"),
                        (Op::Mul, "×"),
                        (Op::Div, "÷"),
                        (Op::Mod, "MOD"),
                        (Op::And, "AND"),
                        (Op::Or, "OR"),
                        (Op::Xor, "XOR"),
                        (Op::Shl, "<< n"),
                        (Op::Shr, ">> n"),
                    ] {
                        let button = ui.add_enabled(operand.is_ok(), egui::Button::new(title));
                        #[cfg(test)]
                        self.targets.insert(format!("op_{title}"), button.rect);
                        if button.clicked()
                            && let Ok(rhs) = operand
                        {
                            match word.apply(op, n, rhs) {
                                Ok(result) => {
                                    self.integer = word.format(result, self.radix);
                                    self.programmer_error = None;
                                }
                                Err(e) => failure = Some(e),
                            }
                        }
                    }
                    ui.separator();
                    for (op, title) in [
                        (Op::Not, "NOT"),
                        (Op::Neg, "NEG"),
                        (Op::Shl, "<< 1"),
                        (Op::Shr, ">> 1"),
                    ] {
                        let button = ui.button(title);
                        #[cfg(test)]
                        self.targets.insert(format!("op_{title}"), button.rect);
                        if button.clicked()
                            && let Ok(result) = word.apply(op, n, 1)
                        {
                            self.integer = word.format(result, self.radix);
                        }
                    }
                });
                if failure.is_some() {
                    self.programmer_error = failure;
                }
                if let Some(e) = &self.programmer_error {
                    ui.label(label(e, 12.0, p.danger, p.language));
                }
                if let Err(e) = operand {
                    ui.label(label(e, 12.0, p.danger, p.language));
                }
                ui.add_space(10.0);
                ui.label(label(
                    "Results wrap to the selected word size. Signed values use two's complement, and >> keeps the sign.",
                    12.0,
                    p.muted,
                    p.language,
                ));
            }
        }
    }
    fn graph(&mut self, ui: &mut egui::Ui, p: Palette) {
        self.header(ui, p, "Plot a graph", "Explore the shape of a function.");
        for (i, name) in CURVES.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(label(format!("{name}(x) ="), 18.0, p.curves[i], p.language));
                ui.add(
                    egui::TextEdit::singleline(&mut self.graphs[i])
                        .id(egui::Id::new(("plot_expression", i)))
                        .font(FontId::monospace(17.0))
                        .desired_width(f32::INFINITY)
                        .hint_text(
                            RichText::new(if i == 0 {
                                String::new()
                            } else {
                                p.tr("Optional")
                            })
                            .color(p.muted.gamma_multiply(0.7)),
                        )
                        .char_limit(512),
                );
            });
        }
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            for f in ["sin(x)", "x^2", "1/x", "sqrt(x)", "tan(x)"] {
                if ui.button(p.tr(f)).clicked() {
                    self.graphs[0] = f.into();
                }
            }
            ui.separator();
            let center = (self.view.center_x, self.view.center_y);
            let zoom_in = ui.button("+").on_hover_text(p.tr("Zoom in"));
            let zoom_out = ui.button("−").on_hover_text(p.tr("Zoom out"));
            let reset = ui.button(p.tr("Reset view"));
            #[cfg(test)]
            {
                self.targets.insert("plot_zoom_in".into(), zoom_in.rect);
                self.targets.insert("plot_reset".into(), reset.rect);
            }
            if zoom_in.clicked() {
                self.view.zoom_at(0.5, center.0, center.1);
            }
            if zoom_out.clicked() {
                self.view.zoom_at(2.0, center.0, center.1);
            }
            if reset.clicked() {
                self.view = Viewport::default();
            }
            ui.separator();
            ui.checkbox(&mut self.show_points, p.tr("Roots and intersections"));
            ui.label(label("RAD", 11.0, p.muted, p.language));
        });
        ui.add(
            egui::Slider::new(&mut self.view.span, plot::MIN_SPAN..=plot::MAX_SPAN)
                .logarithmic(true)
                .text(p.tr("View ±")),
        );
        ui.add_space(8.0);
        let size = egui::vec2(
            ui.available_width(),
            (ui.available_height() - 45.0).clamp(240.0, 540.0),
        );
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
        #[cfg(test)]
        self.targets.insert("plot".into(), rect);
        let to_world = |pos: egui::Pos2, view: &Viewport| {
            (
                view.center_x
                    + (pos.x - rect.center().x) as f64 / (rect.width() / 2.0) as f64 * view.span,
                view.center_y
                    - (pos.y - rect.center().y) as f64 / (rect.height() / 2.0) as f64 * view.span,
            )
        };
        if response.dragged() {
            let d = response.drag_delta();
            self.view.pan(
                -d.x as f64 / rect.width() as f64,
                d.y as f64 / rect.height() as f64,
            );
        }
        if response.double_clicked() {
            self.view = Viewport::default();
        }
        if let Some(pos) = response.hover_pos() {
            // Take the wheel from the surrounding scroll area while over the plot.
            let (scroll, zoom) = ui.input_mut(|i| {
                let scroll = i.smooth_scroll_delta.y;
                i.smooth_scroll_delta = egui::Vec2::ZERO;
                (scroll, i.zoom_delta())
            });
            let factor = (-(scroll as f64) * 0.0025).exp() / zoom as f64;
            if factor != 1.0 {
                let (x, y) = to_world(pos, &self.view);
                self.view.zoom_at(factor, x, y);
            }
            // Keyboard navigation applies while the pointer is over the plot and
            // no text field would otherwise receive the keys.
            if ui.memory(|m| m.focused().is_none()) {
                let center = (self.view.center_x, self.view.center_y);
                let view = &mut self.view;
                ui.input(|i| {
                    for (key, dx, dy) in [
                        (egui::Key::ArrowLeft, -0.1, 0.0),
                        (egui::Key::ArrowRight, 0.1, 0.0),
                        (egui::Key::ArrowUp, 0.0, 0.1),
                        (egui::Key::ArrowDown, 0.0, -0.1),
                    ] {
                        if i.key_pressed(key) {
                            view.pan(dx, dy);
                        }
                    }
                    if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                        view.zoom_at(0.5, center.0, center.1);
                    }
                    if i.key_pressed(egui::Key::Minus) {
                        view.zoom_at(2.0, center.0, center.1);
                    }
                    if i.key_pressed(egui::Key::Home) {
                        *view = Viewport::default();
                    }
                });
            }
        }
        self.view.clamp();
        let view = self.view;
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 10, p.card);
        let map = |(x, y): (f64, f64)| {
            egui::pos2(
                rect.center().x + ((x - view.center_x) / view.span) as f32 * rect.width() / 2.0,
                rect.center().y - ((y - view.center_y) / view.span) as f32 * rect.height() / 2.0,
            )
        };
        let (x_range, y_range) = (view.x_range(), view.y_range());
        let x_step = plot::tick_step(
            2.0 * view.span,
            (rect.width() / 90.0).clamp(4.0, 12.0) as usize,
        );
        let y_step = plot::tick_step(
            2.0 * view.span,
            (rect.height() / 60.0).clamp(4.0, 12.0) as usize,
        );
        // Labels follow the axes, or stay pinned to the edge when an axis is off-screen.
        let axis_x = map((0.0, 0.0))
            .x
            .clamp(rect.left() + 4.0, rect.right() - 48.0);
        let axis_y = map((0.0, 0.0))
            .y
            .clamp(rect.top() + 4.0, rect.bottom() - 16.0);
        for x in plot::ticks(x_range.0, x_range.1, x_step) {
            let px = map((x, 0.0)).x;
            painter.line_segment(
                [egui::pos2(px, rect.top()), egui::pos2(px, rect.bottom())],
                Stroke::new(1.0_f32, if x == 0.0 { p.muted } else { p.border }),
            );
            // Edge ticks would be clipped by the card; their grid line is enough.
            if x != 0.0 && px > rect.left() + 6.0 && px < rect.right() - 40.0 {
                painter.text(
                    egui::pos2(px + 3.0, axis_y + 2.0),
                    egui::Align2::LEFT_TOP,
                    plot::tick_label(x, x_step),
                    FontId::monospace(10.0),
                    p.muted,
                );
            }
        }
        for y in plot::ticks(y_range.0, y_range.1, y_step) {
            let py = map((0.0, y)).y;
            painter.line_segment(
                [egui::pos2(rect.left(), py), egui::pos2(rect.right(), py)],
                Stroke::new(1.0_f32, if y == 0.0 { p.muted } else { p.border }),
            );
            if y != 0.0 && py > rect.top() + 16.0 && py < rect.bottom() - 6.0 {
                painter.text(
                    egui::pos2(axis_x + 3.0, py - 2.0),
                    egui::Align2::LEFT_BOTTOM,
                    plot::tick_label(y, y_step),
                    FontId::monospace(10.0),
                    p.muted,
                );
            }
        }
        let base = self.notebook.context();
        let context = |x: f64| {
            Some(Context {
                angle: Angle::Radians,
                x: Some(Decimal::from_f64_retain(x)?),
                ..base
            })
        };
        let curves: Vec<(usize, &str)> = self
            .graphs
            .iter()
            .enumerate()
            .map(|(i, e)| (i, e.trim()))
            .filter(|(_, e)| !e.is_empty())
            .collect();
        // Parse each curve once per frame; sampling evaluates it many times.
        let parsed: std::collections::HashMap<&str, engine::Expression> = curves
            .iter()
            .filter_map(|&(_, e)| Some((e, engine::Expression::parse(e).ok()?)))
            .collect();
        // Named values work in curves too: a*x^2 after a = 3.
        let variables = self.notebook.engine_variables();
        let eval = |expression: &str, x: f64| -> Option<f64> {
            parsed
                .get(expression)?
                .eval(&context(x)?, &variables)
                .ok()?
                .number
                .to_f64()
                .filter(|v| v.is_finite())
        };
        // Clip slightly outside the card so line joins at the edge stay clean.
        let margin = view.span * 0.05;
        let clip_x = (x_range.0 - margin, x_range.1 + margin);
        let clip_y = (y_range.0 - margin, y_range.1 + margin);
        let mut problems = Vec::new();
        for &(i, expression) in &curves {
            let segments = plot::sample(|x| eval(expression, x), &view, rect.width() as usize);
            if segments.is_empty() {
                let reason = match context(view.center_x)
                    .map(|c| engine::evaluate_with(expression, c, &variables))
                {
                    Some(Err(e)) => p.tr(e.message),
                    _ => p.tr("No plottable values in this range. Change the expression or view."),
                };
                problems.push(format!("{}(x): {reason}", CURVES[i]));
            }
            let stroke = Stroke::new(2.5_f32, p.curves[i]);
            for segment in segments {
                let mut line: Vec<egui::Pos2> = Vec::new();
                let mut end: Option<(f64, f64)> = None;
                for pair in segment.windows(2) {
                    let Some((a, b)) = plot::clip(pair[0], pair[1], clip_x, clip_y) else {
                        continue;
                    };
                    if end != Some(a) {
                        if line.len() > 1 {
                            painter.add(egui::Shape::line(std::mem::take(&mut line), stroke));
                        }
                        line.clear();
                        line.push(map(a));
                    }
                    line.push(map(b));
                    end = Some(b);
                }
                if line.len() > 1 {
                    painter.add(egui::Shape::line(line, stroke));
                }
            }
        }
        let mut findings: Vec<(String, Vec<f64>, Color32)> = Vec::new();
        if self.show_points {
            let samples = (rect.width() as usize / 2).max(100);
            let mut marks = Vec::new();
            for &(i, expression) in &curves {
                let found = plot::roots(|x| eval(expression, x), x_range, samples);
                marks.extend(found.iter().map(|&x| ((x, 0.0), p.curves[i])));
                if !found.is_empty() {
                    findings.push((format!("{}(x) = 0", CURVES[i]), found, p.curves[i]));
                }
            }
            for (k, &(i, a)) in curves.iter().enumerate() {
                for &(j, b) in &curves[k + 1..] {
                    let found = plot::roots(|x| Some(eval(a, x)? - eval(b, x)?), x_range, samples);
                    marks.extend(
                        found
                            .iter()
                            .filter_map(|&x| Some(((x, eval(a, x)?), p.text))),
                    );
                    if !found.is_empty() {
                        findings.push((
                            format!("{}(x) = {}(x)", CURVES[i], CURVES[j]),
                            found,
                            p.muted,
                        ));
                    }
                }
            }
            for ((x, y), color) in marks {
                if (y_range.0..=y_range.1).contains(&y) {
                    painter.circle(map((x, y)), 4.5, p.card, Stroke::new(2.0_f32, color));
                }
            }
        }
        if let Some(pos) = response.hover_pos() {
            let (x, _) = to_world(pos, &view);
            painter.line_segment(
                [
                    egui::pos2(pos.x, rect.top()),
                    egui::pos2(pos.x, rect.bottom()),
                ],
                Stroke::new(1.0_f32, p.muted),
            );
            let mut lines = vec![(format!("x = {}", plot::readout(x)), p.text)];
            for &(i, expression) in &curves {
                if let Some(y) = eval(expression, x) {
                    if (y_range.0..=y_range.1).contains(&y) {
                        painter.circle_filled(map((x, y)), 5.0, p.curves[i]);
                    }
                    lines.push((
                        format!("{}(x) = {}", CURVES[i], plot::readout(y)),
                        p.curves[i],
                    ));
                }
            }
            let galleys: Vec<_> = lines
                .into_iter()
                .map(|(text, color)| painter.layout_no_wrap(text, FontId::monospace(13.0), color))
                .collect();
            let width = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max);
            let height: f32 = galleys.iter().map(|g| g.size().y + 2.0).sum();
            let mut at = rect.left_top() + egui::vec2(18.0, 16.0);
            painter.rect_filled(
                egui::Rect::from_min_size(
                    at - egui::vec2(8.0, 6.0),
                    egui::vec2(width + 16.0, height + 10.0),
                ),
                6,
                p.card.gamma_multiply(0.9),
            );
            for galley in galleys {
                let height = galley.size().y + 2.0;
                painter.galley(at, galley, p.text);
                at.y += height;
            }
        }
        ui.add_space(8.0);
        if curves.is_empty() {
            ui.label(label(
                "Enter a function of x to plot.",
                12.0,
                p.muted,
                p.language,
            ));
        }
        for problem in problems {
            ui.label(label(problem, 12.0, p.danger, p.language));
        }
        for (equation, found, color) in findings {
            const SHOWN: usize = 8;
            let mut list = found
                .iter()
                .take(SHOWN)
                .map(|&x| plot::readout(x))
                .collect::<Vec<_>>()
                .join(", ");
            if found.len() > SHOWN {
                list.push_str(&format!(" … (+{})", found.len() - SHOWN));
            }
            ui.label(
                RichText::new(format!("{equation}:  x ≈ {list}"))
                    .monospace()
                    .size(12.0)
                    .color(color),
            );
        }
        ui.label(label(
            "Drag to pan, scroll to zoom, double-click to reset. Both axes use the same numeric span; angles are in radians.",
            12.0,
            p.muted,
            p.language,
        ));
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
        self.help_window(ctx, p);
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
        if self.settings {
            self.settings_dialog(ctx, p);
        }
    }
    /// Keyboard shortcuts that work everywhere in the window.
    fn shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};
        let pressed = |key| ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, key));
        for (key, page) in [
            (Key::Num1, Page::Calculator),
            (Key::Num2, Page::Converter),
            (Key::Num3, Page::Programmer),
            (Key::Num4, Page::Graph),
            (Key::Num5, Page::Formulas),
        ] {
            if pressed(key) {
                self.page = page;
                self.error = None;
                self.focus = true;
            }
        }
        if pressed(Key::Plus) || pressed(Key::Equals) {
            self.notebook.step_zoom(1);
        }
        if pressed(Key::Minus) {
            self.notebook.step_zoom(-1);
        }
        if pressed(Key::Num0) {
            self.notebook.zoom = 1.0;
        }
        if pressed(Key::Comma) {
            self.settings = !self.settings;
        }
    }
    fn settings_dialog(&mut self, ctx: &egui::Context, p: Palette) {
        use rustcalc::numfmt::DecimalMark;
        let mut open = self.settings;
        egui::Window::new(p.tr("Settings"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(460.0)
            .show(ctx, |ui| {
                let section = |ui: &mut egui::Ui, title: &str| {
                    ui.add_space(8.0);
                    ui.label(label(title, 10.0, p.muted, p.language).strong());
                    ui.add_space(2.0);
                };
                section(ui, "LANGUAGE");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.notebook.language, Language::English, "English");
                    ui.selectable_value(&mut self.notebook.language, Language::Turkish, "Türkçe");
                });
                section(ui, "APPEARANCE");
                ui.horizontal(|ui| {
                    let mut choice = if self.notebook.follow_system_theme {
                        0
                    } else if self.notebook.light {
                        1
                    } else {
                        2
                    };
                    for (value, title) in [(0, "Follow system"), (1, "Light"), (2, "Dark")] {
                        let button = ui.selectable_value(&mut choice, value, p.tr(title));
                        #[cfg(test)]
                        self.targets.insert(format!("theme_{title}"), button.rect);
                        #[cfg(not(test))]
                        let _ = button;
                    }
                    self.notebook.follow_system_theme = choice == 0;
                    if choice != 0 {
                        self.notebook.light = choice == 1;
                    }
                });
                section(ui, "TEXT SIZE");
                ui.horizontal_wrapped(|ui| {
                    let current = self.notebook.zoom();
                    for step in rustcalc::model::ZOOM_STEPS {
                        let text = format!("{}%", (step * 100.0).round());
                        if ui
                            .selectable_label((current - step).abs() < 0.001, text)
                            .clicked()
                        {
                            self.notebook.zoom = step;
                        }
                    }
                });
                ui.label(label(
                    "Ctrl + and Ctrl − change the size; Ctrl 0 resets it.",
                    12.0,
                    p.muted,
                    p.language,
                ));
                section(ui, "NUMBERS");
                let numbers = &mut self.notebook.numbers;
                ui.horizontal(|ui| {
                    ui.label(p.tr("Decimal mark"));
                    for (mark, title) in [
                        (DecimalMark::Automatic, "By language"),
                        (DecimalMark::Point, "Point 1.5"),
                        (DecimalMark::Comma, "Comma 1,5"),
                    ] {
                        ui.selectable_value(&mut numbers.mark, mark, p.tr(title));
                    }
                });
                let grouping = ui.checkbox(&mut numbers.grouping, p.tr("Group digits (1 234 567)"));
                #[cfg(test)]
                self.targets.insert("grouping".into(), grouping.rect);
                #[cfg(not(test))]
                let _ = grouping;
                ui.horizontal(|ui| {
                    ui.label(p.tr("Decimal places"));
                    let shown = match numbers.decimals {
                        None => p.tr("All digits"),
                        Some(d) => d.to_string(),
                    };
                    egui::ComboBox::from_id_salt("decimal_places")
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut numbers.decimals, None, p.tr("All digits"));
                            for d in 0..=10 {
                                ui.selectable_value(&mut numbers.decimals, Some(d), d.to_string());
                            }
                        });
                });
                let example = self.notebook.show("1234567.891");
                ui.label(label(
                    format!("{}  {example}", p.tr("Example:")),
                    12.0,
                    p.muted,
                    p.language,
                ));
                ui.label(label(
                    "Only the display changes: RustCalc still calculates with every digit, and you can type either mark.",
                    12.0,
                    p.muted,
                    p.language,
                ));
                section(ui, "UPDATES");
                if ui.button(p.tr("About & updates")).clicked() {
                    self.about = true;
                }
                ui.add_space(6.0);
                ui.label(label(
                    "Shortcuts: Ctrl 1–5 switch workspaces · F1 help · Ctrl , settings",
                    12.0,
                    p.muted,
                    p.language,
                ));
            });
        self.settings = open;
    }
    /// F1: a short user guide with worked examples, one section per topic.
    fn help_window(&mut self, ctx: &egui::Context, p: Palette) {
        let example = |ui: &mut egui::Ui, rows: &[(&str, &str, &str)]| {
            egui::Grid::new(ui.next_auto_id())
                .num_columns(3)
                .spacing(egui::vec2(14.0, 4.0))
                .show(ui, |ui| {
                    for (input, result, meaning) in rows {
                        ui.label(RichText::new(*input).monospace().color(p.text));
                        ui.label(RichText::new(*result).monospace().color(p.accent));
                        ui.label(label(*meaning, 12.0, p.muted, p.language));
                        ui.end_row();
                    }
                });
        };
        let section =
            |ui: &mut egui::Ui, title: &str, open: bool, body: &mut dyn FnMut(&mut egui::Ui)| {
                egui::CollapsingHeader::new(RichText::new(p.tr(title)).strong())
                    .id_salt(("help", title))
                    // Tests open every section so the translation check sees all text.
                    .default_open(open || cfg!(test))
                    .show(ui, |ui| body(ui));
            };
        let text = |ui: &mut egui::Ui, s: &str| {
            ui.label(label(s, 13.0, p.text, p.language));
        };
        egui::Window::new(p.tr("How to use RustCalc"))
            .open(&mut self.help)
            .collapsible(false)
            .resizable(true)
            .default_width(540.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().max_height(560.0).show(ui, |ui| {
                    section(ui, "Calculating", true, &mut |ui| {
                        text(ui, "Type an expression; the result appears as you type. Press Enter (or =) to save it to history. Start with an operator such as + to continue from the last result.");
                        example(ui, &[
                            ("2 + 3 * 4", "14", "Multiplication before addition"),
                            ("(2 + 3) * 4", "20", "Parentheses first"),
                            ("2^3^2", "512", "Powers group from the right"),
                            ("-2^2", "-4", "The power applies before the minus"),
                            ("2pi, 2(3+4)", "6.28…, 14", "A number next to a name or bracket multiplies"),
                            ("ans * 2", "", "ans is the last result"),
                        ]);
                        text(ui, "Use a comma or a period as the decimal point, never as a thousands separator: 1,5 and 1.5 both mean one and a half.");
                    });
                    section(ui, "Percent", true, &mut |ui| {
                        text(ui, "Percent works like a shop calculator: adding or subtracting a percentage changes the number by that share of itself.");
                        example(ui, &[
                            ("100 + 10%", "110", "Adds 10% of 100"),
                            ("100 - 10%", "90", "Takes off 10% of 100"),
                            ("100 * 10%", "10", "10% of 100"),
                            ("100 / 10%", "1000", "Divides by 0.1"),
                            ("10%", "0.1", "On its own, a percentage is a fraction"),
                            ("100 + 10% + 10%", "121", "Each step uses the running total"),
                        ]);
                    });
                    section(ui, "Exact and approximate results", false, &mut |ui| {
                        text(ui, "Ordinary arithmetic is exact decimal arithmetic, so 0.1 + 0.2 = 0.3. A division that never ends, such as 1/3, is marked ≈ and still gives 1/3*3 = 1.");
                        text(ui, "≈ marks results that were rounded to about 15 significant digits, from functions such as sqrt(2), sin(1) or ln(2). Results proven exact, such as sqrt(144) or sin(30) in degrees, show no ≈.");
                        text(ui, "DEG and RAD choose degrees or radians for sin, cos and tan. Plots always use radians.");
                    });
                    section(ui, "Functions", false, &mut |ui| {
                        text(ui, "Functions: sin, cos, tan, asin, acos, atan, sinh, cosh, tanh, sqrt, cbrt, ln, log, exp, abs, floor, ceil, round, sign, min, max, sum, avg, gcd, lcm, ncr, npr, mod, if, and, or, not. Separate arguments with ; as in max(2; 5). Constants: pi, e, ans, mem.");
                        example(ui, &[
                            ("max(4; 9; 2)", "9", "Largest value"),
                            ("round(2,675; 2)", "2.68", "Round to two decimals"),
                            ("mod(17; 5)", "2", "Remainder"),
                            ("if(7 > 5; 1; 0)", "1", "Choose by a comparison"),
                        ]);
                    });
                    section(ui, "Dates", false, &mut |ui| {
                        text(ui, "Dates are day numbers: date(2026; 12; 31) - today() gives days. Also year, month, day, weekday, addmonths, addyears, addworkdays, months, years, workdays, endofmonth.");
                        text(ui, "The Formulas workspace has ready-made date calculations with date fields.");
                    });
                    section(ui, "Memory and history", false, &mut |ui| {
                        text(ui, "MS stores the current result, M+ and M− add it to or subtract it from memory, MR inserts mem, MC clears it.");
                        text(ui, "History keeps up to 500 calculations. Click a result to reuse it, ↗ to edit its expression, ☆ to pin it, and use Files… to back up or import. Undo deletion restores removed entries during this session.");
                    });
                    section(ui, "Workspaces", false, &mut |ui| {
                        text(ui, "Convert units: pick a category, type a value and choose both units.");
                        text(ui, "Programmer: choose the word size and Signed, type in HEX, DEC, OCT or BIN, click bits to flip them.");
                        text(ui, "Plot a graph: enter up to three functions of x. Drag to move, scroll to zoom, double-click to reset.");
                        text(ui, "Formulas: pick a template or create your own, type values, read the results. Click a result to copy it.");
                    });
                    section(ui, "Keyboard", false, &mut |ui| {
                        for (key, action) in [
                            ("Enter", "Calculate and save to history"),
                            ("Esc", "Clear the expression"),
                            ("Ctrl + L", "Select and edit the expression"),
                            ("Ctrl + Shift + C", "Copy result"),
                            ("F1", "Open this help window"),
                        ] {
                            ui.horizontal(|ui| {
                                ui.add_sized([140.0, 22.0], egui::Label::new(label(key, 13.0, p.accent, p.language)));
                                ui.label(p.tr(action));
                            });
                        }
                    });
                    section(ui, "Privacy and data", false, &mut |ui| {
                        text(ui, "History (up to 500 entries), memory, angle mode, language, theme and your formulas are saved locally. GitHub is contacted only for update checks you request or enable.");
                    });
                });
            });
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
        let run = self.update_runner.clone();
        std::thread::spawn(move || {
            let _ = send.send(run(UpdateJob::Check));
            ctx.request_repaint();
        });
    }
    fn start_update_install(&mut self, plan: UpdatePlan, ctx: &egui::Context) {
        let (send, receive) = mpsc::channel();
        self.update_worker = Some(receive);
        self.update_status = UpdateStatus::Installing;
        let ctx = ctx.clone();
        let run = self.update_runner.clone();
        std::thread::spawn(move || {
            let _ = send.send(run(UpdateJob::Install(Box::new(plan))));
            ctx.request_repaint();
        });
    }
    fn poll_update(&mut self, ctx: &egui::Context) {
        if let Some(receiver) = &self.update_worker {
            match receiver.try_recv() {
                Ok(reply) => {
                    self.update_worker = None;
                    let automatic = std::mem::take(&mut self.update_automatic);
                    match reply {
                        UpdateReply::Checked(Ok(Some(plan))) => {
                            if automatic && self.notebook.auto_update == AutoUpdate::Install {
                                // The same verified install path as a manual update.
                                self.update_automatic = true;
                                self.start_update_install(plan, ctx);
                            } else {
                                if automatic {
                                    self.toast(format!(
                                        "RustCalc update available: {}",
                                        plan.version
                                    ));
                                }
                                self.update_status = UpdateStatus::Available(Box::new(plan));
                            }
                        }
                        UpdateReply::Checked(Ok(None)) => {
                            self.update_status = UpdateStatus::Current;
                        }
                        UpdateReply::Installed(Ok(())) => {
                            if automatic {
                                self.toast("Update installed. Restart RustCalc to use it.");
                            }
                            self.update_status = UpdateStatus::Installed;
                        }
                        // Automatic failures stay quiet; About shows the reason.
                        UpdateReply::Checked(Err(e)) | UpdateReply::Installed(Err(e)) => {
                            self.update_status = UpdateStatus::Failed(e);
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.update_worker = None;
                    self.update_automatic = false;
                    self.update_status =
                        UpdateStatus::Failed("The update worker stopped unexpectedly.".into());
                }
                Err(mpsc::TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(100));
                }
            }
        }
    }
    /// Start a due automatic check. Only idle states qualify, so a manual check,
    /// a pending install or a staged update is never interrupted.
    fn auto_update(&mut self, ctx: &egui::Context) {
        if self.screenshot.is_some() || self.notebook.auto_update == AutoUpdate::Off {
            return;
        }
        // Long-running sessions re-evaluate the daily schedule.
        ctx.request_repaint_after(Duration::from_secs(3600));
        let idle = matches!(
            self.update_status,
            UpdateStatus::Idle | UpdateStatus::Current | UpdateStatus::Failed(_)
        );
        let now = unix_now();
        if self.update_worker.is_none()
            && idle
            && updater::auto_check_due(
                self.notebook.auto_update,
                self.notebook.last_update_check,
                now,
            )
        {
            // Record the attempt first so failures retry tomorrow, not every frame.
            self.notebook.last_update_check = now;
            self.update_automatic = true;
            self.start_update_check(ctx);
        }
    }
    fn set_auto_update(&mut self, mode: AutoUpdate) {
        self.notebook.auto_update = mode;
        self.notebook.update_prompted = true;
    }
    fn restart(ctx: &egui::Context) {
        crate::RESTART.store(true, std::sync::atomic::Ordering::SeqCst);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
    fn about_dialog(&mut self, ctx: &egui::Context, p: Palette) {
        let mut open = self.about;
        egui::Window::new(p.tr("About & updates")).open(&mut open).collapsible(false).resizable(false).default_width(440.0).show(ctx, |ui| {
            ui.label(label(format!("RustCalc {}", env!("CARGO_PKG_VERSION")), 27.0, p.text, p.language));
            ui.label(p.tr("A portable, open-source calculator."));
            ui.hyperlink_to("ersinkoc/rustcalc", "https://github.com/ersinkoc/rustcalc");
            ui.label("MIT License · © 2026 Ersin Koç");
            ui.separator();
            ui.label(p.tr("Connects to GitHub only for update checks you request or enable. Your calculations are never sent."));
            ui.add_space(10.0);
            ui.label(label("AUTOMATIC UPDATES", 10.0, p.muted, p.language).strong());
            ui.horizontal_wrapped(|ui| {
                for (mode, title) in [(AutoUpdate::Off, "Off"), (AutoUpdate::Notify, "Notify me"), (AutoUpdate::Install, "Install automatically")] {
                    let button = ui.selectable_label(self.notebook.auto_update == mode, p.tr(title));
                    #[cfg(test)]
                    self.targets.insert(format!("auto_{title}"), button.rect);
                    if button.clicked() { self.set_auto_update(mode); }
                }
            });
            ui.label(label("Checks at most once a day while RustCalc is open. Updates are verified before installation and take effect after a restart.", 12.0, p.muted, p.language));
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
                    if ui.button(p.tr("Restart")).clicked() { Self::restart(ctx); }
                }
            }
        });
        self.about = open;
    }
}

impl RustCalc {
    fn render(&mut self, ctx: &egui::Context) {
        self.auto_update(ctx);
        self.poll_update(ctx);
        self.poll_history_file();
        self.poll_formula_file();
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
        if let Some(theme) = ctx.system_theme() {
            self.system_light = theme == egui::Theme::Light;
        }
        // RustCalc owns the zoom so it is saved; egui's own shortcuts would
        // change it behind the notebook's back.
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
        self.shortcuts(ctx);
        let zoom = self.notebook.zoom();
        if (ctx.zoom_factor() - zoom).abs() > 0.001 {
            ctx.set_zoom_factor(zoom);
        }
        let p = self.palette();
        let mut visuals = if self.light() {
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
        // Help and About are side windows: the calculator keeps working while
        // they are open, so their examples can be tried right away.
        if self.page == Page::Calculator
            && !self.confirm_clear
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
                // `=` calculates, whether typed alone, typed in the same frame as
                // other keys or at the end of pasted text. After <, > or ! it
                // belongs to a comparison (<=, >=, !=) and is kept as text.
                let comparison = self.expression.ends_with(['<', '>', '!', '='])
                    || rustcalc::model::check_variable_name(self.expression.trim()).is_ok();
                let mut equals = false;
                ctx.input_mut(|i| {
                    i.events.retain_mut(|e| match e {
                        egui::Event::Paste(t) | egui::Event::Text(t) => {
                            if let Some(rest) = t.trim_end().strip_suffix('=') {
                                let in_comparison = if rest.is_empty() {
                                    comparison
                                } else {
                                    rest.ends_with(['<', '>', '!', '='])
                                };
                                if !in_comparison {
                                    equals = true;
                                    *t = rest.to_owned();
                                }
                            }
                            !t.is_empty()
                        }
                        _ => true,
                    })
                });
                if equals {
                    calculate_requested = true;
                }
                let typing = ctx.input(|i| {
                    i.events.iter().find_map(|e| match e {
                        egui::Event::Text(text) | egui::Event::Paste(text) => Some(text.clone()),
                        _ => None,
                    })
                });
                let id = egui::Id::new("expression");
                // A new calculation starts only when the cursor sits at the end;
                // a click inside the previous expression edits it instead.
                let at_end = egui::TextEdit::load_state(ctx, id)
                    .and_then(|s| s.cursor.char_range())
                    .is_none_or(|r| {
                        r.primary == r.secondary
                            && r.primary.index == self.expression.chars().count()
                    });
                if let Some(text) = typing
                    && self.committed
                {
                    if at_end {
                        let continuation = text
                            .trim_start()
                            .starts_with(['+', '-', '*', '/', '^', '%', '!']);
                        self.expression = if continuation {
                            "ans".into()
                        } else {
                            String::new()
                        };
                        let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::one(
                                egui::text::CCursor::new(self.expression.chars().count()),
                            )));
                        state.store(ctx, id);
                    }
                    self.committed = false;
                }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                // Esc first closes what is in front: a menu, or the search text.
                // egui drops focus on Esc before this runs, so the search
                // field's focus is taken from the previous frame.
                if searching || self.search_had_focus {
                    self.search.clear();
                } else if !egui::Popup::is_any_open(ctx) {
                    self.action("clear");
                }
            }
            self.search_had_focus = false;
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
                ctx.copy_text(self.notebook.copy_text(&v.text()));
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
                        .map(|(s, _)| s.clone());
                    if notice.is_none()
                        && !self.notebook.update_prompted
                        && self.screenshot.is_none()
                    {
                        // A one-time, non-blocking question; nothing connects until answered.
                        ui.label(label(
                            "Keep RustCalc up to date automatically?",
                            11.0,
                            p.text,
                            p.language,
                        ));
                        for (mode, title) in [
                            (AutoUpdate::Notify, "Notify me"),
                            (AutoUpdate::Install, "Install automatically"),
                            (AutoUpdate::Off, "Not now"),
                        ] {
                            let button = ui.small_button(p.tr(title));
                            #[cfg(test)]
                            self.targets.insert(format!("prompt_{title}"), button.rect);
                            if button.clicked() {
                                self.set_auto_update(mode);
                            }
                        }
                    } else {
                        ui.label(label(
                            notice
                                .as_deref()
                                .unwrap_or("Runs locally. Your data stays yours."),
                            11.0,
                            if notice.is_some() { p.accent } else { p.muted },
                            p.language,
                        ));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(label("F1  HELP", 10.0, p.muted, p.language));
                        match &self.update_status {
                            UpdateStatus::Available(plan) => {
                                let text = p.tr(format!("Update to {}", plan.version));
                                let button = ui.small_button(RichText::new(text).color(p.accent));
                                #[cfg(test)]
                                self.targets.insert("status_update".into(), button.rect);
                                if button.clicked() {
                                    self.about = true;
                                }
                            }
                            UpdateStatus::Installing => {
                                ui.label(label(
                                    "Downloading and verifying…",
                                    10.0,
                                    p.muted,
                                    p.language,
                                ));
                                ui.spinner();
                            }
                            UpdateStatus::Installed => {
                                let button = ui.small_button(
                                    RichText::new(p.tr("Restart to update")).color(p.accent),
                                );
                                if button.clicked() {
                                    Self::restart(ui.ctx());
                                }
                            }
                            _ => {}
                        }
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
                            Page::Formulas => self.formulas(ui, p),
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
            self.notebook.plots = Some(self.graphs.clone());
            self.notebook.programmer_bits = Some(self.word.bits);
            self.notebook.programmer_signed = self.word.signed;
            self.notebook.workspace = self.page.name().into();
            self.notebook.scientific = self.scientific;
            self.notebook.converter = Some([self.category, self.from, self.to]);
            self.notebook.formula_selected = Some(self.formula_selected.clone());
            eframe::set_value(storage, "notebook-v1", &self.notebook);
            if let Some(raw) = &self.unreadable_state {
                storage.set_string("notebook-v1-unreadable", raw.clone());
            }
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
    fn pasting_and_equals_after_a_result_behave_like_typing() {
        let typed = |app: &mut RustCalc, ctx: &egui::Context, events: Vec<egui::Event>| {
            frame(app, ctx, events, 1180.0);
        };
        let mut app = RustCalc::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        // "2", "+", "3" and "=" arriving in one frame still calculate.
        typed(
            &mut app,
            &ctx,
            ["2", "+", "3", "="]
                .map(|t| egui::Event::Text(t.into()))
                .to_vec(),
        );
        assert_eq!(app.notebook.answer, "5");
        // Pasting a number after a result starts a new calculation.
        typed(&mut app, &ctx, vec![egui::Event::Paste("7".into())]);
        assert_eq!(app.expression, "7");
        typed(
            &mut app,
            &ctx,
            vec![key(egui::Key::Enter, Default::default())],
        );
        assert_eq!(app.notebook.answer, "7");
        // Pasting an operator continues from the result; a trailing = calculates.
        typed(&mut app, &ctx, vec![egui::Event::Paste("*2=".into())]);
        assert_eq!(app.notebook.answer, "14");
        // = after < is part of a comparison, not a calculation.
        typed(&mut app, &ctx, vec![egui::Event::Text("3".into())]);
        for t in ["<", "=", "4"] {
            typed(&mut app, &ctx, vec![egui::Event::Text(t.into())]);
        }
        assert_eq!(app.expression, "3<=4");
    }
    #[test]
    fn escape_in_history_search_clears_the_search_not_the_expression() {
        let mut app = RustCalc {
            expression: "12*3".into(),
            search: "12".into(),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        ctx.memory_mut(|m| m.request_focus(egui::Id::new("history_search")));
        frame(&mut app, &ctx, vec![], 1180.0);
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::Escape, Default::default())],
            1180.0,
        );
        assert_eq!(app.expression, "12*3");
        assert!(app.search.is_empty());
    }
    #[test]
    fn the_calculator_keeps_working_while_help_is_open() {
        let mut app = RustCalc {
            expression: "12*3".into(),
            help: true,
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
        assert_eq!(app.notebook.answer, "36");
    }
    #[test]
    fn workspace_state_is_restored_and_validated() {
        let mut app = RustCalc::default();
        app.notebook.scientific = true;
        app.notebook.converter = Some([5, 1, 0]);
        app.notebook.formula_selected = Some("vat_add".into());
        app.restore_workspace();
        assert!(app.scientific);
        assert_eq!((app.category, app.from, app.to), (5, 1, 0));
        assert_eq!(app.formula_selected, "vat_add");
        // Stale or foreign values fall back to the defaults.
        let mut app = RustCalc::default();
        app.notebook.converter = Some([5, 99, 0]);
        app.notebook.formula_selected = Some("deleted-formula".into());
        app.restore_workspace();
        assert_eq!((app.category, app.from, app.to), (0, 0, 1));
        assert_eq!(app.formula_selected, "loan");
        for (page, name) in Page::ALL {
            assert_eq!(Page::from_name(name), Some(page));
            assert_eq!(page.name(), name);
        }
    }
    #[test]
    fn results_are_shown_in_the_chosen_number_style() {
        let mut app = RustCalc {
            expression: "1234567.5 * 2".into(),
            ..Default::default()
        };
        app.notebook.language = Language::Turkish;
        let ctx = egui::Context::default();
        let text = rendered_text(&frame(&mut app, &ctx, vec![], 1180.0));
        assert!(text.contains("2\u{A0}469\u{A0}135"), "{text}");
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::Enter, Default::default())],
            1180.0,
        );
        // History keeps the canonical result and shows it localized.
        assert_eq!(app.notebook.entries[0].result, "2469135");
        app.expression = "1/8".into();
        app.committed = false;
        let text = rendered_text(&frame(&mut app, &ctx, vec![], 1180.0));
        assert!(text.contains("0,125"), "{text}");
        assert_eq!(app.notebook.copy_text("2469135.25"), "2469135,25");
        app.notebook.numbers.decimals = Some(2);
        app.notebook.numbers.grouping = false;
        assert_eq!(app.notebook.show("2469135"), "2469135,00");
    }
    #[test]
    fn shortcuts_switch_workspaces_zoom_and_open_settings() {
        let mut app = RustCalc::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        let command = egui::Modifiers::COMMAND;
        frame(&mut app, &ctx, vec![key(egui::Key::Num2, command)], 1180.0);
        assert_eq!(app.page, Page::Converter);
        frame(&mut app, &ctx, vec![key(egui::Key::Num5, command)], 1180.0);
        assert_eq!(app.page, Page::Formulas);
        frame(&mut app, &ctx, vec![key(egui::Key::Plus, command)], 1180.0);
        assert_eq!(app.notebook.zoom(), 1.1);
        frame(&mut app, &ctx, vec![], 1180.0);
        assert!((ctx.zoom_factor() - 1.1).abs() < 0.001);
        frame(&mut app, &ctx, vec![key(egui::Key::Minus, command)], 1180.0);
        frame(&mut app, &ctx, vec![key(egui::Key::Minus, command)], 1180.0);
        assert_eq!(app.notebook.zoom(), 0.9);
        frame(&mut app, &ctx, vec![key(egui::Key::Num0, command)], 1180.0);
        assert_eq!(app.notebook.zoom(), 1.0);
        frame(&mut app, &ctx, vec![key(egui::Key::Comma, command)], 1180.0);
        assert!(app.settings);
    }
    #[test]
    fn settings_change_theme_and_digit_grouping() {
        let mut app = RustCalc {
            settings: true,
            ..Default::default()
        };
        let ctx = egui::Context::default();
        // A window's first frame only measures it; buttons settle after that.
        frame(&mut app, &ctx, vec![], 1180.0);
        frame(&mut app, &ctx, vec![], 1180.0);
        click(&mut app, &ctx, "theme_Light");
        assert!(app.notebook.light && !app.notebook.follow_system_theme);
        click(&mut app, &ctx, "theme_Follow system");
        assert!(app.notebook.follow_system_theme);
        assert!(app.notebook.numbers.grouping);
        click(&mut app, &ctx, "grouping");
        assert!(!app.notebook.numbers.grouping);
        // An older save without a zoom value opens at 100%.
        assert_eq!(Notebook::default().zoom(), 1.0);
    }
    #[test]
    fn typing_an_assignment_creates_a_variable_chip() {
        let mut app = RustCalc::default();
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        // "=" after a bare name starts the assignment instead of calculating.
        for t in ["k", "d", "v", "=", "2", "0"] {
            frame(&mut app, &ctx, vec![egui::Event::Text(t.into())], 1180.0);
        }
        assert_eq!(app.expression, "kdv=20");
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::Enter, Default::default())],
            1180.0,
        );
        assert_eq!(app.notebook.variables["kdv"].value, "20");
        frame(&mut app, &ctx, vec![], 1180.0);
        click(&mut app, &ctx, "variable_kdv");
        frame(&mut app, &ctx, vec![], 1180.0);
        assert_eq!(app.expression, "kdv");
        for t in ["*", "3", "="] {
            frame(&mut app, &ctx, vec![egui::Event::Text(t.into())], 1180.0);
        }
        assert_eq!(app.notebook.answer, "60");
        click(&mut app, &ctx, "delete_variable_kdv");
        assert!(app.notebook.variables.is_empty());
    }
    #[test]
    fn turkish_case_rules_apply_to_headings_and_search() {
        assert_eq!(Language::Turkish.upper("Finans"), "FİNANS");
        assert_eq!(Language::Turkish.upper("Tarihler"), "TARİHLER");
        assert_eq!(Language::Turkish.upper("ısı"), "ISI");
        assert_eq!(Language::English.upper("Finance"), "FINANCE");
        let fold = rustcalc::i18n::fold;
        assert!(fold("İndirim").contains(&fold("indirim")));
        assert!(fold("İş günü ekle").contains(&fold("iş")));
        assert!(fold("Isı").contains(&fold("isi")));
    }
    #[test]
    fn enter_does_not_calculate_behind_dialogs_or_other_workspaces() {
        for (page, help, about, confirm_clear) in [
            (Page::Converter, false, false, false),
            (Page::Programmer, false, false, false),
            (Page::Graph, false, false, false),
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
                        Page::Formulas,
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

    fn curve_paths(output: &egui::FullOutput, color: Color32) -> usize {
        fn count(shape: &egui::Shape, color: Color32) -> usize {
            match shape {
                egui::Shape::Path(path) => {
                    usize::from(path.stroke.color == egui::epaint::ColorMode::Solid(color))
                }
                egui::Shape::Vec(shapes) => shapes.iter().map(|s| count(s, color)).sum(),
                _ => 0,
            }
        }
        output.shapes.iter().map(|c| count(&c.shape, color)).sum()
    }
    fn graph_app() -> (RustCalc, egui::Context) {
        let mut app = RustCalc {
            page: Page::Graph,
            ..Default::default()
        };
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        frame(&mut app, &ctx, vec![], 1180.0);
        (app, ctx)
    }
    #[test]
    fn plot_buttons_zoom_around_the_center_and_reset() {
        let (mut app, ctx) = graph_app();
        app.view.pan(0.25, 0.0);
        click(&mut app, &ctx, "plot_zoom_in");
        assert_eq!(app.view.span, 5.0);
        assert_eq!(app.view.center_x, 5.0);
        click(&mut app, &ctx, "plot_reset");
        assert_eq!(app.view, Viewport::default());
    }
    #[test]
    fn wheel_zooms_at_the_pointer_and_drag_pans_the_plot() {
        let (mut app, ctx) = graph_app();
        let rect = app.targets["plot"];
        let anchor = rect.center() + egui::vec2(rect.width() / 4.0, 0.0);
        let world_x = |app: &RustCalc| {
            app.view.center_x
                + (anchor.x - rect.center().x) as f64 / (rect.width() / 2.0) as f64 * app.view.span
        };
        let before = world_x(&app);
        frame(
            &mut app,
            &ctx,
            vec![
                egui::Event::PointerMoved(anchor),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, 5.0),
                    modifiers: Default::default(),
                },
            ],
            1180.0,
        );
        assert!(app.view.span < 10.0, "{:?}", app.view);
        assert!((world_x(&app) - before).abs() < 1e-9, "anchor moved");

        app.view = Viewport::default();
        let start = rect.center();
        let end = start + egui::vec2(rect.width() / 4.0, -rect.height() / 4.0);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(start), button(start, true)],
            1180.0,
        );
        for step in 1..=4 {
            let pos = start + (end - start) * (step as f32 / 4.0);
            frame(&mut app, &ctx, vec![egui::Event::PointerMoved(pos)], 1180.0);
        }
        frame(&mut app, &ctx, vec![button(end, false)], 1180.0);
        // Dragging right and up moves the content, so the view centre moves left and down.
        assert!((app.view.center_x + 5.0).abs() < 0.01, "{:?}", app.view);
        assert!((app.view.center_y + 5.0).abs() < 0.01, "{:?}", app.view);
        assert_eq!(app.view.span, 10.0);
    }
    #[test]
    fn arrow_keys_pan_only_while_no_text_field_has_focus() {
        let (mut app, ctx) = graph_app();
        let center = app.targets["plot"].center();
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(center)],
            1180.0,
        );
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::ArrowRight, Default::default())],
            1180.0,
        );
        assert_eq!(app.view.center_x, 2.0);
        ctx.memory_mut(|m| m.request_focus(egui::Id::new(("plot_expression", 0))));
        frame(&mut app, &ctx, vec![], 1180.0);
        frame(
            &mut app,
            &ctx,
            vec![key(egui::Key::ArrowRight, Default::default())],
            1180.0,
        );
        assert_eq!(app.view.center_x, 2.0);
    }
    #[test]
    fn several_curves_render_in_their_own_colors_and_errors_are_localized() {
        let (mut app, ctx) = graph_app();
        app.graphs = ["sin(x)".into(), "1/x".into(), "x^2".into()];
        let out = frame(&mut app, &ctx, vec![], 1180.0);
        let p = app.palette();
        assert!(curve_paths(&out, p.curves[0]) >= 1);
        // 1/x is split at its pole rather than joined by a vertical line.
        assert_eq!(curve_paths(&out, p.curves[1]), 2);
        assert!(curve_paths(&out, p.curves[2]) >= 1);

        app.graphs = ["sin(x".into(), String::new(), "sqrt(0-1-x^2)".into()];
        app.notebook.language = Language::Turkish;
        let text = rendered_text(&frame(&mut app, &ctx, vec![], 1180.0));
        assert!(text.contains("f(x): Kapanış parantezi eksik"), "{text}");
        assert!(text.contains("h(x): "), "{text}");
        app.graphs = Default::default();
        let text = rendered_text(&frame(&mut app, &ctx, vec![], 1180.0));
        assert!(
            text.contains("Çizmek için x'e bağlı bir fonksiyon girin."),
            "{text}"
        );
    }

    fn fake_plan(version: &str) -> UpdatePlan {
        let asset = |name: &str| updater::Asset {
            name: name.into(),
            browser_download_url: format!(
                "https://github.com/{}/releases/download/v{version}/{name}",
                updater::REPOSITORY
            ),
            size: 10,
            digest: None,
        };
        updater::plan_release(
            updater::Release {
                tag_name: format!("v{version}"),
                draft: false,
                prerelease: false,
                assets: vec![asset(updater::ASSET), asset("SHA256SUMS")],
            },
            "0.0.1",
        )
        .unwrap()
        .unwrap()
    }
    type JobLog = std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>;
    /// A local update runner that records jobs instead of using the network.
    fn stub_runner(check: fn() -> Result<Option<UpdatePlan>, String>) -> (UpdateRunner, JobLog) {
        let jobs = JobLog::default();
        let log = jobs.clone();
        let runner: UpdateRunner = std::sync::Arc::new(move |job| match job {
            UpdateJob::Check => {
                log.lock().unwrap().push("check");
                UpdateReply::Checked(check())
            }
            UpdateJob::Install(_) => {
                log.lock().unwrap().push("install");
                UpdateReply::Installed(Ok(()))
            }
        });
        (runner, jobs)
    }
    fn settle(app: &mut RustCalc, ctx: &egui::Context) -> egui::FullOutput {
        let mut out = frame(app, ctx, vec![], 1180.0);
        for _ in 0..400 {
            if app.update_worker.is_none() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
            out = frame(app, ctx, vec![], 1180.0);
        }
        assert!(app.update_worker.is_none(), "update worker did not finish");
        out
    }
    #[test]
    fn automatic_updates_stay_off_until_the_prompt_is_answered() {
        let (runner, jobs) = stub_runner(|| Ok(Some(fake_plan("9.0.0"))));
        let mut app = RustCalc {
            update_runner: runner,
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let out = settle(&mut app, &ctx);
        assert!(rendered_text(&out).contains("Keep RustCalc up to date automatically?"));
        assert!(jobs.lock().unwrap().is_empty());
        click(&mut app, &ctx, "prompt_Not now");
        assert!(app.notebook.update_prompted);
        assert_eq!(app.notebook.auto_update, AutoUpdate::Off);
        let out = settle(&mut app, &ctx);
        assert!(!rendered_text(&out).contains("Keep RustCalc up to date automatically?"));
        assert!(jobs.lock().unwrap().is_empty());
        assert!(matches!(app.update_status, UpdateStatus::Idle));
    }
    #[test]
    fn notify_mode_checks_once_a_day_and_announces_a_release() {
        let (runner, jobs) = stub_runner(|| Ok(Some(fake_plan("9.0.0"))));
        let mut app = RustCalc {
            update_runner: runner,
            ..Default::default()
        };
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        click(&mut app, &ctx, "prompt_Notify me");
        let out = settle(&mut app, &ctx);
        assert_eq!(*jobs.lock().unwrap(), ["check"]);
        assert!(matches!(app.update_status, UpdateStatus::Available(_)));
        let text = rendered_text(&out);
        assert!(text.contains("RustCalc update available: 9.0.0"), "{text}");
        assert!(text.contains("Update to 9.0.0"), "{text}");
        assert!(app.notebook.last_update_check > 0);
        // Further frames neither re-check nor install.
        for _ in 0..3 {
            settle(&mut app, &ctx);
        }
        assert_eq!(*jobs.lock().unwrap(), ["check"]);
        click(&mut app, &ctx, "status_update");
        assert!(app.about);
    }
    #[test]
    fn install_mode_uses_the_verified_installer_and_waits_for_restart() {
        let (runner, jobs) = stub_runner(|| Ok(Some(fake_plan("9.0.0"))));
        let mut app = RustCalc {
            update_runner: runner,
            ..Default::default()
        };
        app.notebook.language = Language::Turkish;
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        click(&mut app, &ctx, "prompt_Install automatically");
        let mut out = settle(&mut app, &ctx);
        for _ in 0..100 {
            if matches!(app.update_status, UpdateStatus::Installed) {
                break;
            }
            out = settle(&mut app, &ctx);
        }
        assert_eq!(*jobs.lock().unwrap(), ["check", "install"]);
        assert!(matches!(app.update_status, UpdateStatus::Installed));
        let text = rendered_text(&out);
        assert!(text.contains("Güncellemek için yeniden başlat"), "{text}");
        assert!(!crate::RESTART.load(std::sync::atomic::Ordering::SeqCst));
    }
    #[test]
    fn automatic_failures_are_quiet_and_current_versions_change_nothing() {
        let (runner, jobs) = stub_runner(|| Err("Update request failed: offline".into()));
        let mut app = RustCalc {
            update_runner: runner,
            ..Default::default()
        };
        app.notebook.update_prompted = true;
        app.notebook.auto_update = AutoUpdate::Install;
        let ctx = egui::Context::default();
        let out = settle(&mut app, &ctx);
        assert_eq!(*jobs.lock().unwrap(), ["check"]);
        assert!(matches!(app.update_status, UpdateStatus::Failed(_)));
        assert!(app.notice.is_none());
        assert!(!rendered_text(&out).contains("offline"));

        let (runner, jobs) = stub_runner(|| Ok(None));
        app.update_runner = runner;
        app.notebook.last_update_check = 0;
        settle(&mut app, &ctx);
        assert_eq!(*jobs.lock().unwrap(), ["check"]);
        assert!(matches!(app.update_status, UpdateStatus::Current));
        assert!(app.notice.is_none());
    }
    #[test]
    fn update_prompt_fits_compact_windows_in_both_languages() {
        for language in [Language::English, Language::Turkish] {
            let mut app = RustCalc::default();
            app.notebook.language = language;
            let ctx = egui::Context::default();
            frame(&mut app, &ctx, vec![], 720.0);
            frame(&mut app, &ctx, vec![], 720.0);
            for title in ["Notify me", "Install automatically", "Not now"] {
                let rect = app.targets[&format!("prompt_{title}")];
                assert!(
                    rect.right() <= 720.0 && rect.left() >= 0.0,
                    "{language:?} {title} {rect:?}"
                );
            }
        }
    }
    #[test]
    fn programmer_word_size_and_signedness_change_the_meaning_not_the_bits() {
        let mut app = RustCalc {
            page: Page::Programmer,
            integer: "255".into(),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        frame(&mut app, &ctx, vec![], 1180.0);
        assert!(app.targets.contains_key("bit_63"));
        click(&mut app, &ctx, "word_8");
        assert_eq!(app.word.bits, 8);
        app.targets.retain(|k, _| !k.starts_with("bit_"));
        frame(&mut app, &ctx, vec![], 1180.0);
        assert!(app.targets.contains_key("bit_7") && !app.targets.contains_key("bit_8"));
        click(&mut app, &ctx, "signed");
        assert!(app.word.signed);
        assert_eq!(app.integer, "-1");
        app.operand = "1".into();
        click(&mut app, &ctx, "op_+");
        assert_eq!(app.integer, "0");
        app.integer = "127".into();
        click(&mut app, &ctx, "op_+");
        assert_eq!(app.integer, "-128");
        click(&mut app, &ctx, "op_>> 1");
        assert_eq!(app.integer, "-64");
        // Widening keeps the bit pattern (0xC0), so the value becomes positive.
        click(&mut app, &ctx, "word_16");
        assert_eq!(app.integer, "192");
        app.operand = "0".into();
        click(&mut app, &ctx, "op_÷");
        assert_eq!(app.integer, "192");
        let text = rendered_text(&frame(&mut app, &ctx, vec![], 1180.0));
        assert!(text.contains("Division by zero is undefined"), "{text}");
        app.integer = "40000".into();
        let text = rendered_text(&frame(&mut app, &ctx, vec![], 1180.0));
        assert!(text.contains("Allowed range: -32768 … 32767"), "{text}");
    }
    #[test]
    fn plot_lists_roots_and_intersections_and_can_hide_them() {
        let (mut app, ctx) = graph_app();
        app.graphs = ["sin(x)".into(), "0.5".into(), "1/x".into()];
        // A tall window keeps every result line below the plot on screen.
        let tall = |app: &mut RustCalc| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1180.0, 1400.0),
                    )),
                    ..Default::default()
                },
                |ctx| app.render(ctx),
            )
        };
        let text = rendered_text(&tall(&mut app));
        assert!(
            text.contains(
                "f(x) = 0:  x ≈ -9.424778, -6.283185, -3.141593, 0, 3.141593, 6.283185, 9.424778"
            ),
            "{text}"
        );
        assert!(text.contains("f(x) = g(x):  x ≈ "), "{text}");
        assert!(text.contains("g(x) = h(x):  x ≈ 2"), "{text}");
        assert!(
            !text.contains("h(x) = 0"),
            "1/x has a pole, not a root: {text}"
        );
        app.show_points = false;
        let text = rendered_text(&tall(&mut app));
        assert!(!text.contains("x ≈"), "{text}");
    }

    fn formulas_app(language: Language) -> (RustCalc, egui::Context) {
        let mut app = RustCalc {
            page: Page::Formulas,
            ..Default::default()
        };
        app.notebook.language = language;
        app.notebook.update_prompted = true;
        let ctx = egui::Context::default();
        frame(&mut app, &ctx, vec![], 1180.0);
        frame(&mut app, &ctx, vec![], 1180.0);
        (app, ctx)
    }
    fn tall_text(app: &mut RustCalc, ctx: &egui::Context, width: f32) -> String {
        rendered_text(&ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 1600.0),
                )),
                ..Default::default()
            },
            |ctx| app.render(ctx),
        ))
    }
    #[test]
    fn templates_show_live_results_for_entered_values() {
        let (mut app, ctx) = formulas_app(Language::English);
        click(&mut app, &ctx, "formula_vat_add");
        assert_eq!(app.formula_selected, "vat_add");
        let text = tall_text(&mut app, &ctx, 1180.0);
        assert!(
            text.contains("200.00") && text.contains("1,200.00"),
            "{text}"
        );
        app.formula_values
            .insert("vat_add".into(), vec!["2500 * 2".into(), "8".into()]);
        let text = tall_text(&mut app, &ctx, 1180.0);
        assert!(
            text.contains("400.00") && text.contains("5,400.00"),
            "{text}"
        );
        // An invalid input is reported beside the field and blocks dependent results.
        app.formula_values
            .insert("vat_add".into(), vec!["abc".into(), "8".into()]);
        let text = tall_text(&mut app, &ctx, 1180.0);
        assert!(text.contains("Unknown name: abc"), "{text}");
        assert!(
            text.contains("Depends on a value that could not be calculated"),
            "{text}"
        );
    }
    #[test]
    fn date_templates_render_localized_dates() {
        let (mut app, ctx) = formulas_app(Language::Turkish);
        click(&mut app, &ctx, "formula_add_days");
        app.formula_values
            .insert("add_days".into(), vec!["2026-10-21".into(), "10".into()]);
        let text = tall_text(&mut app, &ctx, 1180.0);
        assert!(text.contains("31.10.2026  ·  Cumartesi"), "{text}");
        assert!(
            text.contains("21.10.2026  ·  Çarşamba"),
            "input preview: {text}"
        );
        assert!(text.contains("İki tarih arası gün"), "{text}");
    }
    #[test]
    fn custom_formulas_can_be_created_edited_used_and_deleted() {
        let (mut app, ctx) = formulas_app(Language::English);
        click(&mut app, &ctx, "formula_new");
        let draft = app.formula_draft.as_mut().expect("editor opened");
        assert!(draft.replaces.is_none());
        draft.formula.name = "Room area".into();
        draft.formula.inputs[0] = rustcalc::formulas::Field {
            name: "w".into(),
            label: "Width".into(),
            kind: rustcalc::formulas::Kind::Number,
            default: "3".into(),
        };
        draft.formula.inputs.push(rustcalc::formulas::Field {
            name: "l".into(),
            label: "Length".into(),
            kind: rustcalc::formulas::Kind::Number,
            default: "4".into(),
        });
        draft.formula.outputs[0].expression = "w *".into();
        let text = tall_text(&mut app, &ctx, 1180.0);
        assert!(
            text.contains("Expected a number, constant or function"),
            "{text}"
        );
        click(&mut app, &ctx, "formula_save");
        assert!(
            app.formula_draft.is_some(),
            "an invalid formula cannot be saved"
        );
        assert!(app.notebook.formulas.is_empty());

        app.formula_draft.as_mut().unwrap().formula.outputs[0].expression = "w * l".into();
        let text = tall_text(&mut app, &ctx, 1180.0);
        assert!(text.contains("= 12"), "live preview: {text}");
        click(&mut app, &ctx, "formula_save");
        assert!(app.formula_draft.is_none());
        assert_eq!(app.notebook.formulas.len(), 1);
        let id = app.notebook.formulas[0].id.clone();
        assert!(id.starts_with("custom-"));
        assert_eq!(app.formula_selected, id);
        let text = tall_text(&mut app, &ctx, 1180.0);
        assert!(
            text.contains("Room area") && text.contains("CUSTOM"),
            "{text}"
        );

        click(&mut app, &ctx, "formula_edit");
        let draft = app.formula_draft.as_mut().unwrap();
        assert_eq!(draft.replaces.as_deref(), Some(id.as_str()));
        draft.formula.outputs[0].expression = "w * l * 2".into();
        click(&mut app, &ctx, "formula_save");
        assert_eq!(app.notebook.formulas.len(), 1);
        assert_eq!(app.notebook.formulas[0].id, id);
        assert!(tall_text(&mut app, &ctx, 1180.0).contains("24"));

        click(&mut app, &ctx, "formula_delete");
        assert!(app.formula_delete);
        click(&mut app, &ctx, "formula_delete_confirm");
        assert!(app.notebook.formulas.is_empty());
        assert_eq!(app.formula_selected, "loan");
    }
    #[test]
    fn customizing_a_template_starts_a_translated_copy() {
        let (mut app, ctx) = formulas_app(Language::Turkish);
        click(&mut app, &ctx, "formula_bmi");
        click(&mut app, &ctx, "formula_duplicate");
        let draft = app.formula_draft.as_ref().expect("editor opened");
        assert!(draft.replaces.is_none());
        assert_eq!(draft.formula.name, "Vücut kitle indeksi");
        assert_eq!(draft.formula.inputs[0].label, "Kilo (kg)");
        // The long editor scrolls; save directly rather than clicking below the fold.
        let draft = app.formula_draft.take().unwrap();
        app.save_formula(draft);
        assert_eq!(app.notebook.formulas.len(), 1);
        assert_eq!(app.notebook.formulas[0].category, "", "lists as Custom");
        // The editor also fits a compact window.
        click(&mut app, &ctx, "formula_new");
        frame(&mut app, &ctx, vec![], 720.0);
        frame(&mut app, &ctx, vec![], 720.0);
        let save = app.targets["formula_save"];
        assert!(save.right() <= 720.0, "{save:?}");
        click(&mut app, &ctx, "formula_cancel");
        assert!(app.formula_draft.is_none());
    }
    #[test]
    fn formula_files_merge_on_import_and_errors_are_visible() {
        let (mut app, ctx) = formulas_app(Language::Turkish);
        let mut shared = rustcalc::formulas::Formula::blank(Language::English);
        shared.name = "Shared".into();
        let (send, receive) = mpsc::channel();
        app.formula_file_worker = Some(receive);
        send.send(FormulaFileReply::Imported(Ok(Some(vec![
            shared.clone(),
            shared,
        ]))))
        .unwrap();
        frame(&mut app, &ctx, vec![], 1180.0);
        assert!(app.formula_file_worker.is_none());
        assert_eq!(app.notebook.formulas.len(), 1);
        assert_eq!(app.notice.as_ref().unwrap().0, "Formulas imported: 1");
        let (send, receive) = mpsc::channel();
        app.formula_file_worker = Some(receive);
        send.send(FormulaFileReply::Imported(Err(
            "This is not a RustCalc formula file".into(),
        )))
        .unwrap();
        let text = tall_text(&mut app, &ctx, 1180.0);
        assert!(
            text.contains("Bu bir RustCalc formül dosyası değil"),
            "{text}"
        );
        assert_eq!(app.notebook.formulas.len(), 1);
        // Cancelling a dialog changes nothing.
        let (send, receive) = mpsc::channel();
        app.formula_file_worker = Some(receive);
        send.send(FormulaFileReply::Imported(Ok(None))).unwrap();
        frame(&mut app, &ctx, vec![], 1180.0);
        assert_eq!(app.notebook.formulas.len(), 1);
    }
    #[test]
    fn formulas_plots_and_programmer_word_survive_a_restart() {
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
        let mut app = RustCalc::default();
        let mut custom = rustcalc::formulas::Formula::blank(Language::English);
        custom.id = "custom-1".into();
        custom.name = "Mine".into();
        app.notebook.formulas.push(custom.clone());
        app.graphs = ["cos(x)".into(), "x/2".into(), String::new()];
        app.word = Word {
            bits: 16,
            signed: true,
        };
        let mut storage = Storage::default();
        eframe::App::save(&mut app, &mut storage);
        let notebook: Notebook = eframe::get_value(&storage, "notebook-v1").unwrap();
        let mut restored = RustCalc {
            notebook,
            ..Default::default()
        };
        restored.restore_workspace();
        assert_eq!(restored.notebook.formulas, vec![custom]);
        assert_eq!(restored.graphs, app.graphs);
        assert_eq!(restored.word, app.word);
        // Older notebooks without these fields keep the defaults.
        let mut fresh = RustCalc::default();
        fresh.restore_workspace();
        assert_eq!(fresh.graphs[0], "sin(x)");
        assert_eq!(fresh.word, Word::default());
    }
    #[test]
    fn turkish_screens_contain_no_untranslated_catalogue_text() {
        let en: std::collections::BTreeMap<String, String> =
            serde_json::from_str(include_str!("../locales/en.json")).unwrap();
        let translated: std::collections::BTreeSet<&str> = en
            .keys()
            .filter(|k| Language::Turkish.text(k.as_str()) != **k)
            .map(String::as_str)
            .collect();
        type Setup = Box<dyn Fn(&mut RustCalc)>;
        let screens: Vec<(&str, Setup)> = vec![
            (
                "calculator",
                Box::new(|app| app.expression = "100 + 10%".into()),
            ),
            ("converter", Box::new(|app| app.page = Page::Converter)),
            ("programmer", Box::new(|app| app.page = Page::Programmer)),
            ("graph", Box::new(|app| app.page = Page::Graph)),
            ("formulas", Box::new(|app| app.page = Page::Formulas)),
            (
                "formula editor",
                Box::new(|app| {
                    app.page = Page::Formulas;
                    app.formula_draft = Some(Draft {
                        replaces: None,
                        formula: rustcalc::formulas::Formula::blank(Language::Turkish),
                    });
                }),
            ),
            (
                "date formula",
                Box::new(|app| {
                    app.page = Page::Formulas;
                    app.formula_selected = "age".into();
                }),
            ),
            ("help", Box::new(|app| app.help = true)),
            ("about", Box::new(|app| app.about = true)),
            ("settings", Box::new(|app| app.settings = true)),
            (
                "update prompt",
                Box::new(|app| app.notebook.update_prompted = false),
            ),
        ];
        for (name, setup) in screens {
            let mut app = RustCalc::default();
            app.notebook.language = Language::Turkish;
            app.notebook.update_prompted = true;
            setup(&mut app);
            let ctx = egui::Context::default();
            // Open every help section so its text is rendered too.
            ctx.data_mut(|_| {});
            let mut text = String::new();
            for _ in 0..3 {
                text = rendered_text(&ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1180.0, 2400.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| app.render(ctx),
                ));
            }
            let leaks: Vec<&str> = text
                .lines()
                .map(str::trim)
                .filter(|line| translated.contains(line))
                .collect();
            assert!(leaks.is_empty(), "{name}: untranslated text {leaks:?}");
        }
    }
}
