// SPDX-License-Identifier: AGPL-3.0-only
use lasercat::{domain::MachineProfile, storage};

#[test]
fn legacy_project_round_trips_to_lcc_without_modifying_original() {
    let original = include_bytes!("fixtures/legacy-v1.laser");
    let dir = tempfile::tempdir_in("target").unwrap();
    let legacy = dir.path().join("legacy.laser");
    std::fs::write(&legacy, original).unwrap();
    let project = storage::load(&legacy).unwrap();
    assert_eq!(project.version, 1);
    assert_eq!(project.objects.len(), 26);
    let current = storage::project_save_path(legacy.clone());
    assert_eq!(current.extension().unwrap(), "lcc");
    storage::save(&current, &project).unwrap();
    assert_eq!(storage::load(&current).unwrap(), project);
    assert_eq!(std::fs::read(&legacy).unwrap(), original);
}

#[test]
fn new_device_profile_takes_priority_and_invalid_profile_is_not_hidden() {
    let dir = tempfile::tempdir_in("target").unwrap();
    let old = dir.path().join("LaserStudio/device.json");
    let new = dir.path().join("LaserCat/device.json");
    let mut profile = MachineProfile {
        port: "COM3".into(),
        configured: true,
        width: 300.,
        height: 200.,
        ..Default::default()
    };
    storage::save_device(&old, &profile).unwrap();
    let old_bytes = std::fs::read(&old).unwrap();
    assert_eq!(
        storage::load_device_with_legacy(&new, &old).unwrap(),
        Some(profile.clone())
    );
    profile.port = "COM4".into();
    storage::save_device(&new, &profile).unwrap();
    assert_eq!(
        storage::load_device_with_legacy(&new, &old).unwrap(),
        Some(profile)
    );
    std::fs::write(&new, b"invalid").unwrap();
    assert!(storage::load_device_with_legacy(&new, &old).is_err());
    assert_eq!(std::fs::read(old).unwrap(), old_bytes);
}
