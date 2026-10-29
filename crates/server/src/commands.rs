//! Console commands that touch the world. The server alone runs them; answers are
//! translation keys so the client shows them in the player's language.

use crate::clock::DAY_TICKS;
use crate::game::Game;
use crate::save::WorldMeta;
use strata_core::content::Content;
use strata_core::id::Identifier;
use strata_protocol::commands::{Feedback, TIME_PRESETS, find, words};
use strata_protocol::{GameMode, HotbarState};

pub struct Console<'a> {
    pub game: &'a mut Game,
    pub meta: &'a mut WorldMeta,
    pub content: &'a Content,
}

/// What a command did: the answer, and the new hotbar if it changed.
pub struct Ran {
    pub feedback: Feedback,
    pub hotbar: Option<HotbarState>,
}

impl From<Feedback> for Ran {
    fn from(feedback: Feedback) -> Self {
        Self {
            feedback,
            hotbar: None,
        }
    }
}

impl Console<'_> {
    pub fn run(&mut self, line: &str) -> Ran {
        let words = words(line);
        let Some((&name, args)) = words.split_first() else {
            return Feedback::error("command-empty", &[]).into();
        };
        let usage = || match find(name) {
            Some(spec) => Feedback::error("command-usage", &[("usage", spec.usage_key())]),
            None => Feedback::error("command-unknown", &[("name", name.to_owned())]),
        };
        // Survival worlds will get a "commands allowed" option.
        if find(name).is_some_and(|spec| spec.cheat) && !self.game.is_creative() {
            return Feedback::error("command-not-allowed", &[]).into();
        }
        let ran = match name {
            "tp" => self.teleport(args),
            "give" => return self.give(args).unwrap_or_else(|| usage().into()),
            "gamemode" => self.gamemode(args),
            "time" => self.time(args),
            "seed" => Some(self.seed()),
            _ => None,
        };
        ran.unwrap_or_else(usage).into()
    }

    fn teleport(&mut self, args: &[&str]) -> Option<Feedback> {
        let [x, y, z] = args else {
            return None;
        };
        let feet = self.game.player_body()?.position;
        let target = [
            coordinate(x, feet.x)?,
            coordinate(y, feet.y)?,
            coordinate(z, feet.z)?,
        ];
        if !self.game.teleport(target) {
            return Some(Feedback::error("command-not-allowed", &[]));
        }
        let at = self.game.player_body()?.position;
        let args = [
            ("x", format!("{:.1}", at.x)),
            ("y", format!("{:.1}", at.y)),
            ("z", format!("{:.1}", at.z)),
        ];
        Some(Feedback::ok("command-tp-done", &args))
    }

    fn give(&mut self, args: &[&str]) -> Option<Ran> {
        let [name] = args else {
            return None;
        };
        let id = identifier(name)?;
        let Some(item) = self.content.items.key(&id) else {
            let unknown = Feedback::error("command-give-unknown", &[("item", id.to_string())]);
            return Some(unknown.into());
        };
        let mut hotbar = self.game.hotbar();
        let slot = usize::from(hotbar.selected);
        if hotbar.slots.len() <= slot {
            hotbar.slots.resize(slot + 1, None);
        }
        hotbar.slots[slot] = Some(item);
        if !self.game.set_hotbar(hotbar) {
            return Some(Feedback::error("command-not-allowed", &[]).into());
        }
        Some(Ran {
            feedback: Feedback::ok("command-give-done", &[("item", id.to_string())]),
            hotbar: Some(self.game.hotbar()),
        })
    }

    /// Survival arrives in v0.3: only creative can be chosen for now.
    fn gamemode(&mut self, args: &[&str]) -> Option<Feedback> {
        match args {
            ["creative"] => {
                self.meta.mode = GameMode::Creative;
                self.game.set_mode(GameMode::Creative);
                Some(Feedback::ok("command-gamemode-done", &[]))
            }
            ["survival"] => Some(Feedback::error("command-gamemode-later", &[])),
            _ => None,
        }
    }

    fn time(&mut self, args: &[&str]) -> Option<Feedback> {
        let time = &mut self.meta.day_time;
        match args {
            ["query"] => {}
            ["set", value] => {
                *time = preset_time(value).or_else(|| value.parse().ok())? % DAY_TICKS
            }
            ["add", value] => {
                *time = (*time % DAY_TICKS + value.parse::<u64>().ok()? % DAY_TICKS) % DAY_TICKS
            }
            _ => return None,
        }
        Some(Feedback::ok(
            "command-time-now",
            &[("ticks", time.to_string())],
        ))
    }

    fn seed(&self) -> Feedback {
        let shown = if self.meta.seed_text.is_empty() {
            (self.meta.seed as i64).to_string()
        } else {
            self.meta.seed_text.clone()
        };
        Feedback::ok("command-seed-is", &[("seed", shown)])
    }
}

