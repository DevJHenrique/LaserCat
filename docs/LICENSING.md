# Licensing

LaserCat 0.2.2 is licensed under **GNU Affero General Public License, version 3 only**, using the SPDX identifier **`AGPL-3.0-only`**. The complete, unmodified license text is in [LICENSE](../LICENSE). Project copyright and distribution notices are in [NOTICE](../NOTICE).

## MIT and AGPL-3.0 at a glance

| Topic | MIT | AGPL-3.0 |
| --- | --- | --- |
| Commercial use, modification, and sale | Allowed | Allowed, subject to license terms |
| Distributing covered modified software | May be incorporated into proprietary products with notices preserved | Covered work must remain under the license, with corresponding source available as required |
| Modified software used for remote network interaction | No source-sharing requirement | Users interacting remotely must be offered the corresponding source of the modified version under section 13 |
| Private local changes | No source-publication requirement | Private local changes alone do not require publishing source; distribution/network conditions still matter |
| Notices | Preserve copyright and permission notice | Preserve applicable notices, license, modification notices, and corresponding-source obligations |

This is a summary; the license text governs. See the [MIT license](https://opensource.org/license/mit) and [AGPL-3.0 sections 5, 6, and 13](https://opensource.org/license/agpl-3.0).

## Existing versions and dependencies

Earlier versions were distributed under MIT. Those existing grants are not revoked by this change. Their notice remains in [LICENSES/MIT-legacy.txt](../LICENSES/MIT-legacy.txt). The current release and new original contributions use AGPL-3.0-only.

Dependencies retain their licenses and notices. Do not relabel a dependency as AGPL or remove its original notice. The release includes `THIRD_PARTY_NOTICES.txt` and vendored dependency source. Reused code must have compatible licensing and its attribution preserved.

## Corresponding source

Each release package includes `LaserCat-<version>-source.zip`, containing the application source, exact lockfile, tests, build/installation scripts, documentation, and vendored Cargo dependencies. Its `.cargo/config.toml` redirects crates.io to `vendor/`. Build prerequisites such as Rust, MSVC, and Windows SDK are documented separately.

The source archive is supplied alongside the executable, included in the portable ZIP, and embedded in the installer. Anyone publishing a binary release must make the matching source available from the same release/download location at no additional charge and preserve the required notices. A generic link to a moving development branch is not a substitute for the matching source.

## User projects and output

Using LaserCat does not automatically license independently created artwork, `.lcc` projects, or generated G-code under AGPL. The license applies to covered program code; output is covered only when its content itself constitutes a covered work, as described in section 2.

## Contributions

Original contributions are accepted under AGPL-3.0-only. Contributors must have the right to submit their work, review AI-assisted changes, and preserve third-party rights. No additional CLA is configured.
