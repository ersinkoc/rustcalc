use super::{app::Calc, theme::*};
use gpui::{prelude::*, *};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rustcalc::{
    engine::{Angle, Expression},
    plot,
    programmer::{self, Op},
    units,
};

impl Calc {
    pub fn converter(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.palette;
        let [category, from, to] = self.converter;
        let units = units::CATEGORIES[category].units;
        let input = self.convert.read(cx).value();
        let result = self
            .notebook
            .evaluate(&input)
            .map(|(_, value)| value.number)
            .map_err(|error| error.message);
        let converted = result
            .clone()
            .and_then(|value| units::convert(value, category, from, to));
        column()
            .gap_4()
            .child(self.title("Convert units"))
            .child(
                row()
                    .gap_2()
                    .flex_wrap()
                    .children(units::CATEGORIES.iter().enumerate().map(|(i, category)| {
                        self.button(
                            ("category", i),
                            self.tr(category.name),
                            self.converter[0] == i,
                            cx,
                            move |app, _, _| {
                                app.converter = [i, 0, 1];
                                app.notebook.converter = Some(app.converter);
                                app.unit_menu = None;
                            },
                        )
                    })),
            )
            .child(
                p.card()
                    .p_5()
                    .gap_4()
                    .child(p.label(self.tr("From")))
                    .child(self.entry(&self.convert))
                    .child(self.unit_button(1, units[from], cx))
                    .when(self.unit_menu == Some(1), |c| {
                        c.child(self.unit_choices(1, units, cx))
                    })
                    .child(
                        self.button("swap", self.tr("Swap units"), false, cx, |app, _, _| {
                            app.converter.swap(1, 2);
                            app.notebook.converter = Some(app.converter);
                        })
                        .self_start(),
                    )
                    .child(p.label(self.tr("To")))
                    .child(self.unit_button(2, units[to], cx))
                    .when(self.unit_menu == Some(2), |c| {
                        c.child(self.unit_choices(2, units, cx))
                    })
                    .child(match converted {
                        Ok(value) => row()
                            .gap_3()
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_color(p.accent)
                                    .text_size(rems(2.4))
                                    .child(format!(
                                        "{} {}",
                                        self.notebook.show(&value.normalize().to_string()),
                                        units[to].symbol
                                    )),
                            )
                            .child(self.copy_decimal("copy-conversion", value, cx)),
                        Err(error) => row().text_color(p.danger).child(self.tr(error)),
                    }),
            )
            .when_some(result.ok(), |c, value| {
                c.child(
                    p.card()
                        .p_4()
                        .gap_2()
                        .child(p.label(self.tr("Every unit")))
                        .children(units.iter().enumerate().filter_map(|(i, unit)| {
                            let converted = units::convert(value, category, from, i).ok()?;
                            let text = format!(
                                "{}  ·  {} {}",
                                self.tr(unit.name),
                                self.notebook.show(&converted.normalize().to_string()),
                                unit.symbol
                            );
                            Some(
                                self.button(
                                    ("conversion-row", i),
                                    text,
                                    i == to,
                                    cx,
                                    move |app, _, _| {
                                        app.converter[2] = i;
                                        app.notebook.converter = Some(app.converter);
                                        app.unit_menu = None;
                                    },
                                )
                                .justify_start(),
                            )
                        })),
                )
            })
            .into_any_element()
    }

    fn unit_button(&self, side: usize, unit: units::Unit, cx: &mut Context<Self>) -> Stateful<Div> {
        self.button(
            ("unit", side),
            format!("{}  ·  {}  ▾", self.tr(unit.name), unit.symbol),
            self.unit_menu == Some(side),
            cx,
            move |app, _, _| {
                app.unit_menu = if app.unit_menu == Some(side) {
                    None
                } else {
                    Some(side)
                };
            },
        )
        .justify_start()
    }

    fn unit_choices(&self, side: usize, units: &[units::Unit], cx: &mut Context<Self>) -> Div {
        row()
            .gap_2()
            .flex_wrap()
            .children(units.iter().enumerate().map(|(i, unit)| {
                self.button(
                    ("choose-unit", i),
                    format!("{} ({})", self.tr(unit.name), unit.symbol),
                    self.converter[side] == i,
                    cx,
                    move |app, _, _| {
                        app.converter[side] = i;
                        app.notebook.converter = Some(app.converter);
                        app.unit_menu = None;
                    },
                )
            }))
    }

    pub fn copy_decimal(
        &self,
        id: impl Into<ElementId>,
        value: Decimal,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let text = self.notebook.copy_text(&value.normalize().to_string());
        self.button(id, self.tr("Copy"), false, cx, move |_, _, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
        })
    }

    pub fn programmer(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.palette;
        let word = self.word;
        let value = word.parse(&self.integer.read(cx).value(), self.radix);
        let pattern = value.as_ref().copied().unwrap_or_default();
        let ops = [
            (Op::And, "AND"),
            (Op::Or, "OR"),
            (Op::Xor, "XOR"),
            (Op::Not, "NOT"),
            (Op::Shl, "<<"),
            (Op::Shr, ">>"),
            (Op::Add, "+"),
            (Op::Sub, "−"),
            (Op::Mul, "×"),
            (Op::Div, "÷"),
            (Op::Mod, "MOD"),
            (Op::Neg, "±"),
        ];
        column()
            .gap_4()
            .child(self.title("Programmer"))
            .child(
                row()
                    .gap_2()
                    .flex_wrap()
                    .children(programmer::SIZES.map(|bits| {
                        self.button(
                            ("bits", bits as usize),
                            format!("{bits} bit"),
                            word.bits == bits,
                            cx,
                            move |app, window, cx| app.set_word(bits, app.word.signed, window, cx),
                        )
                    }))
                    .child(div().flex_1())
                    .children(
                        [("Unsigned", false), ("Signed", true)].map(|(label, signed)| {
                            self.button(
                                label,
                                self.tr(label),
                                word.signed == signed,
                                cx,
                                move |app, window, cx| {
                                    app.set_word(app.word.bits, signed, window, cx);
                                },
                            )
                        }),
                    ),
            )
            .child(
                p.card()
                    .p_4()
                    .gap_3()
                    .child(self.entry(&self.integer))
                    .when_some(value.as_ref().err(), |c, error| {
                        c.child(div().text_color(p.danger).child(self.tr(error)))
                    })
                    .children([(16, "HEX"), (10, "DEC"), (8, "OCT"), (2, "BIN")].map(
                        |(radix, name)| {
                            let text = value
                                .as_ref()
                                .map(|&bits| {
                                    if radix == 2 {
                                        word.grouped_binary(bits)
                                    } else {
                                        word.format(bits, radix)
                                    }
                                })
                                .unwrap_or_else(|_| "—".into());
                            let copy = text.clone();
                            row()
                                .gap_3()
                                .child(
                                    self.button(
                                        name,
                                        name,
                                        self.radix == radix,
                                        cx,
                                        move |app, window, cx| {
                                            if let Ok(pattern) = app
                                                .word
                                                .parse(&app.integer.read(cx).value(), app.radix)
                                            {
                                                Self::set(
                                                    &app.integer,
                                                    app.word.format(pattern, radix),
                                                    window,
                                                    cx,
                                                );
                                            }
                                            if let Ok(pattern) = app
                                                .word
                                                .parse(&app.operand.read(cx).value(), app.radix)
                                            {
                                                Self::set(
                                                    &app.operand,
                                                    app.word.format(pattern, radix),
                                                    window,
                                                    cx,
                                                );
                                            }
                                            app.radix = radix;
                                        },
                                    )
                                    .w(px(62.)),
                                )
                                .child(div().flex_1().min_w_0().text_color(p.accent).child(text))
                                .child(
                                    self.button(
                                        ("copy-base", radix as usize),
                                        self.tr("Copy"),
                                        false,
                                        cx,
                                        move |_, _, cx| {
                                            if copy != "—" {
                                                cx.write_to_clipboard(ClipboardItem::new_string(
                                                    copy.clone(),
                                                ));
                                            }
                                        },
                                    )
                                    .text_xs(),
                                )
                        },
                    )),
            )
            .child(
                p.card()
                    .p_4()
                    .gap_3()
                    .child(p.label(self.tr("Bits")))
                    .child(row().gap_2().flex_wrap().children(
                        (0..word.bits).rev().step_by(8).map(|start| {
                            row().gap_1().children((start - 7..=start).rev().map(|bit| {
                                let on = pattern & (1 << bit) != 0;
                                self.button(
                                    ("toggle-bit", bit as usize),
                                    if on { "1" } else { "0" },
                                    on,
                                    cx,
                                    move |app, window, cx| {
                                        if let Ok(bits) =
                                            app.word.parse(&app.integer.read(cx).value(), app.radix)
                                        {
                                            Self::set(
                                                &app.integer,
                                                app.word.format(bits ^ (1 << bit), app.radix),
                                                window,
                                                cx,
                                            );
                                        }
                                    },
                                )
                                .px_1()
                                .py_1()
                                .min_w(px(23.))
                                .text_xs()
                            }))
                        }),
                    )),
            )
            .child(
                p.card()
                    .p_4()
                    .gap_3()
                    .child(p.label(self.tr("Bitwise operations")))
                    .child(row().gap_2().flex_wrap().children(ops.map(|(op, title)| {
                        self.button(title, title, op == self.op, cx, move |app, _, _| {
                            app.op = op
                        })
                    })))
                    .child(
                        row()
                            .gap_3()
                            .when(!matches!(self.op, Op::Not | Op::Neg), |r| {
                                r.child(self.entry(&self.operand).flex_1())
                            })
                            .child(self.button(
                                "apply-op",
                                self.tr("Apply"),
                                true,
                                cx,
                                |app, window, cx| app.apply_op(window, cx),
                            )),
                    ),
            )
            .into_any_element()
    }

    fn set_word(&mut self, bits: u32, signed: bool, window: &mut Window, cx: &mut App) {
        let old = self.word.parse(&self.integer.read(cx).value(), self.radix);
        let operand = self.word.parse(&self.operand.read(cx).value(), self.radix);
        self.word = programmer::Word { bits, signed };
        self.notebook.programmer_bits = Some(bits);
        self.notebook.programmer_signed = signed;
        if let Ok(pattern) = old {
            Self::set(
                &self.integer,
                self.word.format(pattern & self.word.mask(), self.radix),
                window,
                cx,
            );
        }
        if let Ok(pattern) = operand {
            Self::set(
                &self.operand,
                self.word.format(pattern & self.word.mask(), self.radix),
                window,
                cx,
            );
        }
    }

    pub fn apply_op(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let result = (|| {
            let a = self
                .word
                .parse(&self.integer.read(cx).value(), self.radix)?;
            let b = if matches!(self.op, Op::Not | Op::Neg) {
                0
            } else {
                self.word
                    .parse(&self.operand.read(cx).value(), self.radix)?
            };
            self.word.apply(self.op, a, b)
        })();
        match result {
            Ok(bits) => Self::set(
                &self.integer,
                self.word.format(bits, self.radix),
                window,
                cx,
            ),
            Err(error) => self.notice = Some(error),
        }
    }

    pub fn graph(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.palette;
        let view = self.view;
        let values = self.plots.each_ref().map(|input| input.read(cx).value());
        let compiled = values.map(|value| {
            if value.trim().is_empty() {
                Ok(None)
            } else {
                Expression::parse(&value).map(Some)
            }
        });
        let mut context = self.notebook.context();
        context.angle = Angle::Radians;
        let variables = self.notebook.engine_variables();
        let sample = |expr: &Expression, x: f64| {
            let context = rustcalc::engine::Context {
                x: Some(Decimal::try_from(x).ok()?),
                ..context
            };
            expr.eval(&context, &variables).ok()?.number.to_f64()
        };
        let curves = compiled.each_ref().map(|expr| {
            expr.as_ref()
                .ok()
                .and_then(Option::as_ref)
                .map(|expr| plot::sample(|x| sample(expr, x), &view, 600))
                .unwrap_or_default()
        });
        let mut crossings = Vec::new();
        if self.show_roots {
            for (i, expr) in compiled.iter().enumerate() {
                let Ok(Some(expr)) = expr else { continue };
                for x in plot::roots(|x| sample(expr, x), view.x_range(), 400) {
                    crossings.push((i, None, x, 0.));
                }
                for (j, other) in compiled.iter().enumerate().skip(i + 1) {
                    let Ok(Some(other)) = other else { continue };
                    for x in plot::roots(
                        |x| Some(sample(expr, x)? - sample(other, x)?),
                        view.x_range(),
                        400,
                    ) {
                        if let Some(y) = sample(expr, x) {
                            crossings.push((i, Some(j), x, y));
                        }
                    }
                }
            }
        }
        let points = crossings.clone();
        let graph_bounds = self.graph_bounds.clone();
        let chart = canvas(
            move |bounds, _, _| {
                graph_bounds.set(Some(bounds));
            },
            move |bounds, (), window, cx| {
                paint_graph(bounds, view, p, &curves, &points, window, cx);
            },
        )
        .size_full();
        column()
            .gap_4()
            .child(
                row()
                    .gap_2()
                    .child(self.title("Plot a graph"))
                    .child(div().flex_1())
                    .child(self.button("zoom-out", "−", false, cx, |app, _, _| {
                        app.zoom_graph(1.25)
                    }))
                    .child(self.button("zoom-in", "+", false, cx, |app, _, _| app.zoom_graph(0.8)))
                    .child(
                        self.button("reset-graph", self.tr("Reset"), false, cx, |app, _, _| {
                            app.view = plot::Viewport::default()
                        }),
                    ),
            )
            .children(self.plots.iter().enumerate().map(|(i, input)| {
                row()
                    .gap_2()
                    .child(
                        div()
                            .w(px(42.))
                            .text_color(p.curves[i])
                            .child(format!("{}(x)", ["f", "g", "h"][i])),
                    )
                    .child(self.entry(input).flex_1())
                    .when_some(compiled[i].as_ref().err(), |r, error| {
                        r.child(
                            div()
                                .text_color(p.danger)
                                .text_sm()
                                .child(self.tr(&error.message)),
                        )
                    })
            }))
            .child(
                p.card()
                    .h(px(370.))
                    .overflow_hidden()
                    .id("plot")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|app, event: &MouseDownEvent, window, _| {
                            app.graph_drag = Some(event.position);
                            window.focus(&app.focus);
                        }),
                    )
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(|app, _, _, _| app.graph_drag = None),
                    )
                    .on_mouse_move(cx.listener(|app, event: &MouseMoveEvent, _, cx| {
                        if event.pressed_button != Some(MouseButton::Left) {
                            app.graph_drag = None;
                            return;
                        }
                        if let (Some(last), Some(bounds)) = (app.graph_drag, app.graph_bounds.get())
                        {
                            app.graph_drag = Some(event.position);
                            app.view.pan(
                                -f64::from((event.position.x - last.x) / bounds.size.width),
                                f64::from((event.position.y - last.y) / bounds.size.height),
                            );
                            cx.notify();
                        }
                    }))
                    .on_scroll_wheel(cx.listener(|app, event: &ScrollWheelEvent, _, cx| {
                        let Some(bounds) = app.graph_bounds.get() else {
                            return;
                        };
                        let delta = f32::from(event.delta.pixel_delta(px(22.)).y);
                        if delta == 0. {
                            return;
                        }
                        let x = f64::from((event.position.x - bounds.origin.x) / bounds.size.width)
                            - 0.5;
                        let y = 0.5
                            - f64::from((event.position.y - bounds.origin.y) / bounds.size.height);
                        let view = &mut app.view;
                        view.zoom_at(
                            (f64::from(-delta) * 0.009).exp(),
                            view.center_x + x * 2. * view.span,
                            view.center_y + y * 2. * view.span,
                        );
                        cx.stop_propagation();
                        cx.notify();
                    }))
                    .child(chart),
            )
            .child(
                row()
                    .gap_3()
                    .child(self.button(
                        "roots",
                        self.tr("Roots & intersections"),
                        self.show_roots,
                        cx,
                        |app, _, _| app.show_roots = !app.show_roots,
                    ))
                    .child(p.label(format!(
                        "x  {} … {}",
                        plot::readout(view.x_range().0),
                        plot::readout(view.x_range().1)
                    ))),
            )
            .when(self.show_roots && crossings.is_empty(), |c| {
                c.child(p.label(self.tr("No roots or intersections in view")))
            })
            .when(self.show_roots, |c| {
                c.child(row().gap_2().flex_wrap().children(crossings.iter().map(
                    |&(i, other, x, y)| {
                        let name = ["f", "g", "h"];
                        let title = other.map_or_else(
                            || format!("{} = 0", name[i]),
                            |j| format!("{} = {}", name[i], name[j]),
                        );
                        div()
                            .border_1()
                            .border_color(p.line)
                            .rounded(px(6.))
                            .px_2()
                            .py_1()
                            .text_sm()
                            .text_color(p.curves[i])
                            .child(format!(
                                "{title}  ({}, {})",
                                plot::readout(x),
                                plot::readout(y)
                            ))
                    },
                )))
            })
            .into_any_element()
    }

    pub fn zoom_graph(&mut self, factor: f64) {
        self.view
            .zoom_at(factor, self.view.center_x, self.view.center_y);
    }
}

