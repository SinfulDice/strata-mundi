use super::*;
use crate::content::Content;
use crate::id::Identifier;
use crate::pack::PackSet;
use std::path::Path;

fn id(text: &str) -> Identifier {
    text.parse().unwrap()
}

fn content() -> Content {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    let (content, errors) = Content::load(&packs, &PackSet::default());
    assert!(errors.is_empty(), "{errors:?}");
    content
}

fn rules() -> TemperatureRules {
    content().survival.temperature
}

/// A leaked identifier: test surroundings borrow theirs.
fn named(text: &str) -> &'static Identifier {
    Box::leak(Box::new(id(text)))
}

fn noon(biome: &str) -> Surroundings<'static> {
    Surroundings {
        biome: Some(named(biome)),
        height: 100.0,
        daylight: 1.0,
        ..Surroundings::default()
    }
}

fn felt(rules: &TemperatureRules, around: &Surroundings, wetness: f32, worn: Insulation) -> f32 {
    total(&contributions(rules, around, wetness, worn))
}

fn part_of(parts: &[Contribution<'_>], name: &'static str) -> Option<f32> {
    let name = PartName::Engine(name);
    parts.iter().find(|p| p.name == name).map(|p| p.degrees)
}

#[test]
fn the_biome_sets_the_base_and_unknown_biomes_are_mild() {
    let rules = rules();
    assert_eq!(
        felt(&rules, &noon("strata:desert"), 0.0, Insulation::default()),
        40.0
    );
    assert_eq!(
        felt(
            &rules,
            &noon("strata:snowy_tundra"),
            0.0,
            Insulation::default()
        ),
        -3.0
    );
    let unknown = felt(&rules, &noon("mod:nowhere"), 0.0, Insulation::default());
    assert_eq!(unknown, rules.default_biome.day);
}

#[test]
fn every_biome_of_the_base_pack_has_a_temperature() {
    let rules = rules();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs/strata/biomes");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_stem().unwrap().to_string_lossy();
        assert!(
            rules.biomes.contains_key(&id(&format!("strata:{name}"))),
            "{name}"
        );
    }
}

#[test]
fn nights_are_colder_and_a_roof_keeps_some_warmth() {
    let rules = rules();
    let mut night = noon("strata:plains");
    night.daylight = 0.0;
    let open = felt(&rules, &night, 0.0, Insulation::default());
    assert_eq!(open, 12.0, "the plains' night");
    let dusk = Surroundings {
        daylight: 0.5,
        ..night
    };
    assert_eq!(felt(&rules, &dusk, 0.0, Insulation::default()), 16.0);
    night.sheltered = true;
    assert_eq!(felt(&rules, &night, 0.0, Insulation::default()), 16.0);
    let mut shade = noon("strata:desert");
    shade.sheltered = true;
    assert_eq!(
        felt(&rules, &shade, 0.0, Insulation::default()),
        35.0,
        "half the heat beyond 30"
    );
}

#[test]
fn heights_are_colder() {
    let rules = rules();
    let mut peak = noon("strata:mountains");
    peak.height = 240.0;
    let parts = contributions(&rules, &peak, 0.0, Insulation::default());
    assert!((part_of(&parts, "altitude").unwrap() + 6.0).abs() < 1e-4);
    peak.height = 120.0;
    let parts = contributions(&rules, &peak, 0.0, Insulation::default());
    assert_eq!(part_of(&parts, "altitude"), None, "nothing below the line");
}

#[test]
fn water_cools_and_drying_takes_time() {
    let rules = rules();
    let plains = noon("strata:plains");
    assert_eq!(felt(&rules, &plains, 1.0, Insulation::default()), 14.0);
    let diving = Surroundings {
        immersed: true,
        ..plains
    };
    assert_eq!(felt(&rules, &diving, 1.0, Insulation::default()), 8.0);
    let mut body = BodyTemperature::default();
    body.soak(true, &rules);
    assert_eq!(body.wetness, 1.0);
    for _ in 0..600 {
        body.soak(false, &rules);
    }
    assert!((body.wetness - 0.5).abs() < 1e-3, "half dry after 30 s");
    for _ in 0..200 {
        body.heat = 1.0;
        body.soak(false, &rules);
    }
    assert!(body.wetness < 1e-3, "heat dries three times faster");
}

