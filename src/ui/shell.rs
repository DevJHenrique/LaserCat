// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

impl LaserApp {
    pub(super) fn top(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("application-menu")
            .exact_height(31.)
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(35, 45, 62))
                    .inner_margin(egui::Margin::symmetric(9, 0)),
            )
            .show(ctx, |ui| {
                ui.style_mut().visuals = egui::Visuals::dark();
                ui.style_mut().spacing.item_spacing = Vec2::new(8., 0.);
                ui.horizontal(|ui| {
                    let (file_rect, _) =
                        ui.allocate_exact_size(Vec2::new(21., 27.), Sense::hover());
                    icons::paint(ui.painter(), file_rect, Icon::File, Color32::from_gray(240));
                    ui.menu_button(tr("File"), |ui| {
                        ui.add_enabled_ui(!self.locked(), |ui| {
                            if ui.button(tr("New project")).clicked() {
                                self.request_replace(Replace::New, ctx);
                                ui.close_menu();
                            }
                            if ui.button(tr("Open project…")).clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter(tr("LaserCat project (.lcc)"), &["lcc"])
                                    .add_filter(tr("Legacy Laser Studio (.laser)"), &["laser"])
                                    .pick_file()
                                {
                                    self.request_replace(Replace::Open(path), ctx);
                                }
                                ui.close_menu();
                            }
                            if ui.button(tr("Save project")).clicked() {
                                self.save(ctx, false);
                                ui.close_menu();
                            }
                            if ui.button(tr("Save as…")).clicked() {
                                let old = self.path.take();
                                if !self.save(ctx, false) {
                                    self.path = old;
                                }
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button(tr("Import SVG / image…")).clicked() {
                                self.mode = Mode::Prepare;
                                self.pick_import(ctx);
                                ui.close_menu();
                            }
                            if ui
                                .add_enabled(
                                    self.plan.is_some(),
                                    egui::Button::new(tr("Export G-code…")),
                                )
                                .clicked()
                            {
                                self.export_open = true;
                                self.export_smax = self.snapshot.smax.unwrap_or(1000.);
                                ui.close_menu();
                            }
                        });
                        ui.separator();
                        if ui.button(tr("License and source code…")).clicked() {
                            self.license_open = true;
                            ui.close_menu();
                        }
                    });
                    ui.menu_button(tr("Language"), |ui| {
                        for language in Language::ALL {
                            let current = i18n::language() == language;
                            if ui.radio(current, language.native_name()).clicked() {
                                self.set_language(language);
                                ui.close_menu();
                            }
                        }
                    });
                    ui.separator();
                    for (icon, tip, enabled, action) in [
                        (Icon::Save, tr("Save project"), !self.locked(), 0),
                        (
                            Icon::Undo,
                            tr("Undo"),
                            !self.locked() && self.mode == Mode::Prepare && !self.undo.is_empty(),
                            1,
                        ),
                        (
                            Icon::Redo,
                            tr("Redo"),
                            !self.locked() && self.mode == Mode::Prepare && !self.redo.is_empty(),
                            2,
                        ),
                        (Icon::Settings, tr("Configure machine"), !self.locked(), 3),
                    ] {
                        if dark_icon(ui, icon, tip, enabled).clicked() {
                            match action {
                                0 => {
                                    self.save(ctx, false);
                                }
                                1 => self.undo(),
                                2 => self.redo(),
                                _ => {
                                    self.draft = self.project.machine.clone();
                                    self.config_open = true;
                                }
                            }
                        }
                    }
                    let title = self
                        .path
                        .as_ref()
                        .and_then(|p| p.file_stem())
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| tr("Untitled").into());
                    ui.painter().text(
                        Pos2::new(ui.max_rect().center().x, 15.),
                        egui::Align2::CENTER_CENTER,
                        format!("{}{}", title, if self.dirty { " *" } else { "" }),
                        egui::FontId::proportional(12.),
                        Color32::from_gray(235),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!("LaserCat {}", env!("CARGO_PKG_VERSION")))
                                .size(11.)
                                .color(Color32::from_gray(200)),
                        );
                    });
                });
            });
        egui::TopBottomPanel::top("workspace-tabs")
            .exact_height(39.)
            .frame(egui::Frame::NONE.fill(HEADER))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                ui.horizontal(|ui| {
                    if nav_tab(ui, Icon::Box, "", self.mode == Mode::Prepare, 40.).clicked() {
                        self.mode = Mode::Prepare;
                        self.zoom = 1.;
                        self.pan = Vec2::ZERO;
                    }
                    for (mode, icon, label) in [
                        (Mode::Prepare, Icon::Box, tr("Prepare")),
                        (Mode::Preview, Icon::Layers, tr("Preview")),
                        (Mode::Device, Icon::Device, tr("Device")),
                    ] {
                        if nav_tab(
                            ui,
                            icon,
                            label,
                            self.mode == mode,
                            if ui.ctx().screen_rect().width() < 1200. {
                                122.
                            } else {
                                136.
                            },
                        )
                        .clicked()
                        {
                            self.mode = mode;
                        }
                    }
                    if dark_icon(ui, Icon::Settings, tr("Configure machine"), !self.locked())
                        .clicked()
                    {
                        self.draft = self.project.machine.clone();
                        self.config_open = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(14.);
                        let ready = !self.locked()
                            && self.plan.is_some()
                            && self.snapshot.state == State::Ready
                            && self.snapshot.referenced
                            && self.snapshot.laser;
                        if ui
                            .add_enabled(
                                ready,
                                pill(tr("Start job"), 143.).fill(if ready {
                                    ACCENT
                                } else {
                                    Color32::from_gray(120)
                                }),
                            )
                            .clicked()
                        {
                            self.start_job();
                        }
                        ui.add_space(12.);
                        if ui
                            .add_enabled(
                                !self.locked() && !self.project.objects.is_empty(),
                                pill(tr("Generate toolpaths"), 145.),
                            )
                            .clicked()
                        {
                            self.plan(ctx);
                        }
                    });
                });
            });
    }
    pub(super) fn status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status-bar").min_height(32.).show(ctx,|ui|{
            ui.horizontal_wrapped(|ui|{
                ui.colored_label(if self.snapshot.state==State::Ready{ACCENT}else{ORANGE},self.snapshot.state.label());
                if self.snapshot.total>0{ui.add(egui::ProgressBar::new(self.snapshot.accepted as f32/self.snapshot.total.max(1) as f32).desired_width(145.).text(trf!("{accepted} / {total} accepted", accepted = self.snapshot.accepted, total = self.snapshot.total)));}
                if ui.button(tr("Device / connection")).clicked(){self.mode=Mode::Device;}
                if ui.add_enabled(self.snapshot.state==State::Ready&&self.snapshot.referenced,egui::Button::new(tr("Return to origin"))).on_hover_text(tr("Move to the G54 work zero with the laser off. Requires a valid reference.")).clicked(){self.machine.send(Command::ReturnOrigin);}
                if matches!(self.snapshot.state,State::Running|State::Paused){
                    if self.snapshot.state==State::Running&&ui.button(tr("Pause")).clicked(){self.machine.send(Command::Pause);}
                    if self.snapshot.state==State::Paused&&ui.button(tr("Resume")).clicked(){self.machine.send(Command::Resume);}
                    if ui.button(tr("Stop and edit")).clicked(){self.stop_and_edit();}
                }else if self.cancel_to_edit{ui.spinner();ui.label(tr("Stopping…"));}
                else if self.mode!=Mode::Prepare&&ui.button(tr("Back to Prepare")).clicked(){self.mode=Mode::Prepare;}
            });
            if self.snapshot.state==State::Fault{ui.colored_label(Color32::from_rgb(188,48,55),&self.snapshot.message);}
            else if matches!(self.snapshot.state,State::Running|State::Paused)&&self.mode==Mode::Prepare {ui.colored_label(ORANGE,tr("Job in progress: editing is locked. Use Stop and edit to change the project."));}
            else if let Some(b)=&self.busy {ui.horizontal(|ui|{ui.spinner();ui.label(&b.label);ui.add(egui::ProgressBar::new(b.progress).desired_width(160.));if ui.button(tr("Cancel processing")).clicked(){b.cancel.store(true,Ordering::Relaxed);}});}
            else if !self.snapshot.message.is_empty(){ui.colored_label(MUTED,&self.snapshot.message);}
        });
    }
    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.locked() || ctx.wants_keyboard_input() || self.mode != Mode::Prepare {
            return;
        }
        let input = ctx.input(|i| {
            (
                i.modifiers.command,
                i.modifiers.shift,
                i.key_pressed(egui::Key::Z),
                i.key_pressed(egui::Key::Y),
                i.key_pressed(egui::Key::S),
                i.key_pressed(egui::Key::D),
                i.key_pressed(egui::Key::Delete),
            )
        });
        if input.0 {
            if input.2 && !input.1 {
                self.undo();
            }
            if input.3 || (input.2 && input.1) {
                self.redo();
            }
            if input.4 {
                self.save(ctx, false);
            }
            if input.5 {
                self.duplicate();
            }
        }
        if input.6 {
            self.delete();
        }
    }
}