/// `12`, `-3.5`, `~` (where the player is) or `~4` (four blocks from there).
fn coordinate(text: &str, current: f64) -> Option<f64> {
    let value = match text.strip_prefix('~') {
        Some("") => current,
        Some(offset) => current + offset.parse::<f64>().ok()?,
        None => text.parse::<f64>().ok()?,
    };
    value.is_finite().then_some(value)
}

/// `stone` is short for `strata:stone`.
fn identifier(text: &str) -> Option<Identifier> {
    if text.contains(':') {
        text.parse().ok()
    } else {
        Identifier::new("strata", text).ok()
    }
}

/// Sunrise, noon, sunset and midnight, a quarter of a day apart.
fn preset_time(name: &str) -> Option<u64> {
    let index = TIME_PRESETS.iter().position(|preset| *preset == name)?;
    Some(index as u64 * DAY_TICKS / 4)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::rules::BlockRules;
    use glam::DVec3;
    use strata_core::coords::DEFAULT_WORLD_BORDER;
    use strata_core::pack::PackSet;
    use strata_core::physics::Body;

    fn setup() -> (Game, WorldMeta, Content) {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let packs = PackSet::discover(&dir, &mut Vec::new());
        let (content, _) = Content::load(&packs, &PackSet::default());
        let body = Body::at(DVec3::new(0.5, 80.0, 0.5));
        let rules = BlockRules::new(&content);
        let game = Game::new(rules, DEFAULT_WORLD_BORDER, body, GameMode::Creative);
        let meta: WorldMeta = ron::from_str(
            r#"World(format: 1, game_version: "0.1.0", name: "t", seed: 7, seed_text: "",
                mode: Creative, border: 1000, created: 0, last_played: 0, play_ticks: 0)"#,
        )
        .unwrap();
        (game, meta, content)
    }

    #[test]
    fn commands_change_the_world_and_answer() {
        let (mut game, mut meta, content) = setup();
        let mut console = Console {
            game: &mut game,
            meta: &mut meta,
            content: &content,
        };
        assert_eq!(console.run("/tp ~ ~10 5").feedback.key, "command-tp-done");
        let given = console.run("give stone");
        assert!(given.hotbar.is_some() && !given.feedback.error);
        assert!(console.run("give strata:unobtainium").feedback.error);
        assert_eq!(
            console.run("time set noon").feedback.args[0].1,
            (DAY_TICKS / 4).to_string()
        );
        assert_eq!(console.run("seed").feedback.args[0].1, "7");
        assert_eq!(console.run("tp 1 2").feedback.key, "command-usage");
        assert_eq!(console.run("dance").feedback.key, "command-unknown");
        let feet = game.player_body().unwrap().position;
        assert_eq!((feet.y, feet.z), (90.0, 5.0));
        assert_eq!(meta.day_time, DAY_TICKS / 4);
    }

    #[test]
    fn coordinates_may_be_relative() {
        assert_eq!(coordinate("~", 10.0), Some(10.0));
        assert_eq!(coordinate("~-2.5", 10.0), Some(7.5));
        assert_eq!(coordinate("64", 10.0), Some(64.0));
        assert_eq!(coordinate("~x", 10.0), None);
        assert_eq!(coordinate("NaN", 10.0), None);
    }

    #[test]
    fn names_and_times_are_understood() {
        assert_eq!(
            identifier("stone").map(|i| i.to_string()).as_deref(),
            Some("strata:stone")
        );
        assert_eq!(preset_time("noon"), Some(DAY_TICKS / 4));
        assert_eq!(preset_time("teatime"), None);
    }
}