type Crossing = (usize, Option<usize>, f64, f64);

fn paint_graph(
    bounds: Bounds<Pixels>,
    view: plot::Viewport,
    p: Palette,
    curves: &[plot::Segments; 3],
    points: &[Crossing],
    window: &mut Window,
    cx: &mut App,
) {
    let (x0, x1) = view.x_range();
    let (y0, y1) = view.y_range();
    let screen = |x: f64, y: f64| {
        point(
            bounds.origin.x + bounds.size.width * ((x - x0) / (x1 - x0)) as f32,
            bounds.origin.y + bounds.size.height * ((y1 - y) / (y1 - y0)) as f32,
        )
    };
    let mut line = |a, b, width, color| {
        let mut path = PathBuilder::stroke(px(width));
        path.move_to(a);
        path.line_to(b);
        if let Ok(path) = path.build() {
            window.paint_path(path, color);
        }
    };
    let x_step = plot::tick_step(x1 - x0, 10);
    let y_step = plot::tick_step(y1 - y0, 8);
    let xs = plot::ticks(x0, x1, x_step);
    let ys = plot::ticks(y0, y1, y_step);
    for &x in &xs {
        line(
            screen(x, y0),
            screen(x, y1),
            1.,
            if x.abs() < 1e-12 { p.muted } else { p.line },
        );
    }
    for &y in &ys {
        line(
            screen(x0, y),
            screen(x1, y),
            1.,
            if y.abs() < 1e-12 { p.muted } else { p.line },
        );
    }
    for (i, segments) in curves.iter().enumerate() {
        let mut path = PathBuilder::stroke(px(2.2));
        for segment in segments {
            let Some(&(x, y)) = segment.first() else {
                continue;
            };
            path.move_to(screen(x, y));
            for &(x, y) in &segment[1..] {
                path.line_to(screen(x, y));
            }
        }
        if let Ok(path) = path.build() {
            window.paint_path(path, p.curves[i]);
        }
    }
    for &(i, _, x, y) in points {
        if !(y0..=y1).contains(&y) {
            continue;
        }
        let at = screen(x, y);
        window.paint_quad(fill(
            Bounds::new(at - point(px(3.), px(3.)), size(px(6.), px(6.))),
            p.curves[i],
        ));
    }
    let mut label = |text: String, at: Point<Pixels>| {
        let run = TextRun {
            len: text.len(),
            font: window.text_style().font(),
            color: p.muted.into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let line = window
            .text_system()
            .shape_line(text.into(), px(11.), &[run], None);
        let _ = line.paint(at, px(14.), window, cx);
    };
    for &x in &xs {
        let at = screen(x, y0);
        if at.x > bounds.origin.x + px(30.) && at.x < bounds.origin.x + bounds.size.width - px(24.)
        {
            label(plot::tick_label(x, x_step), at + point(px(3.), px(-16.)));
        }
    }
    for &y in &ys {
        let at = screen(x0, y);
        if at.y > bounds.origin.y + px(10.) && at.y < bounds.origin.y + bounds.size.height - px(18.)
        {
            label(plot::tick_label(y, y_step), at + point(px(5.), px(-16.)));
        }
    }
}
