// SPDX-License-Identifier: AGPL-3.0-only
use crate::domain::{Point, SceneObject};

pub fn corners(o: &SceneObject) -> [Point; 4] {
    let (s, c) = o.rotation.to_radians().sin_cos();
    [(-1., -1.), (1., -1.), (1., 1.), (-1., 1.)].map(|(x, y)| {
        let (x, y) = (x * o.width / 2., y * o.height / 2.);
        Point::new(
            o.x + o.width / 2. + x * c - y * s,
            o.y + o.height / 2. + x * s + y * c,
        )
    })
}

pub fn resize(
    o: &mut SceneObject,
    original: &SceneObject,
    corner: usize,
    pointer: Point,
    uniform: bool,
) {
    let anchor = corners(original)[(corner + 2) % 4];
    let sx = if corner == 1 || corner == 2 { 1. } else { -1. };
    let sy = if corner >= 2 { 1. } else { -1. };
    let (s, c) = original.rotation.to_radians().sin_cos();
    let (dx, dy) = (pointer.x - anchor.x, pointer.y - anchor.y);
    let (u, v) = ((dx * c + dy * s) * sx, (-dx * s + dy * c) * sy);
    let (w, h) = if uniform {
        let factor = ((u * original.width + v * original.height)
            / (original.width.powi(2) + original.height.powi(2)))
        .clamp(
            (0.1 / original.width).max(0.1 / original.height),
            (10000. / original.width).min(10000. / original.height),
        );
        (original.width * factor, original.height * factor)
    } else {
        (u.clamp(0.1, 10000.), v.clamp(0.1, 10000.))
    };
    let (x, y) = (sx * w / 2., sy * h / 2.);
    o.width = w;
    o.height = h;
    o.x = anchor.x + x * c - y * s - w / 2.;
    o.y = anchor.y + x * s + y * c - h / 2.;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotated_resize_keeps_opposite_corner_fixed_and_ratio() {
        let mut project = crate::domain::Project::default();
        project.add_asset(crate::import::shape(crate::import::Shape::Rectangle, "").unwrap());
        let mut original = project.objects[0].clone();
        original.rotation = 37.;
        for index in 0..4 {
            let mut resized = original.clone();
            let anchor = corners(&original)[(index + 2) % 4];
            let moving = corners(&original)[index];
            resize(
                &mut resized,
                &original,
                index,
                Point::new(
                    anchor.x + 2. * (moving.x - anchor.x),
                    anchor.y + 2. * (moving.y - anchor.y),
                ),
                true,
            );
            assert!(corners(&resized)[(index + 2) % 4].distance(anchor) < 1e-8);
            assert!((resized.width - original.width * 2.).abs() < 1e-8);
            assert!((resized.height - original.height * 2.).abs() < 1e-8);
        }
    }
}
