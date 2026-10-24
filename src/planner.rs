// SPDX-License-Identifier: AGPL-3.0-only
use crate::{domain::*, import};
use crate::{i18n::tr, trf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MAX_SEGMENTS: usize = 1_000_000;
#[derive(Clone, Debug)]
pub struct Stroke {
    pub points: Vec<Point>,
    pub power: f64,
    pub speed: f64,
    pub constant_power: bool,
    pub object_id: u64,
    pub kind: OperationKind,
}
#[derive(Clone, Debug)]
pub struct JobPlan {
    pub strokes: Vec<Stroke>,
    pub bounds: Bounds,
    pub estimated_seconds: f64,
    pub distance_mm: f64,
    pub machine: MachineProfile,
    pub warnings: Vec<String>,
}

pub fn plan(
    p: &Project,
    cancel: &AtomicBool,
    mut progress: impl FnMut(f32),
) -> Result<JobPlan, String> {
    p.validate()?;
    p.machine.validate()?;
    let mut strokes = vec![];
    let mut work: Vec<_> = p
        .objects
        .iter()
        .flat_map(|o| {
            o.operations
                .iter()
                .filter(|op| op.enabled)
                .map(move |op| (o, op))
        })
        .collect();
    work.sort_by_key(|(_, op)| (op.kind == OperationKind::Cut, op.order));
    let total = work.len().max(1);
    let mut count = 0;
    for (index, (o, op)) in work.into_iter().enumerate() {
        check_cancel(cancel)?;
        let a = p.asset(o.asset_id).ok_or(tr("Asset not found."))?;
        for corner in o.corners(a) {
            validate_point(corner, &p.machine)?;
        }
        let generated = match op.kind {
            OperationKind::Cut | OperationKind::Line | OperationKind::Fill => {
                let Geometry::Vector {
                    vector_safe: true, ..
                } = &a.geometry
                else {
                    return Err(trf!(
                        "{name}: this operation requires a compatible vector SVG.",
                        name = o.name
                    ));
                };
                // Reflatten at the final physical scale to preserve the 0.05 mm tolerance.
                let scale = (o.width / a.width).max(o.height / a.height);
                let refined = import::svg_asset(
                    a.name.clone(),
                    a.bytes.as_ref().clone(),
                    0.05 / scale.max(1.),
                )?;
                let Geometry::Vector { paths, .. } = refined.geometry else {
                    unreachable!()
                };
                let mut paths: Vec<_> = paths
                    .into_iter()
                    .map(|mut l| {
                        l.points = l.points.into_iter().map(|p| o.transform(p, a)).collect();
                        l
                    })
                    .collect();
                if op.kind == OperationKind::Fill {
                    fill(&paths, o, op, cancel)?
                } else {
                    if op.kind == OperationKind::Cut {
                        // Smaller nested contours precede their containing contours. Disjoint contours keep stable order.
                        // Bounding boxes reject most pairs before the point-in-polygon test.
                        let boxes: Vec<_> = paths
                            .iter()
                            .map(|l| Bounds::from_points(l.points.iter().copied()))
                            .collect();
                        let depth: Vec<_> = (0..paths.len())
                            .map(|i| {
                                (0..paths.len())
                                    .filter(|&j| {
                                        i != j
                                            && encloses(boxes[j], boxes[i])
                                            && contains_path(&paths[j], &paths[i])
                                    })
                                    .count()
                            })
                            .collect();
                        let mut indexed: Vec<_> = paths.into_iter().zip(depth).collect();
                        indexed.sort_by_key(|(_, d)| std::cmp::Reverse(*d));
                        paths = indexed.into_iter().map(|(l, _)| l).collect();
                    }
                    paths
                        .into_iter()
                        .filter(|p| p.points.len() > 1)
                        .map(|l| stroke(o, op, l.points, op.power))
                        .collect()
                }
            }
            OperationKind::Raster => raster(o, a, op, cancel)?,
        };
        for _ in 0..op.passes {
            for s in &generated {
                check_cancel(cancel)?;
                count += s.points.len().saturating_sub(1);
                if count > MAX_SEGMENTS {
                    return Err(tr("The job exceeds 1 million segments. Increase the spacing or simplify the drawing.").into());
                }
                for pt in &s.points {
                    validate_point(*pt, &p.machine)?;
                }
                strokes.push(s.clone());
            }
        }
        progress((index + 1) as f32 / total as f32);
    }
    let bounds = Bounds::from_points(strokes.iter().flat_map(|s| s.points.iter().copied()))
        .ok_or(tr("There are no toolpaths to run."))?;
    let mut seconds = 0.;
    let mut distance_mm = 0.;
    let mut previous = Point::default();
    for s in &strokes {
        seconds += previous.distance(s.points[0]) / 3000. * 60.;
        let length = s
            .points
            .windows(2)
            .map(|p| p[0].distance(p[1]))
            .sum::<f64>();
        distance_mm += length;
        seconds += length / s.speed * 60.;
        previous = *s.points.last().unwrap();
    }
    let mut warnings = vec![];
    // Objects without enabled operations emit nothing, so they cannot overlap physically.
    let active: Vec<_> = p
        .objects
        .iter()
        .filter(|o| o.operations.iter().any(|op| op.enabled))
        .collect();
    for (i, o) in active.iter().enumerate() {
        let a = p.asset(o.asset_id).unwrap();
        for other in &active[i + 1..] {
            if o.bounds(a)
                .overlaps(other.bounds(p.asset(other.asset_id).unwrap()))
            {
                warnings.push(trf!("{first} and {second} have overlapping bounding boxes; both operations will run.", first = o.name, second = other.name));
            }
        }
    }
    Ok(JobPlan {
        strokes,
        bounds,
        estimated_seconds: seconds,
        distance_mm,
        machine: p.machine.clone(),
        warnings,
    })
}
fn validate_point(p: Point, m: &MachineProfile) -> Result<(), String> {
    if !p.x.is_finite()
        || !p.y.is_finite()
        || p.x < -0.000_001
        || p.y < -0.000_001
        || p.x > m.width + 0.000_001
        || p.y > m.height + 0.000_001
    {
        return Err(trf!(
            "Toolpath outside the bed: X {x}, Y {y} mm.",
            x = format!("{:.3}", p.x),
            y = format!("{:.3}", p.y)
        ));
    }
    Ok(())
}
fn check_cancel(c: &AtomicBool) -> Result<(), String> {
    if c.load(Ordering::Relaxed) {
        Err(tr("Processing cancelled.").into())
    } else {
        Ok(())
    }
}
fn stroke(o: &SceneObject, op: &Operation, points: Vec<Point>, power: f64) -> Stroke {
    Stroke {
        points,
        power,
        speed: op.speed,
        constant_power: op.constant_power,
        object_id: o.id,
        kind: op.kind,
    }
}
fn inside(p: Point, path: &Polyline) -> bool {
    let mut yes = false;
    for (a, b) in edges(path) {
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            yes = !yes;
        }
    }
    yes
}
fn encloses(outer: Option<Bounds>, inner: Option<Bounds>) -> bool {
    outer.zip(inner).is_some_and(|(o, i)| {
        o.min.x <= i.min.x && o.min.y <= i.min.y && o.max.x >= i.max.x && o.max.y >= i.max.y
    })
}
fn contains_path(outer: &Polyline, inner: &Polyline) -> bool {
    outer.closed
        && inner.closed
        && !inner.points.is_empty()
        && inner.points != outer.points
        && inner.points.iter().all(|&p| inside(p, outer))
}
fn edges(p: &Polyline) -> Vec<(Point, Point)> {
    let mut e: Vec<_> = p.points.windows(2).map(|w| (w[0], w[1])).collect();
    if p.points.len() > 2 && p.points.first() != p.points.last() {
        e.push((*p.points.last().unwrap(), p.points[0]));
    }
    e
}
fn fill(
    paths: &[Polyline],
    o: &SceneObject,
    op: &Operation,
    cancel: &AtomicBool,
) -> Result<Vec<Stroke>, String> {
    let b = Bounds::from_points(
        paths
            .iter()
            .filter(|l| l.fill)
            .flat_map(|l| l.points.iter().copied()),
    )
    .ok_or(tr("The SVG has no filled areas."))?;
    let rows = ((b.max.y - b.min.y) / op.spacing).ceil() as usize;
    if rows > 200_000 {
        return Err(tr("Too many fill lines.").into());
    }
    let mut groups = std::collections::BTreeMap::<u32, Vec<&Polyline>>::new();
    for p in paths.iter().filter(|p| p.fill) {
        groups.entry(p.fill_group).or_default().push(p);
    }
    let mut result = vec![];
    for row in 0..rows {
        check_cancel(cancel)?;
        let y = b.min.y + (row as f64 + 0.5) * op.spacing;
        if y > b.max.y {
            break;
        }
        let mut intervals = vec![];
        for group in groups.values() {
            let evenodd = group[0].even_odd;
            let mut crossings: Vec<(f64, i32)> = vec![];
            for path in group {
                for (a, b) in edges(path) {
                    if (a.y <= y && b.y > y) || (b.y <= y && a.y > y) {
                        crossings.push((
                            a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y),
                            if b.y > a.y { 1 } else { -1 },
                        ));
                    }
                }
            }
            crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut winding = 0i32;
            let mut start = 0.;
            for (x, delta) in crossings {
                let was = if evenodd {
                    winding % 2 != 0
                } else {
                    winding != 0
                };
                winding += delta;
                let now = if evenodd {
                    winding % 2 != 0
                } else {
                    winding != 0
                };
                if !was && now {
                    start = x;
                }
                if was && !now && x - start > 1e-6 {
                    intervals.push((start, x));
                }
            }
        }
        intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged: Vec<(f64, f64)> = vec![];
        for (a, b) in intervals {
            if let Some(last) = merged.last_mut() {
                if a <= last.1 + 1e-6 {
                    last.1 = last.1.max(b);
                    continue;
                }
            }
            merged.push((a, b));
        }
        if row % 2 == 1 {
            merged.reverse();
        }
        for (a, b) in merged {
            let points = if row % 2 == 0 {
                vec![Point::new(a, y), Point::new(b, y)]
            } else {
                vec![Point::new(b, y), Point::new(a, y)]
            };
            result.push(stroke(o, op, points, op.power));
        }
        if result.len() > MAX_SEGMENTS {
            return Err(tr("The fill is too complex.").into());
        }
    }
    Ok(result)
}

