# Changelog

## 0.2.2 — 2026-10-24

- Update eframe/egui from 0.31 to 0.37. Panels now attach to the root `Ui` (`LaserApp::draw` takes `&mut egui::Ui`). A direct `egui` entry enables `monochrome_emoji_fonts`, which egui no longer includes in `default_fonts`, so symbols such as the object color swatches still render. The theme keeps the round slider handle and turns off the new scroll-area edge fade. Dialogs still open inside the area the panels leave free, and the object-overlap highlight keeps its previous tint. egui's own visual changes remain: text rasterization, disabled widgets drawn at half opacity, text fields as tall as buttons, and hint text in the weak text color.
- Update zip 0.6 → 9 (archives are written with `SimpleFileOptions`; the `.lcc` format is unchanged), resvg 0.45 → 0.48 (text is now shaped with harfrust instead of rustybuzz), rfd 0.15 → 0.17, roxmltree 0.20 → 0.21, and the `actions/checkout` and `actions/upload-artifact` GitHub Actions 4 → 7.
- Include the OFL-1.1, Ubuntu Font License, and Hack license texts of the fonts egui embeds (`epaint_default_fonts`) in `THIRD_PARTY_NOTICES.txt`; `scripts/Generate-Notices.ps1` previously skipped them.

## 0.2.1 GitHub build — 2026-10-24

The first GitHub release kept version 0.2.1 but was built from a later commit, so it also included:

- Internationalize the interface (ADR 0005): English is the default language, Brazilian Portuguese stays available from the new **Language** menu, and the choice is saved in `preferences.json`. Core error messages are translated too.
- Add translation catalogs in `locales/` and `tests/i18n.rs`, which rejects missing, stale, or hard-coded Portuguese strings.
- Replace Portuguese shape identifiers with an `import::Shape` enum; code identifiers and source strings are English.
- Translate the installer to English; rename calibration templates to `engraving-calibration.lcc` and `cutting-calibration.lcc`; the simulator now reports `[VER:1.1h:SIMULATOR]`.
- Add the LaserCat logo to the README, the window/taskbar icon, the installer, and the Start menu shortcuts; refresh screenshots in `docs/screenshots/`.
- Set the repository URL to https://github.com/DevJHenrique/LaserCat.
- Enable automatic recovery copies for normal sessions and remove the copy after a clean save, so recovery is no longer offered at every startup. Demo and test sessions never write recovery files.
- Drop embedded assets of deleted objects when saving, keeping `.lcc` files small; undo still restores deleted objects.
- Overlap warnings only consider objects with enabled operations.
- Order nested cut contours with a bounding-box prefilter to keep planning fast for drawings with many paths.
- Reject raster assets with zero pixel dimensions during project validation.
- Make `scripts/Test-Project.ps1` work when its output is redirected under Windows PowerShell 5.1.
- Add `tests/architecture.rs` to enforce module dependency rules.
- Add the agent toolkit (`.claude/` settings, review subagents, skills), `docs/AI_AGENTS.md`, a scoped-task issue template, Dependabot, a code of conduct, `.gitattributes`, `.editorconfig`, and a Portuguese README.

## 0.2.1 — 2026-10-18

- Adopt AGPL-3.0-only for this release; retain earlier MIT notices and permissions.
- Make English the default documentation and collaboration-template language.
- Reduce interface attribution to one short README sentence.
- Include corresponding-source archives and license notices in distribution.
- Expand verification to 75 tests, debug/release CI, failure logs, and a shared test command.
- Isolate demo/UI tests from the operator's local settings and recovery files.

## 0.2.0 — 2026-10-17

- Rename the application, Cargo package, distribution, and installer to LaserCat.
- Document module boundaries and split UI responsibilities.
- Separate protocol and simulator from the GRBL session; share resource limits without coupling storage to import.
- Document `.lcc` while preserving `.laser` v1 compatibility and legacy settings fallback.
- Add contribution instructions, AI instructions, ADRs, issue templates, and pull request templates.

## 0.1.5 — Laser Studio

- Add calibration matrices, a blue/gray palette, collapsible panels, and on-bed transform handles.
- Move numeric transforms into a floating editor.
- Define the user-origin work zero on connection without movement.

## 0.1.4 — Laser Studio

- Persist device profiles and add shared preparation/preview sidebars and a device dashboard.
