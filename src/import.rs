// SPDX-License-Identifier: AGPL-3.0-only
use crate::domain::*;
use crate::{i18n::tr, trf};
use resvg::{tiny_skia, usvg};
use std::{path::Path, sync::Arc};

pub use crate::limits::MAX_ASSET_BYTES;
pub const MAX_PIXELS: u64 = 40_000_000;

pub fn read_file(path: &Path) -> Result<Asset, String> {
    if path.metadata().map_err(|e| e.to_string())?.len() > MAX_ASSET_BYTES {
        return Err(tr("The file exceeds 64 MB.").into());
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    from_bytes(name, bytes)
}

pub fn from_bytes(name: String, bytes: Vec<u8>) -> Result<Asset, String> {
    if bytes.len() as u64 > MAX_ASSET_BYTES {
        return Err(tr("The file exceeds 64 MB.").into());
    }
    if name.to_lowercase().ends_with(".svg") {
        svg_asset(name, bytes, 0.01)
    } else {
        let img = decode_image(&bytes)?;
        let (dx, dy) = density(&bytes).unwrap_or((96., 96.));
        Ok(Asset {
            id: 0,
            name,
            width: img.width() as f64 * 25.4 / dx,
            height: img.height() as f64 * 25.4 / dy,
            geometry: Geometry::Raster {
                pixel_width: img.width(),
                pixel_height: img.height(),
            },
            warnings: vec![],
            bytes: Arc::new(bytes),
        })
    }
}

pub fn decode_image(bytes: &[u8]) -> Result<image::RgbaImage, String> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(20_000);
    limits.max_image_height = Some(20_000);
    limits.max_alloc = Some(200_000_000);
    reader.limits(limits);
    let img = reader
        .decode()
        .map_err(|e| trf!("Invalid image: {error}", error = e))?;
    if img.width() as u64 * img.height() as u64 > MAX_PIXELS {
        return Err(tr("The image exceeds 40 million pixels.").into());
    }
    Ok(img.to_rgba8())
}

fn density(b: &[u8]) -> Option<(f64, f64)> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        let mut i = 8;
        while i + 12 <= b.len() {
            let n = u32::from_be_bytes(b[i..i + 4].try_into().ok()?) as usize;
            if i.checked_add(12 + n)? > b.len() {
                break;
            }
            if &b[i + 4..i + 8] == b"pHYs" && n == 9 && b[i + 16] == 1 {
                let x = u32::from_be_bytes(b[i + 8..i + 12].try_into().ok()?) as f64 * 0.0254;
                let y = u32::from_be_bytes(b[i + 12..i + 16].try_into().ok()?) as f64 * 0.0254;
                if x > 0. && y > 0. {
                    return Some((x, y));
                }
            }
            i += 12 + n;
        }
    }
    if b.starts_with(&[0xff, 0xd8]) {
        let mut i = 2;
        while i + 4 < b.len() && b[i] == 0xff {
            let marker = b[i + 1];
            if marker == 0xda || marker == 0xd9 {
                break;
            }
            let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
            if len < 2 || i + 2 + len > b.len() {
                break;
            }
            if marker == 0xe0 && len >= 16 && &b[i + 4..i + 9] == b"JFIF\0" {
                let unit = b[i + 11];
                let factor = if unit == 1 {
                    1.
                } else if unit == 2 {
                    2.54
                } else {
                    return None;
                };
                let x = u16::from_be_bytes([b[i + 12], b[i + 13]]) as f64 * factor;
                let y = u16::from_be_bytes([b[i + 14], b[i + 15]]) as f64 * factor;
                if x > 0. && y > 0. {
                    return Some((x, y));
                }
            }
            i += 2 + len;
        }
    }
    None
}