fn raster(
    o: &SceneObject,
    a: &Asset,
    op: &Operation,
    cancel: &AtomicBool,
) -> Result<Vec<Stroke>, String> {
    let w = (o.width / op.spacing).ceil().max(1.) as u32;
    let h = (o.height / op.spacing).ceil().max(1.) as u32;
    let img = import::raster_at(a, w, h)?;
    let mut result = vec![];
    const BAYER: [[f64; 4]; 4] = [
        [0., 8., 2., 10.],
        [12., 4., 14., 6.],
        [3., 11., 1., 9.],
        [15., 7., 13., 5.],
    ];
    for row in 0..h {
        check_cancel(cancel)?;
        if op.overscan > 0. {
            let y = a.height - (row as f64 + 0.5) / h as f64 * a.height;
            let points = if row % 2 == 0 {
                vec![
                    o.transform(Point::new(-op.overscan / o.width * a.width, y), a),
                    o.transform(Point::new(0., y), a),
                ]
            } else {
                vec![
                    o.transform(Point::new(a.width + op.overscan / o.width * a.width, y), a),
                    o.transform(Point::new(a.width, y), a),
                ]
            };
            result.push(stroke(o, op, points, 0.));
        }
        let runs_start = result.len();
        let mut start = 0;
        let mut previous = 0.;
        for col in 0..=w {
            let power = if col == w {
                0.
            } else {
                let p = img.get_pixel(col, row).0;
                let gray =
                    (p[0] as f64 * 0.2126 + p[1] as f64 * 0.7152 + p[2] as f64 * 0.0722) / 255.;
                let gray = ((gray - 0.5) * op.contrast + 0.5 + op.brightness).clamp(0., 1.);
                let mut value = if op.inverted { gray } else { 1. - gray };
                if op.dither {
                    value = if value > (BAYER[row as usize % 4][col as usize % 4] + 0.5) / 16. {
                        1.
                    } else {
                        0.
                    };
                }
                // Quantize to limit serial bandwidth; transparency is always unexposed.
                (value * p[3] as f64 / 255. * op.power * 10.).round() / 10.
            };
            if power != previous {
                if previous > 0. {
                    let y = a.height - (row as f64 + 0.5) / h as f64 * a.height;
                    let mut pts = vec![
                        o.transform(Point::new(start as f64 / w as f64 * a.width, y), a),
                        o.transform(Point::new(col as f64 / w as f64 * a.width, y), a),
                    ];
                    if row % 2 == 1 {
                        pts.reverse();
                    }
                    result.push(stroke(o, op, pts, previous));
                }
                start = col;
                previous = power;
            }
        }
        if row % 2 == 1 {
            // Keep right-to-left runs in their physical traversal order.
            result[runs_start..].reverse();
        }
        if op.overscan > 0. {
            let y = a.height - (row as f64 + 0.5) / h as f64 * a.height;
            let points = if row % 2 == 0 {
                vec![
                    o.transform(Point::new(a.width, y), a),
                    o.transform(Point::new(a.width + op.overscan / o.width * a.width, y), a),
                ]
            } else {
                vec![
                    o.transform(Point::new(0., y), a),
                    o.transform(Point::new(-op.overscan / o.width * a.width, y), a),
                ]
            };
            result.push(stroke(o, op, points, 0.));
        }
        if result.len() > MAX_SEGMENTS {
            return Err(tr("The raster exceeds 1 million segments. Increase the spacing.").into());
        }
    }
    Ok(result)
}

