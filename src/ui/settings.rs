use super::{
    app::{Calc, Panel},
    shortcuts::{self, Command},
    theme::*,
};
use gpui::{prelude::*, *};
use rustcalc::{
    i18n::Language,
    model::ZOOM_STEPS,
    numfmt::DecimalMark,
    updater::{self, AutoUpdate, UpdatePlan},
};

pub(super) enum Update {
    Idle,
    Checking,
    Current,
    Available(Box<UpdatePlan>),
    Installing,
    Installed,
    RolledBack(String),
    Failed(String),
}

enum Job {
    Check { automatic: bool },
    Install(Box<UpdatePlan>),
    Rollback,
}

impl Calc {
    pub fn settings(&self, cx: &mut Context<Self>) -> Div {
        let p = self.palette;
        let style = self.notebook.numbers;
        let theme = if self.notebook.follow_system_theme {
            0
        } else if self.notebook.high_contrast {
            3
        } else if self.notebook.light {
            2
        } else {
            1
        };
        column()
            .gap_4()
            .child(self.title("Settings"))
            .child(
                p.card()
                    .gap_3()
                    .p_4()
                    .child(p.label(self.tr("Language")))
                    .child(
                        row().gap_2().children(
                            [
                                (Language::English, "English"),
                                (Language::Turkish, "Türkçe"),
                            ]
                            .map(|(language, label)| {
                                self.button(
                                    label,
                                    label,
                                    self.notebook.language == language,
                                    cx,
                                    move |app, window, cx| {
                                        app.notebook.language = language;
                                        app.update_palette(window, cx);
                                        app.save();
                                    },
                                )
                            }),
                        ),
                    )
                    .child(p.label(self.tr("Appearance")))
                    .child(
                        row().gap_2().flex_wrap().children(
                            ["System", "Dark", "Light", "High contrast"]
                                .into_iter()
                                .enumerate()
                                .map(|(i, label)| {
                                    self.button(
                                        label,
                                        self.tr(label),
                                        i == theme,
                                        cx,
                                        move |app, window, cx| {
                                            app.notebook.follow_system_theme = i == 0;
                                            app.notebook.light = i == 2;
                                            app.notebook.high_contrast = i == 3;
                                            app.update_palette(window, cx);
                                            app.save();
                                        },
                                    )
                                }),
                        ),
                    )
                    .child(p.label(self.tr("Text size")))
                    .child(row().gap_2().flex_wrap().children(
                        ZOOM_STEPS.into_iter().enumerate().map(|(i, zoom)| {
                            self.button(
                                ("zoom", i),
                                format!("{}%", (zoom * 100.).round()),
                                (zoom - self.notebook.zoom()).abs() < 0.001,
                                cx,
                                move |app, _, _| {
                                    app.notebook.zoom = zoom;
                                    app.save();
                                },
                            )
                        }),
                    )),
            )
            .child(
                p.card()
                    .gap_3()
                    .p_4()
                    .child(p.label(self.tr("Decimal mark")))
                    .child(
                        row().gap_2().flex_wrap().children(
                            [
                                (DecimalMark::Automatic, "Automatic"),
                                (DecimalMark::Point, "Point (.)"),
                                (DecimalMark::Comma, "Comma (,)"),
                            ]
                            .map(|(mark, label)| {
                                self.button(
                                    label,
                                    self.tr(label),
                                    style.mark == mark,
                                    cx,
                                    move |app, _, _| app.notebook.numbers.mark = mark,
                                )
                            }),
                        ),
                    )
                    .child(
                        self.button(
                            "grouping",
                            self.tr("Group digits"),
                            style.grouping,
                            cx,
                            |app, _, _| {
                                app.notebook.numbers.grouping = !app.notebook.numbers.grouping;
                            },
                        )
                        .self_start(),
                    )
                    .child(p.label(self.tr("Decimal places")))
                    .child(
                        row()
                            .gap_2()
                            .flex_wrap()
                            .child(self.button(
                                "all-decimals",
                                self.tr("All digits"),
                                style.decimals.is_none(),
                                cx,
                                |app, _, _| app.notebook.numbers.decimals = None,
                            ))
                            .children((0..=10).map(|places| {
                                self.button(
                                    ("decimals", places as usize),
                                    places.to_string(),
                                    style.decimals == Some(places),
                                    cx,
                                    move |app, _, _| app.notebook.numbers.decimals = Some(places),
                                )
                                .px_2()
                            })),
                    ),
            )
            .child(
                p.card()
                    .gap_3()
                    .p_4()
                    .child(
                        row()
                            .justify_between()
                            .gap_2()
                            .child(p.label(self.tr("Keyboard shortcuts")))
                            .child(self.button(
                                "reset-shortcuts",
                                self.tr("Reset shortcuts"),
                                false,
                                cx,
                                |app, _, _| {
                                    app.notebook.shortcuts.clear();
                                    app.shortcut_capture = None;
                                },
                            )),
                    )
                    .children(Command::ALL.map(|command| {
                        let display = if self.shortcut_capture == Some(command) {
                            self.tr("Press a shortcut…")
                        } else {
                            shortcuts::binding(&self.notebook.shortcuts, command)
                                .map_or_else(|| self.tr("Unassigned"), shortcuts::display)
                        };
                        row()
                            .gap_2()
                            .child(div().flex_1().child(self.tr(command.title())))
                            .child(self.button(
                                SharedString::from(format!("record-{}", command.id())),
                                display,
                                self.shortcut_capture == Some(command),
                                cx,
                                move |app, window, _| {
                                    app.shortcut_capture = Some(command);
                                    window.focus(&app.focus);
                                },
                            ))
                            .child(self.button(
                                SharedString::from(format!("remove-{}", command.id())),
                                self.tr("Remove"),
                                false,
                                cx,
                                move |app, _, _| {
                                    shortcuts::assign(&mut app.notebook.shortcuts, command, None);
                                    app.shortcut_capture = None;
                                },
                            ))
                    })),
            )
    }

