use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use strata_core::content::{BlockId, Content};
use strata_core::coords::BlockPos;
use strata_core::pack::PackSet;
use strata_core::updates::{BlockAccess, BlockUpdates, FluidSettings, UpdateRules};

const GROUND: i32 = 64;
const HALF_WIDTH: i32 = 48;
/// Ticks of burning before the measure: about 5 300 fires then burn at once.
const WARM_UP: u32 = 200;
/// The plain is set alight at every third block in both directions.
const LIT_EVERY: i32 = 3;
/// About one step of every fire (`tick_rate` 30 + up to 9).
const MEASURED_TICKS: u32 = 40;

/// Stone, under a 97 × 97 plain of grass blocks covered with tall grass.
struct Plain {
    blocks: HashMap<BlockPos, BlockId>,
    stone: BlockId,
}

impl BlockAccess for Plain {
    fn block(&self, pos: BlockPos) -> Option<BlockId> {
        let default = if pos.y < GROUND {
            self.stone
        } else {
            BlockId::AIR
        };
        Some(self.blocks.get(&pos).copied().unwrap_or(default))
    }

    fn set_block(&mut self, pos: BlockPos, block: BlockId) -> bool {
        self.blocks.insert(pos, block);
        true
    }
}

fn content() -> Content {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    Content::load(&packs, &PackSet::default()).0
}

fn block(content: &Content, name: &str) -> BlockId {
    let id = name.parse().ok();
    id.and_then(|id| content.blocks.key(&id))
        .unwrap_or(BlockId::AIR)
}

fn burning_plain(content: &Content, rules: &Arc<UpdateRules>) -> (Plain, BlockUpdates) {
    let (grass, tall) = (
        block(content, "strata:grass_block"),
        block(content, "strata:tall_grass"),
    );
    let mut world = Plain {
        blocks: HashMap::new(),
        stone: block(content, "strata:stone"),
    };
    let span = -HALF_WIDTH..=HALF_WIDTH;
    for (x, z) in span.clone().flat_map(|x| span.clone().map(move |z| (x, z))) {
        world.blocks.insert(BlockPos::new(x, GROUND - 1, z), grass);
        world.blocks.insert(BlockPos::new(x, GROUND, z), tall);
    }
    let mut sim = BlockUpdates::new(Arc::clone(rules), FluidSettings::default());
    let fire = block(content, "strata:fire");
    let mut changes = Vec::new();
    for (x, z) in span.clone().flat_map(|x| span.clone().map(move |z| (x, z))) {
        if x % LIT_EVERY == 0 && z % LIT_EVERY == 0 {
            sim.set_block(&mut world, BlockPos::new(x, GROUND, z), fire, &mut changes);
        }
    }
    for _ in 0..WARM_UP {
        sim.tick(&mut world, &mut changes);
    }
    (world, sim)
}

fn prairie_fire(c: &mut Criterion) {
    let content = content();
    let rules = Arc::new(UpdateRules::new(&content));
    let mut group = c.benchmark_group("fire");
    group.sample_size(10);
    group.bench_function("prairie_fire_40_ticks", |b| {
        b.iter_batched(
            || burning_plain(&content, &rules),
            |(mut world, mut sim)| {
                let mut changes = Vec::new();
                for _ in 0..MEASURED_TICKS {
                    sim.tick(&mut world, &mut changes);
                }
                (changes.len(), sim.active())
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

criterion_group!(benches, prairie_fire);
criterion_main!(benches);
