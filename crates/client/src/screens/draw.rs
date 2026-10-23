//! Drawing of the interface: HUD (or cinematic bands), the open menu, the on-screen
//! keyboard and the F3 overlay, in one batch of quads.

use super::hud::{HudModel, aimed_block_name};
use super::keyboard::KeyboardModel;
use super::menu::{self, Menu, Shared};
use super::{MAX_AUTO_SCALE, Screens, UiFrame};
use crate::gameplay::Gameplay;
use crate::hotbar::Hotbar;
use crate::input::{Action, InputDevice};
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
        } = frame;
        let fitting = auto_scale(size.0, size.1, MAX_AUTO_SCALE);
        self.scale = match self.settings.interface.scale {
            0 => fitting,
            chosen => chosen.min(auto_scale(size.0, size.1, u32::MAX)),
        };
        self.refresh_font(gpu, content);
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
            let sample = Hotbar::default();
            let hud = match game.as_deref() {
                Some(game) => HudModel {
                    hotbar: &game.hotbar,
                    target: aimed_block_name(game, content, &language),
                },
                None => HudModel {
                    hotbar: &sample,
                    target: Some(content.translations.text(&language, "ui-editor-sample")),
                },
            };
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
            let shared = Shared {
                content,
                settings: &mut self.settings,
                worlds: &self.worlds,
                icons: &self.icons,
                hotbar: game.map(|g| &mut g.hotbar),
                focus: self.focus.as_ref().map(|f| (f.id.clone(), f.index)),
                key_labels: &key_labels,
                newer_version: self.newer_version.as_deref(),
                console: &self.console,
                packs: &self.available_packs,
                effects: &mut Vec::new(),
            };
            let def = menu.def(&self.assets.screens);
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
