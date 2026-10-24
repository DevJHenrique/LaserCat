// SPDX-License-Identifier: AGPL-3.0-only
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Window and taskbar icon, embedded so the executable stays self-contained.
fn icon() -> Option<eframe::egui::IconData> {
    let image = image::load_from_memory(include_bytes!("../assets/icon.png"))
        .ok()?
        .into_rgba8();
    Some(eframe::egui::IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}

fn main() -> eframe::Result {
    let argument = std::env::args_os().nth(1);
    let title = format!("LaserCat {}", env!("CARGO_PKG_VERSION"));
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([1024.0, 680.0]);
    if let Some(icon) = icon() {
        viewport = viewport.with_icon(icon);
    }
    eframe::run_native(
        &title,
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(move |cc| {
            let mut app = if argument.as_deref() == Some(std::ffi::OsStr::new("--demo")) {
                lasercat::ui::LaserApp::demo(&cc.egui_ctx)
            } else {
                lasercat::ui::LaserApp::new(&cc.egui_ctx)
            };
            if let Some(path) = argument.filter(|a| a != "--demo") {
                app.open_initial(path.into(), &cc.egui_ctx);
            }
            Ok(Box::new(app))
        }),
    )
}
