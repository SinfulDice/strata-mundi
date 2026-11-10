//! Console commands that touch the world. The server alone runs them; answers are
//! translation keys so the client shows them in the player's language.

use crate::game::Game;
use crate::save::WorldMeta;
use strata_core::content::Content;
use strata_core::id::Identifier;
use strata_core::sky::{DayCycle, Moment};
use strata_protocol::commands::{Feedback, TIME_PRESETS, find, words};
use strata_protocol::{GameMode, HotbarState};

/// More liquid updates than this per tick would not hold 20 ticks per second.
const MAX_FLUID_BUDGET: u32 = 65_536;

pub struct Console<'a> {
    pub game: &'a mut Game,
    pub meta: &'a mut WorldMeta,
    pub content: &'a Content,
    pub cycle: DayCycle,
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
            "fluids" => self.fluids(args),
            "daycycle" => self.day_cycle(args),
            "fire" => self.fire(args),
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

    /// `set` moves forward to the next moment asked for: time never goes back, so the
    /// moon keeps changing once a night.
    fn time(&mut self, args: &[&str]) -> Option<Feedback> {
        let cycle = self.cycle;
        let time = &mut self.meta.day_time;
        match args {
            [] | ["query"] => {}
            ["set", value] => {
                let target = match preset_time(value) {
                    Some(moment) => cycle.moment(moment),
                    None => value.parse::<u64>().ok()? % cycle.length(),
                };
                *time = next_time_of_day(&cycle, *time, target);
            }
            ["add", value] => *time = time.saturating_add(value.parse().ok()?),
            _ => return None,
        }
        let args = [
            ("ticks", cycle.time_of_day(*time).to_string()),
            ("length", cycle.length().to_string()),
            ("day", (cycle.day(*time) + 1).to_string()),
        ];
        Some(Feedback::ok("command-time-now", &args))
    }

    /// `on`, `off` or `query`: whether the time of day goes on; saved with the world.
    fn day_cycle(&mut self, args: &[&str]) -> Option<Feedback> {
        match args {
            [] | ["query"] => {}
            ["on"] => self.meta.day_cycle = true,
            ["off"] => self.meta.day_cycle = false,
            _ => return None,
        }
        let cycle = if self.meta.day_cycle { "on" } else { "off" };
        Some(Feedback::ok(
            "command-daycycle-now",
            &[("cycle", cycle.to_owned())],
        ))
    }

    /// `on`, `off`, a budget of liquid updates per tick, or `query`; saved with the world.
    fn fluids(&mut self, args: &[&str]) -> Option<Feedback> {
        let fluids = &mut self.meta.fluids;
        match args {
            [] | ["query"] => {}
            ["on"] => fluids.flow = true,
            ["off"] => fluids.flow = false,
            [budget] => {
                let budget = budget.parse::<u32>().ok()?;
                if !(1..=MAX_FLUID_BUDGET).contains(&budget) {
                    let max = MAX_FLUID_BUDGET.to_string();
                    return Some(Feedback::error("command-fluids-budget", &[("max", max)]));
                }
                fluids.updates_per_tick = budget;
            }
            _ => return None,
        }
        self.game.set_fluid_settings(*fluids);
        let flow = if fluids.flow { "on" } else { "off" };
        let args = [
            ("flow", flow.to_owned()),
            ("budget", fluids.updates_per_tick.to_string()),
        ];
        Some(Feedback::ok("command-fluids-now", &args))
    }

    /// `spread on|off`, `destroy on|off` or `query`; saved with the world.
    fn fire(&mut self, args: &[&str]) -> Option<Feedback> {
        let fire = &mut self.meta.fire;
        match args {
            [] | ["query"] => {}
            ["spread", value] => fire.spread = on_off(value)?,
            ["destroy", value] => fire.destroy = on_off(value)?,
            _ => return None,
        }
        self.game.set_fire_settings(*fire);
        let word = |on: bool| if on { "on" } else { "off" }.to_owned();
        let args = [
            ("spread", word(fire.spread)),
            ("destroy", word(fire.destroy)),
        ];
        Some(Feedback::ok("command-fire-now", &args))
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

fn on_off(text: &str) -> Option<bool> {
    match text {
        "on" => Some(true),
        "off" => Some(false),
        _ => None,
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

fn preset_time(name: &str) -> Option<Moment> {
    let index = TIME_PRESETS.iter().position(|preset| *preset == name)?;
    [Moment::Day, Moment::Noon, Moment::Night, Moment::Midnight]
        .get(index)
        .copied()
}

/// The first time from `now` on whose time of day is `target`.
fn next_time_of_day(cycle: &DayCycle, now: u64, target: u64) -> u64 {
    let start = now - cycle.time_of_day(now);
    let today = start.saturating_add(target);
    if today >= now {
        today
    } else {
        today.saturating_add(cycle.length())
    }
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
