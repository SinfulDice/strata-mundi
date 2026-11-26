use crate::audio::{Direction, LIFETIME, Subtitle};
use crate::gameplay::Gameplay;
use crate::inventory::{ClientInventory, SLOTS};
use crate::ui::{LogLine, ScreenModel, SlotMark, Tone};
use strata_core::content::{Content, ItemId, ItemKind};

/// Data shown by the in-game overlay (sample values in the HUD editor).
pub struct HudModel<'a> {
    pub hotbar: &'a ClientInventory,
    pub content: &'a Content,
    /// The hammer area chosen, while a hammer is held.
    pub area: Option<String>,
    /// Translated name of the aimed block, if shown.
    pub target: Option<String>,
    /// How to get off the boat, while riding.
    pub mount: Option<String>,
    /// Sounds heard lately, oldest first.
    pub subtitles: Vec<LogLine>,
    /// The survival bars (none in creative); sample values in the editor.
    pub bars: Bars,
    /// Status effects, translated, with their time left.
    pub effects: Option<String>,
    /// The notice after fainting.
    pub notice: Option<String>,
}

/// How full each bar of the HUD is.
#[derive(Debug, Clone, Default)]
pub struct Bars {
    pub vitals: Option<strata_protocol::VitalsView>,
    /// The bite or sip going on.
    pub using: Option<f32>,
    /// Show the mana bar, empty, before magic exists (option).
    pub empty_mana: bool,
    /// HUD editor: every bar half full.
    pub sample: bool,
}

impl Bars {
    fn gauge(&self, id: &str) -> Option<f32> {
        if self.sample {
            return (id != "use").then_some(0.7);
        }
        if id == "use" {
            return self.using;
        }
        let vitals = self.vitals.as_ref()?;
        let fraction = |value: f32, max: f32| value / max.max(1.0);
        match id {
            "health" => Some(fraction(vitals.health, vitals.max_health)),
            "stamina" => vitals.stamina.map(|s| fraction(s, vitals.max_stamina)),
            "mana" => self.empty_mana.then_some(0.0),
            "food" => vitals.food.map(|f| fraction(f, vitals.max_food)),
            "thirst" => vitals.thirst.map(|t| fraction(t, vitals.max_thirst)),
            "breath" => (vitals.breath < vitals.max_breath)
                .then(|| fraction(f32::from(vitals.breath), f32::from(vitals.max_breath))),
            // Temperature comes with milestone 8b, experience with milestone 9.
            _ => None,
        }
    }
}

impl ScreenModel for HudModel<'_> {
    fn text(&self, id: &str) -> Option<String> {
        match id {
            "target" => self.target.clone(),
            "mount" => self.mount.clone(),
            "area" => self.area.clone(),
            "effects" => self.effects.clone(),
            "notice" => self.notice.clone(),
            _ => None,
        }
    }

    fn gauge(&self, id: &str) -> Option<f32> {
        self.bars.gauge(id)
    }

    fn items(&self, id: &str) -> Vec<Option<ItemId>> {
        match id {
            "hotbar" => (0..SLOTS).map(|i| self.hotbar.slot(i)).collect(),
            _ => Vec::new(),
        }
    }

    fn selected(&self, id: &str) -> Option<usize> {
        (id == "hotbar").then(|| self.hotbar.selected_index())
    }

    fn marks(&self, id: &str) -> Vec<SlotMark> {
        match id {
            "hotbar" => self.hotbar.marks(SLOTS, self.content),
            _ => Vec::new(),
        }
    }

    fn log_lines(&self, id: &str) -> Vec<LogLine> {
        match id {
            "subtitles" => self.subtitles.clone(),
            _ => Vec::new(),
        }
    }
}

/// Translated subtitles with their arrow; fading ones are dimmed.
pub fn subtitle_lines(subtitles: &[Subtitle], content: &Content, language: &str) -> Vec<LogLine> {
    let translations = &content.translations;
    subtitles
        .iter()
        .map(|line| {
            let sound = translations.text(language, &line.key);
            let text = translations.text_with(language, line.direction.key(), &[("sound", sound)]);
            let tone = if line.age > LIFETIME * 0.6 {
                Tone::Dim
            } else {
                Tone::Normal
            };
            LogLine { text, tone }
        })
        .collect()
}

/// What the HUD editor shows in place of real subtitles.
pub fn sample_subtitles() -> Vec<Subtitle> {
    vec![Subtitle {
        key: "subtitle-footsteps".to_owned(),
        direction: Direction::Left,
        age: 0.0,
    }]
}

/// Translated name of the block the player aims at, unless hidden in the options.
pub fn aimed_block_name(game: &Gameplay, content: &Content, language: &str) -> Option<String> {
    if !game.controls.show_block_name {
        return None;
    }
    let block = game.view.world.block_at(game.interaction.target?.block);
    let id = content.block_name(block)?;
    let item = content
        .items
        .key(id)
        .and_then(|item| content.items.get(item))?;
    matches!(item.kind, ItemKind::Block(_)).then(|| {
        content
            .translations
            .text(language, &id.translation_key("block"))
    })
}

/// The effects on the player, translated: « Saignement 12 s · Soin 4 s ».
pub fn effect_line(game: &Gameplay, content: &Content, language: &str) -> Option<String> {
    let view = game.vitals.view.as_ref().filter(|_| !game.is_creative())?;
    let translations = &content.translations;
    let parts: Vec<String> = view
        .effects
        .iter()
        .map(|effect| {
            let name = translations.text(language, &effect.effect.translation_key("effect"));
            let seconds = effect.seconds_left.to_string();
            translations.text_with(
                language,
                "hud-effect",
                &[("effect", name), ("seconds", seconds)],
            )
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
}

/// The survival bars of the player (none in creative).
pub fn player_bars(game: &Gameplay, empty_mana: bool) -> Bars {
    let survival = !game.is_creative();
    Bars {
        vitals: game.vitals.view.clone().filter(|_| survival),
        using: game
            .vitals
            .using
            .as_ref()
            .map(crate::gameplay::ClientUse::progress),
        empty_mana: empty_mana && survival,
        sample: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strata_protocol::VitalsView;

    fn vitals() -> VitalsView {
        VitalsView {
            health: 10.0,
            max_health: 20.0,
            food: Some(15.0),
            max_food: 20.0,
            thirst: None,
            max_thirst: 20.0,
            stamina: Some(50.0),
            max_stamina: 100.0,
            breath: 300,
            max_breath: 300,
            can_sprint: true,
            ..VitalsView::default()
        }
    }

    #[test]
    fn bars_follow_the_gauges_and_hide_what_is_off() {
        let mut bars = Bars {
            vitals: Some(vitals()),
            ..Bars::default()
        };
        assert_eq!(bars.gauge("health"), Some(0.5));
        assert_eq!(bars.gauge("food"), Some(0.75));
        assert_eq!(bars.gauge("thirst"), None, "thirst off");
        assert_eq!(bars.gauge("breath"), None, "full breath");
        assert_eq!(bars.gauge("mana"), None);
        assert_eq!(bars.gauge("temperature"), None);
        bars.empty_mana = true;
        assert_eq!(bars.gauge("mana"), Some(0.0));
        if let Some(view) = &mut bars.vitals {
            view.breath = 150;
        }
        assert_eq!(bars.gauge("breath"), Some(0.5));
        bars.vitals = None;
        assert_eq!(bars.gauge("health"), None, "creative");
        let sample = Bars {
            sample: true,
            ..Bars::default()
        };
        assert!(sample.gauge("experience").is_some());
    }
}
