//! A client predicting its body and its clicks, against the server: breaking the block
//! under the feet must give the same fall on both sides, never a correction.

use super::tests::{World, world};
use std::collections::VecDeque;
use strata_core::content::{BlockId, Face};
use strata_core::coords::BlockPos;
use strata_core::physics::{Body, MoveInput, MovementSettings, step};
use strata_protocol::{ActionId, GameMode, WorldAction};

const UNDER: BlockPos = BlockPos { x: 8, y: 63, z: 8 };

/// The client's side, as `LocalPlayer` and the block predictions do it.
struct Client {
    body: Body,
    movement: MovementSettings,
    sequence: u32,
    pending: VecDeque<(u32, MoveInput)>,
    /// The input after which the block underfoot was predicted broken.
    broken_after: Option<u32>,
}

impl Client {
    fn new(w: &World) -> Self {
        Self {
            body: *w.game.player_body().unwrap(),
            movement: w.game.movement().unwrap(),
            sequence: 0,
            pending: VecDeque::new(),
            broken_after: None,
        }
    }

    /// The terrain for input `input`: the server's before the click, the predicted after.
    fn terrain(&self, input: u32) -> impl Fn(i32, i32, i32) -> bool + use<> {
        let broken = self.broken_after.is_some_and(|after| input > after);
        move |x, y, z| y < 64 && !(broken && BlockPos::new(x, y, z) == UNDER)
    }

    /// One predicted tick, sent to the server (which does not tick yet).
    fn input(&mut self, w: &mut World) {
        self.sequence += 1;
        let input = MoveInput::default();
        let terrain = self.terrain(self.sequence);
        step(&mut self.body, &input, &self.movement, &terrain);
        self.pending.push_back((self.sequence, input));
        w.game.queue_input(self.sequence, input);
    }

    /// The server's state: replays the unacknowledged inputs; the correction it took.
    fn reconcile(&mut self, w: &World) -> f64 {
        let acknowledged = w.game.last_sequence();
        self.pending.retain(|(s, _)| *s > acknowledged);
        let mut body = *w.game.player_body().unwrap();
        for (sequence, input) in &self.pending {
            step(&mut body, input, &self.movement, &self.terrain(*sequence));
        }
        let error = body.position.distance(self.body.position);
        self.body = body;
        error
    }

    fn act(&mut self, w: &mut World, id: u32, action: WorldAction) {
        if matches!(action, WorldAction::Break { pos } if pos == UNDER) {
            self.broken_after = Some(self.sequence);
        }
        w.game.request_action(&mut w.loader, ActionId(id), action);
    }
}

/// Ticks the server and reconciles the client after each input; the largest correction.
fn play(w: &mut World, client: &mut Client, ticks: u32) -> f64 {
    let mut worst: f64 = 0.0;
    for _ in 0..ticks {
        client.input(w);
        w.game.tick(&mut w.loader);
        worst = worst.max(client.reconcile(w));
    }
    worst
}

/// Two inputs still waiting on the server when the click comes, as between two ticks.
fn lagging_client(w: &mut World) -> Client {
    let mut client = Client::new(w);
    assert!(play(w, &mut client, 4) < 1e-9);
    client.input(w);
    client.input(w);
    client
}

#[test]
fn breaking_the_block_underfoot_falls_without_correction() {
    let mut w = world();
    let mut client = lagging_client(&mut w);
    let start = client.body.position.y;
    client.act(&mut w, 1, WorldAction::Break { pos: UNDER });
    assert!(play(&mut w, &mut client, 20) < 1e-9, "prediction corrected");
    assert!(client.body.position.y < start - 0.5, "the player fell");
}

#[test]
fn a_survival_break_underfoot_falls_without_correction() {
    let mut w = world();
    w.game.set_mode(GameMode::Survival);
    let mut client = lagging_client(&mut w);
    let start = WorldAction::StartBreak {
        pos: UNDER,
        face: Face::Up,
    };
    client.act(&mut w, 1, start);
    let stone = w.loader.block(UNDER).unwrap();
    let ticks = w.game.rules.mining.break_ticks(stone, None).unwrap();
    assert!(play(&mut w, &mut client, ticks) < 1e-9);
    client.act(&mut w, 2, WorldAction::Break { pos: UNDER });
    assert!(play(&mut w, &mut client, 20) < 1e-9, "prediction corrected");
    assert_eq!(w.loader.block(UNDER), Some(BlockId::AIR));
}
