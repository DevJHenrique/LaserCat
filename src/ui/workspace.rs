// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

impl LaserApp {
    pub(super) fn left(&mut self, ctx: &egui::Context) {
        let width = if ctx.screen_rect().width() < 1200. {
            310.
        } else {
            350.
        };
        let editable = self.mode == Mode::Prepare && !self.locked();
        egui::SidePanel::left("workspace-sidebar")
            .exact_width(width)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(Color32::from_rgb(248, 248, 248))
                    .inner_margin(10.),
            )
            .show(ctx, |ui| {
                section_toggle(ui, tr("Device"), &mut self.device_expanded);
                if self.device_expanded {
                    egui::Frame::new()
                        .fill(Color32::WHITE)
                        .stroke(Stroke::new(1.0_f32, Color32::from_gray(215)))
                        .corner_radius(4.)
                        .inner_margin(9.)
                        .show(ui, |ui| {
                            ui.set_min_width(width - 40.);
                            ui.horizontal(|ui| {
                                let (rect, _) =
                                    ui.allocate_exact_size(Vec2::new(45., 45.), Sense::hover());
                                paint_machine(ui.painter(), rect);
                                ui.vertical(|ui| {
                                    ui.strong(if self.project.machine.configured {
                                        &self.project.machine.name
                                    } else {
                                        tr("Configure laser machine")
                                    });
                                    ui.label(format!(
                                        "{:.0} × {:.0} mm · {}",
                                        self.project.machine.width,
                                        self.project.machine.height,
                                        if self.simulator {
                                            tr("Simulator")
                                        } else {
                                            "GRBL"
                                        }
                                    ));
                                    ui.colored_label(ACCENT, self.snapshot.state.label());
                                });
                            });
                            ui.horizontal(|ui| {
                                if ui.small_button(tr("Configure device")).clicked() {
                                    self.draft = self.project.machine.clone();
                                    self.config_open = true;
                                }
                                if ui.small_button(tr("Connection")).clicked() {
                                    self.mode = Mode::Device;
                                }
                            });
                        });
                }
                ui.add_space(8.);
                ui.horizontal(|ui| {
                    section_toggle(ui, tr("Objects"), &mut self.objects_expanded);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icons::button(
                            ui,
                            Icon::Import,
                            tr("Import SVG / image"),
                            editable,
                            false,
                        )
                        .clicked()
                        {
                            self.pick_import(ctx);
                        }
                    });
                });
                if self.objects_expanded {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.object_search)
                            .hint_text(tr("Search objects…"))
                            .desired_width(f32::INFINITY),
                    );
                    let tree_height = if self.process_expanded {
                        (ui.available_height() - 280.).clamp(100., 270.)
                    } else {
                        (ui.available_height() - 65.).max(100.)
                    };
                    let query = self.object_search.to_lowercase();
                    egui::ScrollArea::vertical()
                        .id_salt("object-tree")
                        .max_height(tree_height)
                        .min_scrolled_height(tree_height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for outside in [false, true] {
                                let indices: Vec<_> = self
                                    .project
                                    .objects
                                    .iter()
                                    .enumerate()
                                    .filter_map(|(i, o)| {
                                        let asset = self.project.asset(o.asset_id)?;
                                        let b = o.bounds(asset);
                                        let out = b.min.x < 0.
                                            || b.min.y < 0.
                                            || b.max.x > self.project.machine.width
                                            || b.max.y > self.project.machine.height;
                                        (out == outside && o.name.to_lowercase().contains(&query))
                                            .then_some(i)
                                    })
                                    .collect();
                                if outside && indices.is_empty() {
                                    continue;
                                }
                                ui.colored_label(
                                    if outside { ORANGE } else { MUTED },
                                    if outside {
                                        tr("▾ Outside the bed")
                                    } else {
                                        tr("▾ Bed 01")
                                    },
                                );
                                if !outside && self.project.objects.is_empty() {
                                    ui.label(tr("Import SVG, PNG, or JPEG."));
                                }
                                for i in indices {
                                    let o = &mut self.project.objects[i];
                                    let selected = self.selection.contains(&o.id);
                                    egui::Frame::new()
                                        .fill(if selected {
                                            Color32::from_rgb(206, 225, 248)
                                        } else {
                                            Color32::TRANSPARENT
                                        })
                                        .inner_margin(egui::Margin::symmetric(6, 2))
                                        .show(ui, |ui| {
                                            ui.set_min_width(width - 40.);
                                            ui.horizontal(|ui| {
                                                ui.colored_label(PALETTE[i % 5], "■");
                                                let label: String =
                                                    o.name.chars().take(25).collect();
                                                if ui
                                                    .selectable_label(selected, label)
                                                    .on_hover_text(&o.name)
                                                    .clicked()
                                                {
                                                    if !ui.input(|i| {
                                                        i.modifiers.ctrl || i.modifiers.shift
                                                    }) {
                                                        self.selection.clear();
                                                    }
                                                    if !self.selection.insert(o.id) {
                                                        self.selection.remove(&o.id);
                                                    }
                                                }
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        if icons::button(
                                                            ui,
                                                            Icon::Lock,
                                                            tr("Lock / unlock"),
                                                            editable,
                                                            o.locked,
                                                        )
                                                        .clicked()
                                                        {
                                                            o.locked = !o.locked;
                                                        }
                                                    },
                                                );
                                            });
                                        });
                                }
                            }
                        });
                }
                ui.add_space(6.);
                ui.separator();
                section_toggle(ui, tr("Engraving / cutting"), &mut self.process_expanded);
                if self.process_expanded {
                    if ui
                        .add_enabled(
                            editable,
                            egui::Button::new(tr("Position, size, and rotation…")),
                        )
                        .clicked()
                    {
                        self.transform_open = true;
                    }
                    if self.mode == Mode::Preview {
                        ui.colored_label(MUTED, tr("Read-only · edit in the Prepare tab"));
                    }
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt("object-settings")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let old = self.position_settings;
                            self.position_settings = false;
                            ui.add_enabled_ui(editable, |ui| self.properties(ui));
                            self.position_settings = old;
                        });
                }
            });
    }
    pub(super) fn align(&mut self, axis: u8) {
        let bounds = Bounds::from_points(
            self.project
                .objects
                .iter()
                .filter(|o| self.selection.contains(&o.id) && !o.locked)
                .flat_map(|o| o.corners(self.project.asset(o.asset_id).unwrap())),
        );
        if let Some(b) = bounds {
            let dx = self.project.machine.width / 2. - (b.min.x + b.max.x) / 2.;
            let dy = self.project.machine.height / 2. - (b.min.y + b.max.y) / 2.;
            for o in &mut self.project.objects {
                if self.selection.contains(&o.id) && !o.locked {
                    let own = o.bounds(
                        self.project
                            .assets
                            .iter()
                            .find(|a| a.id == o.asset_id)
                            .unwrap(),
                    );
                    match axis {
                        0 => {
                            o.x += dx;
                            o.y += dy;
                        }
                        1 => o.x += b.min.x - own.min.x,
                        2 => o.y += b.min.y - own.min.y,
                        _ => {}
                    }
                }
            }
        }
    }
    pub(super) fn properties(&mut self, ui: &mut egui::Ui) {
        let locked = self.locked();
        let positions = self.position_settings;
        ui.add_enabled_ui(!locked, |ui| {
            if self.selection.len() != 1 {
                ui.colored_label(
                    MUTED,
                    if self.selection.is_empty() {
                        tr("Select an object to edit.")
                    } else {
                        tr("Use the alignment controls for multiple selection.")
                    },
                );
                return;
            }
            let Some(index) = self
                .project
                .objects
                .iter()
                .position(|o| self.selection.contains(&o.id))
            else {
                return;
            };
            let a = self
                .project
                .assets
                .iter()
                .find(|a| a.id == self.project.objects[index].asset_id)
                .cloned()
                .unwrap();
            let o = &mut self.project.objects[index];
            ui.label(egui::RichText::new(&o.name).strong().color(ACCENT));
            ui.add_space(10.);
            if positions {
                ui.text_edit_singleline(&mut o.name);
                ui.add_space(8.);
                ui.add_enabled_ui(!o.locked, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(tr("Scale X / Y"));
                        let mut x = o.width / a.width * 100.;
                        let mut y = o.height / a.height * 100.;
                        if ui
                            .add(
                                egui::DragValue::new(&mut x)
                                    .speed(0.5)
                                    .range(0.1..=100000.)
                                    .suffix(" %"),
                            )
                            .changed()
                        {
                            let w = (a.width * x / 100.).clamp(0.1, 10000.);
                            if o.lock_ratio {
                                o.height = (o.height * w / o.width).clamp(0.1, 10000.);
                            }
                            o.width = w;
                        }
                        if ui
                            .add(
                                egui::DragValue::new(&mut y)
                                    .speed(0.5)
                                    .range(0.1..=100000.)
                                    .suffix(" %"),
                            )
                            .changed()
                        {
                            let h = (a.height * y / 100.).clamp(0.1, 10000.);
                            if o.lock_ratio {
                                o.width = (o.width * h / o.height).clamp(0.1, 10000.);
                            }
                            o.height = h;
                        }
                    });
                    egui::Grid::new("transform")
                        .num_columns(2)
                        .spacing([12., 9.])
                        .show(ui, |ui| {
                            ui.label(tr("Position X"));
                            ui.add(egui::DragValue::new(&mut o.x).speed(0.1).suffix(" mm"));
                            ui.end_row();
                            ui.label(tr("Position Y"));
                            ui.add(egui::DragValue::new(&mut o.y).speed(0.1).suffix(" mm"));
                            ui.end_row();
                            ui.label(tr("Width"));
                            let old = o.width;
                            if ui
                                .add(
                                    egui::DragValue::new(&mut o.width)
                                        .range(0.1..=10_000.)
                                        .speed(0.1)
                                        .suffix(" mm"),
                                )
                                .changed()
                                && o.lock_ratio
                            {
                                o.height *= o.width / old;
                            }
                            ui.end_row();
                            ui.label(tr("Height"));
                            let old = o.height;
                            if ui
                                .add(
                                    egui::DragValue::new(&mut o.height)
                                        .range(0.1..=10_000.)
                                        .speed(0.1)
                                        .suffix(" mm"),
                                )
                                .changed()
                                && o.lock_ratio
                            {
                                o.width *= o.height / old;
                            }
                            ui.end_row();
                            ui.label(tr("Rotation"));
                            ui.add(
                                egui::DragValue::new(&mut o.rotation)
                                    .speed(0.5)
                                    .suffix(" °"),
                            );
                            ui.end_row();
                        });
                    ui.checkbox(&mut o.lock_ratio, tr("Keep aspect ratio"));
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut o.mirror_x, tr("Mirror X"));
                        ui.checkbox(&mut o.mirror_y, tr("Mirror Y"));
                    });
                    if let Geometry::Raster {
                        pixel_width,
                        pixel_height,
                    } = a.geometry
                    {
                        let mut dpi = pixel_width as f64 / o.width * 25.4;
                        if ui
                            .add(
                                egui::DragValue::new(&mut dpi)
                                    .range(1.0..=2400.)
                                    .prefix(tr("Resolution: "))
                                    .suffix(" DPI"),
                            )
                            .changed()
                        {
                            o.width = pixel_width as f64 / dpi * 25.4;
                            o.height = pixel_height as f64 / dpi * 25.4;
                        }
                    }
                });
                return;
            }
            for warning in &a.warnings {
                ui.colored_label(ORANGE, warning);
            }
            ui.add_space(4.);
            ui.separator();
            ui.label(egui::RichText::new(tr("Engraving / cutting operations")).strong());
            let vector = matches!(
                a.geometry,
                Geometry::Vector {
                    vector_safe: true,
                    ..
                }
            );
            let mut remove = None;
            for (i, op) in o.operations.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    ui.add_space(9.);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut op.enabled, format!("{} · {}", i + 1, op.kind.label()));
                        if ui.small_button("×").clicked() {
                            remove = Some(i);
                        }
                    });
                    ui.add_enabled_ui(op.enabled, |ui| {
                        egui::ComboBox::from_id_salt("kind")
                            .selected_text(op.kind.label())
                            .show_ui(ui, |ui| {
                                for kind in [
                                    OperationKind::Line,
                                    OperationKind::Fill,
                                    OperationKind::Cut,
                                    OperationKind::Raster,
                                ] {
                                    if vector || kind == OperationKind::Raster {
                                        ui.selectable_value(&mut op.kind, kind, kind.label());
                                    }
                                }
                            });
                        ui.add(egui::Slider::new(&mut op.power, 0.0..=100.).text(tr("Power %")));
                        field(ui, tr("Speed"), &mut op.speed, 1.0..=100_000., " mm/min");
                        ui.horizontal(|ui| {
                            ui.label(tr("Passes"));
                            ui.add(egui::DragValue::new(&mut op.passes).range(1..=100));
                        });
                        ui.horizontal(|ui| {
                            ui.label(tr("Priority"));
                            ui.add(egui::DragValue::new(&mut op.order));
                        });
                        ui.label(
                            egui::RichText::new(tr(
                                "Engraving runs before cutting. Lower priority first.",
                            ))
                            .small()
                            .color(MUTED),
                        );
                        if matches!(op.kind, OperationKind::Fill | OperationKind::Raster) {
                            field(ui, tr("Spacing"), &mut op.spacing, 0.05..=10., " mm");
                        }
                        if op.kind == OperationKind::Raster {
                            field(
                                ui,
                                tr("Laser-off margin"),
                                &mut op.overscan,
                                0.0..=20.,
                                " mm",
                            );
                            ui.add(
                                egui::Slider::new(&mut op.brightness, -1.0..=1.)
                                    .text(tr("Brightness")),
                            );
                            ui.add(
                                egui::Slider::new(&mut op.contrast, 0.0..=4.).text(tr("Contrast")),
                            );
                            ui.checkbox(&mut op.inverted, tr("Invert image"));
                            ui.checkbox(&mut op.dither, tr("Ordered dithering"));
                        }
                        ui.checkbox(&mut op.constant_power, tr("Constant power (M3)"))
                            .on_hover_text(tr(
                                "Unchecked: dynamic power M4, recommended for lasers.",
                            ));
                    });
                    ui.separator();
                });
            }
            if let Some(i) = remove {
                o.operations.remove(i);
            }
            if ui.button(tr("+ Add operation")).clicked() {
                o.operations.push(Operation::new(if vector {
                    OperationKind::Cut
                } else {
                    OperationKind::Raster
                }));
            }
        });
    }
    pub(super) fn canvas_toolbar(&mut self, ui: &mut egui::Ui) {
        if self.mode == Mode::Preview {
            ui.horizontal(|ui| {
                if icons::button(
                    ui,
                    Icon::Align,
                    tr("Show / hide side panel"),
                    true,
                    self.sidebar_collapsed,
                )
                .clicked()
                {
                    self.sidebar_collapsed = !self.sidebar_collapsed;
                }
                ui.strong(tr("Toolpath preview"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{:.0}%", self.zoom * 100.));
                });
            });
            ui.separator();
            return;
        }
        let editing = self.mode == Mode::Prepare && !self.locked();
        let selected = !self.selection.is_empty();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6., 0.);
            if icons::button(
                ui,
                Icon::Align,
                if self.sidebar_collapsed {
                    tr("Show side panel")
                } else {
                    tr("Collapse side panel")
                },
                self.mode == Mode::Prepare,
                self.sidebar_collapsed,
            )
            .clicked()
            {
                self.sidebar_collapsed = !self.sidebar_collapsed;
            }
            ui.separator();
            if icons::button(ui, Icon::Import, tr("Import SVG / image"), editing, false).clicked() {
                let ctx = ui.ctx().clone();
                self.pick_import(&ctx);
            }
            ui.separator();
            if icons::button(
                ui,
                Icon::Move,
                tr("Object position and movement"),
                editing && selected,
                self.position_settings,
            )
            .clicked()
            {
                self.transform_open = true;
            }
            if icons::button(
                ui,
                Icon::Rotate,
                tr("Object rotation"),
                editing && selected,
                false,
            )
            .clicked()
            {
                self.transform_open = true;
            }
            if icons::button(
                ui,
                Icon::Scale,
                tr("Edit dimensions and scale"),
                editing && selected,
                false,
            )
            .clicked()
            {
                self.transform_open = true;
            }
            if icons::button(
                ui,
                Icon::Mirror,
                tr("Mirror selection in X"),
                editing && selected,
                false,
            )
            .clicked()
            {
                for o in &mut self.project.objects {
                    if self.selection.contains(&o.id) && !o.locked {
                        o.mirror_x = !o.mirror_x;
                    }
                }
            }
            if icons::button(
                ui,
                Icon::Duplicate,
                tr("Duplicate selection"),
                editing && selected,
                false,
            )
            .clicked()
            {
                self.duplicate();
            }
            if icons::button(
                ui,
                Icon::Delete,
                tr("Delete selection"),
                editing && selected,
                false,
            )
            .clicked()
            {
                self.delete();
            }
            ui.separator();
            if icons::button(
                ui,
                Icon::Align,
                tr("Center selection on the bed"),
                editing && selected,
                false,
            )
            .clicked()
            {
                self.align(0);
            }
            ui.menu_button(tr("Align"), |ui| {
                ui.add_enabled_ui(editing, |ui| {
                    ui.checkbox(&mut self.snap, tr("Snap to grid (1 mm)"));
                });
                ui.separator();
                ui.add_enabled_ui(editing && selected, |ui| {
                    for (label, axis) in [
                        (tr("Center on the bed"), 0),
                        (tr("Align left"), 1),
                        (tr("Align bottom"), 2),
                    ] {
                        if ui.button(label).clicked() {
                            self.align(axis);
                            ui.close_menu();
                        }
                    }
                });
            });
            ui.menu_button(tr("Order"), |ui| {
                ui.add_enabled_ui(editing && selected, |ui| {
                    for front in [true, false] {
                        if ui
                            .button(if front {
                                tr("Bring to front")
                            } else {
                                tr("Send to back")
                            })
                            .clicked()
                        {
                            let mut chosen = vec![];
                            self.project.objects.retain(|o| {
                                if self.selection.contains(&o.id) {
                                    chosen.push(o.clone());
                                    false
                                } else {
                                    true
                                }
                            });
                            if front {
                                self.project.objects.extend(chosen);
                            } else {
                                chosen.append(&mut self.project.objects);
                                self.project.objects = chosen;
                            }
                            ui.close_menu();
                        }
                    }
                });
            });
            ui.separator();
            if icons::button(ui, Icon::Rectangle, tr("Add rectangle"), editing, false).clicked() {
                let ctx = ui.ctx().clone();
                self.add_shape(Shape::Rectangle, &ctx);
            }
            if icons::button(ui, Icon::Circle, tr("Add circle"), editing, false).clicked() {
                let ctx = ui.ctx().clone();
                self.add_shape(Shape::Circle, &ctx);
            }
            if icons::button(ui, Icon::Text, tr("Add text"), editing, false).clicked() {
                self.text_open = true;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(MUTED, format!("{:.0}%", self.zoom * 100.));
            });
        });
        ui.separator();
        if self.mode == Mode::Preview {
            ui.horizontal(|ui| {
                ui.strong(tr("Preview"));
                ui.checkbox(&mut self.show_rapid, tr("Show travel moves"));
            });
        }
    }
    pub(super) fn bed_tools(&mut self, ctx: &egui::Context, viewport: Rect) {
        egui::Area::new(egui::Id::new("plate-floating-tools"))
            .fixed_pos(Pos2::new(viewport.right() - 45., viewport.top() + 70.))
            .movable(false)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Color32::from_rgb(237, 237, 237))
                    .stroke(Stroke::new(1.0_f32, Color32::from_gray(190)))
                    .corner_radius(5.)
                    .inner_margin(3.)
                    .show(ui, |ui| {
                        if icons::button(
                            ui,
                            Icon::Fit,
                            tr("Fit the bed to the window"),
                            true,
                            false,
                        )
                        .clicked()
                        {
                            self.zoom = 1.;
                            self.pan = Vec2::ZERO;
                        }
                        ui.add_space(5.);
                        if icons::button(
                            ui,
                            Icon::Grid,
                            tr("Show / hide grid"),
                            true,
                            self.show_grid,
                        )
                        .clicked()
                        {
                            self.show_grid = !self.show_grid;
                        }
                        ui.add_space(5.);
                        if icons::button(
                            ui,
                            Icon::Origin,
                            tr("Return to the work origin (laser off)"),
                            self.snapshot.state == State::Ready && self.snapshot.referenced,
                            false,
                        )
                        .clicked()
                        {
                            self.machine.send(Command::ReturnOrigin);
                        }
                        ui.add_space(5.);
                        let selected = !self.selection.is_empty()
                            && self.mode == Mode::Prepare
                            && !self.locked();
                        if icons::button(
                            ui,
                            Icon::Lock,
                            tr("Lock / unlock selection"),
                            selected,
                            false,
                        )
                        .clicked()
                        {
                            let lock = self
                                .project
                                .objects
                                .iter()
                                .any(|o| self.selection.contains(&o.id) && !o.locked);
                            for o in &mut self.project.objects {
                                if self.selection.contains(&o.id) {
                                    o.locked = lock;
                                }
                            }
                        }
                        ui.add_space(5.);
                        if icons::button(
                            ui,
                            Icon::Settings,
                            tr("Configure machine"),
                            !self.locked(),
                            false,
                        )
                        .clicked()
                        {
                            self.draft = self.project.machine.clone();
                            self.config_open = true;
                        }
                    });
            });
    }
    pub(super) fn canvas(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(CANVAS).inner_margin(egui::Margin::same(8))).show(ctx,|ui|{
            self.canvas_toolbar(ui);

            let (response,painter)=ui.allocate_painter(ui.available_size(),Sense::click_and_drag());
            let viewport=response.rect;
            if !self.project.machine.configured{
                painter.text(viewport.center()-Vec2::new(0.,20.),egui::Align2::CENTER_CENTER,tr("Your bed starts here"),egui::FontId::proportional(28.),HEADER);
                painter.text(viewport.center()+Vec2::new(0.,20.),egui::Align2::CENTER_CENTER,tr("Configure the machine's real dimensions to prepare your first job."),egui::FontId::proportional(15.),MUTED);return;
            }
            let w=self.project.machine.width as f32;let h=self.project.machine.height as f32;
            let base=((viewport.width()-160.)/w).min((viewport.height()-130.)/h).max(0.001);
            if response.hovered(){
                let scroll=ui.input(|i|i.raw_scroll_delta.y);
                if scroll!=0.{let previous=self.zoom;self.zoom=(self.zoom*(scroll*0.002).exp()).clamp(0.2,30.);
                    if let Some(pos)=ui.input(|i|i.pointer.hover_pos()){self.pan=(self.pan-(pos-viewport.center()))*(self.zoom/previous)+(pos-viewport.center());}}
            }
            if response.dragged_by(egui::PointerButton::Middle){self.pan+=ui.input(|i|i.pointer.delta());}
            let scale=base*self.zoom;let origin=viewport.center()+self.pan+Vec2::new(-w*scale/2.,h*scale/2.);
            self.last_view=Some((origin,scale));
            let screen=|p:Point|Pos2::new(origin.x+p.x as f32*scale,origin.y-p.y as f32*scale);
            let world=|p:Pos2|Point::new(((p.x-origin.x)/scale) as f64,((origin.y-p.y)/scale) as f64);
            let bed=Rect::from_two_pos(screen(Point::new(0.,0.)),screen(Point::new(w as f64,h as f64)));
            let handle=Rect::from_min_max(Pos2::new(bed.center().x-75.,bed.top()-18.),Pos2::new(bed.center().x+75.,bed.top()+2.));
            painter.rect_filled(handle,8.,Color32::from_rgb(57,66,78));
            painter.rect_filled(Rect::from_center_size(handle.center(),Vec2::new(64.,4.)),2.,Color32::from_gray(190));
            painter.rect_filled(bed,1.,Color32::from_rgb(57,66,78));
            let label_step=nice_step(55./scale);
            let step=label_step/5.;
            if self.show_grid{for i in 0..=(w/step) as usize {
                let x=i as f32*step;let p=screen(Point::new(x as f64,0.));
                if p.x<viewport.left()||p.x>viewport.right(){continue;}
                painter.line_segment([p,screen(Point::new(x as f64,h as f64))],Stroke::new(1.0_f32,Color32::from_rgb(84,98,113)));
                if i%5==0 {painter.text(Pos2::new(p.x,bed.bottom()+13.),egui::Align2::CENTER_CENTER,format!("{x:.0}"),egui::FontId::monospace(10.),MUTED);}
            }
            for i in 0..=(h/step) as usize {
                let y=i as f32*step;let p=screen(Point::new(0.,y as f64));
                if p.y<viewport.top()||p.y>viewport.bottom(){continue;}
                painter.line_segment([p,screen(Point::new(w as f64,y as f64))],Stroke::new(1.0_f32,Color32::from_rgb(84,98,113)));
                if i%5==0 {painter.text(Pos2::new(bed.left()-9.,p.y),egui::Align2::RIGHT_CENTER,format!("{y:.0}"),egui::FontId::monospace(10.),MUTED);}
            }}
            painter.rect_stroke(bed,1.,Stroke::new(1.2_f32,Color32::from_gray(192)),egui::StrokeKind::Inside);
            painter.line_segment([bed.left_bottom(),bed.right_bottom()],Stroke::new(1.2_f32,Color32::from_rgb(225,83,85)));
            painter.line_segment([bed.left_bottom(),bed.left_top()],Stroke::new(1.2_f32,Color32::from_rgb(60,190,112)));
            let plate_name=self.path.as_ref().and_then(|p|p.file_stem()).map(|s|s.to_string_lossy().into_owned()).unwrap_or_else(||tr("Untitled").into());
            painter.text(bed.left_top()-Vec2::new(0.,28.),egui::Align2::LEFT_CENTER,plate_name,egui::FontId::proportional(20.),Color32::from_rgb(245,116,75));
            painter.text(bed.right_bottom()+Vec2::new(14.,-10.),egui::Align2::LEFT_BOTTOM,"01",egui::FontId::proportional(28.),Color32::from_rgb(245,116,75));
            painter.circle_filled(screen(Point::default()),4.,ACCENT);
            if self.mode==Mode::Prepare {
                if !self.locked(){
                    if response.drag_started_by(egui::PointerButton::Primary) {
                        if let Some(pos)=ui.input(|i|i.pointer.press_origin()) {
                            if self.selection.len()==1 {
                                if let Some(o)=self.project.objects.iter().find(|o|self.selection.contains(&o.id)&&!o.locked) {
                                    let corners=transform::corners(o).map(screen);
                                    if let Some(corner)=corners.iter().position(|c|c.distance(pos)<=11.) {
                                        self.transform_drag=Some(TransformDrag::Resize {original:o.clone(),corner});
                                    } else {
                                        let center=screen(Point::new(o.x+o.width/2.,o.y+o.height/2.));
                                        let mid=corners[2].lerp(corners[3],0.5);
                                        let handle=mid+(mid-center).normalized()*28.;
                                        if handle.distance(pos)<=12. {self.transform_drag=Some(TransformDrag::Rotate {original:o.clone(),angle:((pos.y-center.y) as f64).atan2((pos.x-center.x) as f64)});}
                                    }
                                }
                            }
                        }
                    }
                    if response.drag_started() && self.transform_drag.is_some() {self.transform_history=Some(self.project.clone());}
                    if self.transform_drag.is_none() && (response.clicked()||response.drag_started_by(egui::PointerButton::Primary)) {
                        if let Some(pos)=if response.drag_started() {ui.input(|i|i.pointer.press_origin())} else {response.interact_pointer_pos()} {
                            let p=world(pos);let hit=self.project.objects.iter().rev().find(|o|{
                                let a=self.project.asset(o.asset_id).unwrap();let local=o.inverse(p,a);local.x>=0.&&local.x<=a.width&&local.y>=0.&&local.y<=a.height
                            }).map(|o|o.id);
                            let multi=ui.input(|i|i.modifiers.ctrl||i.modifiers.shift);
                            if let Some(id)=hit{if multi&&response.clicked(){if !self.selection.insert(id){self.selection.remove(&id);}}else if !self.selection.contains(&id){self.selection.clear();self.selection.insert(id);}}
                            else if !multi{self.selection.clear();}
                        }
                    }
                    if response.dragged_by(egui::PointerButton::Primary) {
                        match &self.transform_drag {
                            Some(TransformDrag::Resize {original,corner})=> {
                                if let Some(pos)=response.interact_pointer_pos() {
                                    if let Some(o)=self.project.objects.iter_mut().find(|o|o.id==original.id) {
                                        transform::resize(o,original,*corner,world(pos),original.lock_ratio ^ ui.input(|i|i.modifiers.shift));
                                    }
                                }
                            }
                            Some(TransformDrag::Rotate {original,angle})=> {
                                if let Some(pos)=response.interact_pointer_pos() {
                                    let center=screen(Point::new(original.x+original.width/2.,original.y+original.height/2.));
                                    let current=((pos.y-center.y) as f64).atan2((pos.x-center.x) as f64);
                                    let delta=(current-angle).to_degrees();
                                    if let Some(o)=self.project.objects.iter_mut().find(|o|o.id==original.id) {
                                        o.rotation=(original.rotation-delta).rem_euclid(360.);
                                        if ui.input(|i|i.modifiers.shift) {o.rotation=(o.rotation/15.).round()*15.;}
                                    }
                                }
                            }
                            None=> {
                                let delta=ui.input(|i|i.pointer.delta())/scale;
                                for o in &mut self.project.objects {if self.selection.contains(&o.id)&&!o.locked{o.x+=delta.x as f64;o.y-=delta.y as f64;}}
                            }
                        }
                    }
                    if response.drag_stopped() {
                        if self.transform_drag.is_none()&&self.snap {for o in &mut self.project.objects {if self.selection.contains(&o.id)&&!o.locked{o.x=o.x.round();o.y=o.y.round();}}}
                        self.transform_drag=None;
                        if let Some(before)=self.transform_history.take() {
                            if before!=self.project {self.undo.push_back(before);self.transform_undo_committed=true;while self.undo.len()>30 {self.undo.pop_front();}}
                        }
                    }
                }
                for (index,o) in self.project.objects.iter().enumerate(){
                    let a=self.project.asset(o.asset_id).unwrap();let corners=o.corners(a).map(screen);let color=PALETTE[index%5];
                    let vector=matches!(a.geometry,Geometry::Vector{vector_safe:true,..});
                    let raster=o.operations.iter().any(|op|op.enabled&&op.kind==OperationKind::Raster)||!vector;
                    let filled=o.operations.iter().any(|op|op.enabled&&op.kind==OperationKind::Fill);
                    let texture=if raster{self.textures.get(&a.id)}else if filled{self.masks.get(&a.id)}else{None};
                    if raster{painter.add(egui::Shape::convex_polygon(corners.to_vec(),Color32::from_rgb(225,231,236),Stroke::NONE));}
                    if let Some(texture)=texture{
                        let mut mesh=egui::Mesh::with_texture(texture.id());
                        // Raster/SVG images have a top-left origin; world geometry has a bottom-left origin.
                        for (pos,uv) in [(corners[3],Pos2::new(0.,0.)),(corners[2],Pos2::new(1.,0.)),(corners[1],Pos2::new(1.,1.)),(corners[0],Pos2::new(0.,1.))]{mesh.vertices.push(egui::epaint::Vertex{pos,uv,color:if raster{Color32::WHITE}else{color.gamma_multiply(0.7)}});}
                        mesh.indices=vec![0,1,2,0,2,3];painter.add(egui::Shape::mesh(mesh));
                    }
                    if vector&&!raster{if let Geometry::Vector{paths,..}=&a.geometry{for path in paths{let points:Vec<_>=path.points.iter().map(|p|screen(o.transform(*p,a))).collect();if points.len()>1{painter.add(egui::Shape::line(points,Stroke::new(1.3_f32,color)));}}}}
                    let selected=self.selection.contains(&o.id);let b=o.bounds(a);let outside=b.min.x<0.||b.min.y<0.||b.max.x>w as f64||b.max.y>h as f64;
                    let stroke=Stroke::new(if selected{2.0_f32}else{1.0_f32},if outside{Color32::LIGHT_RED}else{color});
                    for i in 0..4{painter.line_segment([corners[i],corners[(i+1)%4]],stroke);}
                    if selected && self.selection.len()==1 && !o.locked {
                        let handles=transform::corners(o).map(screen);
                        for c in handles {painter.rect_filled(Rect::from_center_size(c,Vec2::splat(9.)),1.,ACCENT);painter.rect_stroke(Rect::from_center_size(c,Vec2::splat(9.)),1.,Stroke::new(1.0_f32,Color32::WHITE),egui::StrokeKind::Inside);}
                        let center=screen(Point::new(o.x+o.width/2.,o.y+o.height/2.));let mid=handles[2].lerp(handles[3],0.5);
                        let handle=mid+(mid-center).normalized()*28.;
                        painter.line_segment([mid,handle],Stroke::new(1.5_f32,ACCENT));painter.circle_filled(handle,6.,ACCENT);painter.circle_stroke(handle,6.,Stroke::new(1.0_f32,Color32::WHITE));
                    }
                    painter.text(screen(Point::new(b.min.x,b.max.y))+Vec2::new(0.,-8.),egui::Align2::LEFT_BOTTOM,&o.name,egui::FontId::proportional(11.),color);
                }
                for (i,o) in self.project.objects.iter().enumerate(){for other in &self.project.objects[i+1..]{
                    let a=o.bounds(self.project.asset(o.asset_id).unwrap());let b=other.bounds(self.project.asset(other.asset_id).unwrap());
                    if a.overlaps(b){let min=Point::new(a.min.x.max(b.min.x),a.min.y.max(b.min.y));let max=Point::new(a.max.x.min(b.max.x),a.max.y.min(b.max.y));
                        painter.rect_filled(Rect::from_two_pos(screen(min),screen(max)),0.,Color32::from_rgba_unmultiplied(248,180,95,30));}
                }}
            } else if let Some(plan)=&self.plan{
                let mut last=None;let total=plan.strokes.iter().map(|s|s.points.len()).sum::<usize>();let stride=(total/80_000).max(1);let mut index=0;
                for s in &plan.strokes {
                    if self.show_rapid{if let Some(p)=last{painter.line_segment([screen(p),screen(s.points[0])],Stroke::new(0.5_f32,Color32::from_rgb(65,79,97)));}}
                    let color=if s.power==0.{Color32::from_gray(165)}else if s.kind==OperationKind::Cut{Color32::from_rgb(255,166,95)}else{Color32::from_rgb(76,218,183)};
                    for pts in s.points.windows(2){if index%stride==0{painter.line_segment([screen(pts[0]),screen(pts[1])],Stroke::new(1.2_f32,color));}index+=1;}
                    last=s.points.last().copied();
                }
                if stride>1 {painter.text(viewport.left_top()+Vec2::splat(16.),egui::Align2::LEFT_TOP,tr("Preview simplified to stay responsive. The G-code contains every toolpath."),egui::FontId::proportional(12.),MUTED);}
            }
            if let Some(pos)=self.snapshot.position {
                let m=&self.project.machine;let p=Point::new(if m.invert_x{m.width-pos[0]}else{pos[0]},if m.invert_y{m.height-pos[1]}else{pos[1]});let pos=screen(p);
                painter.circle_stroke(pos,7.,Stroke::new(2.0_f32,Color32::WHITE));painter.line_segment([pos-Vec2::new(12.,0.),pos+Vec2::new(12.,0.)],Stroke::new(1.0_f32,Color32::WHITE));painter.line_segment([pos-Vec2::new(0.,12.),pos+Vec2::new(0.,12.)],Stroke::new(1.0_f32,Color32::WHITE));
            }
            if self.project.objects.is_empty(){painter.text(bed.center(),egui::Align2::CENTER_CENTER,tr("Drag SVG, PNG, or JPEG onto the bed"),egui::FontId::proportional(19.),MUTED);}
            let axis=viewport.left_bottom()+Vec2::new(27.,-25.);
            painter.line_segment([axis,axis+Vec2::new(35.,0.)],Stroke::new(1.7_f32,Color32::from_rgb(225,83,85)));
            painter.line_segment([axis,axis-Vec2::new(0.,35.)],Stroke::new(1.7_f32,Color32::from_rgb(60,180,112)));
            painter.text(axis+Vec2::new(42.,0.),egui::Align2::CENTER_CENTER,"X",egui::FontId::proportional(12.),Color32::from_rgb(210,70,75));
            painter.text(axis-Vec2::new(0.,43.),egui::Align2::CENTER_CENTER,"Y",egui::FontId::proportional(12.),Color32::from_rgb(45,155,90));
            self.bed_tools(ctx,viewport);
        });
    }
}
