use super::{
    app::{Calc, Panel},
    theme::*,
};
use gpui::{prelude::*, *};
use gpui_component::input::Backspace;
use rustcalc::{
    dates,
    engine::{self, Angle, Value},
    history, i18n, model,
};

impl Calc {
    pub fn preview(&self, cx: &App) -> Result<Value, engine::CalcError> {
        let input = self.expression.read(cx).value();
        if self.committed.as_deref() == Some(input.as_ref()) || input.trim().is_empty() {
            let context = self.notebook.context();
            return Ok(Value {
                number: context.ans,
                approximate: context.ans_approximate,
            });
        }
        self.notebook.evaluate(&input).map(|(_, value)| value)
    }

    pub fn calculate(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = self.expression.read(cx).value();
        if input.trim().is_empty() || self.committed.as_deref() == Some(input.as_ref()) {
            return;
        }
        if let Err(error) = self.notebook.calculate(&input) {
            self.notice = Some(error.message);
        } else {
            let completed = model::complete_brackets(&input).into_owned();
            if completed != input.as_ref() {
                Self::set(&self.expression, completed.clone(), window, cx);
            }
            self.committed = Some(completed);
            self.save();
        }
        cx.notify();
    }

    pub fn focus_expression(&self, window: &mut Window, cx: &mut App) {
        self.expression
            .update(cx, |input, cx| input.focus(window, cx));
    }

