//! Draws a described screen with its model and records the interactive elements.

use super::def::{Node, SLOT, ScreenDef};
use super::model::{Hit, ScreenModel};
use super::painter::Painter;
use super::rect::Rect;
use super::theme::Theme;
use strata_core::lang::Translations;

/// Everything needed to turn keys and colors into pixels.
pub struct UiContext<'a> {
    pub theme: &'a Theme,
    pub translations: &'a Translations,
    pub language: &'a str,
}

impl UiContext<'_> {
    fn tr(&self, key: &str) -> String {
        self.translations.text(self.language, key)
    }
}

/// Draws `def` inside `screen`; returns the interactive elements in drawing order.
pub fn draw_screen(
    def: &ScreenDef,
    painter: &mut Painter<'_>,
    model: &dyn ScreenModel,
    ctx: &UiContext<'_>,
    screen: Rect,
    focus: Option<&Hit>,
) -> Vec<Hit> {
    let mut hits = Vec::new();
    for node in &def.root {
        draw_node(node, painter, model, ctx, screen, focus, &mut hits);
    }
    if let Some(focused) = focus
        && let Some(text) = model.tooltip(&focused.id, focused.index)
    {
        draw_tooltip(painter, ctx, focused.rect, &text, screen);
    }
    hits
}

fn draw_node(
    node: &Node,
    painter: &mut Painter<'_>,
    model: &dyn ScreenModel,
    ctx: &UiContext<'_>,
    parent: Rect,
    focus: Option<&Hit>,
    hits: &mut Vec<Hit>,
) {
    let theme = ctx.theme;
    let is_focused = |id: &str, index: usize| focus.is_some_and(|f| f.id == id && f.index == index);
    match node {
        Node::Shade => painter.fill(parent, theme.shade.0),
        Node::Panel {
            anchor,
            at,
            size,
            children,
            ..
        } => {
            let rect = parent.place(*anchor, *at, *size);
            painter.fill(rect, theme.panel.0);
            painter.bevel(rect, theme.panel_light.0, theme.panel_dark.0);
            for child in children {
                draw_node(child, painter, model, ctx, rect, focus, hits);
            }
        }
        Node::Label {
            anchor,
            at,
            text,
            boxed,
            dim,
            ..
        } => {
            let content = match text.strip_prefix('@') {
                Some(binding) => model.text(binding).unwrap_or_default(),
                None => ctx.tr(text),
            };
            if content.is_empty() {
                return;
            }
            let size = (
                painter.text_width(&content) + 2 * u32::from(*boxed) * 4,
                painter.line_height() + u32::from(*boxed) * 4,
            );
            let rect = parent.place(*anchor, *at, size);
            if *boxed {
                painter.fill(rect, theme.panel.0);
                painter.bevel(rect, theme.panel_light.0, theme.panel_dark.0);
            }
            let inset = 4 * i32::from(*boxed);
            let color = if *dim { theme.text_dim.0 } else { theme.text.0 };
            painter.text(
                rect.x + inset,
                rect.y + inset / 2,
                &content,
                color,
                theme.text_shadow.0,
            );
        }
        Node::Button {
            id,
            anchor,
            at,
            size,
            text,
        } => {
            let rect = parent.place(*anchor, *at, *size);
            let focused = is_focused(id, 0);
            painter.fill(
                rect,
                if focused {
                    theme.panel_light.0
                } else {
                    theme.panel.0
                },
            );
            painter.bevel(rect, theme.panel_light.0, theme.panel_dark.0);
            let label = ctx.tr(text);
            let (cx, cy) = rect.center();
            let w = painter.text_width(&label) as i32;
            let color = if focused {
                theme.accent.0
            } else {
                theme.text.0
            };
            painter.text(
                cx - w / 2,
                cy - painter.line_height() as i32 / 2,
                &label,
                color,
                theme.text_shadow.0,
            );
            hits.push(Hit {
                id: id.clone(),
                index: 0,
                rect,
            });
        }
        Node::TextField {
            id,
            anchor,
            at,
            size,
            placeholder,
        } => {
            let rect = parent.place(*anchor, *at, *size);
            painter.fill(rect, theme.field.0);
            painter.bevel(rect, theme.panel_dark.0, theme.panel_light.0);
            let content = model.text(id).unwrap_or_default();
            let focused = is_focused(id, 0);
            let (text, color) = if content.is_empty() && !focused {
                (ctx.tr(placeholder), theme.text_dim.0)
            } else {
                (
                    format!("{content}{}", if focused { "_" } else { "" }),
                    theme.text.0,
                )
            };
            painter.text(rect.x + 3, rect.y + 1, &text, color, theme.text_shadow.0);
            hits.push(Hit {
                id: id.clone(),
                index: 0,
                rect,
            });
        }
        Node::Tabs {
            id,
            anchor,
            at,
            size,
        } => draw_tabs(
            painter,
            model,
            ctx,
            parent.place(*anchor, *at, *size),
            id,
            focus,
            hits,
        ),
        Node::ItemGrid {
            id,
            anchor,
            at,
            columns,
            rows,
        } => {
            let rect = parent.place(*anchor, *at, (columns * SLOT, rows * SLOT));
            let items = model.items(id);
            for index in 0..(columns * rows) as usize {
                let (cx, cy) = (
                    (index as u32 % columns) * SLOT,
                    (index as u32 / columns) * SLOT,
                );
                let slot = Rect::new(rect.x + cx as i32, rect.y + cy as i32, SLOT, SLOT);
                let item = items.get(index).copied().flatten();
                draw_slot(painter, theme, slot, item, is_focused(id, index), false);
                if item.is_some() {
                    hits.push(Hit {
                        id: id.clone(),
                        index,
                        rect: slot,
                    });
                }
            }
        }
        Node::Slots {
            id,
            anchor,
            at,
            count,
        } => {
            let rect = parent.place(*anchor, *at, (count * SLOT, SLOT));
            let items = model.items(id);
            let selected = model.selected(id);
            for index in 0..*count as usize {
                let slot = Rect::new(rect.x + (index as u32 * SLOT) as i32, rect.y, SLOT, SLOT);
                let item = items.get(index).copied().flatten();
                draw_slot(
                    painter,
                    theme,
                    slot,
                    item,
                    is_focused(id, index),
                    selected == Some(index),
                );
                hits.push(Hit {
                    id: id.clone(),
                    index,
                    rect: slot,
                });
            }
        }
        Node::Crosshair { anchor, at, .. } => {
            let center = parent.place(*anchor, *at, (1, 1));
            let shadow = theme.text_shadow.0;
            painter.fill(Rect::new(center.x - 4, center.y, 9, 1), shadow);
            painter.fill(Rect::new(center.x, center.y - 4, 1, 9), shadow);
            painter.fill(Rect::new(center.x - 4, center.y - 1, 9, 1), theme.text.0);
            painter.fill(Rect::new(center.x - 1, center.y - 4, 1, 9), theme.text.0);
        }
    }
}

