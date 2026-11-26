//! Drawing of the interface: HUD (or cinematic bands), the open menu, the on-screen
//! keyboard and the F3 overlay, in one batch of quads.

use super::hud::{
    Bars, HudModel, aimed_block_name, effect_line, player_bars, sample_subtitles, subtitle_lines,
};
use super::hud_temperature::{
    edge_strength, paint_edges, temperature_hint, temperature_line, temperature_text,
};
use super::keyboard::KeyboardModel;
use super::menu::{self, Menu, Shared};
use super::{MAX_AUTO_SCALE, Screens, UiFrame};
use crate::gameplay::Gameplay;
use crate::input::{Action, InputDevice};
use crate::inventory::ClientInventory;
use crate::render::Gpu;
use crate::ui::{Node, Rect, ScreenDef, UiContext, auto_scale, draw_screen};
use strata_core::content::Content;

impl Screens {
    /// Draws the HUD, the menu on top and the on-screen keyboard over the frame.
    pub fn draw(
        &mut self,
        gpu: &Gpu,
        frame: UiFrame<'_>,
        content: &Content,
        game: Option<&mut Gameplay>,
    ) {
        let UiFrame {
            encoder,
            view: target,
            size,
            debug,
            subtitles,
        } = frame;
        let fitting = auto_scale(size.0, size.1, MAX_AUTO_SCALE);
        self.scale = match self.settings.interface.scale {
            0 => fitting,
            chosen => chosen.min(auto_scale(size.0, size.1, u32::MAX)),
        };
        self.refresh_font(gpu, content);
        self.upload_preview(gpu);
        let screen = Rect::new(0, 0, size.0 / self.scale, size.1 / self.scale);
        let editor = matches!(self.stack.last(), Some(Menu::HudEditor { .. }));
        let language = self.settings.interface.language.clone();
        let labels = &self.key_labels;
        let pad = self.device == InputDevice::Gamepad;
        let key_label = |name: &str| {
            Action::from_name(name)
                .and_then(|action| labels.get(action.index()))
                .map(|(keys, buttons)| if pad { buttons } else { keys }.clone())
                .unwrap_or_else(|| name.to_owned())
        };
        let key_labels = |action: Action| labels.get(action.index()).cloned().unwrap_or_default();
        let mut painter = crate::ui::Painter::new(&self.atlas, self.scale);
        let ctx = UiContext {
            theme: &self.assets.theme,
            translations: &content.translations,
            language: &language,
            key_label: &key_label,
            offsets: None,
            editing: false,
            pointer: self.pointer,
        };
        let mut hits = Vec::new();
        let shot = game.as_deref().and_then(|g| g.cinematic.as_ref());
        if let Some(playing) = shot {
            let subtitle = playing
                .subtitle()
                .map(|key| content.translations.text(&language, key));
            crate::cinematic::paint(
                playing,
                subtitle.as_deref(),
                &mut painter,
                screen,
                ctx.theme,
            );
        } else if (self.in_world && self.stack.is_empty()) || editor {
            let sample = ClientInventory::default();
            let hud = match game.as_deref() {
                Some(game) => HudModel {
                    hotbar: &game.inventory,
                    content,
                    area: crate::hammer::area_line(&game.inventory, content, &language),
                    target: aimed_block_name(game, content, &language),
                    mount: game
                        .is_riding()
                        .then(|| content.translations.text(&language, "hud-dismount")),
                    subtitles: subtitle_lines(subtitles, content, &language),
                    bars: player_bars(game, content, self.settings.interface.show_empty_mana),
                    effects: effect_line(game, content, &language),
                    notice: (game.vitals.fainted > 0.0)
                        .then(|| content.translations.text(&language, "hud-fainted")),
                    temperature: temperature_line(game, content, &language),
                    temperature_hint: temperature_hint(
                        game,
                        content,
                        &language,
                        &self.assets.theme,
                    ),
                },
                None => HudModel {
                    hotbar: &sample,
                    content,
                    area: None,
                    target: Some(content.translations.text(&language, "ui-editor-sample")),
                    mount: Some(content.translations.text(&language, "hud-dismount")),
                    subtitles: subtitle_lines(&sample_subtitles(), content, &language),
                    bars: Bars {
                        sample: true,
                        ..Bars::default()
                    },
                    effects: Some(content.translations.text(&language, "hud-effects-sample")),
                    notice: None,
                    temperature: temperature_text(&sample_temperature(), content, &language),
                    temperature_hint: None,
                },
            };
            if let Some(game) = game.as_deref()
                && self.settings.interface.temperature_edges
                && !game.is_creative()
            {
                let theme = &self.assets.theme;
                let reduced = self.settings.interface.reduce_camera_effects;
                let strength = edge_strength(game.vitals.edges, theme, reduced);
                paint_edges(&mut painter, screen, theme, game.vitals.edges, strength);
            }
            if let Some(game) = game.as_deref()
                && self.settings.interface.hurt_flash
            {
                paint_flash(
                    &mut painter,
                    screen,
                    &self.assets.theme,
                    game.vitals.flash(),
                );
            }
            let hud_ctx = UiContext {
                offsets: Some(&self.settings.interface.hud),
                editing: editor,
                ..ctx
            };
            let focus = self.focus.as_ref().filter(|_| editor);
            let def = &self.assets.screens.hud;
            hits = draw_screen(def, &mut painter, &hud, &hud_ctx, screen, focus);
        }
        crate::debug_overlay::paint(debug, &mut painter, &self.assets.theme);
        if let Some(menu) = self.stack.last_mut() {
            let parts = menu::game_parts(game);
            let shared = Shared {
                content,
                settings: &mut self.settings,
                worlds: &self.worlds,
                icons: &self.icons,
                inventory: parts.inventory,
                container: parts.container,
                crafting: parts.crafting,
                focus: self.focus.as_ref().map(|f| (f.id.clone(), f.index)),
                key_labels: &key_labels,
                newer_version: self.newer_version.as_deref(),
                console: &self.console,
                packs: &self.available_packs,
                generation: &self.generation,
                effects: &mut Vec::new(),
            };
            let def = menu.def(&self.assets.screens);
            self.window = window_rect(def, screen);
            let model = menu::model(menu, shared);
            let focus = self.focus.as_ref();
            hits.extend(draw_screen(
                def,
                &mut painter,
                model.as_ref(),
                &ctx,
                screen,
                focus,
            ));
        }
        self.hits = hits;
        if let Some(keyboard) = self.keyboard.as_mut() {
            let def = &self.assets.screens.keyboard;
            let (mut typed, mut done) = (String::new(), false);
            let model = KeyboardModel {
                keyboard,
                rows: key_rows(def),
                translations: &content.translations,
                language: &language,
                typed: &mut typed,
                done: &mut done,
            };
            let focus = self.focus.as_ref();
            self.keyboard_hits = draw_screen(def, &mut painter, &model, &ctx, screen, focus);
        }
        self.pipeline.draw(
            gpu,
            encoder,
            target,
            size,
            &painter.vertices,
            &painter.indices,
        );
    }
}

