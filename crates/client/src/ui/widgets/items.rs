use super::{Draw, HitKind, Node, Rect};
use crate::ui::SlotMark;
use crate::ui::def::SLOT;
use strata_core::content::ItemId;

pub(super) fn grid(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::ItemGrid {
        id,
        anchor,
        at,
        columns,
        rows,
    } = node
    else {
        return;
    };
    let rect = parent.place(
        *anchor,
        d.ctx.offset(id, *at),
        (columns * SLOT, rows * SLOT),
    );
    let items = d.model.items(id);
    for index in 0..(columns * rows) as usize {
        let (cx, cy) = (
            (index as u32 % columns) * SLOT,
            (index as u32 / columns) * SLOT,
        );
        let slot = Rect::new(rect.x + cx as i32, rect.y + cy as i32, SLOT, SLOT);
        let item = items.get(index).copied().flatten();
        let focused = d.is_focused(id, index);
        draw_slot(d, slot, item, focused, false);
        if item.is_some() {
            d.hit(id, index, slot, HitKind::Slot);
        }
    }
}

/// Inventory slots: as many as the model has (up to the grid), all clickable, empty or not.
pub(super) fn slot_grid(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::SlotGrid {
        id,
        anchor,
        at,
        columns,
        rows,
    } = node
    else {
        return;
    };
    let rect = parent.place(
        *anchor,
        d.ctx.offset(id, *at),
        (columns * SLOT, rows * SLOT),
    );
    let items = d.model.items(id);
    let marks = d.model.marks(id);
    let count = items.len().min((columns * rows) as usize);
    for (index, &item) in items.iter().enumerate().take(count) {
        let (cx, cy) = (
            (index as u32 % columns) * SLOT,
            (index as u32 / columns) * SLOT,
        );
        let slot = Rect::new(rect.x + cx as i32, rect.y + cy as i32, SLOT, SLOT);
        let focused = d.is_focused(id, index);
        draw_slot(d, slot, item, focused, false);
        draw_mark(d, slot, marks.get(index).copied().unwrap_or_default());
        d.hit(id, index, slot, HitKind::Slot);
    }
}

/// Count at the bottom right, wear bar at the bottom, a lock frame, a veil when dimmed.
pub(super) fn draw_mark(d: &mut Draw<'_, '_>, slot: Rect, mark: SlotMark) {
    let theme = d.theme();
    let (text, shadow, accent, shade) = (
        theme.text.0,
        theme.text_shadow.0,
        theme.accent.0,
        theme.shade.0,
    );
    if mark.locked {
        let inner = Rect::new(slot.x + 1, slot.y + 1, slot.w - 2, slot.h - 2);
        d.painter.bevel(inner, accent, accent);
    }
    if let Some(wear) = mark.wear.filter(|&w| w < 1.0) {
        let width = ((slot.w - 6) as f32 * wear.clamp(0.0, 1.0)).round() as u32;
        d.painter.fill(
            Rect::new(slot.x + 3, slot.y + slot.h as i32 - 4, slot.w - 6, 1),
            shadow,
        );
        d.painter.fill(
            Rect::new(slot.x + 3, slot.y + slot.h as i32 - 4, width, 1),
            accent,
        );
    }
    if mark.count > 1 {
        let label = mark.count.to_string();
        let width = d.painter.text_width(&label) as i32;
        let height = d.painter.line_height() as i32;
        let (x, y) = (
            slot.x + slot.w as i32 - 1 - width,
            slot.y + slot.h as i32 - height,
        );
        d.painter.text(x, y, &label, text, shadow);
    }
    if mark.dim {
        d.painter.fill(slot, shade);
    }
}

/// A row of slots (the hotbar). In the HUD editor the whole row is one element.
pub(super) fn slots(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::Slots {
        id,
        anchor,
        at,
        count,
    } = node
    else {
        return;
    };
    let rect = parent.place(*anchor, d.ctx.offset(id, *at), (count * SLOT, SLOT));
    let items = d.model.items(id);
    let marks = d.model.marks(id);
    let selected = d.model.selected(id);
    for index in 0..*count as usize {
        let slot = Rect::new(rect.x + (index as u32 * SLOT) as i32, rect.y, SLOT, SLOT);
        let item = items.get(index).copied().flatten();
        let focused = d.is_focused(id, index);
        draw_slot(d, slot, item, focused, selected == Some(index));
        draw_mark(d, slot, marks.get(index).copied().unwrap_or_default());
        if !d.ctx.editing {
            d.hit(id, index, slot, HitKind::Slot);
        }
    }
    if d.ctx.editing {
        d.element(id, rect);
    }
}

pub(super) fn crosshair(d: &mut Draw<'_, '_>, node: &Node, parent: Rect) {
    let Node::Crosshair { id, anchor, at } = node else {
        return;
    };
    let center = parent.place(*anchor, d.ctx.offset(id, *at), (1, 1));
    let (shadow, color) = (d.theme().text_shadow.0, d.theme().text.0);
    d.painter
        .fill(Rect::new(center.x - 4, center.y, 9, 1), shadow);
    d.painter
        .fill(Rect::new(center.x, center.y - 4, 1, 9), shadow);
    d.painter
        .fill(Rect::new(center.x - 4, center.y - 1, 9, 1), color);
    d.painter
        .fill(Rect::new(center.x - 1, center.y - 4, 1, 9), color);
    if d.ctx.editing {
        d.element(id, Rect::new(center.x - 5, center.y - 5, 11, 11));
    }
}

fn draw_slot(
    d: &mut Draw<'_, '_>,
    slot: Rect,
    item: Option<ItemId>,
    focused: bool,
    selected: bool,
) {
    let theme = d.theme();
    let (fill, dark, border) = (theme.slot.0, theme.panel_dark.0, theme.slot_border.0);
    let frame = if selected {
        theme.accent.0
    } else {
        theme.focus.0
    };
    let inner = Rect::new(slot.x + 1, slot.y + 1, slot.w - 2, slot.h - 2);
    d.painter.fill(inner, fill);
    d.painter.bevel(inner, dark, border);
    if let Some(icon) = item.and_then(|item| d.painter.atlas.icon(item)) {
        d.painter
            .image(Rect::new(slot.x + 2, slot.y + 2, 16, 16), icon);
    }
    if selected || focused {
        d.painter.bevel(slot, frame, frame);
    }
}

/// The stack carried in an inventory screen, drawn under the pointer.
pub(super) fn carried(d: &mut Draw<'_, '_>, (x, y): (i32, i32), item: ItemId, count: u16) {
    let slot = Rect::new(x - SLOT as i32 / 2, y - SLOT as i32 / 2, SLOT, SLOT);
    if let Some(icon) = d.painter.atlas.icon(item) {
        d.painter
            .image(Rect::new(slot.x + 2, slot.y + 2, 16, 16), icon);
    }
    let mark = SlotMark {
        count,
        ..SlotMark::default()
    };
    draw_mark(d, slot, mark);
}
