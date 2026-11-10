#![allow(clippy::unwrap_used)]

use super::*;
use crate::generation::WorldTypeChoice;
use crate::screens::console::ConsoleLog;
use crate::screens::menu::{self, Menu};
use crate::settings::Settings;
use crate::ui::{
    Direction, Hit, Painter, Rect, UiAssets, UiAtlas, UiContext, draw_screen, navigate,
};
use std::collections::BTreeSet;
use std::path::Path;
use strata_core::generation::{FlatLayer, FlatSettings};
use strata_core::pack::PackSet;

fn id(text: &str) -> Identifier {
    text.parse().unwrap()
}

fn sources() -> GenerationSources {
    let flat = FlatSettings {
        layers: vec![
            FlatLayer {
                block: id("strata:bedrock"),
                thickness: 1,
            },
            FlatLayer {
                block: id("strata:dirt"),
                thickness: 3,
            },
        ],
        caves: false,
        features: false,
    };
    let choice = |name: &str, biome: Option<&str>, flat: Option<FlatSettings>| WorldTypeChoice {
        id: id(name),
        biome: biome.map(id),
        flat,
    };
    GenerationSources {
        catalog: Arc::new(GenerationCatalog {
            types: vec![
                choice("strata:normal", None, None),
                choice("strata:flat", Some("strata:plains"), Some(flat)),
                choice("strata:single_biome", Some("strata:plains"), None),
            ],
            biomes: vec![id("strata:plains"), id("strata:desert")],
            sea_level: 120,
        }),
        maps: None,
    }
}

struct Fixture {
    content: Content,
    ui: UiAssets,
    atlas: UiAtlas,
    settings: Settings,
    console: ConsoleLog,
    generation: GenerationSources,
}

fn fixture() -> Fixture {
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
        generation: sources(),
    }
}

impl Fixture {
    /// Draws `menu` on the smallest supported screen; returns its interactive elements.
    fn hits(&mut self, menu: &mut Menu) -> Vec<Hit> {
        let key_labels = |_| (String::new(), String::new());
        let key_label = |name: &str| name.to_owned();
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
        let ctx = UiContext {
            theme: &self.ui.theme,
            translations: &self.content.translations,
            language: "fr",
            key_label: &key_label,
            offsets: None,
            editing: false,
        };
        let def = menu.def(&self.ui.screens);
        let model = menu::model(menu, shared);
        let mut painter = Painter::new(&self.atlas, 1);
        let screen = Rect::new(0, 0, 320, 220);
        draw_screen(def, &mut painter, model.as_ref(), &ctx, screen, None)
    }

    fn form(&mut self) -> CreateForm {
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
        CreateForm::new(&shared)
    }
}

/// Every element can be reached with the arrows / d-pad from the first one.
fn assert_all_reachable(hits: &[Hit], what: &str) {
    let key = |hit: &Hit| (hit.id.clone(), hit.index);
    let mut seen = BTreeSet::from([key(&hits[0])]);
    let mut todo = vec![hits[0].clone()];
    while let Some(hit) = todo.pop() {
        for direction in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            if let Some(next) = navigate(hits, Some(&hit), direction)
                && seen.insert(key(&next))
            {
                todo.push(next);
            }
        }
    }
    let all: BTreeSet<_> = hits.iter().map(key).collect();
    let missed: Vec<_> = all.difference(&seen).collect();
    assert!(missed.is_empty(), "{what}: unreachable {missed:?}");
}

fn ids(hits: &[Hit]) -> BTreeSet<&str> {
    hits.iter().map(|h| h.id.as_str()).collect()
}

#[test]
fn every_element_of_the_creation_pages_is_reachable() {
    let mut fixture = fixture();
    let mut menu = Menu::CreateWorld(Box::new(fixture.form()));
    let hits = fixture.hits(&mut menu);
    assert!(ids(&hits).is_superset(&BTreeSet::from(["world_type", "customize", "create"])));
    assert_all_reachable(&hits, "main page");

    let Menu::CreateWorld(form) = &mut menu else {
        unreachable!()
    };
    form.page = Page::Generation;
    form.editor.cycle_type(1);
    let expected: [&[&str]; 3] = [
        &["world_type", "biome_size", "sea_level", "caves", "ores"],
        &[
            "layers",
            "block",
            "thickness",
            "add",
            "remove",
            "up",
            "down",
        ],
        &["copy", "code_field", "paste", "apply"],
    ];
    for (tab, wanted) in expected.iter().enumerate() {
        if let Menu::CreateWorld(form) = &mut menu {
            form.tab = tab;
        }
        let hits = fixture.hits(&mut menu);
        let found = ids(&hits);
        for id in wanted.iter().chain(&["tabs", "done"]) {
            assert!(found.contains(id), "tab {tab}: no `{id}` in {found:?}");
        }
        assert_all_reachable(&hits, &format!("flat world, tab {tab}"));
    }

    if let Menu::CreateWorld(form) = &mut menu {
        form.editor.cycle_type(1);
        form.tab = 1;
    }
    let hits = fixture.hits(&mut menu);
    assert!(ids(&hits).contains("biomes"));
    assert!(!ids(&hits).contains("layers"));
    assert_all_reachable(&hits, "single biome");
}
