//! The temperature on the HUD: where the marker sits on the scale, the line beside it
//! (zone and degrees, away from comfort or while wet), the hint that says what to do when
//! it gets worse, and the frost or heat at the edges of the screen.

use crate::gameplay::Gameplay;
use crate::ui::{Painter, Rect, Theme};
use strata_core::content::Content;
use strata_core::survival::temperature::TemperatureZone;
use strata_protocol::TemperatureView;

/// Bands drawn from each edge inward, each fainter.
const EDGE_BANDS: u32 = 6;

/// Where the marker sits on the HUD scale, 0 (freezing) to 1 (scorching).
pub fn temperature_gauge(game: &Gameplay, content: &Content) -> Option<f32> {
    let view = player_temperature(game)?;
    Some(content.survival.temperature.gauge(view.felt))
}

/// « Froid 3 °C · mouillé »: shown away from comfort or while wet.
pub fn temperature_line(game: &Gameplay, content: &Content, language: &str) -> Option<String> {
    let view = player_temperature(game)?;
    temperature_text(&view, content, language)
}

pub fn temperature_text(
    view: &TemperatureView,
    content: &Content,
    language: &str,
) -> Option<String> {
    let tr = &content.translations;
    let mut parts = Vec::new();
    if view.zone != TemperatureZone::Comfortable {
        let zone = tr.text(language, &format!("temperature-zone-{}", view.zone.key()));
        let degrees = format!("{}", view.felt.round());
        let args = [("zone", zone), ("degrees", degrees)];
        parts.push(tr.text_with(language, "hud-temperature", &args));
    }
    if view.wet {
        parts.push(tr.text(language, "hud-temperature-wet"));
    }
    let separator = tr.text(language, "hud-effect-separator");
    (!parts.is_empty()).then(|| parts.join(&separator))
}

/// What the zone does and how to get out of it, for a few seconds after it gets worse.
pub fn temperature_hint(game: &Gameplay, content: &Content, language: &str) -> Option<String> {
    player_temperature(game)?;
    let (zone, _) = game.vitals.hint?;
    let key = format!("hud-temperature-hint-{}", zone.key());
    Some(content.translations.text(language, &key))
}

fn player_temperature(game: &Gameplay) -> Option<TemperatureView> {
    let view = game.vitals.view.as_ref().filter(|_| !game.is_creative())?;
    view.temperature
}

/// Frost (`edges` below 0) or heat (above 0) creeping in from the edges of the screen.
pub fn paint_edges(painter: &mut Painter<'_>, screen: Rect, theme: &Theme, edges: f32) {
    let strength = edges.abs().min(1.0);
    if strength <= 0.01 {
        return;
    }
    let base = if edges < 0.0 {
        theme.frost.0
    } else {
        theme.heat.0
    };
    let depth = (screen.w.min(screen.h) / 8).max(EDGE_BANDS);
    let band = depth / EDGE_BANDS;
    for index in 0..EDGE_BANDS {
        let fade = 1.0 - index as f32 / EDGE_BANDS as f32;
        let mut color = base;
        color.0[3] = (f32::from(base.0[3]) * strength * fade / 2.0).round() as u8;
        let inset = (index * band) as i32;
        let (w, h) = (
            screen.w.saturating_sub(2 * index * band),
            screen.h.saturating_sub(2 * index * band),
        );
        let (x, y) = (screen.x + inset, screen.y + inset);
        painter.fill(Rect::new(x, y, w, band), color);
        painter.fill(Rect::new(x, y + h as i32 - band as i32, w, band), color);
        let side = h.saturating_sub(2 * band);
        painter.fill(Rect::new(x, y + band as i32, band, side), color);
        painter.fill(
            Rect::new(x + w as i32 - band as i32, y + band as i32, band, side),
            color,
        );
    }
}
