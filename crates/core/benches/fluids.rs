use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use strata_core::content::{BlockId, Content};
use strata_core::coords::BlockPos;
use strata_core::pack::PackSet;
use strata_core::updates::{BlockAccess, BlockUpdates, FluidSettings, UpdateRules};

const GROUND: i32 = 64;

/// Flat stone below y = 64, air above, with edits on top.
struct Plane {
    blocks: HashMap<BlockPos, BlockId>,
    stone: BlockId,
}

impl BlockAccess for Plane {
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

/// A 32×32 lake beside a 20-block-wide pit, its 32-block wall just broken.
fn breached_lake(content: &Content, rules: &Arc<UpdateRules>) -> (Plane, BlockUpdates) {
    let water = block(content, "strata:water");
    let stone = block(content, "strata:stone");
    let mut world = Plane {
        blocks: HashMap::new(),
        stone,
    };
    for (x, z) in (0..32).flat_map(|x| (0..32).map(move |z| (x, z))) {
        world.blocks.insert(BlockPos::new(x, GROUND, z), water);
    }
    for (x, z) in (-20..0).flat_map(|x| (-8..40).map(move |z| (x, z))) {
        for y in GROUND - 6..GROUND {
            world.blocks.insert(BlockPos::new(x, y, z), BlockId::AIR);
        }
    }
    let mut sim = BlockUpdates::new(Arc::clone(rules), FluidSettings::default());
    for z in 0..32 {
        let wall = BlockPos::new(-1, GROUND, z);
        world.blocks.insert(wall, stone);
        sim.set_block(&mut world, wall, BlockId::AIR, &mut Vec::new());
    }
    (world, sim)
}

fn large_flow(c: &mut Criterion) {
    let content = content();
    let rules = Arc::new(UpdateRules::new(&content));
    let mut group = c.benchmark_group("fluids");
    group.sample_size(10);
    group.bench_function("large_flow_settles", |b| {
        b.iter_batched(
            || breached_lake(&content, &rules),
            |(mut world, mut sim)| {
                let mut changes = Vec::new();
                for _ in 0..5000 {
                    if sim.active() == 0 {
                        break;
                    }
                    sim.tick(&mut world, &mut changes);
                }
                changes.len()
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

criterion_group!(benches, large_flow);
criterion_main!(benches);
