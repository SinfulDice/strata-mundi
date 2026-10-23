//! Drawing of the `ui::rich` markup.

use super::{Draw, Rect};
use crate::ui::rich::{Span, parse};
use strata_core::texture::Rgba;

/// Room around the label of a key cap.
const CAP_PAD: u32 = 2;

pub(super) fn rich_width(d: &Draw<'_, '_>, text: &str) -> u32 {
    parse(text)
        .iter()
        .map(|span| match span {
            Span::Text { text, .. } => d.painter.text_width(text),
            Span::Key(action) => cap_width(d, &(d.ctx.key_label)(action)),
        })
        .sum()
}

pub(super) fn draw_rich(d: &mut Draw<'_, '_>, x: i32, y: i32, text: &str, color: Rgba) -> u32 {
    let mut pen = x;
    for span in parse(text) {
        let theme = d.theme();
        let (accent, shadow) = (theme.accent.0, theme.text_shadow.0);
        pen += match span {
            Span::Text { text, accent: true } => d.painter.text(pen, y, text, accent, shadow),
            Span::Text { text, .. } => d.painter.text(pen, y, text, color, shadow),
            Span::Key(action) => key_cap(d, pen, y, &(d.ctx.key_label)(action)),
        } as i32;
    }
    (pen - x) as u32
}

fn cap_width(d: &Draw<'_, '_>, label: &str) -> u32 {
    d.painter.text_width(label) + 2 * CAP_PAD + 1
}

fn key_cap(d: &mut Draw<'_, '_>, x: i32, y: i32, label: &str) -> u32 {
    let width = cap_width(d, label);
    let theme = d.theme();
    let (fill, light, dark) = (theme.panel_dark.0, theme.panel_light.0, theme.field.0);
    let (text, shadow) = (theme.text.0, theme.text_shadow.0);
    let cap = Rect::new(x, y - 1, width - 1, d.painter.line_height() + 1);
    d.painter.fill(cap, fill);
    d.painter.bevel(cap, light, dark);
    d.painter.text(x + CAP_PAD as i32, y, label, text, shadow);
    width
}
