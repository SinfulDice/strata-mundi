use super::super::tests::{World, block, wait, world};
use super::*;
use strata_core::content::BlockId;

fn place(w: &mut World, id: u32, against: BlockPos) -> Vec<BlockChange> {
    let planks = block(&w.content, "strata:oak_planks");
    let action = BlockAction::Place {
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
    let action = BlockAction::Break { pos: far };
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
fn a_full_queue_answers_at_once() {
    let mut w = world();
    let ground: Vec<BlockPos> = (0..10).map(|x| BlockPos::new(4 + x, 63, 6)).collect();
    for (id, &pos) in (1..).zip(&ground) {
        place(&mut w, id, pos);
    }
    let answered = acknowledged(&w.game.take_messages());
    assert_eq!(
        answered,
        vec![1, 10],
        "the first ran, the tenth found no room"
    );
    wait(&mut w, 2 * MAX_QUEUED_ACTIONS as u32);
    let answered = acknowledged(&w.game.take_messages());
    assert_eq!(answered, (2..10).collect::<Vec<_>>(), "in order, none lost");
}
