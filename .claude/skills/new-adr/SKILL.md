---
name: new-adr
description: Draft a new Architecture Decision Record in docs/adr for LaserCat. Use when a change adds a dependency, file-format version, protocol/controller, crate split, application layer, or otherwise alters ARCHITECTURE.md.
---

# Write an ADR

1. List `docs/adr/` and take the next sequential number (four digits).
2. Create `docs/adr/NNNN-short-title.md` with this structure, in English:
   ```markdown
   # ADR NNNN — Title

   - Status: proposed
   - Date: YYYY-MM-DD
   - Supersedes / relates to: (links, if any)

   ## Context
   The concrete problem and the consumer that needs the change now.

   ## Decision
   What changes, in which module, and which contracts move.

   ## Alternatives
   Options considered and why they were rejected.

   ## Consequences
   Compatibility, testing, documentation, licensing, and migration impact.
   ```
3. Add the ADR to the list in `docs/adr/README.md`.
4. If the decision replaces an earlier one, update the older ADR's status line to point at the new one; never delete history.
5. Update `ARCHITECTURE.md` only for what the decision actually changes. Status stays `proposed` until a maintainer accepts it.
