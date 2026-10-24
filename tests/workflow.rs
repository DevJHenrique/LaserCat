// SPDX-License-Identifier: AGPL-3.0-only
use lasercat::{
    domain::*,
    grbl::{Command, Decoder, Event, MachineSession, Snapshot, State, Stream},
    import, planner, storage,
};
use std::{
    io::Cursor,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

fn profile() -> MachineProfile {
    MachineProfile {
        configured: true,
        width: 300.,
        height: 200.,
        ..Default::default()
    }
}
fn project() -> Project {
    Project {
        machine: profile(),
        ..Default::default()
    }
}
fn svg(content: &str) -> Asset {
    import::from_bytes("test.svg".into(),format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"40mm\" height=\"40mm\" viewBox=\"0 0 40 40\">{content}</svg>").into_bytes()).unwrap()
}
fn calculate(p: &Project) -> planner::JobPlan {
    planner::plan(p, &AtomicBool::new(false), |_| {}).unwrap()
}
fn wait(session: &MachineSession, mut predicate: impl FnMut(&Snapshot) -> bool) -> Snapshot {
    let end = Instant::now() + Duration::from_secs(5);
    let mut last = Snapshot::default();
    while Instant::now() < end {
        if let Ok(Event::Snapshot(s)) = session.rx.recv_timeout(Duration::from_millis(100)) {
            if predicate(&s) {
                return s;
            }
            last = s;
        }
    }
    panic!("Expected state not received; last state: {last:?}");
}

#[test]
fn dimensions_are_never_assumed() {
    assert!(MachineProfile::default().validate().is_err());
    let mut m = profile();
    m.positioning = Positioning::Absolute;
    assert!(m.validate().is_err());
    m.homing = true;
    assert!(m.validate().is_ok());
}

#[test]
fn svg_units_and_orientation_are_preserved() {
    let a = svg("<path d=\"M0 0 L40 0 L40 40 Z\"/>");
    assert!((a.width - 40.).abs() < 0.001);
    let Geometry::Vector { paths, .. } = a.geometry else {
        panic!()
    };
    assert!((paths[0].points[0].y - 40.).abs() < 0.001);
    assert!(paths[0].closed);
}

#[test]
fn svg_transforms_and_curves_are_flattened() {
    let a=svg("<g transform=\"translate(3 4)\"><path d=\"M0 0 C0 20 20 20 20 0\" fill=\"none\" stroke=\"black\"/></g>");
    let Geometry::Vector { paths, .. } = a.geometry else {
        panic!()
    };
    assert!((paths[0].points[0].x - 3.).abs() < 0.001);
    assert!(paths[0].points.len() > 10);
}

#[test]
fn unsupported_vector_effects_are_not_cut_silently() {
    let a=svg("<defs><clipPath id=\"c\"><rect width=\"10\" height=\"10\"/></clipPath></defs><rect width=\"40\" height=\"40\" clip-path=\"url(#c)\"/>");
    assert!(!a.warnings.is_empty());
    let mut p = project();
    p.add_asset(a);
    assert_eq!(p.objects[0].operations[0].kind, OperationKind::Raster);
    p.objects[0].operations[0].kind = OperationKind::Cut;
    assert!(planner::plan(&p, &AtomicBool::new(false), |_| {}).is_err());
}

#[test]
fn external_svg_resources_are_rejected() {
    assert!(import::from_bytes(
        "e.svg".into(),
        br#"<svg xmlns="http://www.w3.org/2000/svg"><image href="file:///secret.png"/></svg>"#
            .to_vec()
    )
    .is_err());
}

#[test]
fn text_is_embedded_as_outlines() {
    let a = import::shape(import::Shape::Text, "Olá Laser").unwrap();
    let normalized = std::str::from_utf8(&a.bytes).unwrap();
    assert!(!normalized.contains("<text"));
    assert!(normalized.contains("<path"));
    let Geometry::Vector { paths, .. } = &a.geometry else {
        panic!()
    };
    assert!(!paths.is_empty());
}

