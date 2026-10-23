//! Inventory-style block icons: the top, south and east faces of a cube in isometric
//! view, drawn on the CPU (also used by the wiki).

use super::{Rgba, RgbaImage};

/// Shade of each visible face, lit from above like the world.
const TOP_SHADE: f32 = 1.0;
const LEFT_SHADE: f32 = 0.8;
const RIGHT_SHADE: f32 = 0.6;

/// `size` × `size` icon; faces may be any (square) texture size.
pub fn block_icon(top: &RgbaImage, south: &RgbaImage, east: &RgbaImage, size: u32) -> RgbaImage {
    let n = size as f32;
    let (top_v, left, right) = ((n / 2.0, 0.0), (0.0, n / 4.0), (n, n / 4.0));
    let (center, bottom) = ((n / 2.0, n / 2.0), (n / 2.0, n));
    let bottom_left = (0.0, n * 0.75);
    let faces = [
        (top, top_v, right, left, TOP_SHADE),
        (south, left, center, bottom_left, LEFT_SHADE),
        (east, center, right, bottom, RIGHT_SHADE),
    ];
    let mut icon = RgbaImage::new(size, size, Rgba([0; 4]));
    for y in 0..size {
        for x in 0..size {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            for (texture, origin, along_u, along_v, shade) in faces {
                if let Some((u, v)) = locate(p, origin, along_u, along_v) {
                    let color = sample(texture, u, v);
                    if color.0[3] > 0 {
                        icon.set(x, y, shaded(color, shade));
                    }
                }
            }
        }
    }
    icon
}

/// Solves `p = origin + u·(a − origin) + v·(b − origin)`; `None` outside the face.
fn locate(p: (f32, f32), origin: (f32, f32), a: (f32, f32), b: (f32, f32)) -> Option<(f32, f32)> {
    let (ax, ay) = (a.0 - origin.0, a.1 - origin.1);
    let (bx, by) = (b.0 - origin.0, b.1 - origin.1);
    let det = ax * by - ay * bx;
    if det.abs() < f32::EPSILON {
        return None;
    }
    let (px, py) = (p.0 - origin.0, p.1 - origin.1);
    let u = (px * by - py * bx) / det;
    let v = (ax * py - ay * px) / det;
    ((0.0..1.0).contains(&u) && (0.0..1.0).contains(&v)).then_some((u, v))
}

fn sample(texture: &RgbaImage, u: f32, v: f32) -> Rgba {
    let x = ((u * texture.width() as f32) as u32).min(texture.width() - 1);
    let y = ((v * texture.height() as f32) as u32).min(texture.height() - 1);
    texture.get(x, y)
}

fn shaded(color: Rgba, shade: f32) -> Rgba {
    let [r, g, b, a] = color.0;
    let s = |c: u8| (f32::from(c) * shade) as u8;
    Rgba([s(r), s(g), s(b), a])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_land_where_expected() {
        let white = RgbaImage::new(16, 16, Rgba([200, 200, 200, 255]));
        let icon = block_icon(&white, &white, &white, 32);
        assert_eq!(icon.get(16, 4), Rgba([200, 200, 200, 255]), "top face");
        assert_eq!(icon.get(6, 20), Rgba([160, 160, 160, 255]), "left face");
        assert_eq!(icon.get(26, 20), Rgba([120, 120, 120, 255]), "right face");
        assert_eq!(icon.get(1, 1).0[3], 0, "corners stay transparent");
    }
}
