// SPDX-License-Identifier: AGPL-3.0-only
use lasercat::{domain::*, import, planner};
use std::sync::atomic::{AtomicBool, Ordering};

fn project() -> Project {
    let mut p = Project {
        machine: MachineProfile {
            configured: true,
            width: 300.,
            height: 200.,
            ..Default::default()
        },
        ..Default::default()
    };
    p.add_asset(import::shape(import::Shape::Rectangle, "").unwrap());
    p
}

#[test]
fn transformations_round_trip_across_rotations_mirrors_and_scales() {
    let p = project();
    let asset = &p.assets[0];
    for rotation in [-360., -90., 0., 37., 180., 359.9] {
        for (mirror_x, mirror_y) in [(false, false), (true, false), (false, true), (true, true)] {
            for (width, height) in [(0.1, 0.1), (17., 93.), (400., 10.)] {
                let o = SceneObject {
                    x: -50.,
                    y: 70.,
                    width,
                    height,
                    rotation,
                    mirror_x,
                    mirror_y,
                    ..p.objects[0].clone()
                };
                for point in [
                    Point::default(),
                    Point::new(asset.width, asset.height),
                    Point::new(3.7, 12.1),
                ] {
                    assert!(o.inverse(o.transform(point, asset), asset).distance(point) < 1e-7);
                }
            }
        }
    }
}

#[test]
fn disabled_operations_do_not_produce_paths_and_passes_repeat_enabled_paths() {
    let mut p = project();
    let disabled = p.add_asset(import::shape(import::Shape::Circle, "").unwrap());
    p.objects[1].operations[0].enabled = false;
    p.objects[1].x = 1000.;
    let once = planner::plan(&p, &AtomicBool::new(false), |_| {}).unwrap();
    p.objects[0].operations[0].passes = 3;
    let triple = planner::plan(&p, &AtomicBool::new(false), |_| {}).unwrap();
    assert!(triple.strokes.iter().all(|s| s.object_id != disabled));
    assert_eq!(triple.strokes.len(), once.strokes.len() * 3);
    assert!((triple.distance_mm - once.distance_mm * 3.).abs() < 1e-8);
}

#[test]
fn cancellation_between_operations_never_returns_a_partial_plan() {
    let mut p = project();
    p.add_asset(import::shape(import::Shape::Circle, "").unwrap());
    let cancel = AtomicBool::new(false);
    let mut reported = false;
    let result = planner::plan(&p, &cancel, |progress| {
        if progress > 0. {
            reported = true;
            cancel.store(true, Ordering::Relaxed);
        }
    });
    assert!(reported);
    assert!(result.unwrap_err().contains("cancelled"));
}

#[test]
fn gcode_scaling_modes_and_travel_remain_valid_for_all_axis_orientations() {
    for (invert_x, invert_y) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut p = project();
        p.machine.invert_x = invert_x;
        p.machine.invert_y = invert_y;
        p.objects[0].operations[0].power = 25.;
        let mut second = Operation::new(OperationKind::Cut);
        second.power = 80.;
        second.constant_power = true;
        p.objects[0].operations.push(second);
        let plan = planner::plan(&p, &AtomicBool::new(false), |_| {}).unwrap();
        let code = plan.gcode(2000.).unwrap();
        assert!(code.iter().any(|line| line == "M4 S0"));
        assert!(code.iter().any(|line| line == "M3 S0"));
        assert!(code.iter().any(|line| line.ends_with("S500.000")));
        assert!(code.iter().any(|line| line.ends_with("S1600.000")));
        for (index, line) in code.iter().enumerate() {
            if line.starts_with("G0 ") {
                assert_eq!(code[index - 1], "M5");
            }
            for token in line.split_whitespace() {
                if let Some(x) = token.strip_prefix('X') {
                    assert!((0. ..=p.machine.width).contains(&x.parse::<f64>().unwrap()));
                }
                if let Some(y) = token.strip_prefix('Y') {
                    assert!((0. ..=p.machine.height).contains(&y.parse::<f64>().unwrap()));
                }
            }
        }
        assert_eq!(code.last().unwrap(), "M5");
        assert!(plan
            .frame()
            .iter()
            .all(|line| !line.starts_with("M3") && !line.starts_with("M4")));
        for invalid in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(plan.gcode(invalid).is_err());
        }
        let repeated = planner::plan(&p, &AtomicBool::new(false), |_| {}).unwrap();
        assert_eq!(code, repeated.gcode(2000.).unwrap());
    }
}
