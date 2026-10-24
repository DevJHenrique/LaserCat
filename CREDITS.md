# Credits

LaserCat is built with the work of the Rust community and these projects:

- [egui/eframe](https://github.com/emilk/egui): desktop UI and interactive rendering.
- [resvg/usvg](https://github.com/linebender/resvg): SVG parsing, normalization, and rendering.
- [image](https://github.com/image-rs/image): raster image processing.
- [serialport-rs](https://github.com/serialport/serialport-rs): serial transport.
- [GRBL](https://github.com/gnea/grbl): controller firmware and documentation of the [1.1 interface](https://github.com/gnea/grbl/wiki/Grbl-v1.1-Interface) and [laser mode](https://github.com/gnea/grbl/wiki/Grbl-v1.1-Laser-Mode).
- Maintainers of the other dependencies pinned in `Cargo.lock`.

The LaserCat logo and icon (`docs/logo*.png`, `assets/icon.*`) were provided by the project maintainer for this project.

Third-party components retain their own licenses. `scripts/Generate-Notices.ps1` collects resolved dependency metadata and license texts for distribution. Source packages also include the exact vendored dependencies.

Copyright (c) 2026 LaserCat contributors. The current project license is [AGPL-3.0-only](LICENSE). Earlier MIT notices are retained in [LICENSES/MIT-legacy.txt](LICENSES/MIT-legacy.txt). See [NOTICE](NOTICE) and [licensing](docs/LICENSING.md).
