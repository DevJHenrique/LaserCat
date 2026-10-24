// SPDX-License-Identifier: AGPL-3.0-only
use crate::i18n::tr;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}
impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn distance(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Positioning {
    UserOrigin,
    Absolute,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MachineProfile {
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub homing: bool,
    pub positioning: Positioning,
    pub invert_x: bool,
    pub invert_y: bool,
    pub port: String,
    pub baud: u32,
    pub configured: bool,
}
impl Default for MachineProfile {
    fn default() -> Self {
        Self {
            name: tr("My machine").into(),
            width: 0.0,
            height: 0.0,
            homing: false,
            positioning: Positioning::UserOrigin,
            invert_x: false,
            invert_y: false,
            port: String::new(),
            baud: 115_200,
            configured: false,
        }
    }
}
impl MachineProfile {
    pub fn validate(&self) -> Result<(), String> {
        if !self.configured
            || !self.width.is_finite()
            || !self.height.is_finite()
            || self.width <= 0.0
            || self.height <= 0.0
            || self.width > 10_000.0
            || self.height > 10_000.0
        {
            return Err(tr("Configure a valid work area (up to 10,000 mm per axis).").into());
        }
        if self.positioning == Positioning::Absolute && !self.homing {
            return Err(tr("An absolute bed requires homing to be configured.").into());
        }
        Ok(())
    }
    pub fn machine_point(&self, p: Point) -> Point {
        Point::new(
            if self.invert_x { self.width - p.x } else { p.x },
            if self.invert_y {
                self.height - p.y
            } else {
                p.y
            },
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub min: Point,
    pub max: Point,
}
impl Bounds {
    pub fn from_points(points: impl IntoIterator<Item = Point>) -> Option<Self> {
        let mut it = points.into_iter();
        let p = it.next()?;
        let mut b = Self { min: p, max: p };
        for p in it {
            b.min.x = b.min.x.min(p.x);
            b.min.y = b.min.y.min(p.y);
            b.max.x = b.max.x.max(p.x);
            b.max.y = b.max.y.max(p.y);
        }
        Some(b)
    }
    pub fn overlaps(self, b: Self) -> bool {
        self.min.x < b.max.x && self.max.x > b.min.x && self.min.y < b.max.y && self.max.y > b.min.y
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Polyline {
    pub points: Vec<Point>,
    pub closed: bool,
    pub fill: bool,
    pub fill_group: u32,
    pub even_odd: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Geometry {
    Vector {
        paths: Vec<Polyline>,
        vector_safe: bool,
    },
    Raster {
        pixel_width: u32,
        pixel_height: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub id: u64,
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub geometry: Geometry,
    pub warnings: Vec<String>,
    #[serde(skip)]
    pub bytes: std::sync::Arc<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationKind {
    Cut,
    Line,
    Fill,
    Raster,
}
impl OperationKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Cut => tr("Cut"),
            Self::Line => tr("Engrave lines"),
            Self::Fill => tr("Engrave fill"),
            Self::Raster => tr("Engrave image"),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    pub kind: OperationKind,
    pub enabled: bool,
    pub power: f64,
    pub speed: f64,
    pub passes: u32,
    pub spacing: f64,
    pub overscan: f64,
    pub brightness: f64,
    pub contrast: f64,
    pub inverted: bool,
    pub dither: bool,
    pub constant_power: bool,
    pub order: i32,
}
impl Operation {
    pub fn new(kind: OperationKind) -> Self {
        Self {
            kind,
            enabled: true,
            power: 20.0,
            speed: 1000.0,
            passes: 1,
            spacing: 0.15,
            overscan: 0.0,
            brightness: 0.0,
            contrast: 1.0,
            inverted: false,
            dither: false,
            constant_power: false,
            order: 0,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self.power.is_finite()
            || !(0.0..=100.0).contains(&self.power)
            || !self.speed.is_finite()
            || self.speed <= 0.0
            || self.speed > 100_000.0
            || !(1..=100).contains(&self.passes)
            || !self.spacing.is_finite()
            || !(0.05..=10.0).contains(&self.spacing)
            || !self.overscan.is_finite()
            || !(0.0..=20.0).contains(&self.overscan)
            || !self.brightness.is_finite()
            || !(-1.0..=1.0).contains(&self.brightness)
            || !self.contrast.is_finite()
            || !(0.0..=4.0).contains(&self.contrast)
        {
            return Err(tr("Invalid operation parameters.").into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneObject {
    pub id: u64,
    pub asset_id: u64,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub rotation: f64,
    pub mirror_x: bool,
    pub mirror_y: bool,
    pub lock_ratio: bool,
    pub locked: bool,
    pub operations: Vec<Operation>,
}
impl SceneObject {
    // Rotation is around the center; the editor and planner use the same mapping.
    pub fn transform(&self, p: Point, asset: &Asset) -> Point {
        let mut x = p.x / asset.width - 0.5;
        let mut y = p.y / asset.height - 0.5;
        if self.mirror_x {
            x = -x;
        }
        if self.mirror_y {
            y = -y;
        }
        x *= self.width;
        y *= self.height;
        let (s, c) = self.rotation.to_radians().sin_cos();
        Point::new(
            self.x + self.width / 2.0 + x * c - y * s,
            self.y + self.height / 2.0 + x * s + y * c,
        )
    }
    pub fn inverse(&self, p: Point, a: &Asset) -> Point {
        let x = p.x - self.x - self.width / 2.0;
        let y = p.y - self.y - self.height / 2.0;
        let (s, c) = self.rotation.to_radians().sin_cos();
        let mut u = (x * c + y * s) / self.width;
        let mut v = (-x * s + y * c) / self.height;
        if self.mirror_x {
            u = -u;
        }
        if self.mirror_y {
            v = -v;
        }
        Point::new((u + 0.5) * a.width, (v + 0.5) * a.height)
    }
    pub fn corners(&self, a: &Asset) -> [Point; 4] {
        [
            Point::new(0., 0.),
            Point::new(a.width, 0.),
            Point::new(a.width, a.height),
            Point::new(0., a.height),
        ]
        .map(|p| self.transform(p, a))
    }
    pub fn bounds(&self, a: &Asset) -> Bounds {
        Bounds::from_points(self.corners(a)).unwrap()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub version: u32,
    pub machine: MachineProfile,
    pub assets: Vec<std::sync::Arc<Asset>>,
    pub objects: Vec<SceneObject>,
    pub next_id: u64,
}
impl PartialEq for Project {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
            && self.machine == other.machine
            && self.next_id == other.next_id
            && self.objects == other.objects
            && self.assets.len() == other.assets.len()
            && self
                .assets
                .iter()
                .zip(&other.assets)
                .all(|(a, b)| std::sync::Arc::ptr_eq(a, b) || a == b)
    }
}
impl Default for Project {
    fn default() -> Self {
        Self {
            version: 1,
            machine: MachineProfile::default(),
            assets: vec![],
            objects: vec![],
            next_id: 1,
        }
    }
}
impl Project {
    pub fn id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    pub fn add_asset(&mut self, mut a: Asset) -> u64 {
        a.id = self.id();
        let id = self.id();
        let kind = match a.geometry {
            Geometry::Raster { .. }
            | Geometry::Vector {
                vector_safe: false, ..
            } => OperationKind::Raster,
            _ => OperationKind::Line,
        };
        self.objects.push(SceneObject {
            id,
            asset_id: a.id,
            name: a.name.clone(),
            x: 10.,
            y: 10.,
            width: a.width,
            height: a.height,
            rotation: 0.,
            mirror_x: false,
            mirror_y: false,
            lock_ratio: true,
            locked: false,
            operations: vec![Operation::new(kind)],
        });
        self.assets.push(std::sync::Arc::new(a));
        id
    }
    pub fn asset(&self, id: u64) -> Option<&Asset> {
        self.assets.iter().find(|a| a.id == id).map(|a| a.as_ref())
    }
    /// Drops embedded assets no object references, e.g. after deleting objects.
    /// Identifiers are not reused, so `next_id` is unchanged.
    pub fn prune_unused_assets(&mut self) {
        let used: std::collections::HashSet<_> = self.objects.iter().map(|o| o.asset_id).collect();
        self.assets.retain(|a| used.contains(&a.id));
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 {
            return Err(tr("Unsupported project version.").into());
        }
        let mut ids = std::collections::HashSet::new();
        let mut max_id = 0;
        for a in &self.assets {
            if !ids.insert(a.id)
                || !a.width.is_finite()
                || !a.height.is_finite()
                || a.width <= 0.
                || a.height <= 0.
            {
                return Err(tr("Invalid asset in project.").into());
            }
            max_id = max_id.max(a.id);
            if let Geometry::Raster {
                pixel_width,
                pixel_height,
            } = a.geometry
            {
                if pixel_width == 0 || pixel_height == 0 {
                    return Err(tr("Invalid asset in project.").into());
                }
            }
            if let Geometry::Vector { paths, .. } = &a.geometry {
                if paths
                    .iter()
                    .flat_map(|p| &p.points)
                    .any(|p| !p.x.is_finite() || !p.y.is_finite())
                {
                    return Err(tr("Invalid geometry.").into());
                }
            }
        }
        for o in &self.objects {
            if !ids.insert(o.id)
                || self.asset(o.asset_id).is_none()
                || [o.x, o.y, o.width, o.height, o.rotation]
                    .iter()
                    .any(|v| !v.is_finite())
                || o.width <= 0.
                || o.height <= 0.
            {
                return Err(tr("Invalid object in project.").into());
            }
            max_id = max_id.max(o.id);
            for op in &o.operations {
                op.validate()?;
            }
        }
        if self.next_id <= max_id || self.next_id == u64::MAX {
            return Err(tr("Invalid identifier sequence.").into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn raster(name: &str) -> Asset {
        Asset {
            id: 0,
            name: name.into(),
            width: 10.,
            height: 10.,
            geometry: Geometry::Raster {
                pixel_width: 4,
                pixel_height: 4,
            },
            warnings: vec![],
            bytes: Default::default(),
        }
    }
    #[test]
    fn pruning_removes_only_unreferenced_assets_and_keeps_ids_valid() {
        let mut p = Project::default();
        let kept = p.add_asset(raster("kept"));
        let deleted = p.add_asset(raster("deleted"));
        let next_id = p.next_id;
        p.objects.retain(|o| o.id != deleted);
        p.prune_unused_assets();
        assert_eq!(p.assets.len(), 1);
        assert_eq!(p.assets[0].name, "kept");
        assert!(p.objects.iter().any(|o| o.id == kept));
        assert_eq!(p.next_id, next_id);
        assert!(p.validate().is_ok());
    }
    #[test]
    fn raster_assets_without_pixels_are_rejected() {
        let mut p = Project::default();
        let mut a = raster("empty");
        a.geometry = Geometry::Raster {
            pixel_width: 0,
            pixel_height: 4,
        };
        p.add_asset(a);
        assert!(p.validate().is_err());
    }
}