#[test]
fn heat_sources_warm_by_distance_and_only_lit_furnaces_count() {
    let content = content();
    let heat = &content.survival.heat;
    let block = |name: &str| content.blocks.key(&id(name)).unwrap();
    let lava = heat.get(block("strata:lava")).unwrap();
    assert_eq!(lava.at(0.0), 20.0);
    assert_eq!(lava.at(3.0), 10.0);
    assert_eq!(lava.at(7.0), 0.0);
    assert!(heat.get(block("strata:torch")).is_some());
    let furnace = block("strata:furnace");
    let facing = content.blocks.get(furnace).unwrap().facing.unwrap();
    assert!(heat.get(furnace).is_none(), "a cold furnace");
    assert!(heat.get(facing.lit(true)).is_some(), "a burning one");
    assert!(heat.get(block("strata:stone")).is_none());
    assert!(heat.reach >= 6 && f32::from(heat.reach) <= MAX_REACH);
    let rules = &content.survival.temperature;
    let mut cave = noon("strata:snowy_tundra");
    cave.heat = lava.at(1.0);
    let parts = contributions(rules, &cave, 0.0, Insulation::default());
    assert!(part_of(&parts, "heat").is_some());
}

#[test]
fn clothing_brings_back_toward_comfort_but_never_past_it() {
    let rules = rules();
    let tundra = noon("strata:snowy_tundra");
    let cloak = Insulation {
        cold: 4.0,
        heat: 0.0,
    };
    assert_eq!(
        felt(&rules, &tundra, 0.0, cloak),
        9.0,
        "4 points, 12 degrees"
    );
    let plains = noon("strata:plains");
    let heavy = Insulation {
        cold: 20.0,
        heat: 0.0,
    };
    assert_eq!(
        felt(&rules, &plains, 0.0, heavy),
        50.0,
        "warm clothes in mild weather overheat"
    );
    let desert = noon("strata:desert");
    assert_eq!(
        felt(&rules, &desert, 0.0, cloak),
        46.0,
        "a cloak in the desert is too much"
    );
    let light = Insulation {
        cold: 0.0,
        heat: 4.0,
    };
    assert_eq!(felt(&rules, &desert, 0.0, light), 28.0);
    let lots = Insulation {
        cold: 0.0,
        heat: 20.0,
    };
    assert_eq!(felt(&rules, &desert, 0.0, lots), rules.comfort());
}

#[test]
fn extra_contributions_and_dimensions_add_up() {
    let mut rules = rules();
    rules.dimensions.insert(id("mod:ember"), 25.0);
    let mut around = noon("strata:plains");
    around.dimension = Some(named("mod:ember"));
    let blizzard = [(id("mod:blizzard"), -10.0)];
    around.extra = &blizzard;
    let parts = contributions(&rules, &around, 0.0, Insulation::default());
    assert_eq!(total(&parts), 35.0);
    assert!(
        parts
            .iter()
            .any(|p| p.name == PartName::Named(&blizzard[0].0))
    );
    assert_eq!(target(&rules, &around, 0.0, Insulation::default()), 35.0);
}

#[test]
fn the_felt_temperature_follows_slowly() {
    let rules = rules();
    let mut body = BodyTemperature::default();
    body.follow(20.0, 20, &rules);
    assert_eq!(body.felt, Some(20.0), "the first sample is taken at once");
    body.follow(0.0, 20, &rules);
    let after = body.felt.unwrap();
    assert!(after < 20.0 && after > 18.0, "{after}");
    for _ in 0..19 {
        body.follow(0.0, 20, &rules);
    }
    let close = body.felt.unwrap();
    assert!((close - 20.0 * (-1.0f32).exp()).abs() < 0.05, "{close}");
}