impl JobPlan {
    pub fn gcode(&self, smax: f64) -> Result<Vec<String>, String> {
        if !smax.is_finite() || smax <= 0. {
            return Err(tr("Invalid power scale.").into());
        }
        let mut lines = vec![
            "G21".into(),
            "G90".into(),
            "G54".into(),
            "G94".into(),
            "M5".into(),
        ];
        let mut last: Option<Point> = None;
        let mut mode = None;
        for s in &self.strokes {
            let p = self.machine.machine_point(s.points[0]);
            if last.is_none_or(|last| last.distance(p) > 0.0001) {
                lines.push("M5".into());
                lines.push(format!("G0 X{:.3} Y{:.3}", p.x.max(0.), p.y.max(0.)));
                mode = None;
            }
            if mode != Some(s.constant_power) {
                lines.push(if s.constant_power { "M3 S0" } else { "M4 S0" }.into());
                mode = Some(s.constant_power);
            }
            for pt in &s.points[1..] {
                let p = self.machine.machine_point(*pt);
                lines.push(format!(
                    "G1 X{:.3} Y{:.3} F{:.1} S{:.3}",
                    p.x.max(0.),
                    p.y.max(0.),
                    s.speed,
                    s.power / 100. * smax
                ));
                last = Some(p);
            }
            // Never leave emission active while the host waits for the next stroke.
            lines.push("G1 S0".into());
        }
        lines.push("M5".into());
        Ok(lines)
    }
    pub fn frame(&self) -> Vec<String> {
        let b = self.bounds;
        let mut lines = vec!["M5".into(), "G21".into(), "G90".into(), "G54".into()];
        for p in [
            b.min,
            Point::new(b.max.x, b.min.y),
            b.max,
            Point::new(b.min.x, b.max.y),
            b.min,
        ] {
            let p = self.machine.machine_point(p);
            lines.push(format!("G1 X{:.3} Y{:.3} F1000 S0", p.x, p.y));
        }
        lines.push("M5".into());
        lines
    }
}
