# ADR 0005 — Interface internationalization

- Status: accepted
- Date: 2026-10-24
- Supersedes the "Portuguese UI unchanged" part of [ADR 0004](0004-license-and-documentation.md)

## Context

The interface was written in Portuguese. The project is opening to an international community, so the maintainer asked for English as the default interface language while keeping Portuguese available. Code identifiers were already English; user-facing strings were hard-coded across the core and the UI, including error messages produced by `domain`, `planner`, `storage`, `import`, and `grbl`.

## Decision

- Add a leaf module `src/i18n.rs` that any layer may use. It holds the `Language` enum, the current language, `tr(english)` for plain text, and `trf!("… {name} …", name = value)` for named placeholders.
- English source strings are the catalog keys (gettext style). Untranslated text falls back to English, so English never needs a catalog.
- Translations live in `locales/<code>.json`, embedded at compile time and parsed with the existing `serde_json` dependency. Portuguese (Brazil) is `locales/pt-BR.json`.
- The chosen language is a local preference stored in `%LOCALAPPDATA%\LaserCat\preferences.json`, separate from projects and the device profile. The default is English. Demo and test sessions never read or write it.
- `tests/i18n.rs` fails when a key used in production code has no translation, when a catalog entry is unused, when placeholders differ, or when Portuguese text is hard-coded in production code.

## Alternatives

- **`fluent` / `rust-i18n` crates.** More features (plural rules, message IDs) at the cost of a new dependency, a build step or macro DSL, and contributor learning. Two languages with simple messages do not need them yet; the JSON format can be migrated if plural handling becomes necessary.
- **Message IDs instead of English keys.** Stable when wording changes, but every call site loses readable text and English needs its own catalog.
- **Translating only the UI layer.** Core errors are shown to the operator, so they must be translated too; returning error codes from every core function would be a much larger API change.

## Consequences

- Changing English wording changes a key; the i18n test then lists the stale and missing entries so translators can follow.
- Messages are translated when created. Log lines already shown keep their original language after switching.
- Adding a language means one JSON file and one `Language` variant; the tests cover it automatically.
- Project files, G-code, and protocol traffic are unaffected. Object names created by the app (e.g. "Rectangle.svg") use the language active at creation.