pub fn svg_tree(bytes: &[u8]) -> Result<usvg::Tree, String> {
    let xml = std::str::from_utf8(bytes).map_err(|_| tr("The SVG must be UTF-8.").to_string())?;
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    for n in doc.descendants() {
        if matches!(
            n.tag_name().name(),
            "foreignObject" | "script" | "animate" | "animateTransform" | "animateMotion" | "set"
        ) {
            return Err(trf!("Unsupported SVG element: {element}. Export a static version with text converted to outlines, or use PNG.", element = n.tag_name().name()));
        }
        for attr in n.attributes() {
            if attr.name() == "href"
                && !attr.value().starts_with('#')
                && !attr.value().starts_with("data:")
            {
                return Err(
                    tr("The SVG references an external resource. Embed images and resources in the file.").into(),
                );
            }
        }
    }
    let opt = usvg::Options {
        fontdb: fonts(),
        ..Default::default()
    };
    usvg::Tree::from_data(bytes, &opt).map_err(|e| trf!("Invalid SVG: {error}", error = e))
}

pub fn svg_asset(name: String, bytes: Vec<u8>, tolerance: f64) -> Result<Asset, String> {
    let initial_tree = svg_tree(&bytes)?;
    let normalized = initial_tree
        .to_string(&usvg::WriteOptions::default())
        .into_bytes();
    // Parsing outlined text again resolves all absolute ancestor transforms just like ordinary paths.
    let tree = svg_tree(&normalized)?;
    let mm = 25.4 / 96.;
    let mut paths = vec![];
    let mut safe = true;
    let mut warnings = vec![];
    let mut group = 0;
    walk(tree.root(), &mut paths, &mut safe, &mut group, tolerance);
    for path in &mut paths {
        for p in &mut path.points {
            p.y = tree.size().height() as f64 * mm - p.y;
        }
    }
    let xml = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    if doc.descendants().any(|n| n.has_tag_name("text")) {
        warnings.push(tr("Text was converted to outlines with the available fonts. Check the preview before running.").into());
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        for n in doc.descendants().filter(|n| n.has_tag_name("text")) {
            if let Some(family) = n.attribute("font-family") {
                let family = family
                    .split(',')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_matches('\'')
                    .trim_matches('"');
                if !["sans-serif", "serif", "monospace"].contains(&family)
                    && db
                        .query(&usvg::fontdb::Query {
                            families: &[usvg::fontdb::Family::Name(family)],
                            ..Default::default()
                        })
                        .is_none()
                {
                    warnings.push(trf!(
                        "Missing font: {family}. A substitute font was used.",
                        family = family
                    ));
                }
            }
        }
    }
    // Unsupported effects can render correctly but cannot be silently used as cutting geometry.
    if doc.descendants().any(|n| {
        matches!(
            n.tag_name().name(),
            "image"
                | "clipPath"
                | "mask"
                | "filter"
                | "pattern"
                | "linearGradient"
                | "radialGradient"
                | "marker"
        )
    }) {
        safe = false;
    }
    if !safe {
        warnings.push(tr("The SVG contains effects, clipping, or images. Vector operations are blocked; use image engraving.").into());
    }
    if paths.iter().map(|p| p.points.len()).sum::<usize>() > 2_000_000 {
        return Err(tr("The SVG is too complex (more than 2 million points).").into());
    }
    Ok(Asset {
        id: 0,
        name,
        width: tree.size().width() as f64 * mm,
        height: tree.size().height() as f64 * mm,
        geometry: Geometry::Vector {
            paths,
            vector_safe: safe,
        },
        warnings,
        bytes: Arc::new(normalized),
    })
}

