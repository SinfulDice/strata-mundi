use super::*;
use std::time::{Duration, Instant};
use strata_core::content::Block;
use strata_core::light::PackedLight;

fn table() -> LightTable {
    let torch = Block {
        light_emission: 14,
        ..Block::air()
    };
    LightTable::new([Block::air(), torch].iter())
}

fn wait(worker: &mut LightWorker) -> (Vec<ColumnPos>, Vec<(ColumnPos, usize, SectionLight)>) {
    let start = Instant::now();
    let (mut lit, mut sections) = (Vec::new(), Vec::new());
    while start.elapsed() < Duration::from_secs(10) {
        for result in worker.finished() {
            lit.extend(result.lit);
            sections.extend(result.sections);
        }
        if worker.is_idle() {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    (lit, sections)
}

#[test]
fn columns_are_lit_once_their_neighbours_arrive_and_torches_update_them() {
    let mut worker = LightWorker::spawn(table()).unwrap();
    let columns = ring(&mut worker);
    let origin = ORIGIN;
    let (lit, sections) = wait(&mut worker);
    assert_eq!(lit, vec![origin], "only the column with its 8 neighbours");
    assert_eq!(sections.len(), 32);
    assert!(sections.iter().all(|(pos, _, _)| *pos == origin));
    let mut column = Arc::clone(&columns[&origin]);
    Arc::make_mut(&mut column).set(4, 100, 4, BlockId(1));
    worker.changed(column, vec![(BlockPos::new(4, 100, 4), BlockId::AIR)]);
    let (lit, changed) = wait(&mut worker);
    assert!(lit.is_empty());
    let section = changed
        .iter()
        .find(|(pos, index, _)| *pos == origin && *index == 6)
        .map(|(_, _, light)| light.clone());
    let cell = strata_core::light::section_index(4, 100 - 96, 4);
    assert_eq!(section.map(|s| s.get(cell)), Some(PackedLight::new(15, 14)));
}

fn ring(worker: &mut LightWorker) -> std::collections::HashMap<ColumnPos, Arc<Column>> {
    let mut columns = std::collections::HashMap::new();
    for pos in around(ORIGIN) {
        let column = Arc::new(Column::empty(pos));
        columns.insert(pos, Arc::clone(&column));
        worker.insert(column);
    }
    columns
}

const ORIGIN: ColumnPos = ColumnPos { x: 0, z: 0 };

fn torch_column() -> Arc<Column> {
    let mut column = Column::empty(ORIGIN);
    column.set(4, 100, 4, BlockId(1));
    Arc::new(column)
}

fn block_light_at_torch(sections: &[(ColumnPos, usize, SectionLight)]) -> Option<u8> {
    let cell = strata_core::light::section_index(4, 100 - 96, 4);
    sections
        .iter()
        .rfind(|(pos, index, _)| *pos == ORIGIN && *index == 6)
        .map(|(_, _, light)| light.get(cell).block())
}

#[test]
fn a_column_sent_again_or_after_its_removal_is_lit_again() {
    let mut worker = LightWorker::spawn(table()).unwrap();
    ring(&mut worker);
    assert_eq!(wait(&mut worker).0, vec![ORIGIN]);
    worker.insert(torch_column());
    let (lit, sections) = wait(&mut worker);
    assert_eq!(lit, vec![ORIGIN], "lit again with its new blocks");
    assert_eq!(block_light_at_torch(&sections), Some(14));
    worker.remove(ORIGIN);
    worker.insert(torch_column());
    let (lit, sections) = wait(&mut worker);
    assert_eq!(lit, vec![ORIGIN]);
    assert_eq!(block_light_at_torch(&sections), Some(14));
}

#[test]
fn edits_jump_ahead_of_inserts_without_reordering_a_column() {
    let at = |x| ColumnPos { x, z: 0 };
    let mut pending = Pending::default();
    pending.push(LightJob::Insert(Arc::new(Column::empty(at(1)))));
    pending.push(LightJob::Insert(Arc::new(Column::empty(at(2)))));
    pending.push(LightJob::Changed(
        Arc::new(Column::empty(at(2))),
        Vec::new(),
    ));
    pending.push(LightJob::Changed(
        Arc::new(Column::empty(at(3))),
        Vec::new(),
    ));
    pending.push(LightJob::Remove(at(1)));
    let order: Vec<_> = std::iter::from_fn(|| pending.pop())
        .map(|job| match job {
            LightJob::Insert(column) => ("insert", column.pos.x),
            LightJob::Changed(column, _) => ("changed", column.pos.x),
            LightJob::Remove(pos) => ("remove", pos.x),
        })
        .collect();
    assert_eq!(order, [("changed", 3), ("remove", 1), ("insert", 2)]);
    assert_eq!(
        pending.merged, 2,
        "the dropped insert and the merged change"
    );
}

#[test]
fn a_batch_is_bounded() {
    let mut pending = Pending::default();
    for x in 0..40 {
        pending.push(LightJob::Insert(Arc::new(Column::empty(ColumnPos {
            x,
            z: 0,
        }))));
    }
    let mut state = LightState::default();
    let stop = AtomicBool::new(false);
    let result = state.batch(&mut pending, &table(), &stop);
    assert!(result.is_some_and(|r| (1..=BATCH_JOBS).contains(&r.jobs)));
    stop.store(true, Ordering::Relaxed);
    assert!(state.batch(&mut pending, &table(), &stop).is_none());
}
