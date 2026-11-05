//! Flood fill of light, adding and removing, in two queues like Minecraft: removing first
//! darkens everything the old light may have reached, then the edge of the dark area and
//! the sources inside it spread light back in.

use super::{BlockSource, Channel, LightTable, Lighting, attenuate, block_at, source};
use crate::content::{BlockId, Face};
use crate::coords::BlockPos;

impl Lighting {
    /// Brings the light up to date after the block at `pos` was `old`. Light only moves
    /// inside lit columns, across their borders too.
    pub fn block_changed(
        &mut self,
        blocks: &impl BlockSource,
        table: &LightTable,
        pos: BlockPos,
        old: BlockId,
    ) {
        let Some(new) = block_at(blocks, pos) else {
            return;
        };
        if self.light_at(pos).is_none() {
            return;
        }
        for channel in Channel::ALL {
            if table.changes_light(channel, old, new) {
                self.relight(blocks, table, channel, pos, new);
            }
        }
    }

    fn relight(
        &mut self,
        blocks: &impl BlockSource,
        table: &LightTable,
        channel: Channel,
        pos: BlockPos,
        block: BlockId,
    ) {
        let Some(light) = self.light_at(pos) else {
            return;
        };
        let level = light.get(channel);
        if level > 0 {
            self.write(pos, light.with(channel, 0));
            self.removals.push_back((pos, level));
            self.unspread(blocks, table, channel);
        }
        let own = source(channel, pos, block, table);
        if let Some(light) = self.light_at(pos)
            && own > light.get(channel)
        {
            self.write(pos, light.with(channel, own));
            self.queue.push_back(pos);
        }
        for face in Face::ALL {
            let near = pos.neighbour(face);
            if self.light_at(near).is_some_and(|l| l.get(channel) > 0) {
                self.queue.push_back(near);
            }
        }
        self.spread(blocks, table, channel);
    }

    /// Spreads the light of every queued block until nothing brightens any more.
    pub(super) fn spread(
        &mut self,
        blocks: &impl BlockSource,
        table: &LightTable,
        channel: Channel,
    ) {
        while let Some(pos) = self.queue.pop_front() {
            let Some(light) = self.light_at(pos) else {
                continue;
            };
            let level = light.get(channel);
            if level <= 1 {
                continue;
            }
            for face in Face::ALL {
                let near = pos.neighbour(face);
                let Some(near_light) = self.light_at(near) else {
                    continue;
                };
                let Some(block) = block_at(blocks, near) else {
                    continue;
                };
                let reached = attenuate(channel, level, face, table.opacity(block));
                if reached > near_light.get(channel) {
                    self.write(near, near_light.with(channel, reached));
                    self.queue.push_back(near);
                }
            }
        }
    }

    /// Darkens what the removed light may have lit; queues the brighter edge and the
    /// sources found inside for `spread`.
    fn unspread(&mut self, blocks: &impl BlockSource, table: &LightTable, channel: Channel) {
        while let Some((pos, level)) = self.removals.pop_front() {
            for face in Face::ALL {
                let near = pos.neighbour(face);
                let Some(near_light) = self.light_at(near) else {
                    continue;
                };
                let near_level = near_light.get(channel);
                let Some(block) = block_at(blocks, near).filter(|_| near_level > 0) else {
                    continue;
                };
                if near_level > attenuate(channel, level, face, table.opacity(block)) {
                    self.queue.push_back(near);
                    continue;
                }
                self.removals.push_back((near, near_level));
                let own = source(channel, near, block, table);
                self.write(near, near_light.with(channel, own));
                if own > 0 {
                    self.queue.push_back(near);
                }
            }
        }
    }
}