fn walk(g: &usvg::Group, out: &mut Vec<Polyline>, safe: &mut bool, group: &mut u32, tol: f64) {
    if g.clip_path().is_some()
        || g.mask().is_some()
        || !g.filters().is_empty()
        || g.opacity().get() != 1.
        || g.blend_mode() != usvg::BlendMode::Normal
    {
        *safe = false;
    }
    for node in g.children() {
        match node {
            usvg::Node::Group(g) => walk(g, out, safe, group, tol),
            usvg::Node::Text(t) => walk(t.flattened(), out, safe, group, tol),
            usvg::Node::Image(_) => *safe = false,
            usvg::Node::Path(p) => {
                if !p.is_visible() {
                    continue;
                }
                if p.stroke()
                    .is_some_and(|s| s.dasharray().is_some() || s.opacity().get() != 1.)
                    || p.fill().is_some_and(|f| f.opacity().get() != 1.)
                {
                    *safe = false;
                }
                *group += 1;
                let transform = p.abs_transform();
                let map = |p: tiny_skia::Point| {
                    let mut p = p;
                    transform.map_point(&mut p);
                    Point::new(p.x as f64 * 25.4 / 96., p.y as f64 * 25.4 / 96.)
                };
                let mut line = Polyline {
                    points: vec![],
                    closed: false,
                    fill: p.fill().is_some(),
                    fill_group: *group,
                    even_odd: p
                        .fill()
                        .is_some_and(|f| f.rule() == usvg::FillRule::EvenOdd),
                };
                for seg in p.data().segments() {
                    match seg {
                        tiny_skia::PathSegment::MoveTo(p) => {
                            if !line.points.is_empty() {
                                out.push(line.clone());
                                line.points.clear();
                                line.closed = false;
                            }
                            line.points.push(map(p));
                        }
                        tiny_skia::PathSegment::LineTo(p) => line.points.push(map(p)),
                        tiny_skia::PathSegment::QuadTo(a, b) => {
                            if let Some(&p) = line.points.last() {
                                let a = map(a);
                                let b = map(b);
                                let c1 = Point::new(
                                    p.x + (a.x - p.x) * 2. / 3.,
                                    p.y + (a.y - p.y) * 2. / 3.,
                                );
                                let c2 = Point::new(
                                    b.x + (a.x - b.x) * 2. / 3.,
                                    b.y + (a.y - b.y) * 2. / 3.,
                                );
                                flatten(p, c1, c2, b, tol, 0, &mut line.points);
                            }
                        }
                        tiny_skia::PathSegment::CubicTo(a, b, c) => {
                            if let Some(&p) = line.points.last() {
                                flatten(p, map(a), map(b), map(c), tol, 0, &mut line.points);
                            }
                        }
                        tiny_skia::PathSegment::Close => {
                            line.closed = true;
                            if let Some(&p) = line.points.first() {
                                if line.points.last() != Some(&p) {
                                    line.points.push(p);
                                }
                            }
                        }
                    }
                }
                if !line.points.is_empty() {
                    out.push(line);
                }
            }
        }
    }
}
fn flatten(a: Point, b: Point, c: Point, d: Point, t: f64, depth: u32, out: &mut Vec<Point>) {
    let distance = |p: Point| {
        let len = a.distance(d);
        if len < 1e-12 {
            a.distance(p)
        } else {
            ((d.x - a.x) * (a.y - p.y) - (a.x - p.x) * (d.y - a.y)).abs() / len
        }
    };
    if depth >= 20
        || (distance(b) <= t
            && distance(c) <= t
            && a.distance(b) + b.distance(c) + c.distance(d) - a.distance(d) <= t * 2.)
    {
        out.push(d);
        return;
    }
    let mid = |a: Point, b: Point| Point::new((a.x + b.x) / 2., (a.y + b.y) / 2.);
    let ab = mid(a, b);
    let bc = mid(b, c);
    let cd = mid(c, d);
    let abc = mid(ab, bc);
    let bcd = mid(bc, cd);
    let m = mid(abc, bcd);
    flatten(a, ab, abc, m, t, depth + 1, out);
    flatten(m, bcd, cd, d, t, depth + 1, out);
}

