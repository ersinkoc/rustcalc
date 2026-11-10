use super::{
    app::{Calc, Field},
    theme::*,
};
use gpui::{prelude::*, *};
use gpui_component::input::InputState;
use rustcalc::{
    formulas::{self, Format, Formula, Kind},
    i18n,
};

struct DraftInput {
    name: Entity<InputState>,
    label: Entity<InputState>,
    value: Entity<InputState>,
    kind: Kind,
}
struct DraftOutput {
    name: Entity<InputState>,
    label: Entity<InputState>,
    value: Entity<InputState>,
    unit: Entity<InputState>,
    format: Format,
}
pub(super) struct DraftForm {
    id: String,
    name: Entity<InputState>,
    category: Entity<InputState>,
    description: Entity<InputState>,
    inputs: Vec<DraftInput>,
    outputs: Vec<DraftOutput>,
}

fn editor(
    value: &str,
    placeholder: &str,
    window: &mut Window,
    cx: &mut Context<Calc>,
) -> Entity<InputState> {
    Calc::input(Field::Draft, value, placeholder, window, cx)
}

impl DraftInput {
    fn new(field: formulas::Field, window: &mut Window, cx: &mut Context<Calc>) -> Self {
        Self {
            name: editor(&field.name, "amount", window, cx),
            label: editor(&field.label, "", window, cx),
            value: editor(&field.default, "", window, cx),
            kind: field.kind,
        }
    }
}
impl DraftOutput {
    fn new(output: formulas::Output, window: &mut Window, cx: &mut Context<Calc>) -> Self {
        Self {
            name: editor(&output.name, "total", window, cx),
            label: editor(&output.label, "", window, cx),
            value: editor(&output.expression, "", window, cx),
            unit: editor(&output.unit, "", window, cx),
            format: output.format,
        }
    }
}

impl Calc {
    pub fn formula(&self) -> Option<&Formula> {
        formulas::builtins()
            .iter()
            .chain(&self.notebook.formulas)
            .find(|f| f.id == self.formula_id)
    }