#[test]
fn missing_font_is_reported() {
    let a = svg("<text x=\"1\" y=\"20\" font-family=\"DefinitelyMissingLaserFont\">ABC</text>");
    assert!(a.warnings.iter().any(|w| w.contains("Missing font")));
}

#[test]
fn scene_transform_inverse_round_trip() {
    let a = svg("<rect width=\"40\" height=\"40\"/>");
    let mut p = project();
    p.add_asset(a);
    let o = &mut p.objects[0];
    o.rotation = 73.;
    o.width = 60.;
    o.height = 20.;
    o.mirror_x = true;
    o.mirror_y = true;
    let a = &p.assets[0];
    let pt = Point::new(5., 14.);
    let back = o.inverse(o.transform(pt, a), a);
    assert!(pt.distance(back) < 1e-9);
}

#[test]
fn overlaps_keep_independent_power_speed_and_passes() {
    let mut p = project();
    p.add_asset(svg("<rect width=\"20\" height=\"20\"/>"));
    p.add_asset(svg("<circle cx=\"10\" cy=\"10\" r=\"8\"/>"));
    p.objects[0].operations[0].power = 15.;
    p.objects[0].operations[0].speed = 700.;
    p.objects[1].operations[0].power = 65.;
    p.objects[1].operations[0].speed = 1400.;
    p.objects[1].operations[0].passes = 2;
    let job = calculate(&p);
    assert_eq!(job.strokes.len(), 3);
    assert!(!job.warnings.is_empty());
    let code = job.gcode(1000.).unwrap().join("\n");
    assert!(code.contains("F700.0 S150.000"));
    assert!(code.contains("F1400.0 S650.000"));
}

#[test]
fn disabled_objects_do_not_produce_overlap_warnings() {
    let mut p = project();
    p.add_asset(svg("<rect width=\"20\" height=\"20\"/>"));
    p.add_asset(svg("<circle cx=\"10\" cy=\"10\" r=\"8\"/>"));
    p.objects[1].operations[0].enabled = false;
    let job = calculate(&p);
    assert!(job.warnings.is_empty(), "{:?}", job.warnings);
    assert!(job.strokes.iter().all(|s| s.object_id == p.objects[0].id));
}

#[test]
fn deeply_nested_cut_contours_run_innermost_first() {
    let mut p = project();
    // Three nested squares plus a disjoint square, declared outermost first.
    p.add_asset(svg(
        "<path fill=\"none\" stroke=\"black\" d=\"M0 0H30V30H0Z M5 5H25V25H5Z M10 10H20V20H10Z M32 0H38V6H32Z\"/>",
    ));
    p.objects[0].operations[0].kind = OperationKind::Cut;
    let job = calculate(&p);
    let widths: Vec<_> = job
        .strokes
        .iter()
        .map(|s| {
            let b = Bounds::from_points(s.points.clone()).unwrap();
            (b.max.x - b.min.x).round()
        })
        .collect();
    let position = |w: f64| widths.iter().position(|&x| x == w).unwrap();
    assert!(position(10.) < position(20.));
    assert!(position(20.) < position(30.));
}

#[test]
fn engraving_precedes_cuts_and_nested_contours_precede_outer() {
    let mut p = project();
    p.add_asset(svg(
        "<path fill-rule=\"evenodd\" d=\"M0 0H40V40H0Z M10 10H30V30H10Z\"/>",
    ));
    let o = &mut p.objects[0];
    o.operations[0].kind = OperationKind::Cut;
    o.operations.push(Operation::new(OperationKind::Line));
    let job = calculate(&p);
    assert_eq!(job.strokes[0].kind, OperationKind::Line);
    let cuts: Vec<_> = job
        .strokes
        .iter()
        .filter(|s| s.kind == OperationKind::Cut)
        .collect();
    let b = Bounds::from_points(cuts[0].points.clone()).unwrap();
    assert!((b.max.x - b.min.x - 20.).abs() < 0.01);
}

