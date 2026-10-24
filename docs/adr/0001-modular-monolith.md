# ADR 0001 — Modular monolith

- Status: accepted
- Date: 2026-10-17

## Context

The core already has testable domain, planning, and protocol behavior, but UI responsibilities were concentrated in one large file. Contributions should remain focused without requiring hardware or services.

## Decision

Keep one Cargo package with library and binary, following [ARCHITECTURE.md](../../ARCHITECTURE.md). Split UI actions, workspace, device, preview, dialogs, shell, and theme. Separate protocol/simulator from GRBL session code. Share resource budgets through a neutral module.

## Alternatives

One crate per layer adds configuration before another consumer exists. Switching toolkits abandons the Rust implementation. Services/plugins require contracts and distribution machinery without a present need.

## Consequences

Core public contracts stay stable. Module boundaries are reviewed/tested but do not isolate dependencies at Cargo level. Another frontend/consumer may justify extracting use cases and crates through a new ADR.
