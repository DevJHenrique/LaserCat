---
name: verify
description: Run LaserCat's full verification (fmt, Clippy with warnings denied, all tests, doctests) and report results faithfully. Use after any Rust change and before proposing a commit or pull request.
---

# Verify a LaserCat change

1. From the repository root run the debug profile:
   ```powershell
   .\scripts\Test-Project.ps1 -Offline
   ```
   Drop `-Offline` if dependencies are not cached yet.
2. For changes to execution (`src/grbl`), planning (`src/planner.rs`), persistence (`src/storage.rs`), or import, also run:
   ```powershell
   .\scripts\Test-Project.ps1 -Offline -Profile release
   ```
3. If a step fails, read `target/test-results/<profile>.log`, fix the cause, and rerun. Never weaken a test, add `#[ignore]`, or relax Clippy to get green.
4. Report: which profiles ran, pass/fail counts, and anything not run. Say explicitly that results come from the simulator and unit tests, not physical hardware.

Narrow loops while iterating are fine (`cargo test --locked --test workflow <name>`), but finish with the full script.
