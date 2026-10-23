//! Draws a described screen with its model and records the interactive elements.

mod controls;
mod items;
mod list;
mod text;

use super::def::{Node, ScreenDef};
use super::model::{Hit, HitKind, ScreenModel};
use super::painter::Painter;
use super::rect::Rect;
use super::theme::Theme;
use std::collections::BTreeMap;
use strata_core::lang::Translations;

/// Everything needed to turn keys and colors into pixels.
#[derive(Clone, Copy)]
pub struct UiContext<'a> {
    pub theme: &'a Theme,
    pub translations: &'a Translations,
    pub language: &'a str,
    /// What to show for `<key:Action>`: the bound key or button, per last device.
    pub key_label: &'a dyn Fn(&str) -> String,
    /// Positions chosen in the HUD editor, by element id.
    pub offsets: Option<&'a BTreeMap<String, (i32, i32)>>,
    /// HUD editor: every element with an id can be selected and moved.
    pub editing: bool,
}

impl UiContext<'_> {
    fn tr(&self, key: &str) -> String {
        self.translations.text(self.language, key)
    }

    fn offset(&self, id: &str, at: (i32, i32)) -> (i32, i32) {
        self.offsets
            .and_then(|offsets| offsets.get(id).copied())
            .unwrap_or(at)
    }
}

/// One drawing pass: where to draw, what with, and the interactive elements found.
struct Draw<'a, 'p> {
    painter: &'a mut Painter<'p>,
    model: &'a dyn ScreenModel,
    ctx: &'a UiContext<'a>,
    focus: Option<&'a Hit>,
    hits: Vec<Hit>,
}

impl Draw<'_, '_> {
    fn is_focused(&self, id: &str, index: usize) -> bool {
        self.focus.is_some_and(|f| f.id == id && f.index == index)
    }

    fn hit(&mut self, id: &str, index: usize, rect: Rect, kind: HitKind) {
        self.hits.push(Hit {
            id: id.to_owned(),
            index,
            rect,
            kind,
        });
    }

    /// A HUD element in the editor: selectable, framed when selected.
    fn element(&mut self, id: &str, rect: Rect) {
        if self.is_focused(id, 0) {
            let accent = self.theme().accent.0;
            self.painter.bevel(rect, accent, accent);
        }
        self.hit(id, 0, rect, HitKind::Element);
    }

    fn theme(&self) -> &Theme {
        self.ctx.theme
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
    let mut draw = Draw {
        painter,
        model,
        ctx,
        focus,
        hits: Vec::new(),
    };
    for node in &def.root {
        draw_node(node, screen, &mut draw);
    }
    if let Some(focused) = focus
        && let Some(text) = model.tooltip(&focused.id, focused.index)
    {
        controls::tooltip(&mut draw, focused.rect, &text, screen);
    }
    draw.hits
}

fn draw_node(node: &Node, parent: Rect, d: &mut Draw<'_, '_>) {
    match node {
        Node::Shade => {
            let shade = d.theme().shade.0;
            d.painter.fill(parent, shade);
        }
        Node::Panel {
            id,
            anchor,
            at,
            size,
            children,
        } => {
            let rect = parent.place(*anchor, d.ctx.offset(id, *at), *size);
            let theme = d.theme();
            let (fill, light, dark) = (theme.panel.0, theme.panel_light.0, theme.panel_dark.0);
            d.painter.fill(rect, fill);
            d.painter.bevel(rect, light, dark);
            for child in children {
                draw_node(child, rect, d);
            }
        }
        Node::Label { .. } => controls::label(d, node, parent),
        Node::Button { .. } => controls::button(d, node, parent),
        Node::TextField { .. } => controls::text_field(d, node, parent),
        Node::Slider { .. } => controls::slider(d, node, parent),
        Node::Tabs { .. } => controls::tabs(d, node, parent),
        Node::KeyGrid { .. } => controls::key_grid(d, node, parent),
        Node::List { .. } => list::list(d, node, parent),
        Node::OptionList { .. } => list::options(d, node, parent),
        Node::ItemGrid { .. } => items::grid(d, node, parent),
        Node::Slots { .. } => items::slots(d, node, parent),
        Node::Crosshair { .. } => items::crosshair(d, node, parent),
        Node::Log { .. } => list::log(d, node, parent),
    }
}
