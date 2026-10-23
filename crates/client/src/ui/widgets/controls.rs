use super::text::{draw_rich, rich_width};
use super::{Draw, HitKind, Node, Rect};

pub(super) fn label(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::Label {
        id,
        anchor,
        at,
        text,
        boxed,
        dim,
        size,
    } = node
    else {
        return;
    };
    let content = match text.strip_prefix('@') {
        Some(binding) => d.model.text(binding).unwrap_or_default(),
        None => d.ctx.tr(text),
    };
    if content.is_empty() {
        return;
    }
    let pad = 4 * u32::from(*boxed);
    let width = if *size > 1 {
        d.painter.text_width(&content) * size
    } else {
        rich_width(d, &content)
    };
    let dims = (width + 2 * pad, d.painter.line_height() * size + pad);
    let rect = parent.place(*anchor, d.ctx.offset(id, *at), dims);
    let theme = d.theme();
    let (panel, light, dark) = (theme.panel.0, theme.panel_light.0, theme.panel_dark.0);
    let color = if *dim { theme.text_dim.0 } else { theme.text.0 };
    let shadow = theme.text_shadow.0;
    if *boxed {
        d.painter.fill(rect, panel);
        d.painter.bevel(rect, light, dark);
    }
    let (x, y) = (rect.x + pad as i32, rect.y + pad as i32 / 2);
    if *size > 1 {
        d.painter.text_sized(x, y, &content, color, shadow, *size);
    } else {
        draw_rich(d, x, y, &content, color);
    }
    if d.ctx.editing {
        d.element(id, rect);
    }
}

fn caption(d: &Draw<'_, '_>, id: &str, text: &str) -> String {
    match text.strip_prefix('@') {
        Some(_) => d.model.text(id).unwrap_or_default(),
        None => d.ctx.tr(text),
    }
}

pub(super) fn button(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::Button {
        id,
        anchor,
        at,
        size,
        text,
    } = node
    else {
        return;
    };
    let rect = parent.place(*anchor, d.ctx.offset(id, *at), *size);
    let focused = d.is_focused(id, 0);
    let label = caption(d, id, text);
    let theme = d.theme();
    let fill = if focused {
        theme.panel_light.0
    } else {
        theme.panel.0
    };
    let (light, dark) = (theme.panel_light.0, theme.panel_dark.0);
    let color = if focused {
        theme.accent.0
    } else {
        theme.text.0
    };
    d.painter.fill(rect, fill);
    d.painter.bevel(rect, light, dark);
    let (cx, cy) = rect.center();
    let width = rich_width(d, &label) as i32;
    let y = cy - d.painter.line_height() as i32 / 2;
    draw_rich(d, cx - width / 2, y, &label, color);
    d.hit(id, 0, rect, HitKind::Button);
}

pub(super) fn text_field(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::TextField {
        id,
        anchor,
        at,
        size,
        placeholder,
    } = node
    else {
        return;
    };
    let rect = parent.place(*anchor, d.ctx.offset(id, *at), *size);
    let content = d.model.text(id).unwrap_or_default();
    let focused = d.is_focused(id, 0);
    let theme = d.theme();
    let (field, light, dark) = (theme.field.0, theme.panel_light.0, theme.panel_dark.0);
    let (dim, normal, shadow) = (theme.text_dim.0, theme.text.0, theme.text_shadow.0);
    let edge = if focused { theme.focus.0 } else { light };
    d.painter.fill(rect, field);
    d.painter.bevel(rect, dark, edge);
    let (text, color) = if content.is_empty() && !focused {
        (d.ctx.tr(placeholder), dim)
    } else {
        let cursor = if focused { "_" } else { "" };
        (format!("{content}{cursor}"), normal)
    };
    d.painter.text(rect.x + 3, rect.y + 1, &text, color, shadow);
    d.hit(id, 0, rect, HitKind::TextField);
}

pub(super) fn slider(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::Slider {
        id,
        anchor,
        at,
        size,
    } = node
    else {
        return;
    };
    let rect = parent.place(*anchor, d.ctx.offset(id, *at), *size);
    let (value, label) = d.model.slider(id).unwrap_or_default();
    let focused = d.is_focused(id, 0);
    let theme = d.theme();
    let (field, light, dark) = (theme.field.0, theme.panel_light.0, theme.panel_dark.0);
    let knob_color = if focused {
        theme.accent.0
    } else {
        theme.panel_light.0
    };
    let (text, shadow) = (theme.text.0, theme.text_shadow.0);
    d.painter.fill(rect, field);
    d.painter.bevel(rect, dark, light);
    let knob_w = 6;
    let travel = rect.w.saturating_sub(knob_w) as f32;
    let knob_x = rect.x + (value.clamp(0.0, 1.0) * travel).round() as i32;
    d.painter
        .fill(Rect::new(knob_x, rect.y, knob_w, rect.h), knob_color);
    let (cx, cy) = rect.center();
    let width = d.painter.text_width(&label) as i32;
    let y = cy - d.painter.line_height() as i32 / 2;
    d.painter.text(cx - width / 2, y, &label, text, shadow);
    d.hit(id, 0, rect, HitKind::Slider);
}