    pub fn insert(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let old = self.expression.read(cx).value();
        let continuation = ["+", "-", "*", "/", "^", "%", "!", "^2", "×", "÷", "−"].contains(&text);
        let start = self.committed.take().is_some()
            || (old.is_empty() && !self.notebook.answer.is_empty() && continuation);
        self.expression.update(cx, |input, cx| {
            if start {
                input.set_value(if continuation { "ans" } else { "" }, window, cx);
            }
            input.replace(text.to_owned(), window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    }

    pub fn clear_expression(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.committed = None;
        Self::set(&self.expression, "", window, cx);
        self.focus_expression(window, cx);
    }

    pub fn copy_result(&mut self, cx: &mut Context<Self>) {
        if let Ok(value) = self.preview(cx) {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.notebook.copy_text(&value.exact_text()),
            ));
            self.notice = Some(self.tr("Result copied"));
        }
    }

    fn calc_action(&mut self, action: &str, window: &mut Window, cx: &mut Context<Self>) {
        match action {
            "equals" => {
                self.calculate(window, cx);
                self.focus_expression(window, cx);
            }
            "clear" => self.clear_expression(window, cx),
            "backspace" => {
                self.committed = None;
                self.focus_expression(window, cx);
                window.dispatch_action(Box::new(Backspace), cx);
            }
            "negate" | "reciprocal" => {
                let old = self.expression.read(cx).value();
                let expression = if self.committed.is_some() {
                    "ans"
                } else {
                    old.as_ref()
                };
                if !expression.is_empty() {
                    let prefix = if action == "negate" { "-" } else { "1/" };
                    self.committed = None;
                    Self::set(
                        &self.expression,
                        format!("{prefix}({expression})"),
                        window,
                        cx,
                    );
                    self.focus_expression(window, cx);
                }
            }
            "MC" => {
                self.notebook.memory.clear();
                self.notebook.memory_approximate = false;
            }
            "MR" => self.insert("mem", window, cx),
            "MS" | "M+" | "M−" => {
                if let Ok(value) = self.preview(cx) {
                    if action == "MS" {
                        self.notebook.memory = value.exact_text();
                        self.notebook.memory_approximate = value.approximate;
                    } else if let Err(error) = self.notebook.memory_add(value, action == "M−") {
                        self.notice = Some(error);
                    }
                }
            }
            _ => self.insert(action, window, cx),
        }
    }

    pub fn calculator(
        &self,
        history_inline: bool,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = self.palette;
        let zoom = self.notebook.zoom();
        let preview = self.preview(cx);
        let result = preview
            .as_ref()
            .map(|value| {
                format!(
                    "{}{}",
                    if value.approximate { "≈ " } else { "" },
                    self.notebook.show(&value.text())
                )
            })
            .unwrap_or_else(|_| "—".into());
        let digits = result.chars().count();
        let result_size: f32 = match digits {
            0..=11 => 3.35,
            12..=22 => 2.35,
            _ => 1.55,
        };
        let fraction = preview
            .as_ref()
            .ok()
            .and_then(|value| engine::fraction(*value, 1000))
            .filter(|(_, denominator)| *denominator != 1)
            .map(|(n, d)| format!("= {n}/{d}"));
        let error = preview.err().map(|error| self.tr(error.message));
        let decimal = if self.notebook.numbers.comma(self.notebook.language) {
            ","
        } else {
            "."
        };
        let keypad = [
            [
                ("AC", "clear"),
                ("(", "("),
                (")", ")"),
                ("⌫", "backspace"),
                ("÷", "/"),
            ],
            [("7", "7"), ("8", "8"), ("9", "9"), ("%", "%"), ("×", "*")],
            [("4", "4"), ("5", "5"), ("6", "6"), ("x²", "^2"), ("−", "-")],
            [
                ("1", "1"),
                ("2", "2"),
                ("3", "3"),
                ("√", "sqrt("),
                ("+", "+"),
            ],
            [
                ("±", "negate"),
                ("0", "0"),
                (decimal, decimal),
                ("ans", "ans"),
                ("=", "equals"),
            ],
        ];
        let scientific = [
            [
                ("sin", "sin("),
                ("cos", "cos("),
                ("tan", "tan("),
                ("ln", "ln("),
                ("log", "log("),
            ],
            [
                ("asin", "asin("),
                ("acos", "acos("),
                ("atan", "atan("),
                ("exp", "exp("),
                ("abs", "abs("),
            ],
            [
                ("π", "pi"),
                ("e", "e"),
                ("xʸ", "^"),
                ("x!", "!"),
                ("1/x", "reciprocal"),
            ],
        ];
        let key_rows = |rows: &[[(&'static str, &'static str); 5]],
                        prefix: &'static str,
                        cx: &mut Context<Self>| {
            rows.iter()
                .enumerate()
                .map(|(i, keys)| {
                    row()
                        .gap_2()
                        .children(keys.iter().enumerate().map(|(j, &(label, action))| {
                            let primary = action == "equals";
                            let number = label.as_bytes().first().is_some_and(u8::is_ascii_digit);
                            self.button(
                                (prefix, i * 5 + j),
                                label,
                                false,
                                cx,
                                move |app, window, cx| app.calc_action(action, window, cx),
                            )
                            .debug_selector(|| action.to_owned())
                            .flex_1()
                            .min_w_0()
                            .h(px(match (compact, prefix) {
                                (true, "key") => 42.,
                                (true, _) => 32.,
                                (false, "key") => 52.,
                                (false, _) => 39.,
                            } * zoom))
                            .when(compact, |b| b.py_1())
                            .px_0()
                            .text_size(rems(if prefix == "key" { 1.35 } else { 1. }))
                            .when(!number, |b| b.text_color(p.accent))
                            .when(primary, |b| {
                                b.bg(p.accent)
                                    .text_color(p.accent_text)
                                    .font_weight(FontWeight::BOLD)
                            })
                        }))
                })
                .collect::<Vec<_>>()
        };
        column()
            .gap_4()
            .when(compact, |c| c.gap_2())
            .child(
                row()
                    .gap_2()
                    .child(self.title("Calculator"))
                    .child(div().flex_1())
                    .when(!history_inline, |r| {
                        r.child(self.button(
                            "show-history",
                            self.tr("History"),
                            false,
                            cx,
                            |app, window, cx| app.open_panel(Panel::History, window, cx),
                        ))
                    }),
            )
            .child(
                p.card()
                    .p_4()
                    .gap_3()
                    .when(compact, |c| c.p_3().gap_2())
                    .child(
                        row()
                            .gap_2()
                            .child(p.label(self.tr("Expression")))
                            .child(div().flex_1())
                            .children([(Angle::Degrees, "DEG"), (Angle::Radians, "RAD")].map(
                                |(angle, label)| {
                                    self.button(
                                        label,
                                        label,
                                        self.notebook.angle == angle,
                                        cx,
                                        move |app, window, cx| {
                                            app.notebook.angle = angle;
                                            app.focus_expression(window, cx);
                                        },
                                    )
                                    .text_xs()
                                    .py_1()
                                    .px_2()
                                },
                            )),
                    )
                    .child(self.field(&self.expression).text_size(rems(1.48)).w_full())
                    .child(
                        row()
                            .border_t_1()
                            .border_color(p.line)
                            .pt_3()
                            .gap_2()
                            .child(
                                column()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_1()
                                    .child(p.label(self.tr("Result")))
                                    .child(
                                        div()
                                            .text_size(rems(if compact {
                                                result_size.min(2.65)
                                            } else {
                                                result_size
                                            }))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(p.accent)
                                            .child(result),
                                    )
                                    .when_some(fraction, |c, value| c.child(p.label(value))),
                            )
                            .child(self.button(
                                "copy-result",
                                self.tr("Copy"),
                                false,
                                cx,
                                |app, _, cx| app.copy_result(cx),
                            )),
                    )
                    .when_some(error, |c, error| {
                        c.child(div().text_sm().text_color(p.danger).child(error))
                    }),
            )
            .child(
                row()
                    .gap_2()
                    .flex_wrap()
                    .children(
                        [("Standard", false), ("Scientific", true)].map(|(label, mode)| {
                            self.button(
                                label,
                                self.tr(label),
                                self.notebook.scientific == mode,
                                cx,
                                move |app, window, cx| {
                                    app.notebook.scientific = mode;
                                    app.focus_expression(window, cx);
                                },
                            )
                        }),
                    )
                    .child(div().flex_1())
                    .children(["MC", "MR", "MS", "M+", "M−"].map(|label| {
                        self.button(label, label, false, cx, move |app, window, cx| {
                            app.calc_action(label, window, cx)
                        })
                        .text_xs()
                        .px_2()
                    })),
            )
            .when(self.notebook.scientific, |c| {
                c.children(key_rows(&scientific, "scientific", cx))
            })
            .children(key_rows(&keypad, "key", cx))
            .when(!self.notebook.memory.is_empty(), |c| {
                c.child(p.label(format!(
                        "M  {}",
                        self.notebook
                            .show_value(&self.notebook.memory, self.notebook.memory_approximate)
                    )))
            })
            .when(!self.notebook.variables.is_empty(), |c| {
                c.child(
                    row()
                        .gap_2()
                        .flex_wrap()
                        .children(self.notebook.variables.iter().map(|(name, value)| {
                            let insert = name.clone();
                            let remove = name.clone();
                            row()
                                .gap_1()
                                .child(self.button(
                                    SharedString::from(format!("var-{name}")),
                                    format!(
                                        "{name} = {}",
                                        self.notebook.show_value(&value.value, value.approximate)
                                    ),
                                    false,
                                    cx,
                                    move |app, window, cx| app.insert(&insert, window, cx),
                                ))
                                .child(self.button(
                                    SharedString::from(format!("remove-var-{name}")),
                                    "×",
                                    false,
                                    cx,
                                    move |app, _, _| {
                                        app.notebook.variables.remove(&remove);
                                    },
                                ))
                        })),
                )
            })
            .into_any_element()
    }

    pub fn history(&self, cx: &mut Context<Self>) -> Div {
        let p = self.palette;
        let query = i18n::fold(&self.search.read(cx).value());
        let offset = dates::local_offset_seconds();
        let mut previous = None;
        let mut entries = Vec::new();
        for (index, entry) in self.notebook.entries.iter().enumerate().rev() {
            let result = self.notebook.show(&entry.result);
            if !query.is_empty()
                && !i18n::fold(&format!("{} {} {result}", entry.expression, entry.result))
                    .contains(&query)
            {
                continue;
            }
            let day = dates::local_day(entry.timestamp, offset);
            if previous != Some(day) {
                entries.push(
                    p.label(dates::format(
                        day,
                        self.notebook.language == i18n::Language::Turkish,
                    ))
                    .pt_2()
                    .into_any_element(),
                );
                previous = Some(day);
            }
            let expression = entry.expression.clone();
            let copy = self.notebook.copy_text(&entry.result);
            let angle = entry.angle;
            entries.push(
                p.card()
                    .p_2()
                    .gap_2()
                    .child(
                        self.button(
                            ("reuse", index),
                            expression.clone(),
                            false,
                            cx,
                            move |app, window, cx| {
                                app.open_page(super::app::Page::Calculator, window, cx);
                                app.committed = None;
                                app.notebook.angle = angle;
                                Self::set(&app.expression, expression.clone(), window, cx);
                            },
                        )
                        .justify_start()
                        .min_w_0()
                        .overflow_hidden()
                        .text_sm(),
                    )
                    .child(
                        row()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_color(p.accent)
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(format!(
                                        "{}{}",
                                        if entry.approximate { "≈ " } else { "" },
                                        result
                                    )),
                            )
                            .child(
                                self.button(
                                    ("pin", index),
                                    self.tr(if entry.pinned { "Unpin" } else { "Pin" }),
                                    entry.pinned,
                                    cx,
                                    move |app, _, _| {
                                        if let Some(entry) = app.notebook.entries.get_mut(index) {
                                            entry.pinned = !entry.pinned;
                                        }
                                    },
                                )
                                .text_xs()
                                .px_1()
                                .py_1(),
                            )
                            .child(
                                self.button(
                                    ("copy-entry", index),
                                    self.tr("Copy"),
                                    false,
                                    cx,
                                    move |_, _, cx| {
                                        cx.write_to_clipboard(ClipboardItem::new_string(
                                            copy.clone(),
                                        ))
                                    },
                                )
                                .text_xs()
                                .px_1()
                                .py_1(),
                            )
                            .child(
                                self.button(
                                    ("delete-entry", index),
                                    "×",
                                    false,
                                    cx,
                                    move |app, _, _| {
                                        app.notebook.delete_history(index);
                                    },
                                )
                                .text_xs()
                                .px_1()
                                .py_1(),
                            ),
                    )
                    .into_any_element(),
            );
        }
        column()
            .gap_3()
            .child(
                row()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.tr("History")),
                    )
                    .when(self.panel == Panel::History, |r| {
                        r.child(self.button(
                            "close-history",
                            self.tr("Back"),
                            false,
                            cx,
                            |app, window, cx| app.open_panel(Panel::History, window, cx),
                        ))
                    }),
            )
            .child(self.entry(&self.search))
            .child(
                row()
                    .gap_2()
                    .flex_wrap()
                    .children(
                        [
                            ("Import", HistoryFile::Import),
                            ("Backup", HistoryFile::Backup),
                            ("Export CSV", HistoryFile::Csv),
                        ]
                        .map(|(label, action)| {
                            self.button(label, self.tr(label), false, cx, move |app, window, cx| {
                                app.history_file(action, window, cx)
                            })
                            .text_xs()
                            .px_2()
                        }),
                    )
                    .when(self.notebook.can_undo_history(), |r| {
                        r.child(
                            self.button("undo-history", self.tr("Undo"), false, cx, |app, _, _| {
                                if let Err(error) = app.notebook.undo_history() {
                                    app.notice = Some(error);
                                }
                            })
                            .text_xs()
                            .px_2(),
                        )
                    })
                    .when(
                        self.notebook.entries.iter().any(|entry| !entry.pinned),
                        |r| {
                            r.child(
                                self.button(
                                    "clear-history",
                                    self.tr("Clear"),
                                    false,
                                    cx,
                                    |app, window, cx| {
                                        let prompt = window.prompt(
                                            PromptLevel::Warning,
                                            &app.tr("Clear history?"),
                                            Some(&app.tr("Pinned entries will stay.")),
                                            &[
                                                PromptButton::ok(app.tr("Clear")),
                                                PromptButton::cancel(app.tr("Cancel")),
                                            ],
                                            cx,
                                        );
                                        cx.spawn_in(window, async move |this, cx| {
                                            if prompt.await == Ok(0) {
                                                let _ = this.update_in(cx, |app, _, cx| {
                                                    app.notebook.clear_unpinned_history();
                                                    app.save();
                                                    cx.notify();
                                                });
                                            }
                                        })
                                        .detach();
                                    },
                                )
                                .text_xs()
                                .px_2(),
                            )
                        },
                    ),
            )
            .when_some(self.pending_history.as_ref(), |c, incoming| {
                c.child(
                    p.card()
                        .p_3()
                        .gap_2()
                        .child(format!(
                            "{} ({})",
                            self.tr("Import history"),
                            incoming.len()
                        ))
                        .child(
                            row()
                                .gap_2()
                                .child(self.button(
                                    "confirm-import",
                                    self.tr("Import"),
                                    true,
                                    cx,
                                    |app, _, _| {
                                        if let Some(entries) = app.pending_history.take() {
                                            match app.notebook.import_history(&entries) {
                                                Ok(count) => {
                                                    app.notice = Some(format!(
                                                        "{} · {count}",
                                                        app.tr("History imported")
                                                    ))
                                                }
                                                Err(error) => app.notice = Some(error),
                                            }
                                        }
                                    },
                                ))
                                .child(self.button(
                                    "cancel-import",
                                    self.tr("Cancel"),
                                    false,
                                    cx,
                                    |app, _, _| app.pending_history = None,
                                )),
                        ),
                )
            })
            .when(entries.is_empty(), |c| {
                c.child(
                    p.label(self.tr(if query.is_empty() {
                        "No calculations yet"
                    } else {
                        "No matching calculations"
                    }))
                    .py_4(),
                )
            })
            .children(entries)
    }

    fn history_file(&mut self, action: HistoryFile, window: &mut Window, cx: &mut Context<Self>) {
        if self.file_busy || self.pending_history.is_some() {
            return;
        }
        self.file_busy = true;
        let output = match action {
            HistoryFile::Import => Ok(Vec::new()),
            HistoryFile::Backup => history::encode(&self.notebook.entries),
            HistoryFile::Csv => Ok(self.notebook.export().into_bytes()),
        };
        let title = self.tr(match action {
            HistoryFile::Import => "Import history",
            HistoryFile::Backup => "Back up history",
            HistoryFile::Csv => "Export CSV",
        });
        let task = cx.background_executor().spawn(async move {
            let output = output?;
            let dialog = rfd::FileDialog::new().set_title(title);
            if matches!(action, HistoryFile::Import) {
                return dialog
                    .add_filter("JSON", &["json"])
                    .pick_file()
                    .map(|path| history::read_backup(&path))
                    .transpose();
            }
            let (ext, name) = match action {
                HistoryFile::Csv => ("csv", "rustcalc-history.csv"),
                _ => ("json", "rustcalc-history.json"),
            };
            if let Some(path) = dialog
                .add_filter(ext, &[ext])
                .set_file_name(name)
                .save_file()
            {
                history::write_file(&path, &output)?;
            }
            Ok(None)
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |app, _, cx| {
                app.file_busy = false;
                match result {
                    Ok(incoming) => app.pending_history = incoming,
                    Err(error) => app.notice = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }
}

#[derive(Clone, Copy)]
enum HistoryFile {
    Import,
    Backup,
    Csv,
}
