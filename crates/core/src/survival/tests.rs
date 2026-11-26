use super::*;
use crate::content::Content;
use crate::pack::PackSet;
use std::collections::BTreeMap;
use std::path::Path;

fn content() -> Content {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    let (content, errors) = Content::load(&packs, &PackSet::default());
    assert!(errors.is_empty(), "{errors:?}");
    content
}

fn context() -> NeedsContext {
    NeedsContext {
        options: SurvivalOptions::default(),
        underwater: false,
        extra_breath: 0.0,
        stamina_burden: 0.0,
        strain: temperature::Strain::NONE,
    }
}

fn id(text: &str) -> Identifier {
    text.parse().unwrap()
}

#[test]
fn the_base_pack_defines_survival() {
    let content = content();
    let survival = &content.survival;
    assert!(
        survival
            .rules
            .drinking
            .salty_biomes
            .contains(&id("strata:ocean"))
    );
    assert!(survival.drink_source.is_some());
    for kind in [
        "fall",
        "drowning",
        "suffocation",
        "starvation",
        "dehydration",
        "lava",
        "fire",
    ] {
        assert!(survival.damage.contains_key(&engine(kind)), "{kind}");
    }
    let apple = content.items.key(&id("strata:apple")).unwrap();
    assert_eq!(survival.consumables[&apple].food, 4.0);
    let lava = content.blocks.key(&id("strata:lava")).unwrap();
    assert_eq!(survival.contact(lava).unwrap().damage, id("strata:lava"));
    let bottle = content.items.key(&id("strata:glass_bottle")).unwrap();
    assert!(survival.fills.contains_key(&bottle));
}

#[test]
fn exhaustion_eats_saturation_then_food() {
    let rules = SurvivalRules::default();
    let mut needs = Needs::full(&rules);
    let mut health = Health::full(rules.health.max);
    needs.saturation = 1.0;
    let sprint = Exertion {
        sprinted: 40.0,
        ..Exertion::default()
    };
    needs.exert(&sprint, &rules, &context());
    needs.tick(&mut health, &rules, &context());
    assert_eq!(needs.saturation, 0.0);
    assert_eq!(needs.food, rules.hunger.max - 0.0);
    needs.exert(&sprint, &rules, &context());
    needs.tick(&mut health, &rules, &context());
    assert_eq!(needs.food, rules.hunger.max - 1.0);
}

#[test]
fn a_fed_player_heals_and_a_starving_one_does_not() {
    let rules = SurvivalRules::default();
    let mut needs = Needs::full(&rules);
    let mut health = Health::full(rules.health.max);
    health.current = 10.0;
    for _ in 0..rules.health.saturated_every {
        needs.tick(&mut health, &rules, &context());
    }
    assert_eq!(health.current, 11.0, "full food and saturation heal fast");
    assert!(needs.exhaustion > 0.0, "healing costs exhaustion");
    needs.food = 0.0;
    needs.saturation = 0.0;
    let mut harm = Vec::new();
    for _ in 0..rules.health.starve_every {
        harm.extend(needs.tick(&mut health, &rules, &context()));
    }
    assert_eq!(health.current, 11.0);
    assert_eq!(
        harm,
        vec![(NeedHarm::Starvation, rules.health.starve_damage)]
    );
}

#[test]
fn starving_stops_at_the_floor() {
    let rules = SurvivalRules::default();
    let mut needs = Needs::full(&rules);
    needs.food = 0.0;
    let mut health = Health::full(rules.health.max);
    health.current = rules.health.starve_floor;
    let mut harm = Vec::new();
    for _ in 0..rules.health.starve_every * 3 {
        harm.extend(needs.tick(&mut health, &rules, &context()));
    }
    assert!(!harm.iter().any(|(kind, _)| *kind == NeedHarm::Starvation));
}

#[test]
fn thirst_goes_down_by_itself_and_hurts_at_zero() {
    let rules = SurvivalRules::default();
    let mut needs = Needs::full(&rules);
    let mut health = Health::full(rules.health.max);
    let ticks = (rules.thirst.per_point / rules.thirst.idle).ceil() as u32 + 1;
    for _ in 0..ticks {
        needs.tick(&mut health, &rules, &context());
    }
    assert_eq!(needs.thirst, rules.thirst.max - 1.0);
    needs.thirst = 0.0;
    let mut harm = Vec::new();
    for _ in 0..rules.thirst.dry_every {
        harm.extend(needs.tick(&mut health, &rules, &context()));
    }
    assert!(harm.contains(&(NeedHarm::Dehydration, rules.thirst.dry_damage)));
}

