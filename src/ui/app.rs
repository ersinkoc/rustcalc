use super::{storage::Storage, theme::*};
use gpui::{prelude::*, *};
use gpui_component::{
    Root, Theme,
    input::{Input, InputEvent, InputState},
};
use rustcalc::{formulas, i18n::Language, model::Notebook, plot, programmer, units};
use std::{path::PathBuf, time::Duration};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Page {
    Calculator,
    Converter,
    Programmer,
    Graph,
    Formulas,
}

impl Page {
    pub const ALL: [Self; 5] = [
        Self::Calculator,
        Self::Converter,
        Self::Programmer,
        Self::Graph,
        Self::Formulas,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Calculator => "calculator",
            Self::Converter => "converter",
            Self::Programmer => "programmer",
            Self::Graph => "graph",
            Self::Formulas => "formulas",
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Calculator => "Calculate",
            Self::Converter => "Convert units",
            Self::Programmer => "Programmer",
            Self::Graph => "Plot a graph",
            Self::Formulas => "Formulas",
        }
    }
    fn icon(self) -> &'static str {
        match self {
            Self::Calculator => "calculator",
            Self::Converter => "convert",
            Self::Programmer => "binary",
            Self::Graph => "graph",
            Self::Formulas => "formulas",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Panel {
    Workspace,
    History,
    Settings,
    Help,
    Updates,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Field {
    Expression,
    Search,
    Convert,
    Integer,
    Operand,
    Plot(usize),
    FormulaSearch,
    Formula(usize),
    Draft,
}

pub(super) struct Calc {
    pub notebook: Notebook,
    storage: Option<Storage>,
    pub page: Page,
    pub panel: Panel,
    pub palette: Palette,
    pub focus: FocusHandle,
    pub expression: Entity<InputState>,
    pub search: Entity<InputState>,
    pub convert: Entity<InputState>,
    pub integer: Entity<InputState>,
    pub operand: Entity<InputState>,
    pub plots: [Entity<InputState>; 3],
    pub formula_search: Entity<InputState>,
    pub formula_inputs: Vec<Entity<InputState>>,
    pub formula_values: std::collections::BTreeMap<String, Vec<String>>,
    pub draft_form: Option<super::formulas::DraftForm>,
    pub formula_id: String,
    pub committed: Option<String>,
    pub notice: Option<String>,
    pub converter: [usize; 3],
    pub unit_menu: Option<usize>,
    pub word: programmer::Word,
    pub radix: u32,
    pub op: programmer::Op,
    pub view: plot::Viewport,
    pub graph_bounds: std::rc::Rc<std::cell::Cell<Option<Bounds<Pixels>>>>,
    pub graph_drag: Option<Point<Pixels>>,
    pub show_roots: bool,
    pub pending_history: Option<Vec<rustcalc::model::Entry>>,
    pub file_busy: bool,
    pub update: super::settings::Update,
    pub isolated: bool,
    pub shortcut_capture: Option<super::shortcuts::Command>,
    pub screenshot: Option<PathBuf>,
}

struct Assets;

fn minimum_window_size() -> Size<Pixels> {
    // Client area in logical pixels, with room for either keypad at normal text size.
    size(px(720.), px(720.))
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        Ok(super::theme::icon_bytes(path).map(std::borrow::Cow::Borrowed))
    }
    fn list(&self, _: &str) -> Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

pub(super) fn run(args: Vec<String>) -> Result<()> {
    let outcome = std::rc::Rc::new(std::cell::RefCell::new(Ok(())));
    let startup = outcome.clone();
    Application::new().with_assets(Assets).run(move |cx| {
        gpui_component::init(cx);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let compact = args.iter().any(|s| s == "--compact");
        let size = if compact {
            size(px(760.), px(760.))
        } else {
            size(px(1200.), px(810.))
        };
        let bounds = Bounds::centered(None, size, cx);
        let result = cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some("RustCalc".into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(minimum_window_size()),
                app_id: Some("rustcalc".into()),
                ..Default::default()
            },
            move |window, cx| {
                let view = cx.new(|cx| Calc::new(&args, window, cx));
                let weak_view = view.downgrade();
                window.on_window_should_close(cx, move |_, cx| {
                    let _ = weak_view.update(cx, |app, _| app.save());
                    true
                });
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match result {
            Ok(_) => cx.activate(true),
            Err(error) => {
                *startup.borrow_mut() = Err(error);
                cx.quit();
            }
        }
    });
    outcome.replace(Ok(()))
}

impl Calc {
    fn new(args: &[String], window: &mut Window, cx: &mut Context<Self>) -> Self {
        let has = |flag: &str| args.iter().any(|arg| arg == flag);
        let arg = |flag: &str| {
            args.windows(2)
                .find(|pair| pair[0] == flag)
                .map(|pair| &pair[1])
        };
        let screenshot = arg("--screenshot").map(PathBuf::from);
        let isolated = screenshot.is_some() || has("--demo");
        let (storage, mut notebook, notice) = if isolated {
            (None, Notebook::default(), None)
        } else {
            match Storage::open() {
                Ok((store, book)) => (Some(store), book, None),
                Err(error) => (
                    None,
                    Notebook::default(),
                    Some(format!(
                        "{} {error}",
                        "Saved data could not be read. The original app.ron was kept."
                    )),
                ),
            }
        };
        if let Some(code) = arg("--lang").and_then(|code| Language::parse(code).ok()) {
            notebook.language = code;
        }
        if has("--light") || has("--high-contrast") {
            notebook.follow_system_theme = false;
            notebook.light = has("--light");
            notebook.high_contrast = has("--high-contrast");
        }
        let mut committed = None;
        if has("--demo") {
            for expr in [
                "1280 * 0.18",
                "sqrt(144) + 8^2",
                "100 + 20%",
                "(2400 + 1800) * 1.2",
            ] {
                let _ = notebook.calculate(expr);
                committed = Some(expr.to_owned());
            }
            notebook.entries[0].pinned = true;
        }
        let page = Page::ALL
            .into_iter()
            .find(|p| has(&format!("--{}", p.key())))
            .or_else(|| {
                Page::ALL
                    .into_iter()
                    .find(|p| p.key() == notebook.workspace)
            })
            .unwrap_or(Page::Calculator);
        let converter = notebook
            .converter
            .filter(|[cat, from, to]| {
                units::CATEGORIES
                    .get(*cat)
                    .is_some_and(|c| *from < c.units.len() && *to < c.units.len())
            })
            .unwrap_or([0, 0, 1]);
        let word = programmer::Word {
            bits: notebook
                .programmer_bits
                .filter(|b| programmer::SIZES.contains(b))
                .unwrap_or(64),
            signed: notebook.programmer_signed,
        };
        let plot_values = notebook
            .plots
            .clone()
            .unwrap_or_else(|| ["sin(x)".into(), String::new(), String::new()]);
        let formula_id = notebook.formula_selected.clone().unwrap_or_default();
        let mut app = Self {
            expression: Self::input(
                Field::Expression,
                committed.as_deref().unwrap_or_default(),
                "1250 * 12",
                window,
                cx,
            ),
            search: Self::input(Field::Search, "", "Search history", window, cx),
            convert: Self::input(Field::Convert, "1", "Value", window, cx),
            integer: Self::input(
                Field::Integer,
                if has("--demo") { "255" } else { "0" },
                "Value",
                window,
                cx,
            ),
            operand: Self::input(Field::Operand, "1", "Value", window, cx),
            plots: std::array::from_fn(|i| {
                Self::input(
                    Field::Plot(i),
                    &plot_values[i],
                    ["sin(x)", "cos(x)", "x/2"][i],
                    window,
                    cx,
                )
            }),
            formula_search: Self::input(Field::FormulaSearch, "", "Search formulas", window, cx),
            focus: cx.focus_handle(),
            storage,
            notebook,
            page,
            panel: if has("--settings") {
                Panel::Settings
            } else {
                Panel::Workspace
            },
            palette: Palette::new(false, false),
            formula_inputs: Vec::new(),
            formula_values: Default::default(),
            draft_form: None,
            formula_id,
            committed,
            notice,
            converter,
            word,
            unit_menu: None,
            radix: 10,
            op: programmer::Op::And,
            view: plot::Viewport::default(),
            graph_bounds: Default::default(),
            graph_drag: None,
            show_roots: true,
            pending_history: None,
            file_busy: false,
            update: super::settings::Update::Idle,
            isolated,
            shortcut_capture: None,
            screenshot,
        };
        if app.formula().is_none() {
            app.formula_id = formulas::builtins()[0].id.clone();
        }
        app.reset_formula_inputs(window, cx);
        app.update_palette(window, cx);
        app.focus_page(window, cx);
        cx.observe_window_appearance(window, |app, window, cx| {
            app.update_palette(window, cx);
            cx.notify();
        })
        .detach();
        cx.on_app_quit(|app, _| {
            app.save();
            std::future::ready(())
        })
        .detach();
        cx.spawn_in(window, async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this
                    .update_in(cx, |app, window, cx| app.tick(window, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        app
    }

    pub fn input(
        field: Field,
        value: &str,
        placeholder: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let value = value.replace(['\r', '\n'], " ");
        let placeholder = placeholder.to_owned();
        let entity = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(value)
                .placeholder(placeholder)
                .validate(|value, _| !value.contains(['\r', '\n']))
        });
        cx.subscribe_in(&entity, window, move |app, _, event, window, cx| {
            match event {
                InputEvent::Change => {
                    if let Field::Plot(i) = field {
                        let plots = app
                            .notebook
                            .plots
                            .get_or_insert_with(|| [String::new(), String::new(), String::new()]);
                        *plots = app
                            .plots
                            .each_ref()
                            .map(|state| state.read(cx).value().to_string());
                        debug_assert!(i < plots.len());
                    }
                    if field == Field::Expression
                        && app
                            .committed
                            .as_deref()
                            .is_some_and(|old| old != app.expression.read(cx).value().as_ref())
                    {
                        app.committed = None;
                    }
                }
                InputEvent::PressEnter { .. }
                    if field == Field::Expression
                        && app.page == Page::Calculator
                        && app.panel == Panel::Workspace =>
                {
                    app.calculate(window, cx)
                }
                InputEvent::PressEnter { .. }
                    if matches!(field, Field::Integer | Field::Operand) =>
                {
                    app.apply_op(window, cx)
                }
                _ => {}
            }
            cx.notify();
        })
        .detach();
        entity
    }

    pub fn field(&self, state: &Entity<InputState>) -> Input {
        Input::new(state)
            .bordered(false)
            .focus_bordered(false)
            .appearance(false)
            .px_2()
    }

    pub fn entry(&self, state: &Entity<InputState>) -> Div {
        div()
            .w_full()
            .min_w_0()
            .border_1()
            .border_color(self.palette.line)
            .rounded(px(8.))
            .bg(self.palette.key)
            .child(self.field(state).w_full())
    }

    pub fn set(
        state: &Entity<InputState>,
        value: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let value = value.into();
        state.update(cx, |input, cx| {
            if value.contains(['\r', '\n']) {
                input.set_value(value.replace(['\r', '\n'], " "), window, cx);
            } else {
                input.set_value(value, window, cx);
            }
        });
    }

    pub fn tr(&self, text: impl AsRef<str>) -> String {
        self.notebook.language.text(text)
    }

    pub fn button(
        &self,
        id: impl Into<ElementId>,
        text: impl Into<SharedString>,
        selected: bool,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> Stateful<Div> {
        self.control(id, div().child(text.into()), selected, cx, on_click)
    }

    fn control(
        &self,
        id: impl Into<ElementId>,
        content: impl IntoElement,
        selected: bool,
        cx: &mut Context<Self>,
        on_click: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> Stateful<Div> {
        let p = self.palette;
        let on_click = std::rc::Rc::new(on_click);
        let keyboard = on_click.clone();
        row()
            .id(id)
            .focusable()
            .tab_stop(true)
            .gap_2()
            .justify_center()
            .cursor_pointer()
            .px_3()
            .py_2()
            .rounded(px(8.))
            .border_1()
            .border_color(if selected { p.accent } else { p.line })
            .bg(if selected { p.selected } else { p.key })
            .text_color(if selected { p.accent } else { p.text })
            .hover(move |style| style.opacity(0.86).border_color(p.muted))
            .focus(move |style| style.border_color(p.accent))
            .child(content)
            .on_key_down(cx.listener(move |app, event: &KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    keyboard(app, window, cx);
                    window.prevent_default();
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .on_click(cx.listener(move |app, _, window, cx| {
                on_click(app, window, cx);
                cx.notify();
            }))
    }

    pub fn title(&self, title: &str) -> Div {
        div()
            .text_size(px(29.))
            .font_weight(FontWeight::SEMIBOLD)
            .child(self.tr(title))
    }

    fn navigation(&self, narrow: bool, cx: &mut Context<Self>) -> Div {
        let p = self.palette;
        column()
            .gap_2()
            .h_full()
            .flex_shrink_0()
            .w(px(if narrow { 62. } else { 195. }))
            .p_3()
            .bg(p.rail)
            .border_r_1()
            .border_color(p.line)
            .child(
                row()
                    .gap_2()
                    .h(px(42.))
                    .pl(px(if narrow { 1. } else { 5. }))
                    .child(
                        row()
                            .justify_center()
                            .size(px(32.))
                            .rounded(px(9.))
                            .bg(p.accent)
                            .child(icon("calculator", p.accent_text).size(px(22.))),
                    )
                    .when(!narrow, |r| {
                        r.child(
                            div()
                                .font_weight(FontWeight::BOLD)
                                .text_lg()
                                .child("RustCalc"),
                        )
                    }),
            )
            .child(column().gap_2().mt_4().children(Page::ALL.map(|page| {
                let active = self.page == page && self.panel == Panel::Workspace;
                let title = self.tr(page.title());
                let tooltip = title.clone();
                self.control(
                    page.key(),
                    row()
                        .gap_3()
                        .child(icon(page.icon(), if active { p.accent } else { p.muted }))
                        .when(!narrow, |r| r.child(title)),
                    active,
                    cx,
                    move |app, window, cx| app.open_page(page, window, cx),
                )
                .when(!narrow, |b| b.justify_start())
                .when(narrow, |b| {
                    b.tooltip(move |window, cx| {
                        gpui_component::tooltip::Tooltip::new(tooltip.clone()).build(window, cx)
                    })
                })
            })))
            .child(div().flex_1())
            .children(
                [
                    (Panel::Updates, "updates", "Updates"),
                    (Panel::Help, "help", "Help"),
                    (Panel::Settings, "settings", "Settings"),
                ]
                .map(|(panel, glyph, title)| {
                    let active = self.panel == panel;
                    let title = self.tr(title);
                    let tooltip = title.clone();
                    self.control(
                        glyph,
                        row()
                            .gap_3()
                            .child(icon(glyph, if active { p.accent } else { p.muted }))
                            .when(!narrow, |r| r.child(title)),
                        active,
                        cx,
                        move |app, window, cx| app.open_panel(panel, window, cx),
                    )
                    .when(!narrow, |b| b.justify_start())
                    .when(narrow, |b| {
                        b.tooltip(move |window, cx| {
                            gpui_component::tooltip::Tooltip::new(tooltip.clone()).build(window, cx)
                        })
                    })
                }),
            )
            .child(self.button(
                "language",
                if self.notebook.language == Language::English {
                    "EN"
                } else {
                    "TR"
                },
                false,
                cx,
                |app, window, cx| {
                    app.notebook.language = if app.notebook.language == Language::English {
                        Language::Turkish
                    } else {
                        Language::English
                    };
                    app.update_placeholders(window, cx);
                    app.save();
                },
            ))
    }

    pub fn open_page(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        self.page = page;
        self.notebook.workspace = page.key().into();
        self.panel = Panel::Workspace;
        self.focus_page(window, cx);
        self.save();
    }

    pub fn open_panel(&mut self, panel: Panel, window: &mut Window, cx: &mut Context<Self>) {
        self.panel = if self.panel == panel {
            Panel::Workspace
        } else {
            panel
        };
        self.shortcut_capture = None;
        self.focus_page(window, cx);
    }

    fn focus_page(&self, window: &mut Window, cx: &mut App) {
        let state = match (self.panel, self.page) {
            (Panel::Workspace, Page::Calculator) => Some(&self.expression),
            (Panel::Workspace, Page::Converter) => Some(&self.convert),
            (Panel::Workspace, Page::Programmer) => Some(&self.integer),
            (Panel::History, _) => Some(&self.search),
            _ => None,
        };
        if let Some(state) = state {
            state.update(cx, |input, cx| input.focus(window, cx));
        } else {
            window.focus(&self.focus);
        }
    }

    fn update_placeholders(&self, window: &mut Window, cx: &mut App) {
        for (state, text) in [
            (&self.search, "Search history"),
            (&self.formula_search, "Search formulas"),
            (&self.convert, "Value"),
            (&self.integer, "Value"),
            (&self.operand, "Value"),
        ] {
            let value = self.tr(text);
            state.update(cx, |state, cx| state.set_placeholder(value, window, cx));
        }
    }

    fn palette_for(&self, window: &Window) -> Palette {
        let light = if self.notebook.follow_system_theme {
            matches!(
                window.appearance(),
                WindowAppearance::Light | WindowAppearance::VibrantLight
            )
        } else {
            self.notebook.light
        };
        let high = if self.notebook.follow_system_theme {
            super::system_high_contrast()
        } else {
            self.notebook.high_contrast
        };
        Palette::new(light, high)
    }

    pub fn update_palette(&mut self, window: &mut Window, cx: &mut App) {
        self.palette = self.palette_for(window);
        let theme = Theme::global_mut(cx);
        theme.colors.foreground = self.palette.text.into();
        theme.colors.muted_foreground = self.palette.muted.into();
        theme.colors.caret = self.palette.accent.into();
        theme.colors.selection = self.palette.selected.into();
        self.update_placeholders(window, cx);
    }

    pub fn save(&mut self) {
        if let Some(storage) = &mut self.storage
            && let Err(error) = storage.save(&self.notebook)
        {
            self.notice = Some(error);
        }
    }

    fn tick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save();
        if self.notebook.follow_system_theme && self.palette != self.palette_for(window) {
            self.update_palette(window, cx);
            cx.notify();
        }
        self.auto_update(window, cx);
        self.capture_screenshot(window, cx);
    }
}

impl Render for Calc {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette;
        let width = f32::from(window.viewport_size().width);
        let zoom = self.notebook.zoom();
        let narrow = width < 910. * zoom;
        let history_inline = width >= 1040. * zoom;
        let compact_calculator = window.viewport_size().height
            < px(if self.notebook.scientific { 950. } else { 750. } * zoom);
        window.set_rem_size(px(15. * zoom));
        let workspace = match self.panel {
            Panel::Workspace => match self.page {
                Page::Calculator => self.calculator(history_inline, compact_calculator, cx),
                Page::Converter => self.converter(cx),
                Page::Programmer => self.programmer(cx),
                Page::Graph => self.graph(cx),
                Page::Formulas => self.formulas(cx),
            },
            Panel::History => self.history(cx).into_any_element(),
            Panel::Settings => self.settings(cx).into_any_element(),
            Panel::Help => self.help(cx).into_any_element(),
            Panel::Updates => self.updates(cx).into_any_element(),
        };
        row()
            .id("rustcalc")
            .track_focus(&self.focus)
            .size_full()
            .bg(p.bg)
            .text_color(p.text)
            .text_size(rems(1.))
            .capture_key_down(cx.listener(Self::key_down))
            .child(self.navigation(narrow, cx))
            .child(
                column()
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .min_h_0()
                    .when_some(self.update_banner(cx), |content, banner| {
                        content.child(banner)
                    })
                    .when_some(self.notice.clone(), |content, notice| {
                        content.child(
                            row()
                                .gap_3()
                                .px_4()
                                .py_2()
                                .bg(p.selected)
                                .child(div().flex_1().child(self.tr(notice)))
                                .child(self.button(
                                    "dismiss",
                                    self.tr("Dismiss"),
                                    false,
                                    cx,
                                    |app, _, _| app.notice = None,
                                )),
                        )
                    })
                    .child(
                        column()
                            .id("workspace")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .p(px(if narrow { 18. } else { 28. }))
                            .child(workspace),
                    ),
            )
            .when(
                history_inline && self.panel == Panel::Workspace && self.page == Page::Calculator,
                |layout| {
                    layout.child(
                        column()
                            .id("history-side")
                            .w(px(310.))
                            .h_full()
                            .flex_shrink_0()
                            .overflow_y_scroll()
                            .border_l_1()
                            .border_color(p.line)
                            .bg(p.rail)
                            .p_4()
                            .child(self.history(cx)),
                    )
                },
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{Calc, Page, Root, minimum_window_size};
    use gpui::{
        AppContext as _, Entity, TestAppContext, VisualTestContext, WindowHandle, px, size,
    };

    fn demo(cx: &mut TestAppContext) -> (WindowHandle<Root>, Entity<Calc>) {
        cx.update(gpui_component::init);
        let mut entity = None;
        let window = cx.add_window(|window, cx| {
            let view = cx.new(|cx| Calc::new(&["--demo".into()], window, cx));
            entity = Some(view.clone());
            Root::new(view, window, cx)
        });
        cx.run_until_parked();
        (window, entity.unwrap())
    }

    #[gpui::test]
    fn all_calculator_keys_fit_the_minimum_window(cx: &mut TestAppContext) {
        let (window, view) = demo(cx);
        let mut visual = VisualTestContext::from_window(window.into(), cx);
        for viewport in [
            size(px(1200.), px(1000.)),
            minimum_window_size(),
            size(px(1200.), px(810.)),
        ] {
            for scientific in [false, true] {
                visual.simulate_resize(viewport);
                view.update(&mut visual, |app, cx| {
                    app.notebook.scientific = scientific;
                    app.notebook.language = rustcalc::i18n::Language::English;
                    cx.notify();
                });
                visual.run_until_parked();
                visual.update(|window, cx| {
                    window.draw(cx).clear();
                });
                let keys = "clear ( ) backspace / 7 8 9 % * 4 5 6 ^2 - 1 2 3 sqrt( + negate 0 . ans equals";
                let scientific_keys =
                    "sin( cos( tan( ln( log( asin( acos( atan( exp( abs( pi e ^ ! reciprocal";
                for key in keys
                    .split_whitespace()
                    .chain(scientific_keys.split_whitespace().filter(|_| scientific))
                {
                    let bounds = visual.debug_bounds(key).unwrap_or_else(|| {
                        panic!("{key} is missing at {viewport:?}, scientific={scientific}")
                    });
                    assert!(
                        bounds.left() >= px(0.)
                            && bounds.top() >= px(0.)
                            && bounds.right() <= viewport.width
                            && bounds.bottom() <= viewport.height,
                        "{key} is clipped: {bounds:?}, viewport={viewport:?}, scientific={scientific}"
                    );
                    assert!(bounds.size.width >= px(32.) && bounds.size.height >= px(32.));
                }
            }
        }
    }

    #[gpui::test]
    fn keyboard_calculates_and_continues_after_a_committed_result(cx: &mut TestAppContext) {
        let (window, view) = demo(cx);
        cx.simulate_keystrokes(window.into(), "4 2 enter");
        view.read_with(cx, |app, cx| {
            assert_eq!(app.expression.read(cx).value().as_ref(), "42");
            assert_eq!(app.notebook.answer, "42");
            assert_eq!(app.notebook.entries.last().unwrap().expression, "42");
        });
        cx.simulate_keystrokes(window.into(), "+ 8 enter");
        view.read_with(cx, |app, _| {
            assert_eq!(app.notebook.answer, "50");
            assert_eq!(app.notebook.entries.last().unwrap().expression, "ans+8");
        });
    }

    #[gpui::test]
    fn history_search_does_not_commit_a_calculation(cx: &mut TestAppContext) {
        let (window, view) = demo(cx);
        window
            .update(cx, |_, window, cx| {
                view.update(cx, |app, cx| {
                    app.search.update(cx, |input, cx| input.focus(window, cx));
                })
            })
            .unwrap();
        cx.run_until_parked();
        cx.simulate_keystrokes(window.into(), "1 2 enter");
        view.read_with(cx, |app, cx| {
            assert_eq!(app.search.read(cx).value().as_ref(), "12");
            assert_eq!(app.notebook.entries.len(), 4);
            assert_eq!(app.notebook.answer, "5040");
        });
        cx.simulate_keystrokes(window.into(), "escape");
        view.read_with(cx, |app, cx| {
            assert!(app.search.read(cx).value().is_empty())
        });
    }

    #[gpui::test]
    fn equals_after_a_comparison_operator_remains_part_of_the_expression(cx: &mut TestAppContext) {
        let (window, view) = demo(cx);
        cx.simulate_keystrokes(window.into(), "3 < = 4 enter");
        view.read_with(cx, |app, cx| {
            assert_eq!(app.expression.read(cx).value().as_ref(), "3<=4");
            assert_eq!(app.notebook.answer, "1");
        });
    }

    #[gpui::test]
    fn shortcut_navigation_keeps_the_saved_notebook_and_returns_to_the_calculator(
        cx: &mut TestAppContext,
    ) {
        let (window, view) = demo(cx);
        for (key, page) in [
            ("2", Page::Converter),
            ("3", Page::Programmer),
            ("4", Page::Graph),
            ("5", Page::Formulas),
            ("1", Page::Calculator),
        ] {
            let modifier = if cfg!(target_os = "macos") {
                "cmd"
            } else {
                "ctrl"
            };
            cx.simulate_keystrokes(window.into(), &format!("{modifier}-{key}"));
            view.read_with(cx, |app, _| {
                assert_eq!(app.page, page);
                assert_eq!(app.notebook.workspace, page.key());
                assert_eq!(app.notebook.answer, "5040");
            });
        }
    }
}
