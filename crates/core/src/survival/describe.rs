//! The tooltip lines of food, drinks and care: what eating, drinking or applying gives,
//! the effects it ends, the sips of a waterskin, and where an empty container fills.

use crate::content::{Content, ItemId};

impl Content {
    pub(crate) fn survival_item_lines(&self, item: ItemId, language: &str) -> Vec<String> {
        let tr = &self.translations;
        let mut lines = Vec::new();
        if let Some(use_) = self.survival.consumables.get(&item) {
            let number = |value: f32| format!("{}", (value * 10.0).round() / 10.0);
            if use_.food > 0.0 {
                let args = [
                    ("food", number(use_.food)),
                    ("saturation", number(use_.saturation)),
                ];
                lines.push(tr.text_with(language, "survival-food-line", &args));
            }
            if use_.thirst > 0.0 {
                let args = [("thirst", number(use_.thirst))];
                lines.push(tr.text_with(language, "survival-thirst-line", &args));
            } else if use_.thirst < 0.0 {
                let args = [("thirst", number(-use_.thirst))];
                lines.push(tr.text_with(language, "survival-salty-line", &args));
            }
            for grant in &use_.effects {
                let effect = tr.text(language, &grant.effect.translation_key("effect"));
                let args = [("effect", effect), ("seconds", number(grant.seconds))];
                lines.push(tr.text_with(language, "survival-effect-line", &args));
            }
            for cured in &use_.cures {
                let effect = tr.text(language, &cured.translation_key("effect"));
                lines.push(tr.text_with(language, "survival-cure-line", &[("effect", effect)]));
            }
            if let Some(sips) = use_.sips {
                let args = [("sips", sips.to_string())];
                lines.push(tr.text_with(language, "survival-sips-line", &args));
            }
        }
        if self.survival.fills.contains_key(&item) {
            lines.push(tr.text(language, "survival-fill-line"));
        }
        lines
    }
}
