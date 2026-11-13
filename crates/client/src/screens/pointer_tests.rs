#![allow(clippy::unwrap_used)]

use super::*;
use crate::input::{Binding, Bindings};
use crate::screens::console::ConsoleLog;
use crate::screens::creation::GenerationSources;
use crate::screens::menu::{self, Menu, Shared};
use crate::screens::options::OptionsState;
use crate::settings::Settings;
use crate::ui::{Painter, Rect, UiAssets, UiAtlas, UiContext, draw_screen};
use std::path::Path;
use strata_core::pack::PackSet;
use winit::keyboard::KeyCode;

const FRAME: f32 = 1.0 / 60.0;
/// Rows of the sound tab.
const MASTER_VOLUME: usize = 0;
const SUBTITLES: usize = 5;

struct Fixture {
    content: Content,
    ui: UiAssets,
    atlas: UiAtlas,
    settings: Settings,
    console: ConsoleLog,
    generation: GenerationSources,
    menu: Menu,
}

fn sound_options() -> Fixture {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let mut errors = Vec::new();
    let packs = PackSet::discover(&assets.join("packs"), &mut errors);
    let ui = UiAssets::load(&assets.join("ui"), &packs, &mut errors).unwrap();
    let (content, _) = Content::load(&packs, &PackSet::default());
    let atlas = UiAtlas::build(&content, ui.font.font(1));
    Fixture {
        content,
        ui,
        atlas,
        settings: Settings::default(),
        console: ConsoleLog::default(),
        generation: GenerationSources::default(),
        menu: Menu::Options(OptionsState {
            tab: 1,
            ..OptionsState::default()
        }),
    }
}

impl Fixture {
    fn with_model<R>(&mut self, f: impl FnOnce(&mut dyn ScreenModel) -> R) -> R {
        let key_labels = |_| (String::new(), String::new());
        let shared = Shared {
            content: &self.content,
            settings: &mut self.settings,
            worlds: &[],
            icons: &[],
            hotbar: None,
            focus: None,
            key_labels: &key_labels,
            newer_version: None,
            console: &self.console,
            packs: &[],
            generation: &self.generation,
            effects: &mut Vec::new(),
        };
        let mut model = menu::model(&mut self.menu, shared);
        f(model.as_mut())
    }

    /// The options row `index` as drawn on the smallest supported screen.
    fn row(&mut self, index: usize) -> Hit {
        let key_label = |name: &str| name.to_owned();
        let ctx = UiContext {
            theme: &self.ui.theme,
            translations: &self.content.translations,
            language: "fr",
            key_label: &key_label,
            offsets: None,
            editing: false,
        };
        let def = self.menu.def(&self.ui.screens);
        let key_labels = |_| (String::new(), String::new());
        let shared = Shared {
            content: &self.content,
            settings: &mut self.settings,
            worlds: &[],
            icons: &[],
            hotbar: None,
            focus: None,
            key_labels: &key_labels,
            newer_version: None,
            console: &self.console,
            packs: &[],
            generation: &self.generation,
            effects: &mut Vec::new(),
        };
        let model = menu::model(&mut self.menu, shared);
        let mut painter = Painter::new(&self.atlas, 1);
        let screen = Rect::new(0, 0, 320, 220);
        draw_screen(def, &mut painter, model.as_ref(), &ctx, screen, None)
            .into_iter()
            .find(|hit| hit.id == "options" && hit.index == index)
            .unwrap()
    }
}

#[test]
fn a_click_in_the_middle_of_a_gauge_then_a_drag_past_its_end() {
    let mut f = sound_options();
    let row = f.row(MASTER_VOLUME);
    let gauge = row_zones(row.rect).gauge;
    let middle = (gauge.x + (gauge.w as i32 - 1) / 2, gauge.y + 1);
    let click = f.with_model(|m| click_value(m, &row, middle));
    assert_eq!(f.settings.sound.master, 50);
    let Click::Drag(track) = click else {
        panic!("a gauge click starts a drag: {click:?}");
    };
    f.with_model(|m| drag_value(m, &track, gauge.x + 3 * gauge.w as i32));
    assert_eq!(f.settings.sound.master, 100, "past the end: the maximum");
    f.with_model(|m| drag_value(m, &track, -40));
    assert_eq!(f.settings.sound.master, 0);
}

#[test]
fn arrows_and_wheel_step_a_choice_row() {
    let mut f = sound_options();
    let row = f.row(SUBTITLES);
    let zones = row_zones(row.rect);
    let before = f.settings.sound.subtitles;
    let next = (zones.next.center().0, row.rect.y + 2);
    assert_eq!(f.with_model(|m| click_value(m, &row, next)), Click::Stepped);
    assert_ne!(f.settings.sound.subtitles, before);
    assert!(f.with_model(|m| wheel_value(m, &row, zones.value.center(), 1)));
    assert_eq!(f.settings.sound.subtitles, before);
    let label = (row.rect.x + 4, row.rect.y + 2);
    assert!(!f.with_model(|m| wheel_value(m, &row, label, 1)), "scrolls");
    assert_eq!(
        f.with_model(|m| click_value(m, &row, label)),
        Click::Ignored
    );
}

#[test]
fn only_gauges_and_sliders_repeat() {
    let mut f = sound_options();
    let (gauge, toggle) = (f.row(MASTER_VOLUME), f.row(SUBTITLES));
    assert!(f.with_model(|m| repeats(m, &gauge)));
    assert!(!f.with_model(|m| repeats(m, &toggle)));
    let slider = Hit {
        kind: HitKind::Slider,
        ..toggle
    };
    assert!(f.with_model(|m| repeats(m, &slider)));
}

fn press(input: &mut InputState, key: KeyCode, pressed: bool) {
    input.set_binding(Binding::Key(key), pressed);
}

/// Steps given by holding → for `seconds` on `focus`.
fn hold_right(
    repeat: &mut ValueRepeat,
    input: &mut InputState,
    focus: &Hit,
    seconds: f32,
    repeats: bool,
) -> i32 {
    let frames = (seconds / FRAME).round() as usize;
    let mut steps = 0;
    for _ in 0..frames {
        steps += repeat.steps(input, FRAME, focus, repeats);
        input.end_frame();
    }
    steps
}

#[test]
fn held_right_repeats_on_a_gauge_but_steps_once_on_a_toggle() {
    let mut f = sound_options();
    let (gauge, toggle) = (f.row(MASTER_VOLUME), f.row(SUBTITLES));
    let mut input = InputState::new(Bindings::default());
    let mut repeat = ValueRepeat::default();
    press(&mut input, KeyCode::ArrowRight, true);
    assert!(hold_right(&mut repeat, &mut input, &gauge, 1.0, true) > 3);
    press(&mut input, KeyCode::ArrowRight, false);
    assert_eq!(
        hold_right(&mut repeat, &mut input, &gauge, 1.0, true),
        0,
        "released"
    );
    press(&mut input, KeyCode::ArrowRight, true);
    assert_eq!(hold_right(&mut repeat, &mut input, &toggle, 1.0, false), 1);
}

#[test]
fn moving_the_focus_restarts_the_delay() {
    let mut f = sound_options();
    let (first, second) = (f.row(MASTER_VOLUME), f.row(MASTER_VOLUME + 1));
    let mut input = InputState::new(Bindings::default());
    let mut repeat = ValueRepeat::default();
    press(&mut input, KeyCode::ArrowRight, true);
    hold_right(&mut repeat, &mut input, &first, 2.0, true);
    let on_second = hold_right(&mut repeat, &mut input, &second, 0.2, true);
    assert_eq!(on_second, 0, "still within the delay");
}