    pub fn reset_formula_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let values = self
            .formula()
            .map(|formula| {
                self.formula_values
                    .get(&formula.id)
                    .filter(|values| values.len() == formula.inputs.len())
                    .cloned()
                    .unwrap_or_else(|| formula.defaults())
            })
            .unwrap_or_default();
        self.formula_inputs = values
            .iter()
            .enumerate()
            .map(|(i, default)| Self::input(Field::Formula(i), default, "", window, cx))
            .collect();
        self.notebook.formula_selected = Some(self.formula_id.clone());
    }

    pub fn formulas(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(draft) = &self.draft_form {
            return self.formula_editor(draft, cx).into_any_element();
        }
        let p = self.palette;
        let query = i18n::fold(&self.formula_search.read(cx).value());
        let library = formulas::builtins()
            .iter()
            .chain(&self.notebook.formulas)
            .filter(|f| {
                let matches = |text: &str| i18n::fold(&self.tr(text)).contains(&query);
                matches(&f.name) || matches(&f.category) || matches(&f.description)
            });
        column()
            .gap_4()
            .child(
                row()
                    .gap_2()
                    .flex_wrap()
                    .child(self.title("Formulas"))
                    .child(div().flex_1())
                    .child(self.button(
                        "import-formulas",
                        self.tr("Import"),
                        false,
                        cx,
                        |app, window, cx| app.formula_file(false, window, cx),
                    ))
                    .when(!self.notebook.formulas.is_empty(), |r| {
                        r.child(self.button(
                            "export-formulas",
                            self.tr("Export"),
                            false,
                            cx,
                            |app, window, cx| app.formula_file(true, window, cx),
                        ))
                    })
                    .child(self.button(
                        "new-formula",
                        self.tr("+ New formula"),
                        true,
                        cx,
                        |app, window, cx| {
                            app.edit_formula(Formula::blank(app.notebook.language), window, cx);
                        },
                    )),
            )
            .child(
                div()
                    .max_w(px(400.))
                    .child(self.entry(&self.formula_search)),
            )
            .child(
                div()
                    .id("formula-library")
                    .max_h(px(205.))
                    .overflow_y_scroll()
                    .child(row().gap_2().flex_wrap().children(library.map(|f| {
                        let id = f.id.clone();
                        self.button(
                            SharedString::from(format!("pick-formula-{id}")),
                            self.tr(&f.name),
                            self.formula_id == id,
                            cx,
                            move |app, window, cx| {
                                let values = app
                                    .formula_inputs
                                    .iter()
                                    .map(|input| input.read(cx).value().to_string())
                                    .collect();
                                app.formula_values.insert(app.formula_id.clone(), values);
                                app.formula_id.clone_from(&id);
                                app.reset_formula_inputs(window, cx);
                            },
                        )
                    }))),
            )
            .when_some(self.formula(), |c, formula| {
                let builtin = formulas::is_builtin(&formula.id);
                let run = formulas::run(
                    formula,
                    &self
                        .formula_inputs
                        .iter()
                        .map(|input| input.read(cx).value().to_string())
                        .collect::<Vec<_>>(),
                    rustcalc::engine::Context {
                        today: Some(rustcalc::dates::today()),
                        ..self.notebook.context()
                    },
                );
                let detail = p
                    .card()
                    .p_4()
                    .gap_3()
                    .child(
                        row()
                            .gap_2()
                            .flex_wrap()
                            .child(
                                div()
                                    .flex_1()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(self.tr(&formula.name)),
                            )
                            .child(self.button(
                                "edit-formula",
                                self.tr(if builtin { "Customize" } else { "Edit" }),
                                false,
                                cx,
                                move |app, window, cx| {
                                    if let Some(formula) = app.formula() {
                                        let draft = if builtin {
                                            formula.localized_copy(app.notebook.language)
                                        } else {
                                            formula.clone()
                                        };
                                        app.edit_formula(draft, window, cx);
                                    }
                                },
                            ))
                            .when(!builtin, |r| {
                                r.child(self.button(
                                    "delete-formula",
                                    self.tr("Delete"),
                                    false,
                                    cx,
                                    |app, window, cx| {
                                        let prompt = window.prompt(
                                            PromptLevel::Warning,
                                            &app.tr("Delete formula?"),
                                            None,
                                            &[
                                                PromptButton::ok(app.tr("Delete")),
                                                PromptButton::cancel(app.tr("Cancel")),
                                            ],
                                            cx,
                                        );
                                        let id = app.formula_id.clone();
                                        cx.spawn_in(window, async move |this, cx| {
                                            if prompt.await == Ok(0) {
                                                let _ = this.update_in(cx, |app, window, cx| {
                                                    app.notebook.formulas.retain(|f| f.id != id);
                                                    app.formula_id
                                                        .clone_from(&formulas::builtins()[0].id);
                                                    app.reset_formula_inputs(window, cx);
                                                    app.save();
                                                    cx.notify();
                                                });
                                            }
                                        })
                                        .detach();
                                    },
                                ))
                            }),
                    )
                    .when(!formula.description.is_empty(), |c| {
                        c.child(p.label(self.tr(&formula.description)))
                    })
                    .child(
                        row().items_start().gap_3().flex_wrap().children(
                            formula
                                .inputs
                                .iter()
                                .zip(&self.formula_inputs)
                                .zip(run.inputs)
                                .map(|((field, input), evaluated)| {
                                    column()
                                        .flex_1()
                                        .min_w(px(175.))
                                        .gap_1()
                                        .child(p.label(self.tr(&field.label)))
                                        .child(self.entry(input))
                                        .when_some(evaluated.err(), |c, error| {
                                            c.child(
                                                div()
                                                    .text_sm()
                                                    .text_color(p.danger)
                                                    .child(self.tr(error)),
                                            )
                                        })
                                }),
                        ),
                    )
                    .children(formula.outputs.iter().zip(run.outputs).enumerate().map(
                        |(i, (output, result))| {
                            let value = result.and_then(|value| {
                                formulas::format(
                                    value,
                                    output.format,
                                    self.notebook.language,
                                    self.notebook.numbers,
                                )
                            });
                            let result = match value {
                                Ok(value) => {
                                    let text = if output.unit.is_empty() {
                                        value
                                    } else {
                                        format!("{value} {}", self.tr(&output.unit))
                                    };
                                    let copy = text.clone();
                                    row()
                                        .gap_2()
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .text_color(p.accent)
                                                .text_size(rems(1.6))
                                                .child(text),
                                        )
                                        .child(self.button(
                                            ("copy-formula", i),
                                            self.tr("Copy"),
                                            false,
                                            cx,
                                            move |_, _, cx| {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    copy.clone(),
                                                ));
                                            },
                                        ))
                                }
                                Err(error) => row().text_color(p.danger).child(self.tr(error)),
                            };
                            column()
                                .gap_1()
                                .border_t_1()
                                .border_color(p.line)
                                .pt_3()
                                .child(p.label(self.tr(&output.label)))
                                .child(result)
                        },
                    ));
                c.child(detail)
            })
            .into_any_element()
    }

    fn edit_formula(&mut self, formula: Formula, window: &mut Window, cx: &mut Context<Self>) {
        if formula.id.is_empty() && self.notebook.formulas.len() >= formulas::MAX_FORMULAS {
            self.notice = Some("A formula file can contain at most 200 formulas".into());
            return;
        }
        let name = editor(&formula.name, "", window, cx);
        name.update(cx, |input, cx| input.focus(window, cx));
        self.draft_form = Some(DraftForm {
            id: formula.id,
            name,
            category: editor(&formula.category, "", window, cx),
            description: editor(&formula.description, "", window, cx),
            inputs: formula
                .inputs
                .into_iter()
                .map(|field| DraftInput::new(field, window, cx))
                .collect(),
            outputs: formula
                .outputs
                .into_iter()
                .map(|output| DraftOutput::new(output, window, cx))
                .collect(),
        });
    }

    fn formula_editor(&self, draft: &DraftForm, cx: &mut Context<Self>) -> Div {
        let p = self.palette;
        let field = |title, state| {
            column()
                .flex_1()
                .min_w_0()
                .gap_1()
                .child(p.label(self.tr(title)))
                .child(self.entry(state))
        };
        column()
            .gap_4()
            .child(
                row()
                    .gap_2()
                    .child(self.title("Formula editor"))
                    .child(div().flex_1())
                    .child(self.button(
                        "cancel-draft",
                        self.tr("Cancel"),
                        false,
                        cx,
                        |app, _, _| app.draft_form = None,
                    ))
                    .child(self.button(
                        "save-draft",
                        self.tr("Save"),
                        true,
                        cx,
                        |app, window, cx| app.save_formula(window, cx),
                    )),
            )
            .child(
                p.card()
                    .p_4()
                    .gap_3()
                    .child(field("Name", &draft.name))
                    .child(field("Category", &draft.category))
                    .child(field("Description", &draft.description)),
            )
            .child(p.label(self.tr("Inputs")))
            .children(draft.inputs.iter().enumerate().map(|(i, input)| {
                p.card()
                    .p_3()
                    .gap_2()
                    .child(
                        row()
                            .gap_2()
                            .child(field("Name", &input.name))
                            .child(field("Label", &input.label)),
                    )
                    .child(
                        row()
                            .gap_2()
                            .items_end()
                            .child(field("Default", &input.value))
                            .child(self.button(
                                ("kind", i),
                                self.tr(if input.kind == Kind::Date {
                                    "Date"
                                } else {
                                    "Number"
                                }),
                                false,
                                cx,
                                move |app, _, _| {
                                    if let Some(draft) = &mut app.draft_form {
                                        let kind = &mut draft.inputs[i].kind;
                                        *kind = if *kind == Kind::Date {
                                            Kind::Number
                                        } else {
                                            Kind::Date
                                        };
                                    }
                                },
                            ))
                            .child(self.button(
                                ("remove-input", i),
                                "×",
                                false,
                                cx,
                                move |app, _, _| {
                                    if let Some(draft) = &mut app.draft_form {
                                        draft.inputs.remove(i);
                                    }
                                },
                            )),
                    )
            }))
            .when(draft.inputs.len() < formulas::MAX_FIELDS, |c| {
                c.child(
                    self.button(
                        "add-input",
                        self.tr("+ Add input"),
                        false,
                        cx,
                        |app, window, cx| {
                            if let Some(draft) = &mut app.draft_form {
                                draft
                                    .inputs
                                    .push(DraftInput::new(Default::default(), window, cx));
                            }
                        },
                    )
                    .self_start(),
                )
            })
            .child(p.label(self.tr("Results")))
            .children(draft.outputs.iter().enumerate().map(|(i, output)| {
                p.card()
                    .p_3()
                    .gap_2()
                    .child(
                        row()
                            .gap_2()
                            .child(field("Name", &output.name))
                            .child(field("Label", &output.label)),
                    )
                    .child(
                        row()
                            .gap_2()
                            .child(field("Expression", &output.value))
                            .child(field("Unit", &output.unit)),
                    )
                    .child(
                        row()
                            .gap_2()
                            .children(Format::ALL.map(|format| {
                                self.button(
                                    ("format", i * Format::ALL.len() + format as usize),
                                    self.tr(format.title()),
                                    output.format == format,
                                    cx,
                                    move |app, _, _| {
                                        if let Some(draft) = &mut app.draft_form {
                                            draft.outputs[i].format = format;
                                        }
                                    },
                                )
                            }))
                            .child(div().flex_1())
                            .child(self.button(
                                ("remove-output", i),
                                "×",
                                false,
                                cx,
                                move |app, _, _| {
                                    if let Some(draft) = &mut app.draft_form {
                                        draft.outputs.remove(i);
                                    }
                                },
                            )),
                    )
            }))
            .when(draft.outputs.len() < formulas::MAX_FIELDS, |c| {
                c.child(
                    self.button(
                        "add-output",
                        self.tr("+ Add result"),
                        false,
                        cx,
                        |app, window, cx| {
                            if let Some(draft) = &mut app.draft_form {
                                draft.outputs.push(DraftOutput::new(
                                    Default::default(),
                                    window,
                                    cx,
                                ));
                            }
                        },
                    )
                    .self_start(),
                )
            })
    }

    fn save_formula(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(draft) = &self.draft_form else {
            return;
        };
        let value = |input: &Entity<InputState>| input.read(cx).value().trim().to_owned();
        let id = if draft.id.is_empty() {
            formulas::new_id(&self.notebook.formulas, super::unix_now())
        } else {
            draft.id.clone()
        };
        let formula = Formula {
            id,
            name: value(&draft.name),
            category: value(&draft.category),
            description: value(&draft.description),
            inputs: draft
                .inputs
                .iter()
                .map(|input| formulas::Field {
                    name: value(&input.name),
                    label: value(&input.label),
                    default: value(&input.value),
                    kind: input.kind,
                })
                .collect(),
            outputs: draft
                .outputs
                .iter()
                .map(|output| formulas::Output {
                    name: value(&output.name),
                    label: value(&output.label),
                    expression: value(&output.value),
                    unit: value(&output.unit),
                    format: output.format,
                })
                .collect(),
        };
        if let Err(error) = formulas::validate(&formula) {
            self.notice = Some(error);
            return;
        }
        self.formula_id.clone_from(&formula.id);
        self.formula_values.remove(&formula.id);
        if let Some(old) = self
            .notebook
            .formulas
            .iter_mut()
            .find(|old| old.id == formula.id)
        {
            *old = formula;
        } else {
            self.notebook.formulas.push(formula);
        }
        self.draft_form = None;
        self.reset_formula_inputs(window, cx);
        self.save();
    }

    fn formula_file(&mut self, export: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.file_busy {
            return;
        }
        self.file_busy = true;
        let formulas = self.notebook.formulas.clone();
        let task = cx.background_executor().spawn(async move {
            let dialog = rfd::FileDialog::new().add_filter("JSON", &["json"]);
            if export {
                if let Some(path) = dialog.set_file_name("rustcalc-formulas.json").save_file() {
                    formulas::write_file(&path, &formulas)?;
                }
                return Ok(None);
            }
            dialog
                .pick_file()
                .map(|path| formulas::read_file(&path))
                .transpose()
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |app, _, cx| {
                app.file_busy = false;
                match result {
                    Ok(Some(incoming)) => {
                        let mut candidate = app.notebook.formulas.clone();
                        match formulas::merge(&mut candidate, incoming, super::unix_now()) {
                            Ok(_) => {
                                app.notebook.formulas = candidate;
                                app.save();
                            }
                            Err(error) => app.notice = Some(error),
                        }
                    }
                    Err(error) => app.notice = Some(error),
                    Ok(None) => {}
                }
                cx.notify();
            });
        })
        .detach();
    }
}
