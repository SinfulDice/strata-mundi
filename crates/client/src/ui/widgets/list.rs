use super::{Draw, HitKind, Node, Rect};

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