/// What the HUD editor shows for the temperature line.
fn sample_temperature() -> strata_protocol::TemperatureView {
    strata_protocol::TemperatureView {
        felt: 3.0,
        zone: strata_core::survival::temperature::TemperatureZone::Cold,
        wet: false,
    }
}

/// The red flash over the screen after a hit, fading.
fn paint_flash(
    painter: &mut crate::ui::Painter<'_>,
    screen: Rect,
    theme: &crate::ui::Theme,
    strength: f32,
) {
    if strength <= 0.0 {
        return;
    }
    let mut color = theme.hurt.0;
    color.0[3] = (f32::from(color.0[3]) * strength.min(1.0)).round() as u8;
    painter.fill(screen, color);
}

/// Where the panel named `window` of a screen lies, with the panels beside it
/// (`window_…`, in groups): clicking there is not clicking outside.
fn window_rect(def: &ScreenDef, screen: Rect) -> Option<Rect> {
    fn panels<'a>(nodes: &'a [Node], found: &mut Vec<&'a Node>) {
        for node in nodes {
            match node {
                Node::Panel { id, .. } if id.starts_with("window") => found.push(node),
                Node::Group { children, .. } => panels(children, found),
                _ => {}
            }
        }
    }
    let mut found = Vec::new();
    panels(&def.root, &mut found);
    let rects = found.into_iter().filter_map(|node| match node {
        Node::Panel {
            anchor, at, size, ..
        } => Some(screen.place(*anchor, *at, *size)),
        _ => None,
    });
    rects.reduce(|a, b| {
        let (x, y) = (a.x.min(b.x), a.y.min(b.y));
        let right = (a.x + a.w as i32).max(b.x + b.w as i32);
        let bottom = (a.y + a.h as i32).max(b.y + b.h as i32);
        Rect::new(x, y, (right - x) as u32, (bottom - y) as u32)
    })
}

/// Rows of the first key grid of the keyboard layout.
pub(super) fn key_rows(def: &ScreenDef) -> &[String] {
    fn find(nodes: &[Node]) -> Option<&[String]> {
        nodes.iter().find_map(|node| match node {
            Node::KeyGrid { rows, .. } => Some(rows.as_slice()),
            Node::Panel { children, .. } => find(children),
            _ => None,
        })
    }
    find(&def.root).unwrap_or(&[])
}
