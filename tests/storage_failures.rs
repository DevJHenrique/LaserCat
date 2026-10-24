// SPDX-License-Identifier: AGPL-3.0-only
use lasercat::{domain::Project, import, storage};
use std::{fs::File, io::Write, path::Path};
use zip::{write::FileOptions, ZipWriter};

fn archive(path: &Path, entries: &[(&str, &[u8])]) {
    let mut zip = ZipWriter::new(File::create(path).unwrap());
    for (name, bytes) in entries {
        zip.start_file(*name, FileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn malformed_or_incomplete_archives_return_errors() {
    let dir = tempfile::tempdir_in("target").unwrap();
    let path = dir.path().join("broken.lcc");
    for bytes in [b"".as_slice(), b"not a zip", b"PK\x03\x04"] {
        std::fs::write(&path, bytes).unwrap();
        assert!(storage::load(&path).is_err());
    }
    archive(&path, &[("unrelated.txt", b"empty project")]);
    assert!(storage::load(&path).is_err());
    archive(&path, &[("manifest.json", b"{invalid")]);
    assert!(storage::load(&path).is_err());
}

#[test]
fn missing_embedded_asset_is_not_accepted_as_a_complete_project() {
    let dir = tempfile::tempdir_in("target").unwrap();
    let path = dir.path().join("missing-resource.lcc");
    let mut project = Project::default();
    project.add_asset(import::shape(import::Shape::Rectangle, "").unwrap());
    let manifest = serde_json::to_vec(&project).unwrap();
    archive(&path, &[("manifest.json", &manifest)]);
    assert!(storage::load(&path).is_err());
}

#[test]
fn invalid_manifest_relationships_and_versions_are_rejected() {
    let dir = tempfile::tempdir_in("target").unwrap();
    let path = dir.path().join("invalid-manifest.lcc");
    let mut project = Project::default();
    project.add_asset(import::shape(import::Shape::Rectangle, "").unwrap());
    for mutation in 0..4 {
        let mut invalid = project.clone();
        match mutation {
            0 => invalid.version = 999,
            1 => invalid.objects[0].asset_id = 999,
            2 => invalid.objects[0].id = invalid.assets[0].id,
            _ => invalid.next_id = 1,
        }
        let manifest = serde_json::to_vec(&invalid).unwrap();
        let resource_name = format!("assets/{}", project.assets[0].id);
        // Include valid resources so a missing ZIP entry cannot mask a validation bug.
        archive(
            &path,
            &[
                ("manifest.json", &manifest),
                (&resource_name, project.assets[0].bytes.as_slice()),
            ],
        );
        assert!(storage::load(&path).is_err(), "mutation {mutation}");
    }
}

#[test]
fn rejected_save_preserves_previous_file_and_leaves_no_partial_project() {
    let dir = tempfile::tempdir_in("target").unwrap();
    let path = dir.path().join("preserved.lcc");
    let project = Project::default();
    storage::save(&path, &project).unwrap();
    let original = std::fs::read(&path).unwrap();
    let invalid = Project {
        version: 2,
        ..project.clone()
    };
    assert!(storage::save(&path, &invalid).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(storage::load(&path).unwrap(), project);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}
