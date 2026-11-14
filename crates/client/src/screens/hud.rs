use crate::audio::{Direction, LIFETIME, Subtitle};
use crate::gameplay::Gameplay;
use crate::hotbar::{Hotbar, SLOTS};
use crate::ui::{LogLine, ScreenModel, SlotMark, Tone};
use strata_core::content::{Content, ItemId, ItemKind};

/// Data shown by the in-game overlay (sample values in the HUD editor).
pub struct HudModel<'a> {
    pub hotbar: &'a Hotbar,
    /// Translated name of the aimed block, if shown.
    pub target: Option<String>,
    /// How to get off the boat, while riding.
    pub mount: Option<String>,
    /// Sounds heard lately, oldest first.
    pub subtitles: Vec<LogLine>,
}

impl ScreenModel for HudModel<'_> {
    fn text(&self, id: &str) -> Option<String> {
        match id {
            "target" => self.target.clone(),
            "mount" => self.mount.clone(),
            _ => None,
        }
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
            "hotbar" => self.hotbar.marks(SLOTS),
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
