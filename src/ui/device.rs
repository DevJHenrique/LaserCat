// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

impl LaserApp {
    pub(super) fn device(&mut self, root: &mut egui::Ui) {
        let ctx = &root.ctx().clone();
        egui::Panel::right("device-controls")
            .exact_size(if ctx.content_rect().width() < 1200. {
                350.
            } else {
                400.
            })
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_gray(248))
                    .inner_margin(14.),
            )
            .show(root, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("device-controls-scroll")
                    .show(ui, |ui| {
                        section_header(ui, Icon::Device, tr("Connection and control"));
                        self.connection_controls(ui, ctx);
                        ui.add_space(14.);
                        ui.separator();
                        if ui
                            .add_enabled(
                                !self.locked(),
                                egui::Button::new(tr("Calibrate engraving / cutting…")),
                            )
                            .clicked()
                        {
                            self.calibration_open = true;
                        }
                        ui.strong(tr("Device settings"));
                        ui.add_space(6.);
                        ui.label(&self.project.machine.name);
                        ui.label(trf!(
                            "Work area: {width} × {height} mm",
                            width = format!("{:.1}", self.project.machine.width),
                            height = format!("{:.1}", self.project.machine.height)
                        ));
                        ui.label(format!(
                            "{} · {} baud",
                            if self.project.machine.port.is_empty() {
                                tr("Port not set")
                            } else {
                                &self.project.machine.port
                            },
                            self.project.machine.baud
                        ));
                        ui.label(match self.project.machine.positioning {
                            Positioning::UserOrigin => tr("Positioning: user origin"),
                            Positioning::Absolute => tr("Positioning: absolute bed"),
                        });
                        ui.label(trf!(
                            "Axis inversion: X {x} / Y {y}",
                            x = if self.project.machine.invert_x {
                                tr("yes")
                            } else {
                                tr("no")
                            },
                            y = if self.project.machine.invert_y {
                                tr("yes")
                            } else {
                                tr("no")
                            }
                        ));
                        if ui
                            .add_enabled(
                                !self.locked(),
                                egui::Button::new(tr("Edit and save device…")),
                            )
                            .clicked()
                        {
                            self.draft = self.project.machine.clone();
                            self.config_open = true;
                        }
                        ui.small(tr("Saved on this computer, independent of the project."));
                        ui.add_space(12.);
                        ui.strong(tr("Settings read from GRBL"));
                        ui.label(if self.snapshot.controller.is_empty() {
                            tr("Controller not identified")
                        } else {
                            &self.snapshot.controller
                        });
                        if self.snapshot.verified {
                            ui.label(trf!(
                                "Maximum power ($30): {value}",
                                value = format!("{:.0}", self.snapshot.smax.unwrap_or(0.))
                            ));
                            ui.label(trf!(
                                "Minimum power ($31): {value}",
                                value = format!("{:.0}", self.snapshot.smin.unwrap_or(0.))
                            ));
                            ui.label(trf!(
                                "Laser mode ($32): {value}",
                                value = if self.snapshot.laser {
                                    tr("enabled")
                                } else {
                                    tr("disabled")
                                }
                            ));
                            ui.label(format!(
                                "Homing ($22): {}",
                                match self.snapshot.homing_enabled {
                                    Some(true) => tr("enabled"),
                                    Some(false) => tr("disabled"),
                                    None => tr("not reported"),
                                }
                            ));
                        } else {
                            ui.colored_label(MUTED, tr("Connect to read the settings."));
                        }
                        ui.small(tr("Firmware parameters are shown for reference only."));
                    });
            });
        self.dialog_area = root.available_rect_before_wrap();
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(CANVAS).inner_margin(16.)).show(root,|ui| {
            egui::ScrollArea::vertical().id_salt("device-dashboard").show(ui,|ui| {
                let card=||egui::Frame::new().fill(Color32::WHITE).corner_radius(4.).inner_margin(16.);
                card().show(ui,|ui| {
                    ui.set_min_width((ui.available_width()-2.).max(0.));
                    ui.horizontal_wrapped(|ui| {
                        ui.heading(tr("Device status"));
                        ui.colored_label(if self.snapshot.state==State::Ready {ACCENT} else {ORANGE},format!("● {}",self.snapshot.state.label()));
                        if self.simulator {ui.colored_label(MUTED,tr("SIMULATOR"));}
                    });
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        if let Some(pos)=self.snapshot.position {ui.monospace(format!("X {:8.3}   Y {:8.3} mm",pos[0],pos[1]));}
                        ui.colored_label(if self.snapshot.referenced {ACCENT} else {ORANGE},if self.snapshot.referenced {tr("Origin set")} else {tr("Origin not set")});
                    });
                    let (r,_)=ui.allocate_exact_size(Vec2::new(ui.available_width(),(ctx.content_rect().height()*0.28).clamp(150.,270.)),Sense::hover());
                    self.paint_job_overview(ui.painter(),r);
                    self.job_toolbar(ui);
                });
                ui.add_space(12.);
                card().show(ui,|ui| {
                    ui.set_min_width((ui.available_width()-2.).max(0.));
                    ui.visuals_mut().extreme_bg_color=Color32::from_gray(232);
                    ui.strong(tr("Engraving / cutting progress"));ui.separator();
                    ui.label(self.path.as_ref().and_then(|p|p.file_stem()).and_then(|s|s.to_str()).unwrap_or(tr("Untitled")));
                    if let Some(job)=&self.job {
                        ui.colored_label(ACCENT,&job.status);
                        let fraction=if job.total>0 {job.accepted as f32/job.total as f32} else {0.};
                        ui.add(egui::ProgressBar::new(fraction).desired_width(ui.available_width()).text(trf!("{accepted} / {total} commands accepted by GRBL", accepted = job.accepted, total = job.total)));
                        let elapsed=job.finished.unwrap_or_else(Instant::now).duration_since(job.started).as_secs_f64();
                        ui.horizontal_wrapped(|ui| {ui.label(trf!("Elapsed: {time}", time = format_time(elapsed)));ui.label(trf!("Estimated time: {time}", time = format_time(job.estimate)));});
                        ui.small(tr("Sending commands does not measure physical execution. Completion requires an idle-state confirmation."));
                    } else {
                        ui.colored_label(MUTED,tr("No job started in this session."));
                        if let Some(plan)=&self.plan {ui.label(trf!("Job prepared · estimate {time}", time = format_time(plan.estimated_seconds)));}
                        ui.add(egui::ProgressBar::new(0.).desired_width(ui.available_width()).text(tr("Waiting for a job")));
                    }
                    ui.horizontal_wrapped(|ui| {
                        if ui.add_enabled(self.snapshot.state==State::Running,egui::Button::new(tr("Pause"))).clicked() {self.machine.send(Command::Pause);}
                        if ui.add_enabled(self.snapshot.state==State::Paused,egui::Button::new(tr("Resume"))).clicked() {self.machine.send(Command::Resume);}
                        if ui.add_enabled(matches!(self.snapshot.state,State::Running|State::Paused|State::Connecting|State::Alarm),egui::Button::new(egui::RichText::new(tr("Stop and edit")).color(Color32::WHITE)).fill(Color32::from_rgb(166,53,60))).clicked() {self.stop_and_edit();}
                        if ui.button(tr("Prepare")).clicked() {self.mode=Mode::Prepare;}
                    });
                    if !self.snapshot.message.is_empty() {ui.colored_label(MUTED,&self.snapshot.message);}
                });
                ui.add_space(12.);
                card().show(ui,|ui| {
                    ui.set_min_width((ui.available_width()-2.).max(0.));
                    ui.strong(tr("Device messages"));
                    egui::ScrollArea::vertical().id_salt("device-log").stick_to_bottom(true).max_height(150.).show(ui,|ui| {
                        for message in &self.logs {ui.label(egui::RichText::new(message).small().color(MUTED));}
                    });
                });
            });
        });
    }
    pub(super) fn paint_job_overview(&self, painter: &egui::Painter, rect: Rect) {
        let m = &self.project.machine;
        if !m.configured {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                tr("Configure the device work area"),
                egui::FontId::proportional(17.),
                MUTED,
            );
            return;
        }
        let scale = ((rect.width() - 60.) / m.width as f32)
            .min((rect.height() - 30.) / m.height as f32)
            .max(0.01);
        let bed = Rect::from_center_size(
            rect.center(),
            Vec2::new(m.width as f32 * scale, m.height as f32 * scale),
        );
        painter.rect_filled(bed, 3., Color32::from_rgb(57, 66, 78));
        let map = |p: Point| {
            Pos2::new(
                bed.left() + p.x as f32 * scale,
                bed.bottom() - p.y as f32 * scale,
            )
        };
        if let Some(plan) = &self.plan {
            for stroke in &plan.strokes {
                for pair in stroke.points.windows(2) {
                    painter.line_segment(
                        [map(pair[0]), map(pair[1])],
                        Stroke::new(
                            1.0_f32,
                            if stroke.kind == OperationKind::Cut {
                                ORANGE
                            } else {
                                ACCENT
                            },
                        ),
                    );
                }
            }
        } else {
            painter.text(
                bed.center(),
                egui::Align2::CENTER_CENTER,
                tr("No toolpaths prepared"),
                egui::FontId::proportional(14.),
                Color32::WHITE,
            );
        }
        if let Some(pos) = self.snapshot.position {
            let point = Point::new(
                if m.invert_x { m.width - pos[0] } else { pos[0] },
                if m.invert_y {
                    m.height - pos[1]
                } else {
                    pos[1]
                },
            );
            if point.x >= 0. && point.x <= m.width && point.y >= 0. && point.y <= m.height {
                painter.circle_stroke(map(point), 5., Stroke::new(2.0_f32, Color32::WHITE));
            }
        }
    }
    pub(super) fn connection_controls(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let disconnected = matches!(self.snapshot.state, State::Disconnected | State::Fault);
        ui.add_enabled_ui(disconnected, |ui| {
            ui.checkbox(&mut self.simulator, tr("GRBL simulator"));
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("port")
                    .width(100.)
                    .selected_text(if self.project.machine.port.is_empty() {
                        tr("COM port")
                    } else {
                        &self.project.machine.port
                    })
                    .show_ui(ui, |ui| {
                        for port in &self.ports {
                            ui.selectable_value(&mut self.project.machine.port, port.clone(), port);
                        }
                    });
                ui.add(
                    egui::TextEdit::singleline(&mut self.project.machine.port)
                        .hint_text("COM3")
                        .desired_width(65.),
                )
                .on_hover_text(tr("Enter the port manually if needed."));
            });
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("baud")
                    .width(110.)
                    .selected_text(format!("{} baud", self.project.machine.baud))
                    .show_ui(ui, |ui| {
                        for baud in [9600, 19200, 38400, 57600, 115200, 230400, 250000] {
                            ui.selectable_value(
                                &mut self.project.machine.baud,
                                baud,
                                format!("{baud} baud"),
                            );
                        }
                    });
                if ui
                    .add_enabled(
                        !self.ports_refreshing,
                        egui::Button::new(tr("Refresh ports")),
                    )
                    .clicked()
                {
                    self.refresh_ports(ctx);
                }
            });
        });
        ui.horizontal(|ui| {
            if ui
                .add_enabled(disconnected, egui::Button::new(tr("Connect")))
                .clicked()
            {
                self.connect();
            }
            if ui
                .add_enabled(!disconnected, egui::Button::new(tr("Disconnect")))
                .clicked()
            {
                self.machine.send(Command::Disconnect);
            }
        });
        if let Some(error) = &self.ports_error {
            ui.colored_label(ORANGE, error);
        } else if self.ports_refreshing {
            ui.colored_label(MUTED, tr("Searching for ports…"));
        } else if self.ports.is_empty() {
            ui.small(tr("No port detected; you can type COM3."));
        }
        ui.add_space(10.);
        ui.strong(tr("Movement · laser off"));
        let ready = self.snapshot.state == State::Ready && self.project.machine.validate().is_ok();
        ui.add_enabled_ui(ready, |ui| {
            ui.horizontal(|ui| {
                ui.label(tr("Step"));
                ui.add(
                    egui::DragValue::new(&mut self.jog_step)
                        .range(0.1..=10.)
                        .suffix(" mm"),
                );
            });
        });
        egui::Grid::new("jog-pad")
            .spacing(Vec2::new(5., 5.))
            .show(ui, |ui| {
                ui.label("");
                if ui
                    .add_enabled(ready, egui::Button::new("Y+").min_size(Vec2::new(55., 32.)))
                    .clicked()
                {
                    self.machine.send(Command::Jog(0., self.jog_step));
                }
                ui.label("");
                ui.end_row();
                if ui
                    .add_enabled(ready, egui::Button::new("X−").min_size(Vec2::new(55., 32.)))
                    .clicked()
                {
                    self.machine.send(Command::Jog(-self.jog_step, 0.));
                }
                if ui
                    .add_enabled(
                        ready && self.snapshot.referenced,
                        egui::Button::new("X0 Y0").min_size(Vec2::new(55., 32.)),
                    )
                    .on_hover_text(tr("Return to the work origin with the laser off"))
                    .clicked()
                {
                    self.machine.send(Command::ReturnOrigin);
                }
                if ui
                    .add_enabled(ready, egui::Button::new("X+").min_size(Vec2::new(55., 32.)))
                    .clicked()
                {
                    self.machine.send(Command::Jog(self.jog_step, 0.));
                }
                ui.end_row();
                ui.label("");
                if ui
                    .add_enabled(ready, egui::Button::new("Y−").min_size(Vec2::new(55., 32.)))
                    .clicked()
                {
                    self.machine.send(Command::Jog(0., -self.jog_step));
                }
                ui.label("");
                ui.end_row();
            });
        ui.add_space(6.);
        if ui
            .add_enabled(ready, egui::Button::new(tr("Set bed zero here")))
            .on_hover_text(tr(
                "Move to the usable lower-left corner. Sets the G54 work origin.",
            ))
            .clicked()
        {
            self.machine.send(
                if self.project.machine.positioning == Positioning::Absolute {
                    Command::ReferenceTable
                } else {
                    Command::Zero
                },
            );
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.project.machine.homing
                        && self.snapshot.homing_enabled == Some(true)
                        && matches!(self.snapshot.state, State::Ready | State::Alarm),
                    egui::Button::new(tr("Homing (sensors)")),
                )
                .on_hover_text(tr("Requires sensors and $22=1 on the controller."))
                .clicked()
            {
                self.machine.send(Command::Home);
            }
            if ui
                .add_enabled(
                    self.snapshot.state == State::Alarm,
                    egui::Button::new(tr("Unlock")),
                )
                .on_hover_text(tr(
                    "After fixing the cause of the alarm, sends $X and invalidates the reference.",
                ))
                .clicked()
            {
                self.machine.send(Command::Unlock);
            }
        });
        if self.snapshot.homing_enabled == Some(false) {
            ui.colored_label(ORANGE, tr("Homing disabled ($22=0). Use the work zero."));
        }
        ui.colored_label(
            if self.snapshot.referenced {
                ACCENT
            } else {
                MUTED
            },
            if self.snapshot.referenced {
                tr("Work origin set.")
            } else {
                tr("Set the work zero to return to the origin. Cancelling invalidates this reference.")
            },
        );
    }
    pub(super) fn job_toolbar(&mut self, ui: &mut egui::Ui) {
        let locked = self.locked();
        let mut start = false;
        if let Some(plan) = &self.plan {
            ui.horizontal_wrapped(|ui| {
                ui.label(trf!(
                    "{toolpaths} toolpaths · about {minutes} min",
                    toolpaths = plan.strokes.len(),
                    minutes = format!("{:.1}", plan.estimated_seconds / 60.)
                ));
                if ui
                    .add_enabled(!locked, egui::Button::new(tr("Export G-code…")))
                    .clicked()
                {
                    self.export_smax = self.snapshot.smax.unwrap_or(1000.);
                    self.export_open = true;
                }
                let ready =
                    !locked && self.snapshot.state == State::Ready && self.snapshot.referenced;
                if ui
                    .add_enabled(ready, egui::Button::new(tr("Frame without laser")))
                    .clicked()
                {
                    self.machine.send(Command::Frame(plan.clone()));
                    self.snapshot.state = State::Running;
                }
                if ui
                    .add_enabled(ready && self.snapshot.laser, pill(tr("Start job"), 135.))
                    .clicked()
                {
                    start = true;
                }
            });
            if !plan.warnings.is_empty() {
                ui.colored_label(
                    ORANGE,
                    trf!(
                        "{count} overlap warning(s). See the messages.",
                        count = plan.warnings.len()
                    ),
                );
            }
            if !self.snapshot.referenced {
                ui.colored_label(MUTED,tr("Connect and set the bed zero to run. Preparing and exporting work offline."));
            }
        } else {
            ui.horizontal(|ui| {
                ui.label(tr("Generate the toolpaths to preview and run the job."));
                if ui
                    .add_enabled(!self.locked(), egui::Button::new(tr("Generate now")))
                    .clicked()
                {
                    self.plan(ui.ctx());
                }
            });
        }
        if start {
            self.start_job();
        }
    }
}
