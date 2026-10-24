// SPDX-License-Identifier: AGPL-3.0-only
//! Keeps translation catalogs in sync with the English keys used in production code.
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Production code only: test modules and comments may mention keys freely.
fn production_sources() -> Vec<(String, String)> {
    let mut files = vec![];
    rust_files(&root().join("src"), &mut files);
    files
        .into_iter()
        .map(|file| {
            let text = std::fs::read_to_string(&file).unwrap();
            let code = text
                .split("#[cfg(test)]")
                .next()
                .unwrap()
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            (file.display().to_string(), code)
        })
        .collect()
}

/// Reads the string literal starting right after an opening quote.
fn literal(rest: &str) -> String {
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some(other) => out.push(other),
                None => break,
            },
            _ => out.push(c),
        }
    }
    out
}

fn used_keys() -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for (_, code) in production_sources() {
        for marker in ["tr(", "trf!("] {
            for (index, _) in code.match_indices(marker) {
                let preceding = code[..index].chars().last();
                if preceding.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                    continue; // e.g. `str(`, `attr(`
                }
                let rest = code[index + marker.len()..].trim_start();
                if let Some(rest) = rest.strip_prefix('"') {
                    keys.insert(literal(rest));
                }
            }
        }
    }
    keys
}

fn placeholders(text: &str) -> BTreeSet<String> {
    text.split('{')
        .skip(1)
        .filter_map(|part| part.split_once('}').map(|(name, _)| name.to_owned()))
        .filter(|name| !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_'))
        .collect()
}

fn catalogs() -> Vec<(String, BTreeMap<String, String>)> {
    let mut result = vec![];
    for entry in std::fs::read_dir(root().join("locales")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "json") {
            let text = std::fs::read_to_string(&path).unwrap();
            let catalog = serde_json::from_str(&text)
                .unwrap_or_else(|e| panic!("{}: invalid JSON: {e}", path.display()));
            result.push((path.display().to_string(), catalog));
        }
    }
    assert!(!result.is_empty(), "no catalogs found in locales/");
    result
}

#[test]
fn every_used_key_is_translated_and_no_entry_is_stale() {
    let used = used_keys();
    assert!(
        used.len() > 100,
        "key extraction looks broken: {}",
        used.len()
    );
    for (file, catalog) in catalogs() {
        let missing: Vec<_> = used.iter().filter(|k| !catalog.contains_key(*k)).collect();
        let stale: Vec<_> = catalog.keys().filter(|k| !used.contains(*k)).collect();
        assert!(
            missing.is_empty() && stale.is_empty(),
            "{file}\nmissing translations: {missing:#?}\nstale entries: {stale:#?}"
        );
    }
}

#[test]
fn translations_keep_placeholders_and_are_not_empty() {
    for (file, catalog) in catalogs() {
        for (key, value) in &catalog {
            assert!(
                !value.trim().is_empty(),
                "{file}: empty translation for {key:?}"
            );
            assert_eq!(
                placeholders(key),
                placeholders(value),
                "{file}: placeholders differ for {key:?}"
            );
        }
    }
}

#[test]
fn production_strings_are_written_in_english() {
    // Portuguese belongs in locales/pt-BR.json; source text and keys stay in English.
    const PORTUGUESE: &[char] = &['ã', 'õ', 'ç', 'á', 'é', 'í', 'ó', 'ú', 'â', 'ê', 'ô', 'à'];
    let mut offenders = vec![];
    for (file, code) in production_sources() {
        if file.ends_with("i18n.rs") {
            continue; // language names are shown in their own language
        }
        for (number, line) in code.lines().enumerate() {
            if line.contains(PORTUGUESE) {
                offenders.push(format!("{file}:{}: {}", number + 1, line.trim()));
            }
        }
    }
    assert!(offenders.is_empty(), "{}", offenders.join("\n"));
}