#[test]
fn fill_respects_evenodd_holes_and_unions_overlapping_shapes() {
    let mut p = project();
    p.add_asset(svg(
        "<path fill-rule=\"evenodd\" d=\"M0 0H40V40H0Z M10 10H30V30H10Z\"/>",
    ));
    p.objects[0].operations[0] = Operation::new(OperationKind::Fill);
    p.objects[0].operations[0].spacing = 1.;
    let job = calculate(&p);
    for s in &job.strokes {
        let middle = Point::new((s.points[0].x + s.points[1].x) / 2., s.points[0].y);
        assert!(!(middle.x > 20. && middle.x < 40. && middle.y > 20. && middle.y < 40.));
    }
}

#[test]
fn fill_respects_nonzero_winding() {
    let mut p = project();
    p.add_asset(svg("<path d=\"M0 0H40V40H0Z M10 10V30H30V10Z\"/>"));
    p.objects[0].operations[0] = Operation::new(OperationKind::Fill);
    p.objects[0].operations[0].spacing = 1.;
    let job = calculate(&p);
    assert!(job.strokes.len() > 40);
}

fn raster_asset() -> Asset {
    let mut img = image::RgbaImage::from_pixel(4, 2, image::Rgba([255, 255, 255, 255]));
    img.put_pixel(0, 0, image::Rgba([0, 0, 0, 255]));
    img.put_pixel(1, 0, image::Rgba([0, 0, 0, 0]));
    img.put_pixel(2, 1, image::Rgba([0, 0, 0, 255]));
    let mut data = Cursor::new(vec![]);
    img.write_to(&mut data, image::ImageFormat::Png).unwrap();
    import::from_bytes("test.png".into(), data.into_inner()).unwrap()
}
#[test]
fn raster_transparency_rotation_and_physical_size() {
    let mut p = project();
    p.add_asset(raster_asset());
    let o = &mut p.objects[0];
    o.width = 4.;
    o.height = 2.;
    o.rotation = 90.;
    o.operations[0].spacing = 1.;
    o.operations[0].power = 50.;
    let job = calculate(&p);
    assert_eq!(job.strokes.len(), 2);
    assert!(job.strokes.iter().all(|s| s.power == 50.));
    let first = &job.strokes[0];
    assert!((first.points[0].x - first.points[1].x).abs() < 1e-8);
    assert!((first.points[0].distance(first.points[1]) - 1.).abs() < 1e-8);
}

#[test]
fn inverted_raster_never_exposes_transparent_pixels() {
    let mut p = project();
    p.add_asset(raster_asset());
    let o = &mut p.objects[0];
    o.width = 4.;
    o.height = 2.;
    o.operations[0].spacing = 1.;
    o.operations[0].inverted = true;
    let job = calculate(&p);
    assert!(!job
        .strokes
        .iter()
        .any(|s| s.points[0].y > 11. && s.points[0].x < 12. && s.points[1].x > 11.));
}

#[test]
fn out_of_bounds_blocks_planning_including_rotated_corners() {
    let mut p = project();
    p.add_asset(svg("<rect width=\"40\" height=\"40\"/>"));
    p.objects[0].x = -1.;
    assert!(planner::plan(&p, &AtomicBool::new(false), |_| {}).is_err());
    p.objects[0].x = 0.;
    p.objects[0].y = 0.;
    p.objects[0].rotation = 45.;
    assert!(planner::plan(&p, &AtomicBool::new(false), |_| {}).is_err());
}

#[test]
fn cancellation_and_bad_parameters_are_reported() {
    let mut p = project();
    p.add_asset(svg("<rect width=\"10\" height=\"10\"/>"));
    assert!(planner::plan(&p, &AtomicBool::new(true), |_| {}).is_err());
    p.objects[0].operations[0].speed = 0.;
    assert!(planner::plan(&p, &AtomicBool::new(false), |_| {}).is_err());
}

#[test]
fn archive_round_trip_and_atomic_overwrite() {
    let mut p = project();
    p.add_asset(svg("<rect width=\"40\" height=\"40\"/>"));
    p.add_asset(raster_asset());
    p.objects[0].rotation = 12.;
    p.objects[1].mirror_x = true;
    let dir = tempfile::tempdir_in("target").unwrap();
    let path = dir.path().join("project.lcc");
    storage::save(&path, &p).unwrap();
    assert_eq!(storage::load(&path).unwrap(), p);
    p.objects[0].name = "Alterado".into();
    storage::save(&path, &p).unwrap();
    assert_eq!(storage::load(&path).unwrap(), p);
}

