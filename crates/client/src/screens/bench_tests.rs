#![allow(clippy::unwrap_used)]
//! Each bench's screen: its own arrangement and colour, its help, its list scrolled by
//! rows, and the arrangements the inventory screen file must hold.

use super::{BENCH_COLUMNS, BENCH_ROWS, LAYOUT_PREFIX};
use crate::crafting::ClientCrafting;
use crate::inventory::ClientInventory;
use crate::screens::slots::{SlotsModel, SlotsState};
use crate::ui::{Node, ScreenModel, SlotPress, UiAssets};
use std::path::Path;
use strata_core::content::Content;
use strata_core::coords::BlockPos;
use strata_core::crafting::BenchLayout;
use strata_core::id::Identifier;
use strata_core::inventory::{Click, Container, ItemStack};
use strata_core::pack::PackSet;
use strata_protocol::InventoryRequest;

const BENCHES: [(&str, BenchLayout); 4] = [
    ("strata:forge", BenchLayout::Forge),
    ("strata:tannery", BenchLayout::Tannery),
    ("strata:loom", BenchLayout::Loom),
    ("strata:jeweller", BenchLayout::Jeweller),
];

fn assets() -> (Content, UiAssets) {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let mut errors = Vec::new();
    let packs = PackSet::discover(&assets.join("packs"), &mut errors);
    let ui = UiAssets::load(&assets.join("ui"), &packs, &mut errors).unwrap();
    let (content, more) = Content::load(&packs, &PackSet::default());
    errors.extend(more);
    assert!(errors.is_empty(), "{errors:#?}");
    (content, ui)
}

/// The view the server sends when `bench` opens: its block, four empty cells, its list.
fn opened(content: &Content, bench: &str) -> ClientCrafting {
    let id: Identifier = bench.parse().unwrap();
    let block = content.blocks.key(&id).unwrap();
    let mut crafting = ClientCrafting::default();
    crafting.view.station = Some((block, BlockPos::new(0, 0, 0)));
    crafting.view.grid = Container::new(4);
    crafting.view.cuts = (content.benches.of(&id))
        .map(|(index, r)| (index, ItemStack::new(r.result, r.count, &content.items)))
        .collect();
    crafting
}

