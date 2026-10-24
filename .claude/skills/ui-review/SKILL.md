---
name: ui-review
description: Render LaserCat's real egui UI to PNG at the supported window sizes and inspect the images. Use for any change under src/ui/ that affects layout, panels, dialogs, or the canvas.
---

# Review UI layout with the renderer

The `render_preview` example CPU-renders the actual UI. It is a layout check, not a native-window capture.

1. Render every affected view at both supported sizes:
   ```powershell
   foreach ($view in 'prepare','preview','device','transform','calibration','collapsed') {
     cargo run --locked --example render_preview -- "target/ui-review/$view-1440.png" 1440 900 $view
     cargo run --locked --example render_preview -- "target/ui-review/$view-1024.png" 1024 680 $view
   }
   ```
   Restrict the list to the views your change touches when that is obvious. Add `pt-BR` as a fifth argument to render the Portuguese interface; translated labels are often longer, so check it whenever text changes.
2. Open each PNG with the Read tool and check: clipped or overlapping text, controls pushed off-screen at 1024×680, unreadable contrast, empty panels, and Portuguese labels still correct.
3. If a new view needs coverage, add it to `LaserApp::demo_with_view` in `src/ui/mod.rs` and to `examples/render_preview.rs`.
4. Report what you inspected and what you could not (native window chrome, DPI scaling, real fonts on other machines). Only update the versioned images in `docs/screenshots/` when the README screenshots are meant to change.