    pub fn help(&self, cx: &mut Context<Self>) -> Div {
        let p = self.palette;
        column()
            .gap_4()
            .child(self.title("Help"))
            .child(
                p.card()
                    .p_4()
                    .gap_3()
                    .children(
                        [
                            ("1250 * 12", "Press Enter or = to calculate."),
                            ("ans + 20%", "Use ans for the previous result."),
                            ("sqrt(144)", "A number next to a name or bracket multiplies"),
                            ("round(1.234; 2)", "Use ; between function arguments."),
                            (
                                "vat = 20",
                                "Name a value, then reuse it in later expressions.",
                            ),
                        ]
                        .map(|(example, tip)| {
                            row()
                                .gap_3()
                                .flex_wrap()
                                .child(div().w(px(175.)).text_color(p.accent).child(example))
                                .child(p.label(self.tr(tip)))
                        }),
                    )
                    .child(
                        self.button(
                            "open-guide",
                            self.tr("User guide"),
                            true,
                            cx,
                            |app, _, cx| {
                                let file = if app.notebook.language == Language::Turkish {
                                    "KULLANIM_KILAVUZU"
                                } else {
                                    "USER_GUIDE"
                                };
                                cx.open_url(&format!(
                                    "https://github.com/ersinkoc/rustcalc/blob/main/docs/{file}.md"
                                ));
                            },
                        )
                        .self_start(),
                    ),
            )
            .child(
                p.card()
                    .p_4()
                    .gap_2()
                    .child(p.label(self.tr("Keyboard shortcuts")))
                    .children(Command::ALL.into_iter().filter_map(|command| {
                        shortcuts::binding(&self.notebook.shortcuts, command).map(|key| {
                            row()
                                .gap_3()
                                .justify_between()
                                .child(self.tr(command.title()))
                                .child(p.label(shortcuts::display(key)))
                        })
                    })),
            )
    }

