use super::super::tests::{World, block, hold, wait, world};
use super::*;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::ColumnPos;

fn place(w: &mut World, id: u32, against: BlockPos) -> Vec<BlockChange> {
    let planks = block(&w.content, "strata:oak_planks");
    hold(w, w.game.rules.block_item(planks));
    let action = WorldAction::Place {
        against,
        face: Face::Up,
        block: planks,
    };
    w.game.request_action(&mut w.loader, ActionId(id), action)
}

fn acknowledged(messages: &[ServerMessage]) -> Vec<u32> {
    messages
        .iter()
        .filter_map(|message| match message {
            ServerMessage::ActionDone(ActionId(id)) => Some(*id),
            _ => None,
        })
        .collect()
}

#[test]
fn two_close_clicks_place_two_blocks() {
    let mut w = world();
    let planks = block(&w.content, "strata:oak_planks");
    let (first, second) = (BlockPos::new(10, 63, 8), BlockPos::new(11, 63, 8));
    assert_eq!(place(&mut w, 1, first).len(), 1);
    assert!(place(&mut w, 2, second).is_empty(), "too soon: waits");
    assert_eq!(acknowledged(&w.game.take_messages()), vec![1]);
    wait(&mut w, 2);
    for pos in [first, second] {
        assert_eq!(w.loader.block(pos.neighbour(Face::Up)), Some(planks));
    }
    assert_eq!(acknowledged(&w.game.take_messages()), vec![2]);
}

#[test]
fn a_refused_click_sends_the_actual_cells_then_its_answer() {
    let mut w = world();
    let far = BlockPos::new(15, 63, 15);
    assert!(place(&mut w, 1, far).is_empty());
    let messages = w.game.take_messages();
    let cells = [far, far.neighbour(Face::Up)];
    let expected: Vec<ServerMessage> = cells
        .iter()
        .map(|&pos| ServerMessage::BlockChanged {
            pos,
            block: w.loader.block(pos).unwrap(),
        })
        .chain([ServerMessage::ActionDone(ActionId(1))])
        .collect();
    assert_eq!(messages, expected);
    assert_eq!(w.loader.block(far.neighbour(Face::Up)), Some(BlockId::AIR));
}

#[test]
fn queued_clicks_are_still_validated() {
    let mut w = world();
    assert_eq!(place(&mut w, 1, BlockPos::new(10, 63, 8)).len(), 1);
    let far = BlockPos::new(15, 63, 15);
    let action = WorldAction::Break { pos: far };
    assert!(
        w.game
            .request_action(&mut w.loader, ActionId(2), action)
            .is_empty()
    );
    w.game.take_messages();
    wait(&mut w, 2);
    assert_ne!(w.loader.block(far), Some(BlockId::AIR), "out of reach");
    let messages = w.game.take_messages();
    assert!(messages.contains(&ServerMessage::BlockChanged {
        pos: far,
        block: w.loader.block(far).unwrap(),
    }));
    assert_eq!(acknowledged(&messages), vec![2]);
}

#[test]
fn a_full_queue_still_answers_in_the_order_of_the_clicks() {
    let mut w = world();
    let planks = block(&w.content, "strata:oak_planks");
    let ground: Vec<BlockPos> = [6, 10]
        .into_iter()
        .flat_map(|z| (4..13).map(move |x| BlockPos::new(x, 63, z)))
        .collect();
    let clicks = 1 + 2 * MAX_PENDING_ACTIONS as u32;
    for (id, &pos) in (1..=clicks).zip(&ground) {
        place(&mut w, id, pos);
    }
    let mut answered = acknowledged(&w.game.take_messages());
    assert_eq!(answered, vec![1]);
    wait(&mut w, 2 * MAX_PENDING_ACTIONS as u32);
    answered.extend(acknowledged(&w.game.take_messages()));
    assert_eq!(answered, (1..=clicks).collect::<Vec<_>>());
    let placed = |pos: &&BlockPos| w.loader.block(pos.neighbour(Face::Up)) == Some(planks);
    let count = ground.iter().take(clicks as usize).filter(placed).count();
    assert_eq!(
        count,
        1 + MAX_PENDING_ACTIONS,
        "the clicks past the queue are refused"
    );
}

#[test]
fn a_refusal_tells_only_about_cells_the_player_sees() {
    let mut w = world();
    let stone = block(&w.content, "strata:stone");
    let mut column = Column::empty(ColumnPos { x: 5, z: 0 });
    column.set(0, 63, 0, stone);
    w.loader.insert(column);
    let far = BlockPos::new(16 * 5, 63, 0);
    assert_eq!(w.loader.block(far), Some(stone), "loaded on the server");
    let action = WorldAction::Break { pos: far };
    w.game.request_action(&mut w.loader, ActionId(1), action);
    let messages = w.game.take_messages();
    assert_eq!(messages, vec![ServerMessage::ActionDone(ActionId(1))]);
}