#[test]
fn zones_strain_then_hurt() {
    let rules = rules();
    assert_eq!(rules.zone(20.0), TemperatureZone::Comfortable);
    assert_eq!(rules.zone(8.0), TemperatureZone::Comfortable);
    assert_eq!(rules.zone(7.9), TemperatureZone::Cold);
    assert_eq!(rules.zone(-4.1), TemperatureZone::Freezing);
    assert_eq!(rules.zone(30.1), TemperatureZone::Hot);
    assert_eq!(rules.zone(42.1), TemperatureZone::Scorching);
    assert!(rules.strain(TemperatureZone::Cold).speed < 1.0);
    assert!(rules.strain(TemperatureZone::Cold).hunger > 0.0);
    assert!(rules.strain(TemperatureZone::Hot).thirst > 0.0);
    assert!(rules.strain(TemperatureZone::Hot).stamina_regen < 1.0);
    assert_eq!(rules.strain(TemperatureZone::Comfortable), Strain::NONE);
    let mut body = BodyTemperature {
        felt: Some(-10.0),
        ..BodyTemperature::default()
    };
    let hits: Vec<_> = (0..160).filter_map(|_| body.harm_tick(&rules)).collect();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].0, id("strata:freezing"));
    body.felt = Some(50.0);
    let hit = (0..80).find_map(|_| body.harm_tick(&rules)).unwrap();
    assert_eq!(hit.0, id("strata:overheating"));
    body.felt = Some(0.0);
    assert!(
        (0..200).all(|_| body.harm_tick(&rules).is_none()),
        "cold strains only"
    );
}

#[test]
fn the_gauge_puts_each_zone_in_its_place() {
    let rules = rules();
    let near = |a: f32, b: f32| (a - b).abs() < 1e-4;
    assert!(near(rules.gauge(-4.0), 1.0 / 6.0));
    assert!(near(rules.gauge(8.0), 1.0 / 3.0));
    assert!(near(rules.gauge(19.0), 0.5));
    assert!(near(rules.gauge(42.0), 5.0 / 6.0));
    assert_eq!(rules.gauge(-100.0), 0.0);
    assert_eq!(rules.gauge(100.0), 1.0);
    assert_eq!(rules.gauge(f32::NAN), 0.0);
}

#[test]
fn the_saved_state_comes_back_within_bounds() {
    let body = BodyTemperature {
        felt: Some(3.5),
        wetness: 0.25,
        ..BodyTemperature::default()
    };
    let text = ron::to_string(&body).unwrap();
    let read: BodyTemperature = ron::from_str(&text).unwrap();
    assert_eq!(read.felt, Some(3.5));
    assert_eq!(read.wetness, 0.25);
    let broken = BodyTemperature {
        felt: Some(f32::INFINITY),
        wetness: 7.0,
        ..BodyTemperature::default()
    }
    .bounded();
    assert_eq!((broken.felt, broken.wetness), (None, 1.0));
}

#[test]
fn broken_rules_are_kept_in_order() {
    let mut rules = TemperatureRules::default();
    rules.zones.hot = -50.0;
    rules.sample_every = 0;
    rules.smoothing = f32::NAN;
    let rules = rules.clamped();
    assert!(rules.zones.hot >= rules.zones.cold);
    assert!(rules.zones.scorching >= rules.zones.hot);
    assert_eq!(rules.sample_every, 1);
    assert_eq!(rules.smoothing, 0.0);
}

#[test]
fn a_zone_is_left_only_past_its_margin() {
    let rules = rules();
    assert_eq!(rules.zones.margin, 0.5);
    let cold = TemperatureZone::Cold;
    assert_eq!(rules.zone_from(cold, 8.3), cold, "just past the threshold");
    assert_eq!(rules.zone_from(cold, 8.6), TemperatureZone::Comfortable);
    let comfortable = TemperatureZone::Comfortable;
    assert_eq!(rules.zone_from(comfortable, 7.7), comfortable);
    assert_eq!(rules.zone_from(comfortable, 7.4), cold);
    assert_eq!(
        rules.zone_from(comfortable, -20.0),
        TemperatureZone::Freezing
    );
    let mut body = BodyTemperature::at(7.0);
    for felt in [8.2, 7.8, 8.4, 7.9] {
        body.follow(
            felt,
            20,
            &TemperatureRules {
                smoothing: 0.0,
                ..rules.clone()
            },
        );
        assert_eq!(body.zone(&rules), cold, "{felt}: no flicker");
    }
}

