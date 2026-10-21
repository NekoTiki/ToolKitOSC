//! Where the panel is, whether you're looking at it, and where a fingertip or laser meets it.
//! Panel space: x to the right of the image, y up, z out of its face toward whoever reads it,
//! in meters from its center.
use super::openvr::Matrix34;
use crate::overlay::model::Corner;

pub type Vec3 = [f32; 3];

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: Vec3, b: Vec3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn length(a: Vec3) -> f32 {
    dot(a, a).sqrt()
}

pub fn position(m: &Matrix34) -> Vec3 {
    [m.m[0][3], m.m[1][3], m.m[2][3]]
}

pub fn column(m: &Matrix34, c: usize) -> Vec3 {
    [m.m[0][c], m.m[1][c], m.m[2][c]]
}

#[cfg(test)]
fn point(m: &Matrix34, p: Vec3) -> Vec3 {
    let mut out = position(m);
    for (r, value) in out.iter_mut().enumerate() {
        *value += m.m[r][0] * p[0] + m.m[r][1] * p[1] + m.m[r][2] * p[2];
    }
    out
}

pub fn direction(m: &Matrix34, d: Vec3) -> Vec3 {
    let mut out = [0.0; 3];
    for (r, value) in out.iter_mut().enumerate() {
        *value = m.m[r][0] * d[0] + m.m[r][1] * d[1] + m.m[r][2] * d[2];
    }
    out
}

pub fn multiply(a: &Matrix34, b: &Matrix34) -> Matrix34 {
    let mut out = Matrix34::default();
    for r in 0..3 {
        for c in 0..4 {
            out.m[r][c] = (0..3).map(|k| a.m[r][k] * b.m[k][c]).sum::<f32>() + if c == 3 { a.m[r][3] } else { 0.0 };
        }
    }
    out
}

/// A world point in the frame of rigid transform `m`.
pub fn into(m: &Matrix34, p: Vec3) -> Vec3 {
    let d = sub(p, position(m));
    [dot(column(m, 0), d), dot(column(m, 1), d), dot(column(m, 2), d)]
}

/// The panel's place on a controller: on the back of the wrist, facing out of the back of the
/// hand. Controller space has x to the right, y up and -z where it points; on a left controller
/// the back of the hand is -x, and turned to look at it the fingers point right and the thumb
/// down.
pub fn wrist_offset(left: bool) -> Matrix34 {
    // shortcut: one placement for every controller, measured on Index controllers; per-model
    // offsets if it sits wrong on others.
    let s = if left { -1.0 } else { 1.0 };
    Matrix34 {
        m: [
            // x of the image, y of the image, its normal, where it sits
            [0.0, 0.0, s, s * 0.03],
            [0.0, -1.0, 0.0, -0.02],
            [s, 0.0, 0.0, 0.11]
        ]
    }
}

/// `m` moved by `x`, `y`, `z` along its own axes.
pub fn offset(m: &Matrix34, x: f32, y: f32, z: f32) -> Matrix34 {
    multiply(m, &Matrix34 { m: [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]] })
}

pub fn distance(a: Vec3, b: Vec3) -> f32 {
    length(sub(a, b))
}

