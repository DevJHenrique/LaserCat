// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

impl LaserApp {
    pub(super) fn preview_panel(&mut self, ctx: &egui::Context) {
        let width = if ctx.screen_rect().width() < 1200. {
            265.
        } else {
            315.
        };
        egui::SidePanel::right("preview-information")
            .exact_width(width)
            .resizable(false)
            .frame(egui::Frame::NONE.fill(CANVAS).inner_margin(8.))
            .show(ctx, |ui| {
                ui.style_mut().visuals = egui::Visuals::dark();
                ui.style_mut().visuals.override_text_color = Some(Color32::from_gray(235));
                let card = || {
                    egui::Frame::new()
                        .fill(Color32::from_gray(65))
                        .corner_radius(7.)
                        .inner_margin(12.)
                };
                card().show(ui, |ui| {
                    ui.set_min_width(width - 40.);
                    ui.strong(tr("Job summary"));
                    ui.separator();
                    if let Some(plan) = &self.plan {
                        ui.label(trf!(
                            "{objects} objects · {toolpaths} toolpaths",
                            objects = self.project.objects.len(),
                            toolpaths = plan.strokes.len()
                        ));
                        ui.label(trf!(
                            "Area: {width} × {height} mm",
                            width = format!("{:.1}", plan.bounds.max.x - plan.bounds.min.x),
                            height = format!("{:.1}", plan.bounds.max.y - plan.bounds.min.y)
                        ));
                        ui.add_space(6.);
                        ui.strong(tr("Time estimate"));
                        ui.heading(format_time(plan.estimated_seconds));
                        ui.label(trf!(
                            "Path length: {length} m",
                            length = format!("{:.2}", plan.distance_mm / 1000.)
                        ));
                        ui.small(tr("Approximate estimate; depends on the machine."));
                        if ui
                            .add_enabled(!self.locked(), egui::Button::new(tr("Export G-code…")))
                            .clicked()
                        {
                            self.export_smax = self.snapshot.smax.unwrap_or(1000.);
                            self.export_open = true;
                        }
                        ui.checkbox(&mut self.show_rapid, tr("Show laser-off travel"));
                        ui.horizontal(|ui| {
                            ui.colored_label(ACCENT, tr("■ Engraving"));
                            ui.colored_label(ORANGE, tr("■ Cutting"));
                        });
                        egui::ScrollArea::vertical()
                            .id_salt("preview-warnings")
                            .max_height(65.)
                            .show(ui, |ui| {
                                for warning in &plan.warnings {
                                    ui.colored_label(ORANGE, warning);
                                }
                            });
                    } else {
                        ui.label(tr("Generate the toolpaths after editing the project."));
                        if ui
                            .add_enabled(
                                !self.locked() && !self.project.objects.is_empty(),
                                egui::Button::new(tr("Generate preview")),
                            )
                            .clicked()
                        {
                            self.plan(ctx);
                        }
                    }
                });
                ui.add_space(8.);
                card().show(ui, |ui| {
                    ui.set_min_width(width - 40.);
                    ui.strong("G-code");
                    ui.small(trf!(
                        "{lines} lines · $30={smax}",
                        lines = self.preview_gcode.len(),
                        smax = format!("{:.0}", self.preview_smax)
                    ));
                    ui.separator();
                    let line_height = ui
                        .spacing()
                        .interact_size
                        .y
                        .max(ui.text_style_height(&egui::TextStyle::Monospace));
                    egui::ScrollArea::both()
                        .id_salt("gcode-lines")
                        .max_height((ui.available_height() - 18.).max(60.))
                        .show_rows(ui, line_height, self.preview_gcode.len(), |ui, range| {
                            for i in range {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{:05}", i + 1))
                                            .monospace()
                                            .color(ORANGE),
                                    );
                                    let code = &self.preview_gcode[i];
                                    ui.label(egui::RichText::new(code).monospace().color(
                                        if code.starts_with(';') {
                                            Color32::GRAY
                                        } else if code.starts_with('M') {
                                            Color32::from_rgb(210, 211, 95)
                                        } else {
                                            Color32::from_gray(240)
                                        },
                                    ));
                                });
                            }
                        });
                });
            });
    }
}
