# Testing strategy

Tests detect regressions before integration and exercise processing/execution without requiring hardware. Passing tests do not establish universal physical compatibility.

## Local verification

From the repository root:

```powershell
.\scripts\Test-Project.ps1
.\scripts\Test-Project.ps1 -Profile release
```

Add `-Offline` when dependencies are available. The script uses the pinned toolchain/lockfile, checks formatting, denies Clippy warnings, runs all test targets, and runs doctests separately. A failed step stops verification with an error. Logs are written to `target/test-results/debug.log` and `release.log`.

Release is tested as well as debug so optimizations and disabled debug checks cannot silently alter behavior. Examples are compiled as test harnesses; their serial-query programs are not executed by the test runner.

## Risk-based coverage

| Area | Protected behavior | Location |
| --- | --- | --- |
| Import | Units, curves, fonts, unsupported SVG, holes, transparency, rotation | `tests/workflow.rs` |
| Transforms | Inverse mapping across scales/rotations/mirrors, fixed opposite corner, gestures, undo | `tests/planning_invariants.rs`, `src/ui/transform.rs`, `src/ui/mod.rs` |
| Planning | Independent parameters, disabled operations, passes, ordering, margins, bounds | `tests/workflow.rs`, `tests/planning_invariants.rs` |
| G-code | M3/M4, power scaling, axis orientations, laser-off travel and framing | `tests/planning_invariants.rs`, `tests/workflow.rs` |
| Archives | Corrupt ZIP/JSON, missing manifest/assets, invalid references/versions, rejected-save preservation | `tests/storage_failures.rs` |
| Compatibility | Legacy v1 to `.lcc`, original preservation, profile priority/fallback | `tests/compatibility.rs` |
| Protocol | Every split boundary, oversized responses, variable-length lines, FIFO, buffer limits | `tests/protocol_boundaries.rs`, `src/grbl/mod.rs` |
| Execution | Identification, zero without movement, alarms, pause/resume, cancellation, disconnect, later Idle confirmation | `src/grbl/mod.rs`, `tests/workflow.rs` |
| UI | Async planning, stale results, preview clearing, selection, return to editing | `src/ui/mod.rs` |
| Calibration | Per-cell parameters, usable area, template persistence | `src/calibration.rs` |
| Recovery | Autosave only outside demo/tests, clean save discards the copy, saved archives drop deleted assets | `src/ui/mod.rs`, `src/domain.rs` |
| Architecture | Module dependency table, no GUI crates or serial access in core modules | `tests/architecture.rs` |
| Translations | Every used key translated, no stale entries, matching placeholders, no hard-coded Portuguese, preferences default to English | `tests/i18n.rs`, `src/i18n.rs`, `src/storage.rs` |

Geometry and fragmentation variations are deterministic. Temporary files live under `target/`; demo/UI tests do not load the operator's personal profile or recovery files.

## Contribution requirements

1. Bug fixes include an automated reproduction that fails without the fix when feasible.
2. New behavior includes the main path and relevant error/boundary cases.
3. Format changes retain old fixtures, test migration, and update the specification.
4. Execution changes preserve worker-side interlocks; a disabled button does not replace protocol validation.
5. Visual changes require flow/layout review at supported window sizes. The renderer is not a native-window capture.
6. Test operator-visible outcomes and meaningful invariants rather than increasing counts or mirroring implementation.

Line-coverage measurement is not configured. Test counts are not coverage percentages. The matrix above records protected behaviors; identify gaps through concrete cases.

## CI and merge protection

`.github/workflows/windows.yml` runs on pushes, pull requests, and manual dispatch:

- **verify** runs both profiles and uploads logs even after failure.
- **package** depends on successful verification before building distribution.

To prevent merging failing PRs, the maintainer must require the **verify** status in the hosted repository's branch protection/ruleset, with checks current for the PR commit. A workflow file cannot configure that policy by itself; confirm the setting on the hosted repository.

## Physical equipment

Automated tests use the simulator and do not move or emit laser light on physical hardware. Operator-authorized physical validation must check reference, limits, and laser-off framing. Record controller, firmware, conditions, and results separately; do not present simulation as physical validation.