#[test]
fn options_turn_needs_off_and_change_their_speed() {
    let rules = SurvivalRules::default();
    let mut needs = Needs::full(&rules);
    let mut health = Health::full(rules.health.max);
    let mut off = context();
    off.options = SurvivalOptions {
        hunger: false,
        thirst: false,
        stamina: false,
        temperature: false,
        rate: 100,
    };
    let sprint = Exertion {
        sprinted: 400.0,
        sprint_ticks: 200,
        ..Exertion::default()
    };
    needs.exert(&sprint, &rules, &off);
    for _ in 0..2000 {
        needs.tick(&mut health, &rules, &off);
    }
    assert_eq!(needs.food, rules.hunger.max);
    assert_eq!(needs.thirst, rules.thirst.max);
    assert_eq!(needs.stamina, rules.stamina.max);
    let (mut slow, mut fast) = (Needs::full(&rules), Needs::full(&rules));
    let mut half = context();
    half.options.rate = 50;
    let mut double = context();
    double.options.rate = 200;
    slow.exert(&sprint, &rules, &half);
    fast.exert(&sprint, &rules, &double);
    assert!((fast.exhaustion - 4.0 * slow.exhaustion).abs() < 1e-3);
}

#[test]
fn sprinting_tires_and_heavy_armour_lowers_the_stamina() {
    let rules = SurvivalRules::default();
    let mut needs = Needs::full(&rules);
    let mut health = Health::full(rules.health.max);
    let sprint = Exertion {
        sprint_ticks: 1,
        ..Exertion::default()
    };
    let mut ticks = 0;
    while needs.can_sprint(&rules, &SurvivalOptions::default()) {
        needs.exert(&sprint, &rules, &context());
        needs.tick(&mut health, &rules, &context());
        ticks += 1;
        assert!(ticks < 10_000);
    }
    assert!(needs.winded);
    for _ in 0..rules.stamina.delay + 1 {
        needs.tick(&mut health, &rules, &context());
    }
    assert!(!needs.can_sprint(&rules, &SurvivalOptions::default()));
    for _ in 0..200 {
        needs.tick(&mut health, &rules, &context());
    }
    assert!(needs.can_sprint(&rules, &SurvivalOptions::default()));
    assert_eq!(needs.stamina, rules.stamina.max);
    let mut heavy = context();
    heavy.stamina_burden = 0.2;
    needs.tick(&mut health, &rules, &heavy);
    assert_eq!(needs.stamina, rules.stamina.max * 0.8);
    needs.food = rules.hunger.sprint_min;
    assert!(!needs.can_sprint(&rules, &SurvivalOptions::default()));
}

#[test]
fn breath_runs_out_under_water_and_worn_breath_adds_to_it() {
    let rules = SurvivalRules::default();
    let mut needs = Needs::full(&rules);
    let mut health = Health::full(rules.health.max);
    let mut under = context();
    under.underwater = true;
    let base = breath_ticks(&rules, 0.0);
    let mut first_harm = None;
    for tick in 0..u32::from(base) + 100 {
        if !needs.tick(&mut health, &rules, &under).is_empty() {
            first_harm.get_or_insert(tick);
        }
    }
    let first = first_harm.unwrap();
    assert!(first >= u32::from(base));
    let mut needs = Needs::full(&rules);
    let mut air = context();
    air.extra_breath = 5.0;
    for _ in 0..40 {
        needs.tick(&mut health, &rules, &air);
    }
    under.extra_breath = 5.0;
    for _ in 0..u32::from(base) + 50 {
        assert!(needs.tick(&mut health, &rules, &under).is_empty());
    }
    needs.tick(&mut health, &rules, &context());
    assert!(needs.breath > 0);
}

#[test]
fn invulnerability_lets_only_a_stronger_hit_through() {
    let mut health = Health::full(20.0);
    assert_eq!(health.hurt(4.0, false, 10), 4.0);
    assert_eq!(health.hurt(3.0, false, 10), 0.0, "weaker: ignored");
    assert_eq!(health.hurt(6.0, false, 10), 2.0, "stronger: the difference");
    assert_eq!(
        health.hurt(1.0, true, 10),
        1.0,
        "piercing harm goes through"
    );
    for _ in 0..10 {
        health.tick();
    }
    assert_eq!(health.hurt(3.0, false, 10), 3.0);
}

#[test]
fn armour_and_resistance_lessen_damage() {
    let rules = SurvivalRules::default().armor;
    assert_eq!(reduced(10.0, None, 0.0, &rules), 10.0);
    assert!((reduced(10.0, Some(10), 0.0, &rules) - 6.0).abs() < 1e-4);
    assert!(
        (reduced(10.0, Some(100), 0.0, &rules) - 2.0).abs() < 1e-4,
        "capped"
    );
    assert!((reduced(10.0, None, 50.0, &rules) - 5.0).abs() < 1e-4);
    assert_eq!(reduced(10.0, None, 150.0, &rules), 0.0);
}

