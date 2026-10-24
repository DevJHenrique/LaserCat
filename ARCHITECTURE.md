# LaserCat architecture

Status: **adopted**, first implemented in 0.2.0. Supporting decisions are recorded in [docs/adr](docs/adr/README.md).

## Structure

LaserCat is a **layered modular monolith**, with a library and a desktop executable in one Cargo package. Responsibility boundaries exist in code without requiring services, plugins, or one crate per file. Contributors can exercise the core without hardware.

```text
src/
  main.rs                 desktop startup and arguments
  lib.rs                  module API
  domain.rs               projects, profiles, assets, objects, operations
  limits.rs               shared resource budgets
  i18n.rs                 interface language and translation lookup
  import.rs               SVG, fonts, raster, normalization
  calibration.rs          calibration project generation
  planner.rs              JobPlan, bounds, ordering, G-code
  storage.rs              project archives, profiles, recovery
  grbl/
    mod.rs                session, serial transport, state machine
    protocol.rs           Decoder and Stream; no I/O
    simulator.rs          simulated transport using the same protocol
  ui/
    mod.rs                application state, render loop, composition
    actions.rs            desktop use cases and background tasks
    workspace.rs          bed, selection, operation editing
    transform.rs          transform-handle geometry
    preview.rs            summary and G-code presentation
    device.rs             machine controls and monitoring
    dialogs.rs            configuration, calibration, project dialogs
    shell.rs              menus, navigation, status, shortcuts
    theme.rs              styling and shared components
    icons.rs              original vector icons
```

## Allowed dependencies

| Module | May depend on | Must remain independent of |
| --- | --- | --- |
| `domain` | std, serde, and i18n | UI, filesystem, serial, planner |
| `limits` | constants and std | other application modules |
| `i18n` | std, serde, embedded `locales/*.json` | other application modules (every layer may use it) |
| `import` | domain, limits, image/SVG/font libraries | UI, GRBL, project persistence |
| `calibration` | domain and import | UI, serial, storage |
| `planner` | domain and raster import | UI, project files, transport |
| `storage` | domain, limits, JSON, ZIP, filesystem | UI, planner, import, serial |
| `grbl` | domain, JobPlan, protocol, transport | UI, storage, artwork formats |
| `ui` | modules above | domain layers must never depend on it |

The UI is the composition root. `ui/actions.rs` currently coordinates workers and uses `egui::Context` to request repaint; domain, planning, and protocol code receive no graphics context. A second frontend or headless consumer would justify extracting use cases into an `application` layer with neutral events, through a new ADR. That extraction is not an already-implemented layer.

Avoid dependency cycles, widgets in domain code, and direct serial access from drawing code. New dependencies require a concrete need and a license review.

`tests/architecture.rs` checks this table mechanically on every test run (production code only; `#[cfg(test)]` modules may use other layers as fixtures). When a decision changes the table, update the test, this document, and an ADR together.

## Core contracts

- **`MachineProfile`:** declared configuration. Bed dimensions are not discovered or written to firmware automatically. Local preferences and physical reference have separate lifecycles.
- **`Project`:** editable scene with embedded assets. Immutable resources are shared through `Arc`; history does not duplicate every asset byte.
- **`SceneObject` / `Operation`:** editing and transforms are separate from processing. Visual order does not determine execution; overlaps remain independent operations.
- **`JobPlan`:** a plan built from an immutable snapshot. Preview, export, and execution share it. Scene changes invalidate it; stale-revision background results are discarded.
- **`MachineSession`:** `Command`/`Event` facade with a dedicated worker. `Snapshot` observes the controller; it is not persisted configuration.
- **`Decoder` / `Stream`:** response fragmentation and transmit-window accounting without I/O. Public reexports remain `grbl::{Decoder, Stream}`.

## Coordinates

The editor uses millimeters, X right, Y up. Imported artwork is normalized. Objects rotate around their center. Mirroring and axis inversion do not change electrical firmware settings.

`SceneObject::transform` and `inverse` are the reference mappings for editing and planning. Resizing a rotated object keeps the opposite corner fixed; uniform scaling is an object option. New tools must preserve this relationship and test rotated/mirrored cases as relevant.

## Concurrency and execution

The graphics thread coordinates tasks and presents events. Heavy import, planning, calibration, saves, and communication run in workers. Processing cancellation is cooperative; decoding one asset may finish before checking cancellation.

The GRBL worker owns transport and queue. Components never send serial bytes concurrently. The project is frozen during execution/pause. Cancellation clears the queue, waits for the state transition, and invalidates reference. Faults and disconnections never resume a job automatically.

Accepted commands are not completed physical motion. Completion requires all acknowledgements and a subsequent `Idle` query. Worker-side validation remains mandatory even when a UI button is disabled.

## Persistence

The [`.lcc` format](docs/FILE_FORMAT.md) is versioned. Opening a project neither connects nor moves hardware. Legacy `.laser` v1 remains readable. Desktop saves normalize the extension; the low-level storage API honors the exact caller-supplied path.

Local profiles, recovery, and projects are distinct stores. A configured local profile overrides the profile in an opened project and requires replanning. Legacy `LaserStudio` files are fallbacks when new files are absent. An invalid new file reports an error rather than silently hiding it with fallback.

Format changes require a specification update, legacy fixture, read tests, and migration strategy. Renaming the application or extension alone does not bump manifest version.

## Evolution rules

1. Implement a rule in its owning module. UI code does not calculate toolpaths or assemble emission instructions.
2. Extract modules when responsibilities, state, and tests become hard to understand, rather than to satisfy an arbitrary line count.
3. Extract crates when there is a stable reusable API, a dependency-isolation requirement, or another actual consumer.
4. Another controller protocol should introduce a session/capability facade instead of manufacturer conditions in the canvas.
5. Another frontend justifies extracting the use cases from `ui/actions.rs`; core data, planning, and persistence already run without a window.

## Verification

Use domain/planner tests for geometry, fixtures for import/archive compatibility, the simulator for protocol behavior, and egui interaction tests for editing gestures. Use the actual UI renderer for layout review. Execution changes cover fragmentation, buffers, alarms, pause/cancel, disconnects, and physical-completion barriers as appropriate.

Windows CI runs formatting, Clippy, debug/release tests, then packaging. See [CONTRIBUTING.md](CONTRIBUTING.md) and [testing](docs/TESTING.md).