fn draw_slot(
    painter: &mut Painter<'_>,
    theme: &Theme,
    slot: Rect,
    item: Option<strata_core::content::ItemId>,
    focused: bool,
    selected: bool,
) {
    let inner = Rect::new(slot.x + 1, slot.y + 1, slot.w - 2, slot.h - 2);
    painter.fill(inner, theme.slot.0);
    painter.bevel(inner, theme.panel_dark.0, theme.slot_border.0);
    if let Some(icon) = item.and_then(|item| painter.atlas.icon(item)) {
        painter.image(Rect::new(slot.x + 2, slot.y + 2, 16, 16), icon);
    }
    if selected || focused {
        let color = if selected {
            theme.accent.0
        } else {
            theme.focus.0
        };
        painter.bevel(slot, color, color);
    }
}

fn draw_tabs(
    painter: &mut Painter<'_>,
    model: &dyn ScreenModel,
    ctx: &UiContext<'_>,
    rect: Rect,
    id: &str,
    focus: Option<&Hit>,
    hits: &mut Vec<Hit>,
) {
    let theme = ctx.theme;
    let selected = model.selected(id);
    let mut x = rect.x;
    for (index, title) in model.tabs(id).iter().enumerate() {
        let width = painter.text_width(title) + 8;
        let tab = Rect::new(x, rect.y, width, rect.h);
        let active = selected == Some(index);
        painter.fill(
            tab,
            if active {
                theme.panel_light.0
            } else {
                theme.panel_dark.0
            },
        );
        let focused = focus.is_some_and(|f| f.id == id && f.index == index);
        let color = if active || focused {
            theme.accent.0
        } else {
            theme.text.0
        };
        painter.text(x + 4, rect.y + 1, title, color, theme.text_shadow.0);
        if focused {
            painter.bevel(tab, theme.focus.0, theme.focus.0);
        }
        hits.push(Hit {
            id: id.to_owned(),
            index,
            rect: tab,
        });
        x += width as i32 + 2;
    }
}

fn draw_tooltip(
    painter: &mut Painter<'_>,
    ctx: &UiContext<'_>,
    anchor: Rect,
    text: &str,
    screen: Rect,
) {
    let theme = ctx.theme;
    let w = painter.text_width(text) + 8;
    let h = painter.line_height() + 3;
    let x = (anchor.x + anchor.w as i32 + 2).min(screen.x + screen.w as i32 - w as i32);
    let y = (anchor.y - h as i32).max(screen.y);
    let rect = Rect::new(x, y, w, h);
    painter.fill(rect, theme.panel_dark.0);
    painter.bevel(rect, theme.accent.0, theme.accent.0);
    painter.text(x + 4, y + 1, text, theme.text.0, theme.text_shadow.0);
}
