//! Formulas workspace: ready-made templates and user formulas with live results.
use super::*;
use rustcalc::formulas::{self, Field, Format, Formula, Kind, Output};

/// Built-in category order; user categories follow.
const CATEGORIES: [&str; 4] = ["Finance", "Dates", "Everyday", "Health"];

pub(super) enum FormulaFileReply {
    Exported(Result<bool, String>),
    Imported(Result<Option<Vec<Formula>>, String>),
}

/// Working state of the formula editor.
pub(super) struct Draft {
    /// The custom formula being replaced, or `None` for a new one.
    pub(super) replaces: Option<String>,
    pub(super) formula: Formula,
}

/// Placeholder text, dimmed so it never reads as a typed value.
/// A small ✕ button, named "Remove" for screen readers.
fn remove_button(ui: &mut egui::Ui, p: Palette) -> bool {
    let button = ui.small_button("✕").on_hover_text(p.tr("Remove"));
    spoken(&button, p.tr("Remove"));
    button.clicked()
}
fn hint(p: Palette, text: &str) -> RichText {
    RichText::new(p.tr(text)).color(p.muted.gamma_multiply(0.7))
}

impl RustCalc {
    pub(super) fn find_formula(&self, id: &str) -> Option<Formula> {
        formulas::builtins()
            .iter()
            .chain(&self.notebook.formulas)
            .find(|f| f.id == id)
            .cloned()
    }
    fn formula_context(&self) -> Context {
        Context {
            today: Some(rustcalc::dates::today()),
            ..self.notebook.context()
        }
    }
    /// Current input text for a formula, reset when its inputs changed shape.
    fn formula_inputs(&mut self, formula: &Formula) -> &mut Vec<String> {
        let values = self
            .formula_values
            .entry(formula.id.clone())
            .or_insert_with(|| formula.defaults());
        if values.len() != formula.inputs.len() {
            *values = formula.defaults();
        }
        values
    }
    pub(super) fn formulas(&mut self, ui: &mut egui::Ui, p: Palette) {
        self.header(
            ui,
            p,
            "Formulas",
            "Ready-made and custom calculations. Enter values, get results.",
        );
        if let Some(error) = self.formula_error.clone() {
            ui.horizontal_wrapped(|ui| {
                ui.label(label(error, 13.0, p.danger, p.language));
                if ui.small_button(p.tr("Dismiss")).clicked() {
                    self.formula_error = None;
                }
            });
            ui.add_space(6.0);
        }
        if self.formula_draft.is_some() {
            self.formula_editor(ui, p);
            return;
        }
        if ui.available_width() >= 760.0 {
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(230.0, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(230.0);
                        let height = (ui.available_height() - 50.0).max(300.0);
                        self.formula_list(ui, p, height);
                    },
                );
                ui.separator();
                ui.vertical(|ui| self.formula_detail(ui, p));
            });
        } else {
            egui::CollapsingHeader::new(p.tr("All formulas"))
                .id_salt("formula_list_compact")
                .default_open(false)
                .show(ui, |ui| self.formula_list(ui, p, 220.0));
            ui.add_space(6.0);
            self.formula_detail(ui, p);
        }
    }
    fn formula_list(&mut self, ui: &mut egui::Ui, p: Palette, height: f32) {
        let search = ui.add(
            egui::TextEdit::singleline(&mut self.formula_search)
                .id(egui::Id::new("formula_search"))
                .hint_text(hint(p, "Search formulas"))
                .desired_width(f32::INFINITY),
        );
        field_name(&search, p.tr("Search formulas"));
        ui.add_space(4.0);
        let query = rustcalc::i18n::fold(self.formula_search.trim());
        let language = p.language;
        let matches = |f: &Formula, builtin: bool| {
            let text = |s: &str| {
                if builtin {
                    language.text(s)
                } else {
                    s.to_owned()
                }
            };
            query.is_empty()
                || rustcalc::i18n::fold(&text(&f.name)).contains(&query)
                || rustcalc::i18n::fold(&text(&f.description)).contains(&query)
                || rustcalc::i18n::fold(&f.name).contains(&query)
        };
        let mut groups: Vec<(String, Vec<(String, String)>)> = Vec::new();
        let mut push = |category: &str, id: &str, name: String| match groups
            .iter_mut()
            .find(|(c, _)| c == category)
        {
            Some((_, items)) => items.push((id.to_owned(), name)),
            None => groups.push((category.to_owned(), vec![(id.to_owned(), name)])),
        };
        for category in CATEGORIES {
            for f in formulas::builtins()
                .iter()
                .filter(|f| f.category == category && matches(f, true))
            {
                push(category, &f.id, p.tr(&f.name));
            }
        }
        for f in self.notebook.formulas.iter().filter(|f| matches(f, false)) {
            let category = if f.category.trim().is_empty() {
                "Custom"
            } else {
                f.category.trim()
            };
            push(category, &f.id, f.name.clone());
        }
        egui::ScrollArea::vertical()
            .id_salt("formula_list")
            .max_height(height)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                if groups.is_empty() {
                    ui.label(label(
                        "No formulas match your search.",
                        12.0,
                        p.muted,
                        p.language,
                    ));
                }
                for (category, items) in groups {
                    ui.add_space(6.0);
                    ui.label(
                        label(
                            p.language.upper(&p.tr(&category)),
                            10.0,
                            p.muted,
                            p.language,
                        )
                        .strong(),
                    );
                    for (id, name) in items {
                        let selected = self.formula_selected == id;
                        let response = ui.selectable_label(selected, name);
                        #[cfg(test)]
                        self.targets.insert(format!("formula_{id}"), response.rect);
                        if response.clicked() {
                            self.formula_selected = id;
                            self.formula_delete = false;
                        }
                    }
                }
            });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            let new = ui.button(p.tr("+ New formula"));
            #[cfg(test)]
            self.targets.insert("formula_new".into(), new.rect);
            if new.clicked() {
                self.formula_draft = Some(Draft {
                    replaces: None,
                    formula: Formula::blank(language),
                });
            }
            let busy = self.formula_file_worker.is_some();
            if ui
                .add_enabled(!busy, egui::Button::new(p.tr("Import…")))
                .on_hover_text(p.tr("Add formulas from a RustCalc formula file"))
                .clicked()
            {
                self.start_formula_file(false, ui.ctx());
            }
            if ui
                .add_enabled(
                    !busy && !self.notebook.formulas.is_empty(),
                    egui::Button::new(p.tr("Export…")),
                )
                .on_hover_text(p.tr("Save your formulas to share or back up"))
                .clicked()
            {
                self.start_formula_file(true, ui.ctx());
            }
        });
    }
    fn formula_detail(&mut self, ui: &mut egui::Ui, p: Palette) {
        let Some(formula) = self.find_formula(&self.formula_selected) else {
            ui.label(label("Choose a formula.", 14.0, p.muted, p.language));
            return;
        };
        let builtin = formulas::is_builtin(&formula.id);
        let text = |s: &str| if builtin { p.tr(s) } else { s.to_owned() };
        ui.horizontal_wrapped(|ui| {
            ui.label(label(text(&formula.name), 22.0, p.text, p.language));
            ui.label(label(
                if builtin { "TEMPLATE" } else { "CUSTOM" },
                10.0,
                p.accent,
                p.language,
            ));
        });
        if !formula.description.is_empty() {
            ui.label(label(text(&formula.description), 13.0, p.muted, p.language));
        }
        ui.add_space(10.0);
        let context = self.formula_context();
        let language = p.language;
        let mut values = self.formula_inputs(&formula).clone();
        let run = formulas::run(&formula, &values, context);
        let mut changed = false;
        card(p).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(label("INPUTS", 10.0, p.muted, p.language).strong());
            ui.add_space(4.0);
            egui::Grid::new(("formula_inputs", &formula.id))
                .num_columns(2)
                .spacing(egui::vec2(12.0, 6.0))
                .show(ui, |ui| {
                    for (i, field) in formula.inputs.iter().enumerate() {
                        ui.label(text(&field.label));
                        ui.horizontal(|ui| {
                            let edit = ui.add(
                                egui::TextEdit::singleline(&mut values[i])
                                    .id(egui::Id::new(("formula_input", &formula.id, i)))
                                    .font(FontId::monospace(15.0))
                                    .desired_width(190.0)
                                    .char_limit(200),
                            );
                            field_name(&edit, field.label.as_str());
                            #[cfg(test)]
                            self.targets
                                .insert(format!("input_{}", field.name), edit.rect);
                            changed |= edit.changed();
                            if field.kind == Kind::Date
                                && ui
                                    .small_button(p.tr("Today"))
                                    .on_hover_text(p.tr("Use today's date"))
                                    .clicked()
                            {
                                values[i] = rustcalc::dates::format(
                                    rustcalc::dates::today(),
                                    language == Language::Turkish,
                                );
                                changed = true;
                            }
                            match &run.inputs[i] {
                                Err(e) => {
                                    ui.label(label(e, 12.0, p.danger, p.language));
                                }
                                Ok(v) if field.kind == Kind::Date => {
                                    if let Ok(shown) = formulas::format(
                                        *v,
                                        Format::Date,
                                        language,
                                        self.notebook.numbers,
                                    ) {
                                        ui.label(label(shown, 12.0, p.muted, p.language));
                                    }
                                }
                                // 250.000 is 250 here; say so before it skews a result.
                                Ok(v) if formulas::looks_grouped(&values[i]) => {
                                    ui.label(label(
                                        format!(
                                            "= {}  ·  {}",
                                            v.text(),
                                            p.tr("No thousands separators")
                                        ),
                                        12.0,
                                        p.danger,
                                        p.language,
                                    ));
                                }
                                Ok(_) => {}
                            }
                        });
                        ui.end_row();
                    }
                });
            ui.add_space(4.0);
            if ui.small_button(p.tr("Reset values")).clicked() {
                values = formula.defaults();
                changed = true;
            }
        });
        if changed {
            *self.formula_inputs(&formula) = values;
        }
        ui.add_space(10.0);
        card(p).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(label("RESULTS", 10.0, p.muted, p.language).strong());
            for (output, value) in formula.outputs.iter().zip(&run.outputs) {
                ui.add_space(6.0);
                ui.label(label(text(&output.label), 12.0, p.muted, p.language));
                match value.as_ref().map_err(String::clone).and_then(|v| {
                    formulas::format(*v, output.format, language, self.notebook.numbers)
                }) {
                    Ok(shown) => {
                        let unit = if output.unit.is_empty() {
                            String::new()
                        } else {
                            format!(" {}", text(&output.unit))
                        };
                        let response = ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!("{shown}{unit}"))
                                        .size(22.0)
                                        .color(p.accent),
                                )
                                .frame(false),
                            )
                            .on_hover_text(p.tr("Copy"));
                        if response.clicked() {
                            // Copy the plain number as shown (988.86, not all 28
                            // digits), so it pastes into the calculator.
                            let plain = match (output.format, value) {
                                (Format::Number | Format::Money | Format::Percent, Ok(v)) => {
                                    let places = if output.format == Format::Number {
                                        10
                                    } else {
                                        2
                                    };
                                    let rounded = v.number.round_dp_with_strategy(
                                        places,
                                        rust_decimal::RoundingStrategy::MidpointAwayFromZero,
                                    );
                                    self.notebook.copy_text(&rounded.normalize().to_string())
                                }
                                _ => shown,
                            };
                            ui.ctx().copy_text(plain);
                            self.toast("Result copied");
                        }
                    }
                    Err(e) => {
                        ui.label(label(e, 13.0, p.danger, p.language));
                    }
                }
            }
        });
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            let duplicate = ui.button(p.tr(if builtin {
                "Customize a copy"
            } else {
                "Duplicate"
            }));
            #[cfg(test)]
            self.targets
                .insert("formula_duplicate".into(), duplicate.rect);
            if duplicate.clicked() {
                let mut copy = if builtin {
                    formula.localized_copy(language)
                } else {
                    Formula {
                        id: String::new(),
                        ..formula.clone()
                    }
                };
                if !builtin {
                    copy.name = format!("{} {}", copy.name, p.tr("(copy)"));
                }
                self.formula_draft = Some(Draft {
                    replaces: None,
                    formula: copy,
                });
            }
            if !builtin {
                let edit = ui.button(p.tr("Edit"));
                #[cfg(test)]
                self.targets.insert("formula_edit".into(), edit.rect);
                if edit.clicked() {
                    self.formula_draft = Some(Draft {
                        replaces: Some(formula.id.clone()),
                        formula: formula.clone(),
                    });
                }
                if self.formula_delete {
                    let confirm =
                        ui.button(RichText::new(p.tr("Delete permanently")).color(p.danger));
                    #[cfg(test)]
                    self.targets
                        .insert("formula_delete_confirm".into(), confirm.rect);
                    if confirm.clicked() {
                        self.notebook.formulas.retain(|f| f.id != formula.id);
                        self.formula_values.remove(&formula.id);
                        self.formula_selected = formulas::builtins()[0].id.clone();
                        self.formula_delete = false;
                        self.toast("Formula deleted");
                    }
                    if ui.button(p.tr("Cancel")).clicked() {
                        self.formula_delete = false;
                    }
                } else {
                    let delete = ui.button(p.tr("Delete"));
                    #[cfg(test)]
                    self.targets.insert("formula_delete".into(), delete.rect);
                    if delete.clicked() {
                        self.formula_delete = true;
                    }
                }
            }
        });
        ui.add_space(8.0);
        ui.label(label(
            "Inputs accept expressions such as 1500*12 or today() + 30. Dates: 2026-10-21 or 21.10.2026. Click a result to copy it.",
            12.0,
            p.muted,
            p.language,
        ));
    }
    fn formula_editor(&mut self, ui: &mut egui::Ui, p: Palette) {
        let Some(draft) = self.formula_draft.as_mut() else {
            return;
        };
        let formula = &mut draft.formula;
        let title = if draft.replaces.is_some() {
            "Edit formula"
        } else {
            "New formula"
        };
        ui.label(label(title, 20.0, p.text, p.language));
        ui.add_space(6.0);
        egui::Grid::new("formula_meta")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 6.0))
            .show(ui, |ui| {
                ui.label(p.tr("Name"));
                let name = ui.add(
                    egui::TextEdit::singleline(&mut formula.name)
                        .id(egui::Id::new("formula_name"))
                        .desired_width(320.0)
                        .char_limit(60),
                );
                field_name(&name, p.tr("Name"));
                #[cfg(test)]
                self.targets.insert("formula_name".into(), name.rect);
                #[cfg(not(test))]
                let _ = name;
                ui.end_row();
                ui.label(p.tr("Category"));
                let category = ui.add(
                    egui::TextEdit::singleline(&mut formula.category)
                        .hint_text(hint(p, "Custom"))
                        .desired_width(200.0)
                        .char_limit(40),
                );
                field_name(&category, p.tr("Category"));
                ui.end_row();
                ui.label(p.tr("Description"));
                let description = ui.add(
                    egui::TextEdit::multiline(&mut formula.description)
                        .desired_rows(2)
                        .desired_width(f32::INFINITY)
                        .char_limit(500),
                );
                field_name(&description, p.tr("Description"));
                ui.end_row();
            });
        ui.add_space(10.0);
        ui.label(label("INPUTS", 10.0, p.muted, p.language).strong());
        ui.label(label(
            "Variable names start with a letter and use letters, digits or _.",
            11.0,
            p.muted,
            p.language,
        ));
        let mut remove = None;
        for (i, field) in formula.inputs.iter_mut().enumerate() {
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut field.name)
                        .hint_text(hint(p, "name"))
                        .font(FontId::monospace(14.0))
                        .desired_width(110.0)
                        .char_limit(32),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut field.label)
                        .hint_text(hint(p, "Label"))
                        .desired_width(170.0)
                        .char_limit(80),
                );
                egui::ComboBox::from_id_salt(("field_kind", i))
                    .selected_text(p.tr(match field.kind {
                        Kind::Number => "Number",
                        Kind::Date => "Date",
                    }))
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut field.kind, Kind::Number, p.tr("Number"));
                        ui.selectable_value(&mut field.kind, Kind::Date, p.tr("Date"));
                    });
                ui.add(
                    egui::TextEdit::singleline(&mut field.default)
                        .hint_text(hint(p, "Default"))
                        .font(FontId::monospace(14.0))
                        .desired_width(120.0)
                        .char_limit(200),
                );
                if remove_button(ui, p) {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            formula.inputs.remove(i);
        }
        if formula.inputs.len() < formulas::MAX_FIELDS && ui.button(p.tr("+ Add input")).clicked() {
            let n = formula.inputs.len() + 1;
            formula.inputs.push(Field {
                name: format!("input{n}"),
                label: format!("{} {n}", p.tr("Input")),
                kind: Kind::Number,
                default: "0".into(),
            });
        }
        ui.add_space(10.0);
        ui.label(label("RESULTS", 10.0, p.muted, p.language).strong());
        ui.label(label(
            "Each result can use the inputs and the results above it.",
            11.0,
            p.muted,
            p.language,
        ));
        let preview = formulas::run(
            formula,
            &formula.defaults(),
            Context {
                today: Some(rustcalc::dates::today()),
                ..self.notebook.context()
            },
        );
        let mut remove = None;
        let count = formula.outputs.len();
        for (i, output) in formula.outputs.iter_mut().enumerate() {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut output.name)
                        .hint_text(hint(p, "name"))
                        .font(FontId::monospace(14.0))
                        .desired_width(110.0)
                        .char_limit(32),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut output.label)
                        .hint_text(hint(p, "Label"))
                        .desired_width(170.0)
                        .char_limit(80),
                );
                egui::ComboBox::from_id_salt(("output_format", i))
                    .selected_text(p.tr(output.format.title()))
                    .width(130.0)
                    .show_ui(ui, |ui| {
                        for format in Format::ALL {
                            ui.selectable_value(&mut output.format, format, p.tr(format.title()));
                        }
                    });
                ui.add(
                    egui::TextEdit::singleline(&mut output.unit)
                        .hint_text(hint(p, "Unit"))
                        .desired_width(60.0)
                        .char_limit(20),
                );
                if count > 1 && remove_button(ui, p) {
                    remove = Some(i);
                }
            });
            let expression = ui.add(
                egui::TextEdit::singleline(&mut output.expression)
                    .id(egui::Id::new(("output_expression", i)))
                    .hint_text(hint(p, "Expression, e.g. price * (1 + rate / 100)"))
                    .font(FontId::monospace(15.0))
                    .desired_width(f32::INFINITY)
                    .char_limit(1024),
            );
            #[cfg(test)]
            self.targets
                .insert(format!("output_expression_{i}"), expression.rect);
            #[cfg(not(test))]
            let _ = expression;
            match preview.outputs.get(i) {
                Some(Ok(value)) => {
                    let shown =
                        formulas::format(*value, output.format, p.language, self.notebook.numbers)
                            .unwrap_or_else(|e| p.tr(e));
                    ui.label(label(format!("= {shown}"), 12.0, p.accent, p.language));
                }
                Some(Err(e)) => {
                    ui.label(label(e, 12.0, p.danger, p.language));
                }
                None => {}
            }
        }
        if let Some(i) = remove {
            formula.outputs.remove(i);
        }
        if formula.outputs.len() < formulas::MAX_FIELDS && ui.button(p.tr("+ Add result")).clicked()
        {
            let n = formula.outputs.len() + 1;
            formula.outputs.push(Output {
                name: format!("result{n}"),
                label: format!("{} {n}", p.tr("Result")),
                expression: "0".into(),
                format: Format::Number,
                unit: String::new(),
            });
        }
        ui.add_space(8.0);
        egui::CollapsingHeader::new(p.tr("Functions you can use"))
            .id_salt("formula_functions")
            .show(ui, |ui| {
                ui.label(
                    RichText::new(engine::function_names().collect::<Vec<_>>().join("  "))
                        .monospace()
                        .size(12.0),
                );
                ui.label(label(
                    "Separate arguments with ;  —  e.g. max(a; b), if(a > b; a; b), round(x; 2). Compare with < > <= >= == !=. Dates are day numbers: end - start gives days; date(2026; 12; 31), today(), addmonths(d; 3), workdays(a; b).",
                    12.0,
                    p.muted,
                    p.language,
                ));
            });
        ui.add_space(10.0);
        let valid = formulas::validate(formula);
        if let Err(e) = &valid {
            ui.label(label(e, 13.0, p.danger, p.language));
        }
        let mut save = false;
        let mut cancel = false;
        ui.horizontal(|ui| {
            let button = ui.add_enabled(valid.is_ok(), egui::Button::new(p.tr("Save formula")));
            #[cfg(test)]
            self.targets.insert("formula_save".into(), button.rect);
            save = button.clicked();
            let button = ui.button(p.tr("Cancel"));
            #[cfg(test)]
            self.targets.insert("formula_cancel".into(), button.rect);
            cancel = button.clicked();
        });
        if cancel {
            self.formula_draft = None;
        } else if save && let Some(draft) = self.formula_draft.take() {
            self.save_formula(draft);
        }
    }
    pub(super) fn save_formula(&mut self, draft: Draft) {
        let mut formula = draft.formula;
        formula.category = formula.category.trim().to_owned();
        match draft.replaces {
            Some(id) => {
                formula.id = id.clone();
                if let Some(slot) = self.notebook.formulas.iter_mut().find(|f| f.id == id) {
                    *slot = formula;
                }
                self.formula_values.remove(&id);
                self.formula_selected = id;
            }
            None => {
                if self.notebook.formulas.len() >= formulas::MAX_FORMULAS {
                    self.formula_error = Some("You can keep at most 200 custom formulas".into());
                    return;
                }
                formula.id = formulas::new_id(&self.notebook.formulas, unix_now());
                self.formula_selected = formula.id.clone();
                self.notebook.formulas.push(formula);
            }
        }
        self.toast("Formula saved");
    }
    fn start_formula_file(&mut self, export: bool, ctx: &egui::Context) {
        let (send, receive) = mpsc::channel();
        self.formula_file_worker = Some(receive);
        let custom = self.notebook.formulas.clone();
        let language = self.notebook.language;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let dialog =
                rfd::FileDialog::new().add_filter(language.text("RustCalc formulas"), &["json"]);
            let reply = if export {
                FormulaFileReply::Exported(
                    match dialog
                        .set_title(language.text("Export…"))
                        .set_file_name("rustcalc-formulas.json")
                        .save_file()
                    {
                        Some(path) => formulas::write_file(&path, &custom).map(|_| true),
                        None => Ok(false),
                    },
                )
            } else {
                FormulaFileReply::Imported(
                    dialog
                        .set_title(language.text("Import…"))
                        .pick_file()
                        .map(|path| formulas::read_file(&path))
                        .transpose(),
                )
            };
            let _ = send.send(reply);
            ctx.request_repaint();
        });
    }
    pub(super) fn poll_formula_file(&mut self) {
        let Some(receiver) = &self.formula_file_worker else {
            return;
        };
        match receiver.try_recv() {
            Ok(reply) => {
                self.formula_file_worker = None;
                match reply {
                    FormulaFileReply::Exported(Ok(true)) => self.toast("Formulas exported"),
                    FormulaFileReply::Imported(Ok(Some(incoming))) => {
                        match formulas::merge(&mut self.notebook.formulas, incoming, unix_now()) {
                            Ok(added) => self.toast(format!("Formulas imported: {added}")),
                            Err(e) => self.formula_error = Some(e),
                        }
                    }
                    FormulaFileReply::Exported(Err(e)) | FormulaFileReply::Imported(Err(e)) => {
                        self.formula_error = Some(e)
                    }
                    _ => {}
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.formula_file_worker = None;
                self.formula_error =
                    Some("The formula file operation could not finish. Please try again.".into());
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
    }
}
