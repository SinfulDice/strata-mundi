use serde::Deserialize;

/// Smooth curve through `(input, output)` points, sorted by input. Flat before the first
/// point and after the last one; eased between points so the terrain never shows a crease.
#[derive(Debug, Clone, Deserialize)]
#[serde(from = "Vec<(f64, f64)>")]
pub struct Spline {
    points: Vec<(f64, f64)>,
}

impl From<Vec<(f64, f64)>> for Spline {
    fn from(mut points: Vec<(f64, f64)>) -> Self {
        points.retain(|(x, y)| x.is_finite() && y.is_finite());
        points.sort_by(|a, b| a.0.total_cmp(&b.0));
        Self { points }
    }
}

impl Spline {
    pub fn constant(value: f64) -> Self {
        Self {
            points: vec![(0.0, value)],
        }
    }

    /// The same curve with its outputs multiplied.
    pub fn scaled(&self, factor: f64) -> Self {
        let points = self.points.iter().map(|&(x, y)| (x, y * factor)).collect();
        Self { points }
    }

    pub fn at(&self, x: f64) -> f64 {
        let Some(&(first_x, first_y)) = self.points.first() else {
            return 0.0;
        };
        // Also catches NaN, which would otherwise index before the first point.
        if x.is_nan() || x <= first_x {
            return first_y;
        }
        let index = self.points.partition_point(|&(px, _)| px <= x);
        match self.points.get(index) {
            None => self.points[index - 1].1,
            Some(&(x1, y1)) => {
                let (x0, y0) = self.points[index - 1];
                y0 + (y1 - y0) * smoothstep(x0, x1, x)
            }
        }
    }
}

pub fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    if edge1 <= edge0 {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_its_points_and_stays_flat_outside() {
        let spline = Spline::from(vec![(1.0, 10.0), (-1.0, 0.0)]);
        assert_eq!(spline.at(-5.0), 0.0);
        assert_eq!(spline.at(-1.0), 0.0);
        assert!((spline.at(0.0) - 5.0).abs() < 1e-9);
        assert_eq!(spline.at(1.0), 10.0);
        assert_eq!(spline.at(9.0), 10.0);
        assert_eq!(spline.at(f64::NAN), 0.0);
    }

    #[test]
    fn empty_spline_is_zero() {
        assert_eq!(Spline::from(Vec::new()).at(0.3), 0.0);
    }
}
