//! Draws egui's triangles into an RGBA buffer on the CPU, for the wrist panel's `SetOverlayRaw`. The panel is small
//! and only redrawn when something changes, so this needs no GPU: a GPU texture would be copied
//! back to the CPU for `SetOverlayRaw` anyway.
use std::collections::HashMap;

use egui::epaint::textures::TexturesDelta;
use egui::epaint::{ClippedPrimitive, ImageData, Primitive, TextureId, Vertex};

struct Texture {
    size: [usize; 2],
    /// Premultiplied RGBA.
    pixels: Vec<[f32; 4]>
}

impl Texture {
    fn sample(&self, u: f32, v: f32) -> [f32; 4] {
        let [w, h] = self.size;
        let x = (u * w as f32 - 0.5).clamp(0.0, (w - 1) as f32);
        let y = (v * h as f32 - 0.5).clamp(0.0, (h - 1) as f32);
        let (x0, y0) = (x as usize, y as usize);
        let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let at = |x: usize, y: usize| self.pixels[y * w + x];
        let mut out = [0.0; 4];
        for (c, value) in out.iter_mut().enumerate() {
            let top = at(x0, y0)[c] * (1.0 - fx) + at(x1, y0)[c] * fx;
            let bottom = at(x0, y1)[c] * (1.0 - fx) + at(x1, y1)[c] * fx;
            *value = top * (1.0 - fy) + bottom * fy;
        }
        out
    }
}

#[derive(Default)]
pub struct Raster {
    textures: HashMap<TextureId, Texture>
}

fn rgba(color: egui::Color32) -> [f32; 4] {
    color.to_array().map(|c| c as f32 / 255.0)
}

impl Raster {
    pub fn update_textures(&mut self, delta: &TexturesDelta) {
        for (id, change) in &delta.set {
            let (size, pixels): ([usize; 2], Vec<[f32; 4]>) = match &change.image {
                ImageData::Color(image) => (image.size, image.pixels.iter().map(|c| rgba(*c)).collect()),
                ImageData::Font(image) => (image.size, image.srgba_pixels(None).map(rgba).collect())
            };
            match change.pos {
                Some([x, y]) => {
                    let Some(texture) = self.textures.get_mut(id) else {
                        continue;
                    };
                    for row in 0..size[1] {
                        let start = (y + row) * texture.size[0] + x;
                        texture.pixels[start..start + size[0]].copy_from_slice(&pixels[row * size[0]..(row + 1) * size[0]]);
                    }
                }
                None => {
                    self.textures.insert(*id, Texture { size, pixels });
                }
            }
        }
        for id in &delta.free {
            self.textures.remove(id);
        }
    }

    /// Draws everything into `out` (premultiplied RGBA floats, `width` × `height`), cleared first.
    pub fn draw(&self, primitives: &[ClippedPrimitive], pixels_per_point: f32, out: &mut [[f32; 4]], width: usize, height: usize) {
        out.fill([0.0; 4]);
        for primitive in primitives {
            let Primitive::Mesh(mesh) = &primitive.primitive else {
                continue;
            };
            let Some(texture) = self.textures.get(&mesh.texture_id) else {
                continue;
            };
            let clip = primitive.clip_rect;
            let clip = [
                (clip.min.x * pixels_per_point).max(0.0) as usize,
                (clip.min.y * pixels_per_point).max(0.0) as usize,
                ((clip.max.x * pixels_per_point).ceil() as usize).min(width),
                ((clip.max.y * pixels_per_point).ceil() as usize).min(height)
            ];
            for triangle in mesh.indices.chunks_exact(3) {
                let [a, b, c] = [0, 1, 2].map(|i| &mesh.vertices[triangle[i] as usize]);
                triangle_into(texture, [a, b, c], pixels_per_point, clip, out, width);
            }
        }
    }
}

