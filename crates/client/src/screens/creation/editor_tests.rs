#![allow(clippy::unwrap_used)]

use super::*;

fn id(name: &str) -> Identifier {
    format!("strata:{name}").parse().unwrap()
}

fn layer(name: &str, thickness: u16) -> FlatLayer {
    FlatLayer {
        block: id(name),
        thickness,
    }
}

fn catalog() -> Arc<GenerationCatalog> {
    let flat = FlatSettings {
        layers: vec![
            layer("bedrock", 1),
            layer("dirt", 2),
            layer("grass_block", 1),
        ],
        caves: false,
        features: false,
    };
    Arc::new(GenerationCatalog {
        types: vec![
            WorldTypeChoice {
                id: id("normal"),
                biome: None,
                flat: None,
            },
            WorldTypeChoice {
                id: id("flat"),
                biome: Some(id("plains")),
                flat: Some(flat),
            },
            WorldTypeChoice {
                id: id("single_biome"),
                biome: Some(id("plains")),
                flat: None,
            },
        ],
        biomes: vec![id("plains"), id("desert")],
        sea_level: 120,
    })
}

fn editor() -> GenerationEditor {
    let blocks = ["bedrock", "dirt", "grass_block", "stone", "sand"].map(id);
    GenerationEditor::new(catalog(), blocks.to_vec())
}

fn flat_editor() -> GenerationEditor {
    let mut editor = editor();
    editor.cycle_type(1);
    assert_eq!(editor.settings.world_type, id("flat"));
    editor
}

fn names(editor: &GenerationEditor) -> Vec<(&str, u16)> {
    editor
        .layers()
        .iter()
        .map(|l| (l.block.name(), l.thickness))
        .collect()
}

#[test]
fn sliders_stay_in_their_ranges() {
    let mut editor = editor();
    editor.step(Slider::BiomeSize, -100);
    assert_eq!(editor.value(Slider::BiomeSize), 25);
    editor.step(Slider::Caves, 1000);
    assert_eq!(editor.value(Slider::Caves), 400);
    editor.set_fraction(Slider::Ores, -3.0);
    assert_eq!(editor.value(Slider::Ores), 0);
    editor.set_fraction(Slider::BiomeSize, 0.5);
    assert_eq!(editor.value(Slider::BiomeSize) % 25, 0);
    editor.step(Slider::SeaLevel, 1000);
    assert_eq!(editor.settings.sea_level, Some(240));
    assert!((editor.fraction(Slider::SeaLevel) - 1.0).abs() < 1e-6);
}

#[test]
fn the_default_sea_level_is_stored_as_default() {
    let mut editor = editor();
    assert_eq!(editor.value(Slider::SeaLevel), 120);
    editor.step(Slider::SeaLevel, 1);
    assert_eq!(editor.settings.sea_level, Some(124));
    editor.step(Slider::SeaLevel, -1);
    assert_eq!(editor.settings.sea_level, None);
    assert!(editor.is_default(Slider::SeaLevel));
}

#[test]
fn layers_are_added_removed_and_moved() {
    let mut editor = flat_editor();
    assert_eq!(
        names(&editor),
        [("bedrock", 1), ("dirt", 2), ("grass_block", 1)]
    );
    editor.layer = 0;
    editor.add_layer(id("stone"));
    assert_eq!(editor.layer, 1);
    editor.change_thickness(1, 4);
    assert_eq!(
        names(&editor),
        [
            ("bedrock", 1),
            ("stone", 5),
            ("dirt", 2),
            ("grass_block", 1)
        ]
    );
    editor.move_layer(true);
    assert_eq!(editor.layer, 2);
    assert_eq!(editor.layers()[2].block, id("stone"));
    editor.move_layer(true);
    editor.move_layer(true);
    assert_eq!(editor.layer, 3, "the top layer cannot go higher");
    editor.set_block(id("sand"));
    editor.remove_layer();
    assert_eq!(
        names(&editor),
        [("bedrock", 1), ("dirt", 2), ("grass_block", 1)]
    );
    assert_eq!(editor.layer, 2);
    for _ in 0..5 {
        editor.remove_layer();
    }
    assert_eq!(editor.layers().len(), 1, "a flat world keeps one layer");
    editor.change_thickness(0, -10);
    assert_eq!(editor.layers()[0].thickness, 1);
    editor.change_thickness(0, 1000);
    assert_eq!(editor.layers()[0].thickness, *LAYER_THICKNESS.end());
}

#[test]
fn unchanged_layers_and_biome_stay_default() {
    let mut editor = flat_editor();
    editor.layer = 1;
    editor.move_layer(true);
    editor.move_layer(false);
    assert_eq!(editor.settings.flat, None);
    editor.set_biome(id("desert"));
    assert_eq!(editor.biome(), Some(&id("desert")));
    editor.set_biome(id("plains"));
    assert_eq!(editor.settings.biome, None);
    editor.cycle_type(1);
    assert_eq!(editor.layers(), &[], "single biome has no layers");
    editor.cycle_type(1);
    assert_eq!(editor.settings.world_type, id("normal"));
}

#[test]
fn codes_give_back_the_settings() {
    let mut editor = flat_editor();
    editor.add_layer(id("sand"));
    editor.set_biome(id("desert"));
    editor.step(Slider::Caves, 2);
    let code = editor.code();
    let edited = editor.settings.clone();
    let mut other = self::editor();
    other.apply_code(&code).unwrap();
    assert_eq!(other.settings, edited);
    assert_eq!(other.code(), code);
}

#[test]
fn bad_codes_are_refused_without_change() {
    let mut editor = editor();
    let before = editor.settings.clone();
    let refused = editor.apply_code("S1-NOTACODE").unwrap_err();
    assert_eq!(refused.message, "world-code-malformed");
    let unknown = WorldgenSettings {
        world_type: id("moon"),
        ..WorldgenSettings::default()
    };
    let refused = editor.apply_code(&unknown.to_code()).unwrap_err();
    assert_eq!(refused.message, "world-settings-unknown-type");
    assert_eq!(refused.id.as_deref(), Some("strata:moon"));
    let unknown_block = WorldgenSettings {
        world_type: id("flat"),
        flat: Some(FlatSettings {
            layers: vec![layer("cheese", 1)],
            caves: false,
            features: false,
        }),
        ..WorldgenSettings::default()
    };
    let refused = editor.apply_code(&unknown_block.to_code()).unwrap_err();
    assert_eq!(refused.message, "world-settings-unknown-block");
    assert_eq!(editor.settings, before);
}

#[test]
fn codes_drop_what_their_type_does_not_use() {
    let mut editor = editor();
    let stray = WorldgenSettings {
        world_type: id("normal"),
        biome: Some(id("desert")),
        flat: Some(FlatSettings {
            layers: vec![layer("sand", 2)],
            caves: false,
            features: false,
        }),
        ..WorldgenSettings::default()
    };
    editor.apply_code(&stray.to_code()).unwrap();
    assert_eq!(editor.settings, WorldgenSettings::default());
}