fn with_model<R>(
    content: &Content,
    crafting: &ClientCrafting,
    state: &mut SlotsState,
    inventory: &mut ClientInventory,
    f: impl FnOnce(&mut SlotsModel<'_>) -> R,
) -> R {
    let mut effects = Vec::new();
    let mut model = SlotsModel {
        state,
        inventory,
        container: None,
        crafting: Some(crafting),
        effects: &mut effects,
        content,
        language: "fr",
    };
    f(&mut model)
}

#[test]
fn each_bench_shows_its_own_arrangement_colour_and_help() {
    let (content, _) = assets();
    for (bench, layout) in BENCHES {
        let crafting = opened(&content, bench);
        let mut state = SlotsState::default();
        let mut inventory = ClientInventory::default();
        with_model(&content, &crafting, &mut state, &mut inventory, |model| {
            assert!(model.shown("bench_side"), "{bench}");
            assert!(!model.shown("player_side") && !model.shown("table_side"));
            for other in BenchLayout::ALL {
                let group = format!("{LAYOUT_PREFIX}{}", other.name());
                assert_eq!(model.shown(&group), other == layout, "{bench}: {group}");
            }
            let id: Identifier = bench.parse().unwrap();
            let accent = content.benches.style(&id).accent.unwrap();
            assert_eq!(model.color("bench_band_title"), Some(accent.0));
            assert_eq!(model.color("bench_band_work"), Some(accent.0));
            let help = content
                .translations
                .text("fr", &id.translation_key("bench"));
            assert_eq!(model.text("bench_note").as_deref(), Some(help.as_str()));
            let shown = model.items("bench_choices");
            assert!(!shown.is_empty() && shown.len() <= BENCH_COLUMNS * BENCH_ROWS);
        });
    }
}

#[test]
fn a_long_list_scrolls_by_rows_and_choices_name_the_recipe_shown() {
    let (content, _) = assets();
    let crafting = opened(&content, "strata:forge");
    let total = crafting.view.cuts.len();
    assert!(
        total > BENCH_COLUMNS * BENCH_ROWS,
        "the forge needs scrolling"
    );
    let mut state = SlotsState::default();
    let mut inventory = ClientInventory::default();
    with_model(&content, &crafting, &mut state, &mut inventory, |model| {
        assert!(model.shown("bench_scroll"));
        let first = model.items("bench_choices")[0];
        model.activate("bench_later", 0);
        assert_ne!(model.items("bench_choices")[0], first, "one row further");
        model.scroll("bench_choices", 100);
        let last_rows = total.div_ceil(BENCH_COLUMNS).saturating_sub(BENCH_ROWS);
        assert_eq!(
            model.state.bench_scroll, last_rows,
            "no further than the end"
        );
        let shown = model.items("bench_choices").len();
        assert_eq!(shown, total - last_rows * BENCH_COLUMNS);
        assert!(model.press("bench_choices", 0, SlotPress::Click(Click::Primary)));
        model.activate("bench_earlier", 0);
        assert_eq!(model.state.bench_scroll, last_rows - 1);
    });
    let expected = crafting.view.cuts[(state.bench_scroll + 1) * BENCH_COLUMNS].0;
    assert_eq!(
        inventory.take_requests(),
        [InventoryRequest::ChooseCut(expected)]
    );
    let tannery = opened(&content, "strata:tannery");
    with_model(&content, &tannery, &mut state, &mut inventory, |model| {
        assert!(!model.shown("bench_scroll"), "the tannery's list fits");
        assert_eq!(model.items("bench_choices").len(), tannery.view.cuts.len());
    });
}

/// The children of group `wanted`, searched through panels and groups.
fn find<'a>(nodes: &'a [Node], wanted: &str) -> Option<&'a [Node]> {
    nodes.iter().find_map(|node| match node {
        Node::Group { id, children } if id == wanted => Some(children.as_slice()),
        Node::Panel { children, .. } | Node::Group { children, .. } => find(children, wanted),
        _ => None,
    })
}

fn ids(nodes: &[Node], out: &mut Vec<String>) {
    for node in nodes {
        if let Some(id) = node.id() {
            out.push(id.to_owned());
        }
        if let Node::Panel { children, .. } | Node::Group { children, .. } = node {
            ids(children, out);
        }
    }
}

#[test]
fn every_arrangement_has_cells_list_result_and_scroll_buttons() {
    let (_, ui) = assets();
    let side = find(&ui.screens.slots.root, "bench_side").unwrap();
    for layout in BenchLayout::ALL {
        let group = format!("{LAYOUT_PREFIX}{}", layout.name());
        let nodes = find(side, &group).unwrap_or_else(|| panic!("{group}"));
        let mut found = Vec::new();
        ids(nodes, &mut found);
        for wanted in [
            "bench_band_title",
            "bench_title",
            "bench",
            "bench_choices",
            "bench_result",
            "bench_earlier",
            "bench_later",
            "bench_note",
        ] {
            assert!(found.iter().any(|id| id == wanted), "{group}: {wanted}");
        }
        let list = nodes.iter().find_map(|node| match node {
            Node::ItemGrid { columns, rows, .. } => Some((*columns, *rows)),
            _ => None,
        });
        let shape = (BENCH_COLUMNS as u32, BENCH_ROWS as u32);
        assert_eq!(list, Some(shape), "{group}: the list the model pages");
        let cells = nodes.iter().find_map(|node| match node {
            Node::SlotGrid {
                id, columns, rows, ..
            } if id == "bench" => Some(columns * rows),
            _ => None,
        });
        assert_eq!(cells, Some(4), "{group}");
    }
}