#[test]
fn falls_hurt_beyond_the_safe_height() {
    let rules = SurvivalRules::default().fall;
    assert_eq!(fall_damage(3.0, &rules), 0.0);
    assert_eq!(fall_damage(4.2, &rules), 2.0);
    let mut health = Health::full(20.0);
    let mut fallen = 0.0;
    let mut y = 80.0;
    health.follow_fall(y, 0.0, true, false);
    for step in 0..40 {
        let speed = -(f64::from(step) * 1.6);
        y += speed * 0.05;
        let landed = y <= 70.0;
        fallen = health.follow_fall(y.max(70.0), speed, landed, false);
        if landed {
            break;
        }
    }
    assert!(fallen > 9.0 && fallen <= 10.0, "{fallen}");
    health.follow_fall(80.0, -20.0, false, false);
    assert_eq!(health.follow_fall(70.0, 0.0, false, true), 0.0, "water");
    assert_eq!(health.follow_fall(69.0, 0.0, true, false), 0.0);
}

#[test]
fn one_effect_never_stacks_the_strongest_stays() {
    let content = content();
    let defs = &content.survival.effects;
    let mut effects = StatusEffects::default();
    let healing = |seconds, strength| EffectGrant {
        effect: id("strata:healing"),
        seconds,
        strength,
    };
    assert!(effects.give(&healing(10.0, 0.5)));
    assert!(effects.give(&healing(5.0, 1.0)), "stronger replaces");
    assert!(!effects.give(&healing(30.0, 0.5)), "weaker changes nothing");
    assert_eq!(effects.0.len(), 1);
    assert_eq!(effects.0[0].strength, 1.0);
    let heals: f32 = (0..200)
        .flat_map(|_| effects.tick(defs, false))
        .map(|pulse| match pulse {
            EffectPulse::Heal(amount) => amount,
            EffectPulse::Damage { .. } => 0.0,
        })
        .sum();
    assert_eq!(heals, 5.0);
    assert!(effects.0.is_empty());
}

#[test]
fn bleeding_hurts_until_cured_and_burning_ends_in_water() {
    let content = content();
    let defs = &content.survival.effects;
    let mut effects = StatusEffects::default();
    effects.give(&EffectGrant {
        effect: id("strata:bleeding"),
        seconds: 60.0,
        strength: 1.0,
    });
    let pulses: Vec<_> = (0..80).flat_map(|_| effects.tick(defs, false)).collect();
    assert_eq!(pulses.len(), 2);
    assert!(effects.cure(&[id("strata:bleeding")]));
    assert!(effects.0.is_empty());
    effects.give(&EffectGrant {
        effect: id("strata:burning"),
        seconds: 8.0,
        strength: 1.0,
    });
    effects.tick(defs, true);
    assert!(effects.0.is_empty());
}

fn fire() -> (SurvivalRules, StatusEffectDef, DamageType) {
    let potion = StatusEffectDef {
        harmful: false,
        heal: None,
        damage: None,
        ends_in_water: false,
        stats: vec![(id("strata:fire_resistance"), 50.0)],
    };
    let kind = DamageType {
        armor: false,
        resistance: Some(id("strata:fire_resistance")),
        ignores_invulnerability: false,
        shakes: true,
    };
    (SurvivalRules::default(), potion, kind)
}

#[test]
fn a_worn_resistance_or_an_effect_the_stronger_counts() {
    let (rules, potion, kind) = fire();
    let defs = BTreeMap::from([(id("strata:potion"), potion)]);
    let mut worn = BTreeMap::new();
    worn.insert(id("strata:fire_resistance"), 25.0);
    let hit = |defences: &Defences<'_>| {
        let mut health = Health::full(20.0);
        take_hit(&mut health, Some(&kind), 8.0, defences, (&rules, &defs))
    };
    let worn_only = Defences {
        worn: Some(&worn),
        ..Defences::default()
    };
    assert_eq!(hit(&worn_only), 6.0, "worn: 25 %");
    let mut effects = StatusEffects::default();
    effects.give(&EffectGrant {
        effect: id("strata:potion"),
        seconds: 10.0,
        strength: 1.0,
    });
    let both = Defences {
        worn: Some(&worn),
        effects: Some(&effects),
        ..Defences::default()
    };
    assert_eq!(hit(&both), 4.0, "the effect's 50 % wins, never 75 %");
    worn.insert(id("strata:fire_resistance"), 75.0);
    let strong = Defences {
        worn: Some(&worn),
        effects: Some(&effects),
        ..Defences::default()
    };
    assert_eq!(hit(&strong), 2.0, "the worn 75 % wins");
}

#[test]
fn armour_never_takes_more_than_its_cap() {
    let mut rules = SurvivalRules::default();
    rules.armor.max = 0.5;
    let kind = DamageType {
        armor: true,
        resistance: None,
        ignores_invulnerability: false,
        shakes: true,
    };
    let defences = Defences {
        protection: 30,
        ..Defences::default()
    };
    let mut health = Health::full(20.0);
    let dealt = take_hit(
        &mut health,
        Some(&kind),
        10.0,
        &defences,
        (&rules, &BTreeMap::new()),
    );
    assert_eq!(dealt, 5.0);
    let plain = take_hit(
        &mut Health::full(20.0),
        None,
        10.0,
        &defences,
        (&rules, &BTreeMap::new()),
    );
    assert_eq!(plain, 10.0, "unknown kind: no armour");
}
