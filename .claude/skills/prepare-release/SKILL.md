---
name: prepare-release
description: Prepare a LaserCat release locally - align versions, changelog, templates, verification, and packaging. Use only when a maintainer asks to prepare a release; publishing is a separate explicit request.
---

# Prepare a release (local only)

Publishing binaries, tags, or GitHub releases requires a separate, explicit request naming the destination. This skill stops at local artifacts.

1. Ask for the target version if it was not given. Update it consistently in:
   - `Cargo.toml` (`version`), then run `cargo build` once (without `--locked`) so only the `lasercat` entry in `Cargo.lock` changes; check `git diff Cargo.lock`.
   - `packaging/Installer.cs` version strings.
   - `README.md` / `README.pt-BR.md` status line.
2. Move `CHANGELOG.md` *Unreleased* entries under `## <version> — <date>`.
3. If calibration generation changed, regenerate templates:
   ```powershell
   cargo run --locked --example generate_calibration -- calibration
   ```
4. Run both verification profiles (see the `verify` skill) and the `ui-review` skill if the UI changed.
5. Build packages:
   ```powershell
   .\scripts\Build-Package.ps1
   ```
6. Inspect `dist/<version>/`: portable executable, portable ZIP, installer, and `LaserCat-<version>-source.zip` with license notices. Confirm the source archive builds offline with its bundled `.cargo/config.toml`.
7. Report what was produced and verified. Do not commit `dist/` or `target/`.