#[test]
fn unknown_version_and_invalid_ids_are_rejected() {
    let mut p = project();
    p.version = 2;
    assert!(p.validate().is_err());
    p.version = 1;
    p.add_asset(raster_asset());
    p.next_id = 1;
    assert!(p.validate().is_err());
}

#[test]
fn decoder_handles_fragmented_and_interleaved_responses() {
    let mut d = Decoder::default();
    assert!(d.push(b"o").unwrap().is_empty());
    assert_eq!(
        d.push(b"k\r\n<Idle|WPos:0,0,0>\nerr").unwrap(),
        vec!["ok", "<Idle|WPos:0,0,0>"]
    );
    assert_eq!(d.push(b"or:2\n").unwrap(), vec!["error:2"]);
    assert!(d.push(&vec![b'x'; 4097]).is_err());
}

#[test]
fn serial_buffer_limit_and_physical_completion_barrier() {
    let mut s = Stream::default();
    s.begin(vec!["X".repeat(60); 3]).unwrap();
    assert!(s.next_line().is_some());
    assert!(s.next_line().is_some());
    assert!(s.next_line().is_none());
    s.acknowledge().unwrap();
    assert!(s.next_line().is_some());
    s.acknowledge().unwrap();
    s.acknowledge().unwrap();
    assert!(s.waiting_idle);
    assert!(!s.status(true));
    s.queried_after_ack = true;
    assert!(!s.status(false));
    assert!(s.status(true));
    assert!(!s.active());
}

#[test]
fn streaming_rejects_injected_newlines_and_oversized_lines() {
    let mut s = Stream::default();
    assert!(s.begin(vec!["G1 X0\nM3".into()]).is_err());
    assert!(s.begin(vec!["X".repeat(127)]).is_err());
    assert!(s.acknowledge().is_err());
}

#[test]
fn gcode_and_frame_always_disable_laser_during_travel() {
    let mut p = project();
    p.add_asset(svg("<rect width=\"40\" height=\"40\"/>"));
    let job = calculate(&p);
    let code = job.gcode(1000.).unwrap();
    assert_eq!(code.last().unwrap(), "M5");
    for (i, line) in code.iter().enumerate() {
        if line.starts_with("G0 ") {
            assert_eq!(code[i - 1], "M5");
        }
    }
    assert!(job
        .frame()
        .iter()
        .all(|l| !l.starts_with("M3") && !l.starts_with("M4")));
}

#[test]
fn simulator_full_flow_reference_run_and_disconnect() {
    let s = MachineSession::new();
    s.send(Command::Connect(profile(), true));
    let ready = wait(&s, |s| s.state == State::Ready);
    assert!(ready.verified);
    assert_eq!(ready.smax, Some(1000.));
    assert!(ready.referenced);
    s.send(Command::Zero);
    wait(&s, |s| s.state == State::Ready && s.referenced);
    let mut p = project();
    p.add_asset(svg("<rect width=\"20\" height=\"20\"/>"));
    s.send(Command::Start(calculate(&p)));
    wait(&s, |s| s.state == State::Running);
    let complete = wait(&s, |s| s.physical_complete);
    assert_eq!(complete.accepted, complete.total);
    assert_eq!(complete.state, State::Ready);
    s.send(Command::Disconnect);
    let off = wait(&s, |s| s.state == State::Disconnected);
    assert!(!off.referenced);
}

#[test]
fn simulator_pause_cancel_and_reconnect_require_new_reference() {
    let s = MachineSession::new();
    s.send(Command::Connect(profile(), true));
    wait(&s, |s| s.state == State::Ready);
    s.send(Command::Zero);
    wait(&s, |s| s.referenced);
    let mut p = project();
    p.add_asset(svg("<circle cx=\"20\" cy=\"20\" r=\"15\"/>"));
    s.send(Command::Start(calculate(&p)));
    wait(&s, |s| s.state == State::Running);
    s.send(Command::Pause);
    wait(&s, |s| s.state == State::Paused);
    s.send(Command::Cancel);
    let reset = wait(&s, |s| s.state == State::Ready && !s.referenced);
    assert!(!reset.physical_complete);
    assert!(!reset.homed);
}

