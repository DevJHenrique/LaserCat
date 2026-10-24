# Development

Start with [CONTRIBUTING.md](../CONTRIBUTING.md), [ARCHITECTURE.md](../ARCHITECTURE.md), and [licensing](LICENSING.md). Windows x64 is the current target; the Cargo package is `lasercat`.

## Environment

Install Rust through rustup, MSVC Build Tools, and Windows SDK. Preserve the pinned toolchain and lockfile. The UI requires OpenGL 3.3. The application is Rust; its installer bootstrap uses C#/.NET Framework.

```powershell
cargo run --locked -- --demo
.\scripts\Test-Project.ps1
.\scripts\Test-Project.ps1 -Profile release
cargo doc --locked --no-deps
```

Use `-Offline` for the test script or `--offline` for Cargo when dependencies are available. CI checks debug/release, stores logs, and packages only after verification. CI neither publishes releases nor connects hardware. No distribution-signing certificate is configured.

## Test structure

- Unit tests alongside domain, protocol, transforms, calibration, storage, and UI.
- `tests/workflow.rs`: import, planning, persistence, and simulated GRBL flows.
- `tests/compatibility.rs`: legacy projects and local-profile priority.
- `tests/storage_failures.rs`: malformed archives and data preservation.
- `tests/protocol_boundaries.rs`: fragmentation, buffer limits, and queue cancellation.
- `tests/planning_invariants.rs`: geometry, G-code, passes, and cancellation.
- `tests/fixtures/legacy-v1.laser`: frozen legacy fixture; do not regenerate it to hide regressions.

Test observable behavior and invariants. Hardware examples are not automatic physical tests. See [TESTING.md](TESTING.md).

## Visual review

```powershell
cargo run --locked --example render_preview -- target/prepare.png 1440 900 prepare
cargo run --locked --example render_preview -- target/preview.png 1440 900 preview
cargo run --locked --example render_preview -- target/device.png 1440 900 device
cargo run --locked --example render_preview -- target/transform.png 1024 680 transform
cargo run --locked --example render_preview -- target/calibration.png 1440 900 calibration
cargo run --locked --example render_preview -- target/collapsed.png 1024 680 collapsed
cargo run --locked --example render_preview -- target/prepare-pt.png 1440 900 prepare pt-BR
```

The example CPU-renders the actual egui UI. It checks layout, not a native desktop capture or physical behavior. An optional fifth argument selects the interface language. README screenshots live in `docs/screenshots/`; the logo lives in `docs/logo.png`, `docs/logo-dark.png`, and the window/installer icon in `assets/`.

## Calibration templates

```powershell
cargo run --locked --example generate_calibration -- calibration
```

The generator writes `.lcc` files without assuming physical machine dimensions. UI-generated matrices use the configured device and validate area. Inspect binary changes through the project loader/renderer rather than text-editing ZIP files.

## Windows release

1. Align Cargo metadata, installer version, README, changelog, and license notices.
2. Run both test profiles; inspect relevant UI changes and regenerate templates if needed.
3. Run `scripts/Build-Package.ps1`. It compiles the binary, vendors pinned dependencies into an isolated source staging directory, and creates matching binary/source distributions.
4. Inspect `dist/<version>/`: portable executable, portable ZIP, installer, and `LaserCat-<version>-source.zip`. Verify the source archive builds with its included `.cargo/config.toml`.
5. Publish binaries and matching source together only when a publishing request and destination are defined.

The source archive includes project source, tests, build/install scripts, documentation, license notices, lockfile, and vendored crates. Rust/MSVC/Windows SDK remain documented build prerequisites. Registry access is unnecessary when rebuilding the source archive with the installed toolchain and `--offline`.

Installer location: `%LOCALAPPDATA%\Programs\LaserCat`. Profiles/recovery live under `%LOCALAPPDATA%\LaserCat`. Legacy settings fallback does not remove a previous installation or shortcuts.
