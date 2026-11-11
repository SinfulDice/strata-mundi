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
        wrap,
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
    let lines = match wrap {
        Some(width) if *size == 1 => wrap_lines(d, &content, *width),
        _ => vec![content],
    };
    let pad = 4 * u32::from(*boxed);
    let width = lines
        .iter()
        .map(|line| {
            if *size > 1 {
                d.painter.text_width(line) * size
            } else {
                rich_width(d, line)
            }
        })
        .max()
        .unwrap_or(0);
    let line_height = d.painter.line_height() * size;
    let dims = (width + 2 * pad, line_height * lines.len() as u32 + pad);
    let rect = parent.place(*anchor, d.ctx.offset(id, *at), dims);
    let theme = d.theme();
    let (panel, light, dark) = (theme.panel.0, theme.panel_light.0, theme.panel_dark.0);
    let color = if *dim { theme.text_dim.0 } else { theme.text.0 };
    let shadow = theme.text_shadow.0;
    if *boxed {
        d.painter.fill(rect, panel);
        d.painter.bevel(rect, light, dark);
    }
    let x = rect.x + pad as i32;
    for (row, line) in lines.iter().enumerate() {
        let y = rect.y + pad as i32 / 2 + (row as u32 * line_height) as i32;
        if *size > 1 {
            d.painter.text_sized(x, y, line, color, shadow, *size);
        } else {
            draw_rich(d, x, y, line, color);
        }
    }
    if d.ctx.editing {
        d.element(id, rect);
    }
}

/// Lines of at most `width` pixels, cut between words; a word too long for a line (a
/// code) is cut anywhere, unless it holds markup.
fn wrap_lines(d: &Draw<'_, '_>, text: &str, width: u32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split(' ') {
            let joined = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if rich_width(d, &joined) <= width {
                line = joined;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            line = word.to_owned();
            while !word.contains('<') && rich_width(d, &line) > width {
                let cut = fitting_prefix(d, &line, width);
                lines.push(line[..cut].to_owned());
                line = line[cut..].to_owned();
            }
        }
        lines.push(line);
    }
    lines
}

/// Bytes of the longest start of `text` that fits in `width` (at least one character).
fn fitting_prefix(d: &Draw<'_, '_>, text: &str, width: u32) -> usize {
    let mut fitting = text.chars().next().map_or(0, char::len_utf8);
    for (index, ch) in text.char_indices().skip(1) {
        if d.painter.text_width(&text[..index + ch.len_utf8()]) > width {
            break;
        }
        fitting = index + ch.len_utf8();
    }
    fitting
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
    let text = tail_fitting(d, &text, rect.w.saturating_sub(6));
    d.painter.text(rect.x + 3, rect.y + 1, text, color, shadow);
    d.hit(id, 0, rect, HitKind::TextField);
}

/// The end of `text` that fits in `width` (long text shows its last characters, as typed).
fn tail_fitting<'t>(d: &Draw<'_, '_>, text: &'t str, width: u32) -> &'t str {
    let mut start = text.len();
    for (index, _) in text.char_indices().rev() {
        if d.painter.text_width(&text[index..]) > width {
            break;
        }
        start = index;
    }
    &text[start..]
}

/// The model's picture, or an empty frame while there is none.
pub(super) fn image(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::Image {
        id,
        anchor,
        at,
        size,
    } = node
    else {
        return;
    };
    let rect = parent.place(*anchor, d.ctx.offset(id, *at), *size);
    let theme = d.theme();
    let (field, light, dark) = (theme.field.0, theme.panel_light.0, theme.panel_dark.0);
    match d.model.image(id) {
        Some(source) => d.painter.image(rect, source),
        None => d.painter.fill(rect, field),
    }
    d.painter.bevel(rect, dark, light);
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
