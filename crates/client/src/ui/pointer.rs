//! Where the pointer lands inside a slider or an options row: shared by the drawing and
//! the input handling so both always agree.

use super::model::OptionRow;
use super::rect::Rect;

/// Width of a slider's knob, in interface pixels.
pub const SLIDER_KNOB: u32 = 6;
/// Width of the click area of each ‹ › arrow of a choice row.
pub const ARROW_WIDTH: u32 = 9;

/// Value (0 to 1) under `x` on a track whose knob is `knob` pixels wide: the knob's
/// centre follows the pointer, so both ends are reachable and clicks past them clamp.
pub fn track_fraction(track: Rect, knob: u32, x: i32) -> f32 {
    let knob = knob.max(1);
    let travel = track.w.saturating_sub(knob).max(1) as f32;
    let offset = (x - track.x - (knob / 2) as i32) as f32;
    (offset / travel).clamp(0.0, 1.0)
}

/// The right-hand part of an options row: its gauge, or its value between ‹ ›.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowZones {
    /// Clickable: from the gauge's start to the end of the row (margins included).
    pub value: Rect,
    /// Drawn bar, and the track a click or drag on `value` is measured on.
    pub gauge: Rect,
    pub previous: Rect,
    pub next: Rect,
}

pub fn row_zones(line: Rect) -> RowZones {
    let width = (line.w / 3).max(10);
    let x = line.x + line.w as i32 - width as i32 - 2;
    let clickable = width + 2;
    let arrow = ARROW_WIDTH.min(width / 2);
    RowZones {
        value: Rect::new(x, line.y, clickable, line.h),
        gauge: Rect::new(x, line.y + 1, width, line.h.saturating_sub(2)),
        previous: Rect::new(x, line.y, arrow, line.h),
        next: Rect::new(x + (clickable - arrow) as i32, line.y, arrow, line.h),
    }
}

/// What a click at `x` on an options row does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RowPart {
    /// On the gauge: this value, and a drag follows.
    Gauge(f32),
    /// On an arrow: one step back or forward.
    Step(i32),
    /// Anywhere else: the row's usual action (toggle, open, next value).
    Body,
}

pub fn row_part(line: Rect, row: &OptionRow, (x, y): (i32, i32)) -> RowPart {
    let zones = row_zones(line);
    if row.gauge.is_some() && zones.value.contains(x, y) {
        return RowPart::Gauge(track_fraction(zones.gauge, 1, x));
    }
    if row.arrows && zones.previous.contains(x, y) {
        return RowPart::Step(-1);
    }
    if row.arrows && zones.next.contains(x, y) {
        return RowPart::Step(1);
    }
    RowPart::Body
}

/// Whether the wheel over `(x, y)` changes the row's value (else it scrolls the list).
pub fn wheel_adjusts(line: Rect, row: &OptionRow, (x, y): (i32, i32)) -> bool {
    (row.gauge.is_some() || row.arrows) && row_zones(line).value.contains(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(gauge: Option<f32>, arrows: bool) -> OptionRow {
        OptionRow {
            label: "l".into(),
            value: "v".into(),
            gauge,
            arrows,
            highlight: false,
        }
    }

    #[test]
    fn slider_ends_are_reached_and_clicks_past_them_clamp() {
        let track = Rect::new(10, 0, 100, 14);
        let half = (SLIDER_KNOB / 2) as i32;
        assert_eq!(track_fraction(track, SLIDER_KNOB, 10), 0.0);
        assert_eq!(track_fraction(track, SLIDER_KNOB, 10 + half), 0.0);
        assert_eq!(track_fraction(track, SLIDER_KNOB, 109), 1.0);
        assert_eq!(track_fraction(track, SLIDER_KNOB, 10 + half + 94), 1.0);
        assert_eq!(track_fraction(track, SLIDER_KNOB, 500), 1.0);
        assert_eq!(track_fraction(track, SLIDER_KNOB, -50), 0.0);
        assert!((track_fraction(track, SLIDER_KNOB, 10 + half + 47) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn gauge_first_and_last_pixels_are_the_bounds() {
        let track = Rect::new(0, 0, 101, 10);
        assert_eq!(track_fraction(track, 1, 0), 0.0);
        assert_eq!(track_fraction(track, 1, 100), 1.0);
        assert!((track_fraction(track, 1, 25) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn rows_split_into_gauge_arrows_and_body() {
        let line = Rect::new(0, 0, 300, 12);
        let zones = row_zones(line);
        let y = 5;
        let gauge = row(Some(0.5), false);
        assert_eq!(
            row_part(line, &gauge, (zones.gauge.x, y)),
            RowPart::Gauge(0.0)
        );
        let last = zones.gauge.x + zones.gauge.w as i32 - 1;
        assert_eq!(row_part(line, &gauge, (last, y)), RowPart::Gauge(1.0));
        let row_end = line.x + line.w as i32 - 1;
        assert_eq!(row_part(line, &gauge, (row_end, 0)), RowPart::Gauge(1.0));
        assert_eq!(row_part(line, &gauge, (4, y)), RowPart::Body);
        let choice = row(None, true);
        assert_eq!(
            row_part(line, &choice, (zones.value.x + 1, y)),
            RowPart::Step(-1)
        );
        assert_eq!(row_part(line, &choice, (last, y)), RowPart::Step(1));
        let middle = zones.value.center().0;
        assert_eq!(row_part(line, &choice, (middle, y)), RowPart::Body);
        assert_eq!(row_part(line, &row(None, false), (last, y)), RowPart::Body);
    }

    #[test]
    fn wheel_changes_values_only_over_them() {
        let line = Rect::new(0, 0, 300, 12);
        let middle = row_zones(line).value.center();
        assert!(wheel_adjusts(line, &row(None, true), middle));
        assert!(wheel_adjusts(line, &row(Some(0.2), false), middle));
        assert!(!wheel_adjusts(line, &row(None, false), middle));
        assert!(!wheel_adjusts(line, &row(None, true), (4, 5)));
    }
}
