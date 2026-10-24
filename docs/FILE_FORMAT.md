# LaserCat project format (`.lcc`)

This specification describes implemented **manifest v1**, used by LaserCat 0.2.1. Application and manifest versions are independent. Reference implementation: [storage](../src/storage.rs), [domain](../src/domain.rs), and the auxiliary [JSON Schema](schemas/lcc-v1.schema.json).

## Container

Generated files are ZIP archives using Deflate compression. Preferred extension: `.lcc`. The project has no registered custom MIME type; the container is ZIP.

```text
project.lcc
  manifest.json
  assets/1
  assets/3
  ...
```

`manifest.json` is UTF-8 JSON. Each manifest asset requires an `assets/<id>` entry containing normalized SVG or image bytes. Reopening requires no external artwork files. The loader reads known entries directly rather than extracting archive paths to the filesystem.

## Manifest

| Field | Type | Meaning |
| --- | --- | --- |
| `version` | integer | Must be `1`; unknown versions are rejected |
| `machine` | object | Embedded profile, without live connection/reference |
| `assets` | array | Immutable resources, geometry, dimensions, warnings |
| `objects` | array | Objects, transforms, locks, operations |
| `next_id` | u64 integer | Greater than every allocated ID and less than u64::MAX |

IDs are unique across assets/objects. `asset_id` references an existing asset. Asset `bytes` is omitted from JSON and stored in ZIP entries. Numbers must be finite. All v1 fields are written. Current serde parsing ignores unknown fields, but an unknown format version remains an error.

### Machine profile

Fields: `name`, `width`, `height`, `homing`, `positioning`, `invert_x`, `invert_y`, `port`, `baud`, `configured`. Positioning is `UserOrigin` or `Absolute`.

Projects may have an unconfigured machine. Planning/movement require valid bed dimensions and the current session's reference rules. The configured local profile takes priority when opening a project in the desktop application; an embedded profile never connects the machine automatically.

### Assets and geometry

Asset fields: `id`, `name`, `width`, `height`, `geometry`, `warnings`. Physical dimensions use millimeters. Geometry uses an externally tagged serde enum:

```json
{"Vector":{"paths":[],"vector_safe":true}}
```

Each path contains `points` (`x`,`y`), `closed`, `fill`, `fill_group`, and `even_odd`. Raster geometry is `{"Raster":{"pixel_width":100,"pixel_height":100}}`. Do not mark unsupported SVG geometry safe merely to enable cutting. Imported text is converted to embedded outlines.

### Objects

Fields: `id`, `asset_id`, `name`, `x`, `y`, `width`, `height`, `rotation`, `mirror_x`, `mirror_y`, `lock_ratio`, `locked`, `operations`.

X points right, Y points up. Rotation is in degrees around the center; X/Y refer to the rectangle before rotation. Width/height are positive. Array order is visual; operation settings and planning determine execution order.

### Operations

Fields: `kind`, `enabled`, `power`, `speed`, `passes`, `spacing`, `overscan`, `brightness`, `contrast`, `inverted`, `dither`, `constant_power`, `order`.

Kinds: `Cut`, `Line`, `Fill`, `Raster`. Power: 0–100%; speed: >0 up to 100,000 mm/min; passes: 1–100; spacing: 0.05–10 mm; overscan: 0–20 mm; brightness: -1–1; contrast: 0–4. `constant_power` selects M3 instead of M4. The planner validates operation parameters and trajectory bounds.

## Limits and writes

- Manifest: at most 64 MiB.
- Each asset: at most 64 MiB.
- Manifest plus all asset bytes: at most 512 MiB.
- Processing budgets: 40 million pixels and one million segments.

Saving validates the project, writes a same-directory temporary file, synchronizes, and atomically replaces the destination. Rust validation is authoritative for identifiers, references, limits, and semantics. JSON Schema checks structure; it does not replace these rules.

## Compatibility and migration

Legacy `.laser` v1 uses the same container and remains readable. The desktop offers `.lcc` and legacy `.laser` filters. Saving a legacy project uses `.lcc` and preserves the original; the low-level `storage::save` API honors its exact input path.

Renaming a file extension does not convert other formats. `.lbrn2`, `.3mf`, and G-code are not LaserCat projects. Frozen compatibility fixture: [legacy-v1.laser](../tests/fixtures/legacy-v1.laser).

Incompatible changes require a version increment, explicit reader/migration strategy where feasible, and real previous-version fixtures. Do not regenerate old fixtures with new code to hide a regression.

## Data outside the project

Local `device.json` uses `{ "version": 1, "machine": { ... } }` and atomic writes. Recovery is a separate `.lcc` project. Open ports, observed position, completed homing, physical reference, transmission queues, and generated plans are not restored from project files. Execution requires a currently validated session.