/// `share` of the way from `a` to `b`.
pub fn lerp(a: Vec3, b: Vec3, share: f32) -> Vec3 {
    [a[0] + (b[0] - a[0]) * share, a[1] + (b[1] - a[1]) * share, a[2] + (b[2] - a[2]) * share]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn normalize(a: Vec3) -> Vec3 {
    let l = length(a).max(1e-6);
    [a[0] / l, a[1] / l, a[2] / l]
}

/// An overlay at `at` with its face toward `eye`, kept upright along `up`.
pub fn facing(at: Vec3, eye: Vec3, up: Vec3) -> Matrix34 {
    let z = normalize(sub(eye, at));
    let x = normalize(cross(up, z));
    let y = cross(z, x);
    Matrix34 { m: [[x[0], y[0], z[0], at[0]], [x[1], y[1], z[1], at[1]], [x[2], y[2], z[2], at[2]]] }
}

/// The angle in degrees between the ways from `eye` to `a` and to `b`.
pub fn angle_between(eye: Vec3, a: Vec3, b: Vec3) -> f32 {
    let (a, b) = (sub(a, eye), sub(b, eye));
    let cos = dot(a, b) / (length(a) * length(b)).max(1e-6);
    cos.clamp(-1.0, 1.0).acos().to_degrees()
}

/// Where the HUD sits in your view, in head space (x right, y up, -z ahead): `distance` meters
/// away toward `corner`, facing you. `half_width` and `half_height` are half its size, in
/// degrees, so its outer edges stay at the same place in your view whatever its size.
pub fn hud_offset(corner: Corner, distance: f32, half_width: f32, half_height: f32) -> Matrix34 {
    // shortcut: fixed edges that suit a ~100° field of view; a setting if headsets differ much.
    let (right, top) = match corner {
        Corner::BottomLeft => (false, false),
        Corner::BottomRight => (true, false),
        Corner::TopLeft => (false, true),
        Corner::TopRight => (true, true)
    };
    let yaw = (HUD_EDGE_SIDE - half_width).max(0.0).to_radians() * if right { 1.0 } else { -1.0 };
    let pitch = if top { (HUD_EDGE_TOP - half_height).max(0.0) } else { -(HUD_EDGE_BOTTOM - half_height).max(0.0) }.to_radians();
    let at = [distance * yaw.sin() * pitch.cos(), distance * pitch.sin(), -distance * yaw.cos() * pitch.cos()];
    facing(at, [0.0; 3], [0.0, 1.0, 0.0])
}

/// How far out the HUD's outer edges reach in your view, in degrees from straight ahead.
const HUD_EDGE_SIDE: f32 = 32.0;
const HUD_EDGE_TOP: f32 = 24.0;
const HUD_EDGE_BOTTOM: f32 = 28.0;

/// The angle in degrees between the panel's face and the way to your eyes.
pub fn look_angle(panel: &Matrix34, eyes: Vec3) -> f32 {
    let to_eyes = sub(eyes, position(panel));
    let normal = column(panel, 2);
    let cos = dot(normal, to_eyes) / (length(to_eyes) * length(normal)).max(1e-6);
    cos.clamp(-1.0, 1.0).acos().to_degrees()
}

/// Where a ray from `origin` along `dir` meets the panel's face, as panel-space x and y, and how
/// far it went. None when it points away or comes from behind.
pub fn ray_hit(panel: &Matrix34, origin: Vec3, dir: Vec3) -> Option<([f32; 2], f32)> {
    let local_origin = into(panel, origin);
    let local_dir = [dot(column(panel, 0), dir), dot(column(panel, 1), dir), dot(column(panel, 2), dir)];
    if local_origin[2] <= 0.0 || local_dir[2] >= -1e-6 {
        return None;
    }
    let t = -local_origin[2] / local_dir[2];
    Some(([local_origin[0] + t * local_dir[0], local_origin[1] + t * local_dir[1]], t * length(dir)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: Matrix34 = Matrix34 { m: [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0]] };

    #[test]
    fn the_wrist_offset_is_a_rotation() {
        for left in [true, false] {
            let m = wrist_offset(left);
            let (x, y, z) = (column(&m, 0), column(&m, 1), column(&m, 2));
            // Right-handed: x × y = z.
            let cross = [x[1] * y[2] - x[2] * y[1], x[2] * y[0] - x[0] * y[2], x[0] * y[1] - x[1] * y[0]];
            assert_eq!(cross, z);
        }
    }

    #[test]
    fn into_undoes_point() {
        let panel = multiply(&Matrix34 { m: [[0.0, -1.0, 0.0, 1.0], [1.0, 0.0, 0.0, 2.0], [0.0, 0.0, 1.0, 3.0]] }, &wrist_offset(true));
        let p = [0.1, -0.2, 0.3];
        let back = into(&panel, point(&panel, p));
        assert!(back.iter().zip(p).all(|(a, b)| (a - b).abs() < 1e-5), "{back:?}");
    }

    #[test]
    fn looking_straight_at_it_is_zero_degrees() {
        assert!(look_angle(&IDENTITY, [0.0, 0.0, 0.5]) < 0.01);
        assert!((look_angle(&IDENTITY, [0.5, 0.0, 0.5]) - 45.0).abs() < 0.01);
        assert!(look_angle(&IDENTITY, [0.0, 0.0, -0.5]) > 179.0);
    }

    #[test]
    fn the_hud_faces_you_from_its_corner() {
        for corner in [Corner::BottomLeft, Corner::BottomRight, Corner::TopLeft, Corner::TopRight] {
            let m = hud_offset(corner, 1.5, 10.0, 6.0);
            let at = position(&m);
            assert!((length(at) - 1.5).abs() < 1e-4 && at[2] < 0.0, "{at:?}");
            assert_eq!(at[0] > 0.0, matches!(corner, Corner::BottomRight | Corner::TopRight));
            assert_eq!(at[1] > 0.0, matches!(corner, Corner::TopLeft | Corner::TopRight));
            assert!(look_angle(&m, [0.0; 3]) < 0.01);
            // Upright: its x stays level.
            assert!(column(&m, 0)[1].abs() < 1e-5);
        }
        assert!((angle_between([0.0; 3], [0.0, 0.0, -1.0], [1.0, 0.0, -1.0]) - 45.0).abs() < 0.01);
    }

    #[test]
    fn rays_hit_the_front_only() {
        let (hit, distance) = ray_hit(&IDENTITY, [0.02, 0.01, 0.4], [0.0, 0.0, -1.0]).unwrap();
        assert!((hit[0] - 0.02).abs() < 1e-6 && (hit[1] - 0.01).abs() < 1e-6 && (distance - 0.4).abs() < 1e-6);
        assert!(ray_hit(&IDENTITY, [0.0, 0.0, 0.4], [0.0, 0.0, 1.0]).is_none());
        assert!(ray_hit(&IDENTITY, [0.0, 0.0, -0.4], [0.0, 0.0, 1.0]).is_none());
    }
}
