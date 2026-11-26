//! A gauge: a bar filled from 0 to 1 (a furnace's flame, the HUD's health), or a scale of
//! coloured stretches with a marker at the value (the HUD's temperature).

use super::{Draw, Node, Rect};

pub(super) fn gauge(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::Gauge {
        id,
        anchor,
        at,
        size,
        vertical,
        optional,
        segments,
    } = node
    else {
        return;
    };
    let rect = parent.place(*anchor, d.ctx.offset(id, *at), *size);
    let given = d.model.gauge(id);
    if *optional && given.is_none() && !d.ctx.editing {
        return;
    }
    let value = given.unwrap_or(0.5).clamp(0.0, 1.0);
    let theme = d.ctx.theme;
    d.painter.fill(rect, theme.field.0);
    d.painter
        .bevel(rect, theme.panel_dark.0, theme.panel_light.0);
    let inner = Rect::new(
        rect.x + 1,
        rect.y + 1,
        rect.w.saturating_sub(2),
        rect.h.saturating_sub(2),
    );
    if segments.is_empty() {
        let value = given.unwrap_or(0.0).clamp(0.0, 1.0);
        fill(d, id, inner, value, *vertical);
    } else {
        scale(d, inner, segments, value);
    }
    if d.ctx.editing {
        d.element(id, rect);
    }
}

fn fill(d: &mut Draw<'_, '_>, id: &str, inner: Rect, value: f32, vertical: bool) {
    let theme = d.ctx.theme;
    let color = theme.bars.get(id).unwrap_or(&theme.accent).0;
    let filled = if vertical {
        let height = (inner.h as f32 * value).round() as u32;
        Rect::new(
            inner.x,
            inner.y + (inner.h - height) as i32,
            inner.w,
            height,
        )
    } else {
        let width = (inner.w as f32 * value).round() as u32;
        Rect::new(inner.x, inner.y, width, inner.h)
    };
    if filled.w > 0 && filled.h > 0 {
        d.painter.fill(filled, color);
    }
}

/// The stretches side by side, then a marker three pixels wide, outlined, at `value`.
fn scale(d: &mut Draw<'_, '_>, inner: Rect, segments: &[(f32, String)], value: f32) {
    let theme = d.ctx.theme;
    let x_at = |fraction: f32| inner.x + (inner.w as f32 * fraction.clamp(0.0, 1.0)).round() as i32;
    let mut start = inner.x;
    for (end, color) in segments {
        let end = x_at(*end);
        let color = theme.bars.get(color.as_str()).unwrap_or(&theme.accent).0;
        if end > start {
            let stretch = Rect::new(start, inner.y, (end - start) as u32, inner.h);
            d.painter.fill(stretch, color);
        }
        start = start.max(end);
    }
    let middle = x_at(value).clamp(inner.x + 1, inner.x + inner.w as i32 - 2);
    let outline = Rect::new(middle - 2, inner.y - 1, 5, inner.h + 2);
    d.painter.fill(outline, theme.panel_dark.0);
    d.painter
        .fill(Rect::new(middle - 1, inner.y, 3, inner.h), theme.text.0);
}
