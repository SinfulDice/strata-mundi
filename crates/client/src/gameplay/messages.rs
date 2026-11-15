//! What the server tells the player's session: welcome, corrections, blocks, inventory.

use super::*;
use strata_protocol::ServerMessage;

impl Gameplay {
    /// Applies one message of the server to the local view of the world.
    pub fn apply(
        &mut self,
        message: ServerMessage,
        renderer: &mut Renderer,
        connection: &ClientConnection,
    ) {
        match message {
            ServerMessage::Welcome {
                dimension,
                body,
                mut movement,
                border,
                inventory,
                mode,
                look,
                day,
                biomes,
            } => {
                self.mode = mode;
                self.interaction.mining.clear();
                let tints = biomes.iter().map(|biome| biome.tints).collect();
                self.view.world.set_biome_tints(tints);
                self.biomes = biomes;
                self.dimension = Some(dimension);
                self.day = Some(DayClock::at(day.time, day.flows));
                self.day_cycle = Some(DayCycle::new(&day.cycle));
                self.border = border;
                (self.camera.yaw, self.camera.pitch) = (look[0], look[1]);
                self.inventory.restore(*inventory);
                self.inventory.creative = mode == GameMode::Creative;
                movement.auto_jump = self.controls.auto_jump;
                movement.step_up = self.controls.step_up;
                connection.send(ClientMessage::MovementPreferences {
                    auto_jump: movement.auto_jump,
                    step_up: movement.step_up,
                });
                self.player = Some(LocalPlayer::new(body, movement));
            }
            ServerMessage::PlayerState { sequence, body } => {
                let terrain = terrain(&self.view.world, &self.info, self.border);
                if let Some(player) = &mut self.player {
                    player.reconcile(sequence, body, &terrain);
                }
            }
            ServerMessage::Column { column, biomes } => self.view.insert(*column, biomes),
            ServerMessage::UnloadColumn(pos) => self.view.remove(pos, renderer),
            ServerMessage::BlockChanged { pos, block } => self.view.receive_block(pos, block),
            ServerMessage::ActionDone(id) => self.view.acknowledge(id),
            ServerMessage::Cracking {
                breaker,
                pos,
                progress,
            } => self.interaction.mining.report(breaker, pos, progress),
            ServerMessage::Inventory(inventory) => self.inventory.receive(*inventory),
            ServerMessage::ContainerOpened(container) => {
                self.container = Some(*container);
                self.container_opened = true;
            }
            ServerMessage::ContainerContents(contents) => {
                if let Some(open) = &mut self.container {
                    open.contents = *contents;
                }
            }
            ServerMessage::ContainerClosed => self.container = None,
            ServerMessage::Furnace(view) => {
                if let Some(open) = &mut self.container {
                    open.furnace = Some(view);
                    self.furnace_clock.restart();
                }
            }
            message @ (ServerMessage::Crafting(_)
            | ServerMessage::Recipes { .. }
            | ServerMessage::RecipeLearned(_)) => {
                self.crafting.receive(message);
            }
            ServerMessage::Feedback(feedback) => self.feedback.push(feedback),
            ServerMessage::EntitySpawned(state) => self.view.world.entities.spawn(state),
            ServerMessage::EntityMoved { id, position, yaw } => {
                self.view.world.entities.moved(id, position, yaw);
            }
            ServerMessage::EntityRemoved(id) => {
                self.view.world.entities.remove(id);
                self.interaction.mining.forget(id);
            }
            ServerMessage::Mode(mode) => {
                self.mode = mode;
                self.inventory.creative = mode == GameMode::Creative;
            }
            ServerMessage::Mounted(mount) => {
                self.interaction.mount = mount;
                if let Some(player) = &mut self.player {
                    player.riding = mount.is_some();
                }
            }
            ServerMessage::DayTime { time, cycle } => match &mut self.day {
                Some(day) => day.receive(time, cycle),
                None => self.day = Some(DayClock::at(time, cycle)),
            },
            ServerMessage::Tick { .. } | ServerMessage::Saved => {}
        }
    }
}