#[test]
fn raster_fallback_is_96_dpi() {
    let a = raster_asset();
    assert!((a.width - 4. * 25.4 / 96.).abs() < 1e-9);
    assert!(Arc::strong_count(&a.bytes) >= 1);
}

#[test]
fn normalized_svg_preserves_text_geometry() {
    let a = import::shape(import::Shape::Text, "LASER").unwrap();
    let b = import::from_bytes(a.name.clone(), a.bytes.as_ref().clone()).unwrap();
    let bounds = |a: &Asset| {
        let Geometry::Vector { paths, .. } = &a.geometry else {
            panic!()
        };
        Bounds::from_points(paths.iter().flat_map(|p| p.points.iter().copied())).unwrap()
    };
    let (a, b) = (bounds(&a), bounds(&b));
    assert!(a.min.distance(b.min) < 0.01, "{a:?} versus {b:?}");
    assert!(a.max.distance(b.max) < 0.01);
}

#[test]
fn raster_margins_are_included_in_bounds_validation() {
    let mut p = project();
    p.add_asset(raster_asset());
    let o = &mut p.objects[0];
    o.x = 0.;
    o.width = 4.;
    o.height = 2.;
    o.operations[0].spacing = 1.;
    o.operations[0].overscan = 2.;
    assert!(planner::plan(&p, &AtomicBool::new(false), |_| {}).is_err());
    p.objects[0].x = 10.;
    let job = calculate(&p);
    assert!(job.strokes.iter().any(|s| s.power == 0.));
    assert!((job.bounds.min.x - 8.).abs() < 1e-8);
}

#[test]
fn animation_is_rejected_instead_of_silently_losing_geometry() {
    assert!(import::from_bytes("animated.svg".into(),br#"<svg xmlns="http://www.w3.org/2000/svg"><rect width="10" height="10"><animate attributeName="x" to="20"/></rect></svg>"#.to_vec()).is_err());
}

#[test]
fn project_snapshots_share_large_immutable_resources() {
    let mut p = project();
    p.add_asset(svg("<circle cx=\"20\" cy=\"20\" r=\"10\"/>"));
    let snapshot = p.clone();
    assert!(Arc::ptr_eq(&p.assets[0], &snapshot.assets[0]));
}

#[test]
fn initial_connection_works_before_machine_dimensions_are_defined() {
    let s = MachineSession::new();
    s.send(Command::Connect(MachineProfile::default(), true));
    let ready = wait(&s, |s| s.state == State::Ready);
    assert!(ready.verified);
    s.send(Command::Zero);
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut rejected = false;
    while Instant::now() < deadline {
        if let Ok(Event::Log(message)) = s.rx.recv_timeout(Duration::from_millis(50)) {
            if message.contains("work area") {
                rejected = true;
                break;
            }
        }
    }
    assert!(
        rejected,
        "Movement must stay blocked until dimensions are configured."
    );
}

#[test]
fn connection_error_is_published_to_the_interface() {
    let s = MachineSession::new();
    s.send(Command::Connect(MachineProfile::default(), false));
    let failed = wait(&s, |s| s.state == State::Fault);
    assert!(failed.message.contains("COM3"));
    assert!(!failed.referenced);
}

#[test]
fn simulator_returns_to_work_origin_without_homing() {
    let session = MachineSession::new();
    session.send(Command::Connect(profile(), true));
    wait(&session, |s| s.state == State::Ready);
    session.send(Command::Zero);
    wait(&session, |s| s.state == State::Ready && s.referenced);
    session.send(Command::Jog(5., 7.));
    wait(&session, |s| {
        s.state == State::Ready && s.position.is_some_and(|p| p[0] == 5. && p[1] == 7.)
    });
    session.send(Command::ReturnOrigin);
    let ready = wait(&session, |s| {
        s.state == State::Ready && s.position.is_some_and(|p| p[0] == 0. && p[1] == 0.)
    });
    assert!(ready.referenced);
    assert!(!ready.homed);
}
