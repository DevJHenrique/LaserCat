# ADR 0002 — `.lcc` projects

- Status: accepted
- Date: 2026-10-17

## Context

An editable project must retain objects, operations, units, transforms, and embedded resources. SVG and G-code do not capture all this data; application-specific project formats do not establish universal interoperability.

## Decision

Use the requested `.lcc` extension for the existing ZIP/JSON container. Publish its specification/schema. Preserve manifest v1 and `.laser` v1 reading through a frozen fixture. Desktop saves use `.lcc` without deleting legacy projects.

## Alternatives

SVG loses operation/state information; G-code loses editing. Labeling the current ZIP `.lbrn2` or `.3mf` would claim unsupported compatibility. Interoperability requires a dedicated adapter and tests.

## Consequences

This is an application-specific extension with no claim of registration/exclusivity. Compatibility depends on content/version. Incompatible changes require versioning, specifications, and explicit migration.
