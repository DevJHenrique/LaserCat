// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

impl LaserApp {
    pub(super) fn editing_dialogs(&mut self, ctx: &egui::Context) {
        if self.transform_open && self.mode == Mode::Prepare {
            let mut open = true;
            egui::Window::new(tr("Transform selection")).open(&mut open).collapsible(false).resizable(false)
                .default_pos(Pos2::new((ctx.screen_rect().width()/2.-150.).max(320.),120.)).default_width(330.)
                .show(ctx,|ui| {
                    ui.label(tr("Bed coordinates · millimeters"));
                    let old=self.position_settings;self.position_settings=true;
                    self.properties(ui);self.position_settings=old;
                    ui.separator();
                    ui.small(tr("Drag the corners to resize and the round handle to rotate. Shift toggles the aspect ratio or snaps rotation to 15°."));
                    if ui.button(tr("Done")).clicked() {self.transform_open=false;}
                });
            if !open {
                self.transform_open = false;
            }
        }
        if self.calibration_open {
            let mut open = true;
            let mut save = false;
            egui::Window::new(tr("Calibration · power and speed")).open(&mut open).collapsible(false).resizable(false).default_width(420.).show(ctx,|ui| {
                ui.label(tr("Template file with independent samples and labels."));
                ui.horizontal(|ui| {ui.radio_value(&mut self.calibration.cut,false,tr("Fill engraving"));ui.radio_value(&mut self.calibration.cut,true,tr("Vector cutting"));});
                field(ui,tr("Start power"),&mut self.calibration.min_power,0. ..=100.," %");
                field(ui,tr("End power"),&mut self.calibration.max_power,0. ..=100.," %");
                field(ui,tr("Start speed"),&mut self.calibration.min_speed,1. ..=100000.," mm/min");
                field(ui,tr("End speed"),&mut self.calibration.max_speed,1. ..=100000.," mm/min");
                ui.horizontal(|ui| {ui.label(tr("Rows and columns"));ui.add(egui::DragValue::new(&mut self.calibration.steps).range(2..=10));});
                field(ui,tr("Sample size"),&mut self.calibration.cell,2. ..=30.," mm");
                ui.horizontal(|ui| {ui.label(tr("Passes"));ui.add(egui::DragValue::new(&mut self.calibration.passes).range(1..=100));});
                let size=25.+self.calibration.steps as f64*(self.calibration.cell+3.);
                ui.label(trf!("Required area: {size} × {size} mm", size = format!("{size:.0}")));
                ui.small(tr("Columns: power (%). Rows, bottom to top: speed (mm/min). Labels use the lowest power and highest speed in the matrix."));
                if !self.project.machine.configured {ui.colored_label(ORANGE,tr("Configure and apply the work area before creating the file."));}
                save=ui.add_enabled(!self.locked()&&self.project.machine.configured,pill(tr("Save calibration file…"),240.)).clicked();
            });
            if !open {
                self.calibration_open = false;
            }
            if save {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter(tr("LaserCat project"), &[storage::PROJECT_EXTENSION])
                    .set_file_name(format!(
                        "{}.lcc",
                        if self.calibration.cut {
                            tr("cutting-calibration")
                        } else {
                            tr("engraving-calibration")
                        }
                    ))
                    .save_file()
                {
                    let path = storage::project_save_path(path);
                    let cancel = self.start_work(tr("Creating calibration file"));
                    let config = self.calibration.clone();
                    let machine = self.project.machine.clone();
                    let tx = self.tx.clone();
                    let ctx = ctx.clone();
                    std::thread::spawn(move || {
                        let result = config.build(machine).and_then(|project| {
                            if cancel.load(Ordering::Relaxed) {
                                return Err(tr("Calibration cancelled.").into());
                            }
                            storage::save(&path, &project).map(|_| path)
                        });
                        let _ = tx.send(WorkEvent::CalibrationSaved(result));
                        ctx.request_repaint();
                    });
                    self.calibration_open = false;
                    self.config_open = false;
                }
            }
        }
    }
    pub(super) fn dialogs(&mut self, ctx: &egui::Context) {
        self.editing_dialogs(ctx);
        if self.license_open {
            let mut open = true;
            egui::Window::new(tr("LaserCat · license and source code")).open(&mut open).default_width(620.).show(ctx,|ui| {
                ui.strong("Copyright (c) 2026 LaserCat contributors");
                ui.label("GNU Affero General Public License v3 only (AGPL-3.0-only)");
                ui.label(tr("No warranty. You may redistribute the program under the terms of this license."));
                ui.label(trf!("Corresponding source included in the package: LaserCat-{version}-source.zip", version = env!("CARGO_PKG_VERSION")));
                ui.separator();
                egui::ScrollArea::both().max_height(350.).show(ui,|ui| {ui.monospace(include_str!("../../LICENSE"));});
            });
            self.license_open = open;
        }
        if self.config_open {
            let locked = self.locked();
            let mut close = false;
            egui::Window::new(tr("Configure machine")).collapsible(false).resizable(false).default_width(440.).show(ctx,|ui|{
                ui.label(tr("Enter the real work area. These values do not change the firmware."));ui.add_space(10.);
                if ui.add_enabled(!locked,egui::Button::new(tr("Power / speed calibration matrix…"))).clicked() {self.calibration_open=true;}
                ui.add_enabled_ui(!locked,|ui|{
                    ui.text_edit_singleline(&mut self.draft.name);
                    field(ui,tr("Work width"),&mut self.draft.width,0.0..=10_000.," mm");field(ui,tr("Work height"),&mut self.draft.height,0.0..=10_000.," mm");
                    ui.checkbox(&mut self.draft.homing,tr("Machine has homing configured"));
                    ui.radio_value(&mut self.draft.positioning,Positioning::UserOrigin,tr("User-defined origin"));
                    ui.add_enabled_ui(self.draft.homing,|ui|{ui.radio_value(&mut self.draft.positioning,Positioning::Absolute,tr("Absolute bed referenced after homing"));});
                    ui.horizontal(|ui|{ui.checkbox(&mut self.draft.invert_x,tr("Invert X on the bed"));ui.checkbox(&mut self.draft.invert_y,tr("Invert Y on the bed"));});
                    ui.horizontal(|ui|{ui.label(tr("USB port"));ui.text_edit_singleline(&mut self.draft.port);});
                    ui.horizontal(|ui|{ui.label("Baud rate");ui.add(egui::DragValue::new(&mut self.draft.baud).range(9600..=921600));});
                    ui.colored_label(MUTED,tr("The G54 origin must match the usable lower-left corner. On an absolute bed, home first and then move to that corner before setting the zero."));
                    ui.add_space(10.);ui.horizontal(|ui|{
                        let mut candidate=self.draft.clone();candidate.configured=true;let valid=candidate.validate();
                        if ui.add_enabled(valid.is_ok(),pill(tr("Apply settings"),150.)).clicked(){
                            if self.project.machine!=candidate&&self.snapshot.state!=State::Disconnected{self.machine.send(Command::Disconnect);}
                            if self.persist_device {
                                match storage::save_device(&self.device_settings_path, &candidate) {
                                    Ok(()) => { self.saved_machine=candidate.clone(); self.project.machine=candidate; close=true; self.recovery_enabled=true; },
                                    Err(error) => self.log(trf!("Could not save the device: {error}", error = error)),
                                }
                            } else { self.project.machine=candidate; close=true; }
                        }
                        if ui.button(tr("Configure later and connect")).clicked(){close=true;}
                        if let Err(e)=valid{ui.colored_label(ORANGE,e);}
                    });
                });
                if locked{ui.label(tr("Wait for the current operation to change the settings."));if ui.button(tr("Close")).clicked(){close=true;}}
            });
            if close {
                self.config_open = false;
            }
        }
        if self.text_open {
            let mut action = None;
            egui::Window::new(tr("Add text"))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(tr("Arial font; the text will be converted to outlines."));
                    ui.text_edit_singleline(&mut self.text_input);
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !self.text_input.trim().is_empty() && !self.locked(),
                                egui::Button::new(tr("Add")),
                            )
                            .clicked()
                        {
                            action = Some(true);
                        }
                        if ui.button(tr("Cancel")).clicked() {
                            action = Some(false);
                        }
                    });
                });
            if let Some(add) = action {
                self.text_open = false;
                if add {
                    self.add_shape(Shape::Text, ctx);
                }
            }
        }
        if self.export_open {
            let mut action = None;
            egui::Window::new(tr("Export G-code"))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(tr(
                        "Confirm the $30 scale of the machine that will run the file.",
                    ));
                    field(
                        ui,
                        tr("Maximum power ($30)"),
                        &mut self.export_smax,
                        1.0..=100_000.,
                        "",
                    );
                    ui.label(tr(
                        "Requires laser mode $32=1, $31=0, and a G54 origin set on the machine.",
                    ));
                    ui.horizontal(|ui| {
                        if ui.button(tr("Choose file…")).clicked() {
                            action = Some(true);
                        }
                        if ui.button(tr("Cancel")).clicked() {
                            action = Some(false);
                        }
                    });
                });
            if let Some(export) = action {
                self.export_open = false;
                if export {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("G-code", &["gcode", "nc"])
                        .set_file_name("trabalho.gcode")
                        .save_file()
                    {
                        if let Some(plan) = self.plan.clone() {
                            self.start_work("Exportando G-code");
                            let tx = self.tx.clone();
                            let ctx = ctx.clone();
                            let smax = self.export_smax;
                            std::thread::spawn(move || {
                                let result = plan.gcode(smax).and_then(|lines| {
                                    std::fs::write(path, lines.join("\n") + "\n")
                                        .map_err(|e| e.to_string())
                                });
                                let _ = tx.send(WorkEvent::Exported(result));
                                ctx.request_repaint();
                            });
                        }
                    }
                }
            }
        }
        if self.pending.is_some() {
            let mut action = 0;
            egui::Window::new(tr("Unsaved changes")).collapsible(false).resizable(false).show(ctx,|ui|{
                ui.label(tr("There are unsaved changes or an operation in progress. Closing stops sending to the machine."));
                ui.horizontal(|ui|{if ui.button(tr("Save and continue")).clicked(){action=1;}
if ui.button(tr("Continue without saving")).clicked(){action=2;}
if ui.button(tr("Cancel")).clicked(){action=3;}});
            });
            match action {
                1 => {
                    self.after_save = self.pending.take();
                    if !self.save(ctx, false) {
                        self.pending = self.after_save.take();
                    }
                }
                2 => {
                    let p = self.pending.take().unwrap();
                    self.replace(p, ctx);
                }
                3 => self.pending = None,
                _ => {}
            }
        }
        if self.recovery_available {
            let mut action = 0;
            egui::Window::new(tr("Recovery available"))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(tr("There is an automatic copy of a previous job."));
                    ui.horizontal(|ui| {
                        if ui.button(tr("Recover job")).clicked() {
                            action = 1;
                        }
                        if ui.button(tr("Ignore for this session")).clicked() {
                            action = 2;
                        }
                    });
                });
            if action > 0 {
                self.recovery_available = false;
                self.recovery_enabled = true;
                if action == 1 {
                    self.config_open = false;
                    self.request_replace(
                        Replace::Open(
                            storage::recovery_source().unwrap_or_else(storage::recovery_path),
                        ),
                        ctx,
                    );
                }
            }
        }
    }
}