#[test]
fn negative_clothing_takes_one_further_from_comfort() {
    let rules = rules();
    let tundra = noon("strata:snowy_tundra");
    let cursed = Insulation {
        cold: -2.0,
        heat: 0.0,
    };
    assert_eq!(felt(&rules, &tundra, 0.0, cursed), -9.0);
    let desert = noon("strata:desert");
    let stifling = Insulation {
        cold: 0.0,
        heat: -2.0,
    };
    assert_eq!(felt(&rules, &desert, 0.0, stifling), 46.0);
}

mod scan {
    use super::*;
    use crate::chunk::Column;
    use crate::coords::{BlockPos, ColumnPos};
    use glam::DVec3;
    use std::collections::HashMap;

    struct World {
        columns: HashMap<ColumnPos, Column>,
    }

    impl World {
        fn new() -> Self {
            let columns = (-1..=1)
                .flat_map(|x| (-1..=1).map(move |z| ColumnPos { x, z }))
                .map(|pos| (pos, Column::empty(pos)))
                .collect();
            Self { columns }
        }

        fn set(&mut self, pos: BlockPos, block: crate::content::BlockId) {
            let column = self.columns.get_mut(&pos.column()).unwrap();
            column.set(
                pos.x.rem_euclid(16) as u32,
                pos.y,
                pos.z.rem_euclid(16) as u32,
                block,
            );
        }

        fn find(&self, content: &Content, centre: DVec3) -> Option<HeatFound> {
            let solid = |pos: BlockPos| {
                let column = self.columns.get(&pos.column());
                let block = column.map(|c| {
                    c.get(
                        pos.x.rem_euclid(16) as u32,
                        pos.y,
                        pos.z.rem_euclid(16) as u32,
                    )
                });
                block.is_some_and(|b| content.blocks.get(b).is_some_and(|b| b.solid))
            };
            let survival = &content.survival;
            strongest_heat(
                &survival.heat,
                survival.temperature.walls,
                centre,
                |pos| self.columns.get(&pos),
                solid,
            )
        }
    }

    #[test]
    fn the_strongest_source_counts_and_walls_hold_heat_back() {
        let content = content();
        let block = |name: &str| content.blocks.key(&id(name)).unwrap();
        let mut world = World::new();
        let centre = DVec3::new(0.5, 70.9, 0.5);
        assert_eq!(world.find(&content, centre), None);
        world.set(BlockPos::new(2, 70, 0), block("strata:torch"));
        world.set(BlockPos::new(-3, 70, 0), block("strata:campfire"));
        let found = world.find(&content, centre).unwrap();
        assert_eq!(
            found.source,
            BlockPos::new(-3, 70, 0),
            "the campfire beats the torch"
        );
        let open = found.warmth;
        world.set(BlockPos::new(-2, 70, 0), block("strata:stone"));
        world.set(BlockPos::new(-2, 71, 0), block("strata:stone"));
        let walled = world.find(&content, centre).unwrap();
        assert!(
            (walled.warmth - open * 0.25).abs() < 1e-4,
            "{} vs {open}",
            walled.warmth
        );
        world.set(BlockPos::new(-3, 70, 0), block("strata:unlit_campfire"));
        let torch = world.find(&content, centre).unwrap();
        assert_eq!(
            torch.source,
            BlockPos::new(2, 70, 0),
            "a campfire put out is cold"
        );
        world.set(BlockPos::new(20, 70, 0), block("strata:lava"));
        assert_eq!(
            world.find(&content, centre).unwrap().source,
            torch.source,
            "beyond reach"
        );
    }
}
