use gpui::{Div, Rgba, Svg, div, prelude::*, px, rgb, svg};

pub(super) trait AlignStart: gpui::Styled + Sized {
    fn self_start(mut self) -> Self {
        self.style().align_self = Some(gpui::AlignItems::FlexStart);
        self
    }
}
impl<T: gpui::Styled> AlignStart for T {}

pub(super) fn icon(path: &'static str, color: Rgba) -> Svg {
    svg()
        .path(path)
        .size(px(17.))
        .text_color(color)
        .flex_shrink_0()
}

pub(super) fn icon_bytes(path: &str) -> Option<&'static [u8]> {
    macro_rules! outline {
        ($content:literal) => {
            concat!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">"#, $content, "</svg>").as_bytes()
        };
    }
    Some(match path {
        "calculator" => outline!(
            r#"<rect x="5" y="2.8" width="14" height="18.4" rx="3"/><path d="M8.5 7h7m-7 4h1m5 0h1m-7 4h1m5 0h1m-7 3h1m5 0h1"/>"#
        ),
        "convert" => outline!(r#"<path d="M3 8h17m-4-4 4 4-4 4m5 4H4m4-4-4 4 4 4"/>"#),
        "binary" => outline!(
            r#"<rect x="3.5" y="4" width="17" height="16" rx="3"/><path d="M7.5 9h3v6h-3zM14 10l2-1v6"/>"#
        ),
        "graph" => outline!(r#"<path d="M4 4v16h16M6 15c4 1 4-7 8-7 3 0 3 3 6 3"/>"#),
        "formulas" => outline!(
            r#"<path d="M16 4h-3c-2 0-3 2-3.5 5L8 17c-.3 2-1 3-3 3H3m4-10h7m0 5 5 5m0-5-5 5"/>"#
        ),
        "updates" => outline!(
            r#"<path d="M20 10a8 8 0 0 0-14-4l-2 2m0-5v5h5m-5 6a8 8 0 0 0 14 4l2-2m0 5v-5h-5"/>"#
        ),
        "settings" => outline!(r#"<path d="M5 6h14M5 12h14M5 18h14m-9-14v4m5 2v4m-7 2v4"/>"#),
        "help" => outline!(
            r#"<circle cx="12" cy="12" r="9"/><path d="M9.5 9.5a2.5 2.5 0 1 1 4.1 1.9C12.5 12.4 12 13 12 14m0 3h.01"/>"#
        ),
        _ => return None,
    })
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct Palette {
    pub bg: Rgba,
    pub rail: Rgba,
    pub panel: Rgba,
    pub key: Rgba,
    pub line: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub accent: Rgba,
    pub accent_text: Rgba,
    pub selected: Rgba,
    pub danger: Rgba,
    pub curves: [Rgba; 3],
}

impl Palette {
    pub fn new(light: bool, high: bool) -> Self {
        if high {
            return Self {
                bg: rgb(0),
                rail: rgb(0),
                panel: rgb(0),
                key: rgb(0),
                line: rgb(0xffffff),
                text: rgb(0xffffff),
                muted: rgb(0xffffff),
                accent: rgb(0xffff00),
                accent_text: rgb(0),
                selected: rgb(0x262600),
                danger: rgb(0xffa0a0),
                curves: [rgb(0xffff00), rgb(0x80ffff), rgb(0xffa0ff)],
            };
        }
        if light {
            Self {
                bg: rgb(0xf3f6f5),
                rail: rgb(0xe9efed),
                panel: rgb(0xffffff),
                key: rgb(0xf9fbfa),
                line: rgb(0xcbd9d4),
                text: rgb(0x172723),
                muted: rgb(0x4e625b),
                accent: rgb(0x00664a),
                accent_text: rgb(0xffffff),
                selected: rgb(0xdceee6),
                danger: rgb(0xa42a40),
                curves: [rgb(0x007653), rgb(0x6341aa), rgb(0xa64728)],
            }
        } else {
            Self {
                bg: rgb(0x111619),
                rail: rgb(0x161d20),
                panel: rgb(0x1a2427),
                key: rgb(0x1a2226),
                line: rgb(0x2c3b40),
                text: rgb(0xe9f1ee),
                muted: rgb(0xa2b5af),
                accent: rgb(0x9ce8c3),
                accent_text: rgb(0x102b21),
                selected: rgb(0x223b33),
                danger: rgb(0xffa4ac),
                curves: [rgb(0x9ce8c3), rgb(0xc6b5fb), rgb(0xffbf8f)],
            }
        }
    }

    pub fn card(self) -> Div {
        column()
            .bg(self.panel)
            .border_1()
            .border_color(self.line)
            .rounded(px(14.))
    }

    pub fn label(self, text: impl Into<gpui::SharedString>) -> Div {
        div().text_sm().text_color(self.muted).child(text.into())
    }
}

pub(super) fn row() -> Div {
    div().flex().flex_row().items_center()
}

pub(super) fn column() -> Div {
    div().flex().flex_col()
}