const TAB_GAP: u32 = 2;

pub(super) fn tabs(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::Tabs {
        id,
        anchor,
        at,
        size,
    } = node
    else {
        return;
    };
    let rect = parent.place(*anchor, d.ctx.offset(id, *at), *size);
    let selected = d.model.selected(id);
    let titles = d.model.tabs(id);
    let pad = tab_padding(d, &titles, rect.w);
    let mut x = rect.x;
    for (index, title) in titles.iter().enumerate() {
        let width = d.painter.text_width(title) + 2 * pad;
        let tab = Rect::new(x, rect.y, width, rect.h);
        let active = selected == Some(index);
        let focused = d.is_focused(id, index);
        let theme = d.theme();
        let fill = if active {
            theme.panel_light.0
        } else {
            theme.panel_dark.0
        };
        let color = if active || focused {
            theme.accent.0
        } else {
            theme.text.0
        };
        let (shadow, focus) = (theme.text_shadow.0, theme.focus.0);
        d.painter.fill(tab, fill);
        d.painter
            .text(x + pad as i32, rect.y + 1, title, color, shadow);
        if focused {
            d.painter.bevel(tab, focus, focus);
        }
        d.hit(id, index, tab, HitKind::Button);
        x += (width + TAB_GAP) as i32;
    }
}

/// Room on each side of a tab title: 4 pixels, less when the tabs would not fit.
fn tab_padding(d: &Draw<'_, '_>, titles: &[String], width: u32) -> u32 {
    let count = titles.len().max(1) as u32;
    let text: u32 = titles.iter().map(|t| d.painter.text_width(t)).sum();
    let spare = width.saturating_sub(text + TAB_GAP * (count - 1));
    (spare / (2 * count)).clamp(1, 4)
}

/// On-screen keyboard: each character is a key; the model receives its index.
pub(super) fn key_grid(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::KeyGrid {
        id,
        anchor,
        at,
        rows,
        key,
    } = node
    else {
        return;
    };
    let columns = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) as u32;
    let size = (columns * (key + 1), rows.len() as u32 * (key + 1));
    let rect = parent.place(*anchor, d.ctx.offset(id, *at), size);
    let mut index = 0;
    for (row, line) in rows.iter().enumerate() {
        for (column, ch) in line.chars().enumerate() {
            let x = rect.x + (column as u32 * (key + 1)) as i32;
            let y = rect.y + (row as u32 * (key + 1)) as i32;
            let cap = Rect::new(x, y, *key, *key);
            let focused = d.is_focused(id, index);
            let label = d
                .model
                .text(&format!("{id}:{ch}"))
                .unwrap_or_else(|| ch.to_string());
            let theme = d.theme();
            let fill = if focused {
                theme.panel_light.0
            } else {
                theme.panel_dark.0
            };
            let color = if focused {
                theme.accent.0
            } else {
                theme.text.0
            };
            let (light, dark, shadow) = (theme.panel_light.0, theme.field.0, theme.text_shadow.0);
            d.painter.fill(cap, fill);
            d.painter.bevel(cap, light, dark);
            let width = d.painter.text_width(&label) as i32;
            let ty = y + (*key as i32 - d.painter.line_height() as i32) / 2;
            d.painter
                .text(x + (*key as i32 - width) / 2, ty, &label, color, shadow);
            d.hit(id, index, cap, HitKind::Button);
            index += 1;
        }
    }
}

pub(super) fn tooltip(d: &mut Draw<'_, '_>, anchor: Rect, text: &str, screen: Rect) {
    let w = d.painter.text_width(text) + 8;
    let h = d.painter.line_height() + 3;
    let x = (anchor.x + anchor.w as i32 + 2).min(screen.x + screen.w as i32 - w as i32);
    let y = (anchor.y - h as i32).max(screen.y);
    let rect = Rect::new(x, y, w, h);
    let theme = d.theme();
    let (fill, accent, color, shadow) = (
        theme.panel_dark.0,
        theme.accent.0,
        theme.text.0,
        theme.text_shadow.0,
    );
    d.painter.fill(rect, fill);
    d.painter.bevel(rect, accent, accent);
    d.painter.text(x + 4, y + 1, text, color, shadow);
}
