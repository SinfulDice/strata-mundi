//! The client's copy of the crafting grid and of the recipes the player knows, as the
//! server last told them.

use std::collections::BTreeSet;
use strata_core::content::RecipeId;
use strata_core::inventory::{Container, CraftGrid};
use strata_protocol::{CraftingView, ServerMessage};

#[derive(Debug, Clone, PartialEq)]
pub struct ClientCrafting {
    pub view: CraftingView,
    known: BTreeSet<RecipeId>,
    /// Every recipe counts as known (creative, or the world option).
    pub all_known: bool,
    /// A work station just opened: its screen must come up.
    pub station_opened: bool,
    /// The last recipe learnt by experimenting, shown until the screen closes.
    pub learned: Option<RecipeId>,
}

impl Default for ClientCrafting {
    fn default() -> Self {
        let side = CraftGrid::INVENTORY;
        Self {
            view: CraftingView {
                side,
                grid: Container::new(usize::from(side * side)),
                result: None,
                hint: false,
                station: None,
            },
            known: BTreeSet::new(),
            all_known: false,
            station_opened: false,
            learned: None,
        }
    }
}

impl ClientCrafting {
    /// Recipes learnt (the journal rebuilds its list when this changes).
    pub fn known_count(&self) -> usize {
        self.known.len()
    }

    pub fn knows(&self, recipe: RecipeId) -> bool {
        self.all_known || self.known.contains(&recipe)
    }

    /// Takes the crafting messages; others are left to the caller.
    pub fn receive(&mut self, message: ServerMessage) -> Option<ServerMessage> {
        match message {
            ServerMessage::Crafting(view) => {
                self.station_opened |= self.view.station.is_none() && view.station.is_some();
                self.view = *view;
            }
            ServerMessage::Recipes { known, all } => {
                self.known = known.into_iter().collect();
                self.all_known = all;
            }
            ServerMessage::RecipeLearned(recipe) => {
                self.known.insert(recipe);
                self.learned = Some(recipe);
            }
            other => return Some(other),
        }
        None
    }

    /// The screen closed: the server gives the station's grid back.
    pub fn closed(&mut self) {
        *self = Self {
            known: std::mem::take(&mut self.known),
            all_known: self.all_known,
            ..Self::default()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strata_core::content::BlockId;
    use strata_core::coords::BlockPos;

    #[test]
    fn a_station_view_opens_its_screen_and_learning_is_remembered() {
        let mut crafting = ClientCrafting::default();
        let mut view = crafting.view.clone();
        view.station = Some((BlockId(3), BlockPos::new(0, 0, 0)));
        assert!(
            crafting
                .receive(ServerMessage::Crafting(Box::new(view)))
                .is_none()
        );
        assert!(crafting.station_opened);
        crafting.receive(ServerMessage::RecipeLearned(RecipeId(4)));
        assert!(crafting.knows(RecipeId(4)) && !crafting.knows(RecipeId(5)));
        crafting.closed();
        assert!(crafting.knows(RecipeId(4)));
        assert_eq!((crafting.view.side, crafting.learned), (2, None));
        assert!(crafting.receive(ServerMessage::Saved).is_some());
    }
}