    fn update_modes(&self, cx: &mut Context<Self>) -> Div {
        row().gap_2().flex_wrap().children(
            [
                (AutoUpdate::Off, "Off"),
                (AutoUpdate::Notify, "Notify me"),
                (AutoUpdate::Install, "Install automatically"),
            ]
            .map(|(mode, label)| {
                self.button(
                    label,
                    self.tr(label),
                    self.notebook.update_prompted && self.notebook.auto_update == mode,
                    cx,
                    move |app, window, cx| {
                        app.notebook.update_prompted = true;
                        app.notebook.auto_update = mode;
                        app.save();
                        app.auto_update(window, cx);
                    },
                )
            }),
        )
    }

    pub fn update_banner(&self, cx: &mut Context<Self>) -> Option<Div> {
        let p = self.palette;
        if self.isolated || self.panel == Panel::Updates {
            return None;
        }
        if !self.notebook.update_prompted {
            return Some(
                column()
                    .gap_2()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(p.line)
                    .child(p.label(self.tr("Keep RustCalc up to date automatically?")))
                    .child(self.update_modes(cx)),
            );
        }
        let (message, action, restart) = match &self.update {
            Update::Available(plan) => (
                self.tr(format!("RustCalc update available: {}", plan.version)),
                "Details",
                false,
            ),
            Update::Installed => (
                self.tr("Update installed. Restart RustCalc to use it."),
                "Restart",
                true,
            ),
            Update::RolledBack(version) => (
                format!("{} · {version}", self.tr("Rollback ready")),
                "Restart",
                true,
            ),
            _ => return None,
        };
        Some(
            row()
                .gap_2()
                .px_4()
                .py_2()
                .bg(p.selected)
                .child(div().flex_1().child(message))
                .child(self.button(
                    "update-banner",
                    self.tr(action),
                    true,
                    cx,
                    move |app, window, cx| {
                        if restart {
                            app.restart(cx);
                        } else {
                            app.open_panel(Panel::Updates, window, cx);
                        }
                    },
                )),
        )
    }

