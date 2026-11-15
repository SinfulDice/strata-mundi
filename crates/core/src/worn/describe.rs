//! What the interface tells about sets: the pieces worn out of the set's, and each
//! bonus, the one reached marked.

use super::WornStats;
use crate::content::{Content, ItemId};

impl Content {
    /// For each set `item` belongs to: « Leather set: 2/4 worn », then one line per
    /// bonus (« 2 pieces: +1 protection », marked when reached). Nothing while the
    /// item's stats are not known.
    pub fn set_lines(&self, item: ItemId, worn: &WornStats, language: &str) -> Vec<String> {
        if !self.stats_known(item) {
            return Vec::new();
        }
        let tr = &self.translations;
        let mut lines = Vec::new();
        for (index, set) in self.worn.sets.iter().enumerate() {
            if !set.pieces.contains(&item) {
                continue;
            }
            let progress = worn.set(index);
            let args = [
                ("set", tr.text(language, &set.id.translation_key("set"))),
                ("worn", progress.map_or(0, |p| p.worn).to_string()),
                ("total", set.size().to_string()),
            ];
            lines.push(tr.text_with(language, "set-worn-line", &args));
            for (at, bonus) in set.bonuses.iter().enumerate() {
                let effects: Vec<String> = (bonus.effects.iter())
                    .map(|effect| self.effect_text(effect, language))
                    .collect();
                let reached = progress.and_then(|p| p.bonus) == Some(at);
                let args = [
                    ("pieces", bonus.pieces.to_string()),
                    ("effects", effects.join(", ")),
                ];
                let key = if reached {
                    "set-bonus-active"
                } else {
                    "set-bonus-line"
                };
                lines.push(tr.text_with(language, key, &args));
            }
        }
        lines
    }
}