pub fn preview(a: &Asset, max_side: u32) -> Result<image::RgbaImage, String> {
    match &a.geometry {
        Geometry::Raster { .. } => {
            let img = decode_image(&a.bytes)?;
            Ok(image::imageops::thumbnail(&img, max_side, max_side))
        }
        Geometry::Vector { .. } => {
            let tree = svg_tree(&a.bytes)?;
            let scale = max_side as f32 / tree.size().width().max(tree.size().height());
            let w = (tree.size().width() * scale).ceil().max(1.) as u32;
            let h = (tree.size().height() * scale).ceil().max(1.) as u32;
            let mut pix = tiny_skia::Pixmap::new(w, h).ok_or(tr("Could not render the SVG."))?;
            resvg::render(
                &tree,
                tiny_skia::Transform::from_scale(scale, scale),
                &mut pix.as_mut(),
            );
            let mut bytes = pix.data().to_vec();
            for p in bytes.chunks_exact_mut(4) {
                if p[3] > 0 {
                    for k in 0..3 {
                        p[k] = ((p[k] as u32 * 255) / p[3] as u32).min(255) as u8;
                    }
                }
            }
            image::RgbaImage::from_raw(w, h, bytes).ok_or(tr("Invalid preview.").into())
        }
    }
}

pub fn raster_at(a: &Asset, width: u32, height: u32) -> Result<image::RgbaImage, String> {
    if width as u64 * height as u64 > MAX_PIXELS {
        return Err(
            tr("The job resolution exceeds 40 million pixels; increase the spacing.").into(),
        );
    }
    match a.geometry {
        Geometry::Raster { .. } => Ok(image::imageops::resize(
            &decode_image(&a.bytes)?,
            width,
            height,
            image::imageops::FilterType::Triangle,
        )),
        _ => {
            let tree = svg_tree(&a.bytes)?;
            let mut pix = tiny_skia::Pixmap::new(width, height).ok_or(tr("Invalid resolution."))?;
            resvg::render(
                &tree,
                tiny_skia::Transform::from_scale(
                    width as f32 / tree.size().width(),
                    height as f32 / tree.size().height(),
                ),
                &mut pix.as_mut(),
            );
            let mut bytes = pix.data().to_vec();
            for p in bytes.chunks_exact_mut(4) {
                if p[3] > 0 {
                    for k in 0..3 {
                        p[k] = ((p[k] as u32 * 255) / p[3] as u32).min(255) as u8;
                    }
                }
            }
            image::RgbaImage::from_raw(width, height, bytes)
                .ok_or(tr("Invalid rasterization.").into())
        }
    }
}

/// Built-in editable shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Rectangle,
    Circle,
    Text,
}
impl Shape {
    pub fn label(self) -> &'static str {
        match self {
            Self::Rectangle => tr("Rectangle"),
            Self::Circle => tr("Circle"),
            Self::Text => tr("Text"),
        }
    }
}

pub fn shape(kind: Shape, text: &str) -> Result<Asset, String> {
    let content = match kind {
        Shape::Rectangle => {
            "<rect x=\"0\" y=\"0\" width=\"40\" height=\"25\" fill=\"black\"/>".into()
        }
        Shape::Circle => "<circle cx=\"20\" cy=\"12.5\" r=\"12.5\" fill=\"black\"/>".into(),
        Shape::Text => format!(
            "<text x=\"1\" y=\"18\" font-family=\"Arial\" font-size=\"16\">{}</text>",
            text.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        ),
    };
    let width = if kind == Shape::Text {
        (text.chars().count() as f64 * 11.).max(40.)
    } else {
        40.
    };
    let svg=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}mm\" height=\"25mm\" viewBox=\"0 0 {width} 25\">{content}</svg>");
    from_bytes(format!("{}.svg", kind.label()), svg.into_bytes())
}

pub fn fonts() -> Arc<usvg::fontdb::Database> {
    static FONTS: std::sync::OnceLock<Arc<usvg::fontdb::Database>> = std::sync::OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            db.set_sans_serif_family("Arial");
            Arc::new(db)
        })
        .clone()
}
