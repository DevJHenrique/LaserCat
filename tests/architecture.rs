// SPDX-License-Identifier: AGPL-3.0-only
//! Enforces the dependency table in ARCHITECTURE.md so boundary regressions fail in CI
//! instead of relying on review. Test modules (`#[cfg(test)]` to end of file) are exempt.
use std::path::Path;

const MODULES: [&str; 9] = [
    "calibration",
    "domain",
    "grbl",
    "i18n",
    "import",
    "limits",
    "planner",
    "storage",
    "ui",
];
const GUI_CRATES: [&str; 3] = ["egui", "eframe", "rfd"];

/// (module source, application modules it must not use). `i18n` is a leaf every layer may use.
const RULES: [(&str, &[&str]); 8] = [
    (
        "src/i18n.rs",
        &[
            "calibration",
            "domain",
            "grbl",
            "import",
            "limits",
            "planner",
            "storage",
            "ui",
        ],
    ),
    (
        "src/domain.rs",
        &[
            "calibration",
            "grbl",
            "import",
            "limits",
            "planner",
            "storage",
            "ui",
        ],
    ),
    (
        "src/limits.rs",
        &[
            "calibration",
            "domain",
            "grbl",
            "import",
            "planner",
            "storage",
            "ui",
        ],
    ),
    (
        "src/import.rs",
        &["calibration", "grbl", "planner", "storage", "ui"],
    ),
    ("src/calibration.rs", &["grbl", "storage", "ui"]),
    ("src/planner.rs", &["calibration", "grbl", "storage", "ui"]),
    (
        "src/storage.rs",
        &["calibration", "grbl", "import", "planner", "ui"],
    ),
    ("src/grbl", &["calibration", "import", "storage", "ui"]),
];

fn production_sources(path: &Path) -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let full = root.join(path);
    let files = if full.is_dir() {
        std::fs::read_dir(&full)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "rs"))
            .collect()
    } else {
        vec![full]
    };
    files
        .into_iter()
        .map(|file| {
            let text = std::fs::read_to_string(&file).unwrap();
            let production = text.split("#[cfg(test)]").next().unwrap().to_owned();
            (file.display().to_string(), production)
        })
        .collect()
}

/// Application modules referenced through `crate::module` or `use crate::{module, ...}`.
fn crate_references(source: &str) -> Vec<String> {
    let mut found = vec![];
    for (index, _) in source.match_indices("crate::") {
        let rest = &source[index + "crate::".len()..];
        let scope = if let Some(group) = rest.strip_prefix('{') {
            let mut depth = 1;
            let end = group
                .char_indices()
                .find(|&(_, c)| {
                    depth += match c {
                        '{' => 1,
                        '}' => -1,
                        _ => 0,
                    };
                    depth == 0
                })
                .map_or(group.len(), |(i, _)| i);
            // Only top-level entries of the group name modules.
            let mut top = String::new();
            let mut nested = 0;
            for c in group[..end].chars() {
                match c {
                    '{' => nested += 1,
                    '}' => nested -= 1,
                    _ if nested == 0 => top.push(c),
                    _ => {}
                }
            }
            top
        } else {
            rest.chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect()
        };
        for entry in scope.split(',') {
            let name: String = entry
                .trim()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if MODULES.contains(&name.as_str()) {
                found.push(name);
            }
        }
    }
    found
}

#[test]
fn reference_parser_handles_paths_and_groups() {
    assert_eq!(crate_references("use crate::domain::Point;"), ["domain"]);
    assert_eq!(
        crate_references("use crate::{domain::{A, B}, import, limits::MAX};"),
        ["domain", "import", "limits"]
    );
    assert_eq!(crate_references("let x = crate::ui::run();"), ["ui"]);
    assert!(crate_references("use std::fs;").is_empty());
}

#[test]
fn modules_respect_documented_dependency_rules() {
    let mut violations = vec![];
    for (path, forbidden) in RULES {
        for (file, source) in production_sources(Path::new(path)) {
            for module in crate_references(&source) {
                if forbidden.contains(&module.as_str()) {
                    violations.push(format!("{file} uses crate::{module}"));
                }
            }
            for gui in GUI_CRATES {
                if source.contains(&format!("{gui}::")) {
                    violations.push(format!("{file} uses GUI crate {gui}"));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "Architecture boundary violations (see ARCHITECTURE.md):\n{}",
        violations.join("\n")
    );
}

#[test]
fn core_modules_do_not_open_serial_ports() {
    for path in [
        "src/domain.rs",
        "src/planner.rs",
        "src/storage.rs",
        "src/import.rs",
    ] {
        for (file, source) in production_sources(Path::new(path)) {
            assert!(
                !source.contains("serialport::"),
                "{file} must not access serial hardware"
            );
        }
    }
}