    pub fn updates(&self, cx: &mut Context<Self>) -> Div {
        let p = self.palette;
        let status = match &self.update {
            Update::Idle => None,
            Update::Checking => Some(self.tr("Checking…")),
            Update::Current => Some(self.tr("You are using the latest version.")),
            Update::Available(plan) => {
                Some(format!("{} → {}", env!("CARGO_PKG_VERSION"), plan.version))
            }
            Update::Installing => Some(self.tr("Downloading and verifying…")),
            Update::Installed => Some(self.tr("Update installed. Restart RustCalc to use it.")),
            Update::RolledBack(version) => {
                Some(format!("{} · {version}", self.tr("Rollback ready")))
            }
            Update::Failed(error) => Some(self.tr(error)),
        };
        let can_check = matches!(
            self.update,
            Update::Idle | Update::Current | Update::Failed(_) | Update::Available(_)
        );
        let can_rollback = can_check
            && updater::running_executable()
                .is_ok_and(|exe| updater::previous_path(&exe).is_file());
        column().gap_4().child(self.title("About & updates"))
            .child(p.card().p_4().gap_3()
                .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(format!("RustCalc {}", env!("CARGO_PKG_VERSION"))))
                .child(p.label(self.tr("Connects to GitHub only for update checks you request or enable. Your calculations are never sent.")))
                .child(self.button("release-page", self.tr("Releases"), false, cx,
                    |_, _, cx| cx.open_url(updater::RELEASES_URL)).self_start())
                .child(p.label(self.tr("Automatic updates")))
                .child(self.update_modes(cx))
                .child(p.label(self.tr("Checks at most once a day while RustCalc is open. Updates are verified before installation and take effect after a restart."))))
            .child(p.card().p_4().gap_3()
                .when_some(status, |c, text| c.child(div().text_color(if matches!(self.update, Update::Failed(_)) { p.danger } else { p.accent }).child(text)))
                .child(row().gap_2().flex_wrap()
                    .when(can_check, |r| r.child(self.button("check-updates", self.tr("Check for updates"), true, cx, |app, window, cx| {
                        app.update_job(Job::Check { automatic: false }, window, cx);
                    })))
                    .when_some(if let Update::Available(plan) = &self.update { Some(plan.clone()) } else { None },
                        |r, plan| {
                            let url = format!("{}/tag/{}", updater::RELEASES_URL, plan.tag);
                            r.child(self.button("install-update", self.tr("Install update"), true, cx, move |app, window, cx| {
                                app.update_job(Job::Install(plan.clone()), window, cx);
                            })).child(self.button("release-notes", self.tr("Release notes"), false, cx, move |_, _, cx| cx.open_url(&url)))
                        })
                    .when(can_rollback, |r| r.child(self.button("rollback", self.tr("Restore previous version"), false, cx, |app, window, cx| {
                        app.update_job(Job::Rollback, window, cx);
                    })))
                    .when(matches!(self.update, Update::Installed | Update::RolledBack(_)), |r| r.child(
                        self.button("restart", self.tr("Restart"), true, cx, |app, _, cx| app.restart(cx))))))
    }

    pub fn auto_update(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = super::unix_now();
        if self.isolated
            || updater::PLATFORM_ASSET.is_none()
            || !matches!(
                self.update,
                Update::Idle | Update::Current | Update::Failed(_)
            )
            || !updater::auto_check_due(
                self.notebook.auto_update,
                self.notebook.last_update_check,
                now,
            )
        {
            return;
        }
        self.notebook.last_update_check = now;
        self.save();
        self.update_job(Job::Check { automatic: true }, window, cx);
    }

    fn update_job(&mut self, job: Job, window: &mut Window, cx: &mut Context<Self>) {
        enum Outcome {
            Checked(Option<Box<UpdatePlan>>, bool),
            Installed,
            RolledBack(String),
        }
        if matches!(
            self.update,
            Update::Checking | Update::Installing | Update::Installed | Update::RolledBack(_)
        ) {
            return;
        }
        self.update = if matches!(job, Job::Check { .. }) {
            Update::Checking
        } else {
            Update::Installing
        };
        let task = cx.background_executor().spawn(async move {
            match job {
                Job::Check { automatic } => {
                    updater::check().map(|plan| Outcome::Checked(plan.map(Box::new), automatic))
                }
                Job::Install(plan) => updater::install(&plan).map(|()| Outcome::Installed),
                Job::Rollback => updater::rollback().map(Outcome::RolledBack),
            }
        });
        cx.spawn_in(window, async move |this, cx| {
            let outcome = task.await;
            let _ = this.update_in(cx, |app, window, cx| {
                app.update = match outcome {
                    Ok(Outcome::Checked(_, true))
                        if app.notebook.auto_update == AutoUpdate::Off =>
                    {
                        Update::Idle
                    }
                    Ok(Outcome::Checked(Some(plan), automatic)) => {
                        app.update = Update::Current;
                        if automatic && app.notebook.auto_update == AutoUpdate::Install {
                            app.update_job(Job::Install(plan), window, cx);
                            cx.notify();
                            return;
                        }
                        Update::Available(plan)
                    }
                    Ok(Outcome::Checked(None, _)) => Update::Current,
                    Ok(Outcome::Installed) => Update::Installed,
                    Ok(Outcome::RolledBack(version)) => Update::RolledBack(version),
                    Err(error) => Update::Failed(error),
                };
                cx.notify();
            });
        })
        .detach();
    }

    fn restart(&mut self, cx: &mut App) {
        self.save();
        crate::RESTART.store(true, std::sync::atomic::Ordering::SeqCst);
        cx.quit();
    }
}
