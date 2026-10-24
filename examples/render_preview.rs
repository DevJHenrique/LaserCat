// SPDX-License-Identifier: AGPL-3.0-only
//! CPU rendering of the actual egui application for layout QA without a desktop/GPU.
use eframe::egui::{self, Color32, ColorImage, ImageData, Pos2, Rect, TextureId, Vec2};
use std::collections::HashMap;

fn main() {
    let ctx = egui::Context::default();
    let mut textures: HashMap<TextureId, ColorImage> = HashMap::new();
    let args: Vec<_> = std::env::args().collect();
    // Optional fifth argument: interface language code, e.g. "pt-BR".
    if let Some(language) = args.get(5).and_then(|code| {
        lasercat::i18n::Language::ALL
            .into_iter()
            .find(|l| l.code() == code)
    }) {
        lasercat::i18n::set_language(language);
    }
    let mut app = lasercat::ui::LaserApp::demo_with_view(
        &ctx,
        args.get(4).map(String::as_str).unwrap_or("prepare"),
    );
    let width = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1440u32);
    let height = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(900u32);
    let mut output = None;
    for frame in 0..15 {
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(width as f32, height as f32),
            )),
            time: Some(frame as f64 / 60.),
            ..Default::default()
        };
        let result = ctx.run(raw, |ctx| app.draw(ctx));
        for (id, delta) in &result.textures_delta.set {
            let image = match &delta.image {
                ImageData::Color(img) => img.as_ref().clone(),
                ImageData::Font(img) => ColorImage {
                    size: img.size,
                    pixels: img.srgba_pixels(None).collect(),
                },
            };
            if let Some([x, y]) = delta.pos {
                let target = textures.get_mut(id).unwrap();
                for row in 0..image.size[1] {
                    let start = (y + row) * target.size[0] + x;
                    target.pixels[start..start + image.size[0]].copy_from_slice(
                        &image.pixels[row * image.size[0]..(row + 1) * image.size[0]],
                    );
                }
            } else {
                textures.insert(*id, image);
            }
        }
        output = Some(result);
    }
    let output = output.unwrap();
    let primitives = ctx.tessellate(output.shapes, 1.);
    let mut img = image::RgbaImage::from_pixel(width, height, image::Rgba([16, 22, 31, 255]));
    for primitive in primitives {
        let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive else {
            continue;
        };
        let Some(texture) = textures.get(&mesh.texture_id) else {
            continue;
        };
        for triangle in mesh.indices.chunks_exact(3) {
            let [a, b, c] = [
                mesh.vertices[triangle[0] as usize],
                mesh.vertices[triangle[1] as usize],
                mesh.vertices[triangle[2] as usize],
            ];
            let cross =
                |p: Pos2, q: Pos2, r: Pos2| (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
            let area = cross(a.pos, b.pos, c.pos);
            if area.abs() < 1e-8 {
                continue;
            }
            let minx = a
                .pos
                .x
                .min(b.pos.x)
                .min(c.pos.x)
                .max(primitive.clip_rect.min.x)
                .max(0.)
                .floor() as u32;
            let maxx = a
                .pos
                .x
                .max(b.pos.x)
                .max(c.pos.x)
                .min(primitive.clip_rect.max.x)
                .min(width as f32)
                .ceil() as u32;
            let miny = a
                .pos
                .y
                .min(b.pos.y)
                .min(c.pos.y)
                .max(primitive.clip_rect.min.y)
                .max(0.)
                .floor() as u32;
            let maxy = a
                .pos
                .y
                .max(b.pos.y)
                .max(c.pos.y)
                .min(primitive.clip_rect.max.y)
                .min(height as f32)
                .ceil() as u32;
            for y in miny..maxy {
                for x in minx..maxx {
                    let p = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
                    if !primitive.clip_rect.contains(p) {
                        continue;
                    }
                    let wa = cross(b.pos, c.pos, p) / area;
                    let wb = cross(c.pos, a.pos, p) / area;
                    let wc = 1. - wa - wb;
                    if wa < -1e-5 || wb < -1e-5 || wc < -1e-5 {
                        continue;
                    }
                    let uv = a.uv.to_vec2() * wa + b.uv.to_vec2() * wb + c.uv.to_vec2() * wc;
                    let tx = ((uv.x * texture.size[0] as f32) as usize).min(texture.size[0] - 1);
                    let ty = ((uv.y * texture.size[1] as f32) as usize).min(texture.size[1] - 1);
                    let tex = texture.pixels[ty * texture.size[0] + tx].to_array();
                    let mut color = [0.; 4];
                    for (i, value) in color.iter_mut().enumerate() {
                        *value = (a.color.to_array()[i] as f32 * wa
                            + b.color.to_array()[i] as f32 * wb
                            + c.color.to_array()[i] as f32 * wc)
                            * tex[i] as f32
                            / 255.;
                    }
                    let dst = img.get_pixel_mut(x, y);
                    for i in 0..3 {
                        dst.0[i] = (color[i] + dst.0[i] as f32 * (1. - color[3] / 255.))
                            .clamp(0., 255.) as u8;
                    }
                }
            }
        }
    }
    let path = args
        .get(1)
        .map(String::as_str)
        .unwrap_or("target/preview.png");
    img.save(path).unwrap();
    println!("Interface preview: {path}");
    let _ = Color32::WHITE;
}