fn triangle_into(texture: &Texture, v: [&Vertex; 3], ppp: f32, clip: [usize; 4], out: &mut [[f32; 4]], width: usize) {
    let p = v.map(|v| [v.pos.x * ppp, v.pos.y * ppp]);
    let area = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[2][0] - p[0][0]) * (p[1][1] - p[0][1]);
    if area.abs() < 1e-6 {
        return;
    }
    let min_x = (p.iter().map(|p| p[0]).fold(f32::MAX, f32::min).floor().max(0.0) as usize).max(clip[0]);
    let min_y = (p.iter().map(|p| p[1]).fold(f32::MAX, f32::min).floor().max(0.0) as usize).max(clip[1]);
    let max_x = (p.iter().map(|p| p[0]).fold(f32::MIN, f32::max).ceil().max(0.0) as usize).min(clip[2]);
    let max_y = (p.iter().map(|p| p[1]).fold(f32::MIN, f32::max).ceil().max(0.0) as usize).min(clip[3]);
    if min_x >= max_x || min_y >= max_y {
        return;
    }
    let colors = v.map(|v| rgba(v.color));
    // Most of the panel is flat shapes, all at the font texture's white texel: one sample does.
    let flat_uv = v[0].uv == v[1].uv && v[1].uv == v[2].uv;
    let flat_texel = flat_uv.then(|| texture.sample(v[0].uv.x, v[0].uv.y));
    let flat_color = (colors[0] == colors[1] && colors[1] == colors[2]).then_some(colors[0]);

    // Each barycentric weight is linear in x and y: start it at the first pixel's center and step.
    let inv = 1.0 / area;
    let edge = |a: [f32; 2], b: [f32; 2]| {
        let (dx, dy) = (-(b[1] - a[1]) * inv, (b[0] - a[0]) * inv);
        let at = |x: f32, y: f32| ((b[0] - a[0]) * (y - a[1]) - (x - a[0]) * (b[1] - a[1])) * inv;
        (at(min_x as f32 + 0.5, min_y as f32 + 0.5), dx, dy)
    };
    let edges = [edge(p[1], p[2]), edge(p[2], p[0]), edge(p[0], p[1])];
    let columns = (max_x - min_x) as f32;
    for y in min_y..max_y {
        let row = (y - min_y) as f32;
        let mut w = edges.map(|(start, _, dy)| start + dy * row);
        // The pixels of this row inside all three edges, give or take one: egui fills rounded
        // shapes as fans of long thin triangles, whose boxes are mostly empty.
        let (mut from, mut to) = (0.0f32, columns);
        for (weight, (_, dx, _)) in w.iter().zip(&edges) {
            if *dx > 0.0 {
                from = from.max(-weight / dx - 1.0);
            } else if *dx < 0.0 {
                to = to.min(weight / -dx + 1.0);
            } else if *weight < 0.0 {
                to = -1.0;
            }
        }
        if to < from {
            continue;
        }
        let (from, to) = (from.max(0.0) as usize, (to.ceil() as usize).min(max_x - min_x));
        let line = &mut out[y * width..(y + 1) * width];
        let py = y as f32 + 0.5;
        for (x, dst) in (min_x + from..).zip(&mut line[min_x + from..min_x + to]) {
            // Exactly, not stepped: a pixel on an edge two triangles share must land in both, as
            // rounding that piles up along a row would drop it from one.
            let px = x as f32 + 0.5;
            let exact = |a: [f32; 2], b: [f32; 2]| ((b[0] - a[0]) * (py - a[1]) - (px - a[0]) * (b[1] - a[1])) / area;
            w = [exact(p[1], p[2]), exact(p[2], p[0]), exact(p[0], p[1])];
            if w[0] >= 0.0 && w[1] >= 0.0 && w[2] >= 0.0 {
                let tex = flat_texel.unwrap_or_else(|| {
                    let u = w[0] * v[0].uv.x + w[1] * v[1].uv.x + w[2] * v[2].uv.x;
                    let t = w[0] * v[0].uv.y + w[1] * v[1].uv.y + w[2] * v[2].uv.y;
                    texture.sample(u, t)
                });
                let color = flat_color.unwrap_or_else(|| {
                    let mut color = [0.0; 4];
                    for (c, value) in color.iter_mut().enumerate() {
                        *value = w[0] * colors[0][c] + w[1] * colors[1][c] + w[2] * colors[2][c];
                    }
                    color
                });
                let alpha = color[3] * tex[3];
                for c in 0..4 {
                    dst[c] = color[c] * tex[c] + dst[c] * (1.0 - alpha);
                }
            }
        }
    }
}

/// Premultiplied floats to the straight 8-bit RGBA SteamVR takes.
pub fn to_rgba8(pixels: &[[f32; 4]], out: &mut Vec<u8>) {
    out.clear();
    for p in pixels {
        let a = p[3].clamp(0.0, 1.0);
        if a <= 0.0 {
            out.extend_from_slice(&[0; 4]);
            continue;
        }
        let inv = 1.0 / a;
        let straight = |c: f32| ((c * inv).clamp(0.0, 1.0) * 255.0) as u8;
        out.extend_from_slice(&[straight(p[0]), straight(p[1]), straight(p[2]), (a * 255.0) as u8]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_a_filled_rect() {
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(20.0, 10.0))), ..Default::default() }, |ctx| {
            ctx.layer_painter(egui::LayerId::background()).rect_filled(egui::Rect::from_min_size(egui::pos2(5.0, 2.0), egui::vec2(10.0, 6.0)), 0.0, egui::Color32::RED);
        });
        let mut raster = Raster::default();
        raster.update_textures(&output.textures_delta);
        let primitives = ctx.tessellate(output.shapes, 1.0);
        let mut pixels = vec![[0.0; 4]; 20 * 10];
        raster.draw(&primitives, 1.0, &mut pixels, 20, 10);
        let mut bytes = Vec::new();
        to_rgba8(&pixels, &mut bytes);
        let at = |x: usize, y: usize| &bytes[(y * 20 + x) * 4..(y * 20 + x) * 4 + 4];
        assert_eq!(at(10, 5), [255, 0, 0, 255]);
        assert_eq!(at(1, 1)[3], 0);
    }
}
