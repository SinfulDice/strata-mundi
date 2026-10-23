use super::{Draw, HitKind, Node, Rect};
use crate::ui::model::Tone;

/// Rows of `row_height` pixels: icon on the left (16:9), title and detail beside it.
pub(super) fn list(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::List {
        id,
        anchor,
        at,
        size,
        row_height,
    } = node
    else {
        return;
    };
    let rect = parent.place(*anchor, d.ctx.offset(id, *at), *size);
    let theme = d.theme();
    let (field, light, dark) = (theme.field.0, theme.panel_light.0, theme.panel_dark.0);
    d.painter.fill(rect, field);
    d.painter.bevel(rect, dark, light);
    let selected = d.model.selected(id);
    let fitting = (rect.h / row_height.max(&1)) as usize;
    for (index, row) in d.model.rows(id).into_iter().take(fitting).enumerate() {
        let y = rect.y + (index as u32 * row_height) as i32;
        let line = Rect::new(rect.x + 1, y + 1, rect.w - 2, row_height - 2);
        let focused = d.is_focused(id, index);
        let theme = d.theme();
        let (accent, focus, panel) = (theme.accent.0, theme.focus.0, theme.panel.0);
        let title_color = match (row.dim, focused) {
            (true, _) => theme.text_dim.0,
            (false, true) => accent,
            (false, false) => theme.text.0,
        };
        let (dim, shadow) = (theme.text_dim.0, theme.text_shadow.0);
        if selected == Some(index) {
            d.painter.fill(line, panel);
        }
        let icon_h = row_height - 4;
        let icon_w = icon_h * 16 / 9;
        let icon_rect = Rect::new(line.x + 1, line.y + 1, icon_w, icon_h);
        match row.icon {
            Some(source) => d.painter.image(icon_rect, source),
            None => d.painter.fill(icon_rect, dark),
        }
        let text_x = icon_rect.x + icon_w as i32 + 4;
        d.painter
            .text(text_x, line.y + 1, &row.title, title_color, shadow);
        let detail_y = line.y + d.painter.line_height() as i32 + 1;
        d.painter.text(text_x, detail_y, &row.detail, dim, shadow);
        if focused {
            d.painter.bevel(line, focus, focus);
        } else if selected == Some(index) {
            d.painter.bevel(line, accent, accent);
        }
        d.hit(id, index, line, HitKind::Button);
    }
}

/// Settings rows: name left, value right, a gauge under ranges.
pub(super) fn options(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::OptionList {
        id,
        anchor,
        at,
        size,
        row_height,
    } = node
    else {
        return;
    };
    let rect = parent.place(*anchor, d.ctx.offset(id, *at), *size);
    let fitting = (rect.h / (*row_height).max(1)) as usize;
    for (index, row) in d
        .model
        .option_rows(id)
        .into_iter()
        .take(fitting)
        .enumerate()
    {
        let y = rect.y + (index as u32 * row_height) as i32;
        let line = Rect::new(rect.x, y, rect.w, row_height - 1);
        let focused = d.is_focused(id, index);
        let theme = d.theme();
        let fill = if focused {
            theme.panel_light.0
        } else {
            theme.panel_dark.0
        };
        let (text, accent, shadow, field) = (
            theme.text.0,
            theme.accent.0,
            theme.text_shadow.0,
            theme.field.0,
        );
        let value_color = if row.highlight || focused {
            accent
        } else {
            text
        };
        d.painter.fill(line, fill);
        let text_y = y + (*row_height as i32 - d.painter.line_height() as i32) / 2;
        d.painter.text(line.x + 4, text_y, &row.label, text, shadow);
        let value_width = d.painter.text_width(&row.value) as i32;
        let value_x = line.x + line.w as i32 - value_width - 4;
        if let Some(gauge) = row.gauge {
            let bar_w = (line.w / 3).max(10);
            let bar = Rect::new(
                line.x + line.w as i32 - bar_w as i32 - 2,
                y + 1,
                bar_w,
                row_height - 3,
            );
            let filled = (gauge.clamp(0.0, 1.0) * bar.w as f32).round() as u32;
            d.painter.fill(bar, field);
            d.painter
                .fill(Rect::new(bar.x, bar.y, filled, bar.h), theme_bar(d));
        }
        d.painter
            .text(value_x, text_y, &row.value, value_color, shadow);
        d.hit(id, index, line, HitKind::Option);
    }
}

fn theme_bar(d: &Draw<'_, '_>) -> strata_core::texture::Rgba {
    d.theme().panel.0
}

/// As many of the newest lines as fit, over a shade, bottom-aligned.
pub(super) fn log(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::Log {
        id,
        anchor,
        at,
        size,
    } = node
    else {
        return;
    };
    let rect = parent.place(*anchor, d.ctx.offset(id, *at), *size);
    let height = d.painter.line_height() as i32 + 1;
    let fits = (rect.h as i32 / height).max(0) as usize;
    let lines = d.model.log_lines(id);
    let shown = &lines[lines.len().saturating_sub(fits)..];
    if !shown.is_empty() {
        let used = shown.len() as i32 * height + 2;
        let top = rect.y + rect.h as i32 - used;
        d.painter.fill(
            Rect::new(rect.x, top, rect.w, used as u32),
            d.theme().shade.0,
        );
    }
    let theme = d.theme();
    let (normal, dim, error) = (theme.text.0, theme.text_dim.0, theme.accent.0);
    let shadow = theme.text_shadow.0;
    for (row, line) in shown.iter().enumerate() {
        let color = match line.tone {
            Tone::Normal => normal,
            Tone::Dim => dim,
            Tone::Error => error,
        };
        let y = rect.y + rect.h as i32 - (shown.len() - row) as i32 * height;
        d.painter.text(rect.x + 2, y, &line.text, color, shadow);
    }
}
