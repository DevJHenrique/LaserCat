# ADR 0004 — AGPL-3.0 and English documentation

- Status: accepted; the Portuguese-only UI statement is superseded by [ADR 0005](0005-internationalization.md)
- Date: 2026-10-18
- Supersedes the previous project license choice and the expanded attribution scope in ADR 0003

## Context

The project owner requested AGPL version 3, English default documentation, and a short interface-inspiration acknowledgement without repeated upstream discussion.

## Decision

Use **AGPL-3.0-only** for release 0.2.1 and new original contributions. Retain the complete license text, project notices, legacy MIT notice, and dependency licenses. Include matching corresponding source and pinned vendored dependencies with binary packages.

Use English for default documentation, agent instructions, and issue/PR templates. The application's current Portuguese UI is unchanged. Keep the single interface-inspiration sentence in README.

## Alternatives

MIT permits closed derivatives subject to notice preservation. An `-or-later` grant would additionally authorize future license versions; version 3 only makes the requested scope explicit.

## Consequences

Redistribution and covered modified network use must follow AGPL requirements. Existing MIT grants are not revoked. Independently created user artwork/output is not automatically covered by the application license. Release tooling supplies the matching source archive, notices, and build instructions.
