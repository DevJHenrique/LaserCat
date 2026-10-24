// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

impl LaserApp {
    pub(super) fn locked(&self) -> bool {
        self.busy.is_some()
            || matches!(self.snapshot.state, State::Running | State::Paused)
            || (self.cancel_to_edit && self.snapshot.state == State::Connecting)
    }
    /// Switches the interface language immediately and remembers it on this computer.
    pub(super) fn set_language(&mut self, language: Language) {
        i18n::set_language(language);
        if let Some(path) = &self.preferences_file {
            let preferences = storage::Preferences { language };
            if let Err(e) = storage::save_preferences(path, &preferences) {
                self.log(trf!("Could not save preferences: {error}", error = e));
            }
        }
    }
    pub(super) fn log(&mut self, s: impl Into<String>) {
        self.logs.push_back(s.into());
        while self.logs.len() > 150 {
            self.logs.pop_front();
        }
    }
    pub(super) fn start_work(&mut self, label: &str) -> Arc<AtomicBool> {
        let cancel = Arc::new(AtomicBool::new(false));
        self.busy = Some(Busy {
            label: label.into(),
            progress: 0.,
            cancel: cancel.clone(),
        });
        cancel
    }
    pub(super) fn texture(&mut self, ctx: &egui::Context, id: u64, img: image::RgbaImage) {
        let size = [img.width() as usize, img.height() as usize];
        if self
            .project
            .asset(id)
            .is_some_and(|a| matches!(a.geometry, Geometry::Vector { .. }))
        {
            let mut mask = img.clone();
            for p in mask.pixels_mut() {
                p.0[0] = 255;
                p.0[1] = 255;
                p.0[2] = 255;
            }
            self.masks.insert(
                id,
                ctx.load_texture(
                    format!("mask-{id}"),
                    egui::ColorImage::from_rgba_unmultiplied(size, mask.as_raw()),
                    egui::TextureOptions::LINEAR,
                ),
            );
        }
        self.textures.insert(
            id,
            ctx.load_texture(
                format!("asset-{id}"),
                egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()),
                egui::TextureOptions::LINEAR,
            ),
        );
    }
    pub(super) fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                WorkEvent::CalibrationSaved(result) => {
                    self.busy = None;
                    match result {
                        Ok(path) => self.request_replace(Replace::Open(path), ctx),
                        Err(e) => self.log(e),
                    }
                }
                WorkEvent::Ports(result) => {
                    self.ports_refreshing = false;
                    match result {
                        Ok(ports) => {
                            self.ports = ports;
                            self.ports_error = None;
                        }
                        Err(e) => {
                            if self.ports_error.as_ref() != Some(&e) {
                                self.log(e.clone());
                            }
                            self.ports_error = Some(e);
                        }
                    }
                }
                WorkEvent::Imported(result) => match result {
                    Ok((asset, img)) => {
                        let id = self.project.add_asset(asset);
                        let asset_id = self.project.objects.last().unwrap().asset_id;
                        self.texture(ctx, asset_id, img);
                        self.selection.clear();
                        self.selection.insert(id);
                        for warning in self.project.assets.last().unwrap().warnings.clone() {
                            self.log(warning);
                        }
                    }
                    Err(e) => self.log(e),
                },
                WorkEvent::Texture(id, result) => match result {
                    Ok(img) => self.texture(ctx, id, img),
                    Err(e) => self.log(e),
                },
                WorkEvent::Loaded(result, path) => {
                    self.busy = None;
                    match result {
                        Ok(p) => {
                            self.project = p;
                            if self.saved_machine.configured {
                                self.project.machine = self.saved_machine.clone();
                            }
                            self.path = if Some(&path) == storage::recovery_source().as_ref() {
                                None
                            } else {
                                Some(path)
                            };
                            self.selection.clear();
                            self.textures.clear();
                            self.masks.clear();
                            self.plan = None;
                            self.preview_gcode.clear();
                            self.undo.clear();
                            self.redo.clear();
                            self.dirty = false;
                            self.revision += 1;
                            self.zoom = 1.;
                            self.pan = Vec2::ZERO;
                            self.draft = self.project.machine.clone();
                            self.config_open = !self.project.machine.configured;
                            self.recovery_available = false;
                            self.recovery_enabled = true;
                            let assets = self.project.assets.clone();
                            let tx = self.tx.clone();
                            let ctx = ctx.clone();
                            std::thread::spawn(move || {
                                for a in assets {
                                    let _ = tx
                                        .send(WorkEvent::Texture(a.id, import::preview(&a, 1024)));
                                    ctx.request_repaint();
                                }
                            });
                            self.log(tr("Project opened."));
                        }
                        Err(e) => self.log(e),
                    }
                }
                WorkEvent::Saved(result, path, revision, recovery) => {
                    self.busy = None;
                    match result {
                        Ok(()) => {
                            if !recovery {
                                self.path = Some(path);
                                if revision == self.revision {
                                    self.dirty = false;
                                    // The saved project now supersedes the automatic copy.
                                    if let Some(recovery) = &self.recovery_file {
                                        if let Err(e) = storage::discard_recovery(recovery) {
                                            self.log(trf!(
                                                "Could not remove the recovery copy: {error}",
                                                error = e
                                            ));
                                        }
                                    }
                                }
                                self.log(tr("Project saved."));
                                if let Some(action) = self.after_save.take() {
                                    self.replace(action, ctx);
                                }
                            } else {
                                self.recovery_revision = revision;
                            }
                        }
                        Err(e) => {
                            self.after_save = None;
                            self.log(trf!("Could not save: {error}", error = e));
                        }
                    }
                }
                WorkEvent::Progress(progress) => {
                    if let Some(b) = &mut self.busy {
                        b.progress = progress;
                    }
                }
                WorkEvent::Planned(result, revision) => {
                    self.busy = None;
                    if revision == self.revision {
                        match result {
                            Ok((plan, code)) => {
                                for warning in &plan.warnings {
                                    self.log(warning.clone());
                                }
                                self.plan = Some(plan);
                                self.preview_gcode = code;
                                self.mode = Mode::Preview;
                                self.log(tr("Toolpaths ready. Check the preview before running."));
                            }
                            Err(e) => self.log(e),
                        }
                    }
                }
                WorkEvent::Exported(result) => {
                    self.busy = None;
                    match result {
                        Ok(()) => self.log(tr("G-code exported.")),
                        Err(e) => self.log(e),
                    }
                }
                WorkEvent::Finished => self.busy = None,
            }
        }
        while let Ok(event) = self.machine.rx.try_recv() {
            match event {
                Event::Snapshot(s) => {
                    if let Some(job) = &mut self.job {
                        if job.finished.is_none() {
                            if matches!(s.state, State::Running | State::Paused)
                                && !self.cancel_to_edit
                            {
                                job.observed = true;
                                job.accepted = s.accepted;
                                job.total = s.total;
                                job.status = if s.state == State::Paused {
                                    tr("Paused")
                                } else {
                                    tr("Engraving / cutting in progress")
                                }
                                .into();
                            }
                            if s.physical_complete
                                || ((job.observed || self.cancel_to_edit)
                                    && matches!(
                                        s.state,
                                        State::Ready
                                            | State::Alarm
                                            | State::Fault
                                            | State::Disconnected
                                    ))
                            {
                                job.finished = Some(Instant::now());
                                job.status = if s.physical_complete {
                                    tr("Complete · controller idle")
                                } else {
                                    tr("Interrupted · job not completed")
                                }
                                .into();
                            }
                        }
                    }
                    if self.cancel_to_edit
                        && (self.cancel_acknowledged
                            || matches!(s.state, State::Fault | State::Disconnected))
                        && matches!(
                            s.state,
                            State::Ready | State::Alarm | State::Fault | State::Disconnected
                        )
                    {
                        self.mode = Mode::Prepare;
                        self.cancel_to_edit = false;
                        self.log(tr("Interruption complete. Edit the objects in the Prepare tab; set the reference again before the next run."));
                    }
                    self.snapshot = s;
                }
                Event::CancellationStarted => self.cancel_acknowledged = true,
                Event::Log(s) => self.log(s),
            }
        }
    }
    pub(super) fn refresh_ports(&mut self, ctx: &egui::Context) {
        if self.ports_refreshing {
            return;
        }
        self.ports_refreshing = true;
        self.last_ports_scan = Instant::now();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(WorkEvent::Ports(grbl::available_ports()));
            ctx.request_repaint();
        });
    }
    pub(super) fn connect(&mut self) {
        self.project.machine.port = grbl::normalize_port(&self.project.machine.port);
        if self.persist_device && !self.simulator {
            let mut candidate = self.saved_machine.clone();
            candidate.port = self.project.machine.port.clone();
            candidate.baud = self.project.machine.baud;
            match storage::save_device(&self.device_settings_path, &candidate) {
                Ok(()) => self.saved_machine = candidate,
                Err(e) => self.log(trf!("Could not save the connection: {error}", error = e)),
            }
        }
        self.snapshot.state = State::Connecting;
        self.snapshot.message = if self.simulator {
            tr("Connecting to the simulator…").into()
        } else {
            trf!(
                "Connecting to {port} at {baud} baud…",
                port = self.project.machine.port,
                baud = self.project.machine.baud
            )
        };
        self.machine.send(Command::Connect(
            self.project.machine.clone(),
            self.simulator,
        ));
    }
    pub(super) fn stop_and_edit(&mut self) {
        self.cancel_to_edit = true;
        self.cancel_acknowledged = false;
        self.snapshot.state = State::Connecting;
        self.snapshot.physical_complete = false;
        self.snapshot.message = tr("Stopping to return to editing…").into();
        self.machine.send(Command::Cancel);
    }
    pub(super) fn import_files(&mut self, paths: Vec<PathBuf>, ctx: &egui::Context) {
        if paths.is_empty() || self.locked() {
            return;
        }
        let cancel = self.start_work(tr("Importing files"));
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let n = paths.len();
            for (i, path) in paths.into_iter().enumerate() {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let result = import::read_file(&path)
                    .and_then(|a| import::preview(&a, 1024).map(|img| (a, img)));
                let _ = tx.send(WorkEvent::Imported(result));
                let _ = tx.send(WorkEvent::Progress((i + 1) as f32 / n as f32));
                ctx.request_repaint();
            }
            let _ = tx.send(WorkEvent::Finished);
            ctx.request_repaint();
        });
    }
    pub(super) fn add_shape(&mut self, kind: Shape, ctx: &egui::Context) {
        let cancel = self.start_work(tr("Creating shape"));
        let tx = self.tx.clone();
        let text = self.text_input.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            if !cancel.load(Ordering::Relaxed) {
                let result = import::shape(kind, &text)
                    .and_then(|a| import::preview(&a, 1024).map(|img| (a, img)));
                let _ = tx.send(WorkEvent::Imported(result));
            }
            let _ = tx.send(WorkEvent::Finished);
            ctx.request_repaint();
        });
    }
    pub(super) fn save(&mut self, ctx: &egui::Context, recovery: bool) -> bool {
        let path = if recovery {
            self.recovery_file.clone()
        } else {
            self.path.clone().or_else(|| {
                rfd::FileDialog::new()
                    .add_filter(tr("LaserCat project"), &[storage::PROJECT_EXTENSION])
                    .set_file_name(format!("{}.lcc", tr("My job")))
                    .save_file()
            })
        };
        let path = path.map(|path| {
            if recovery {
                path
            } else {
                storage::project_save_path(path)
            }
        });
        let Some(path) = path else {
            return false;
        };
        self.start_work(if recovery {
            tr("Saving recovery copy")
        } else {
            tr("Saving project")
        });
        let mut p = self.project.clone();
        // Undo history may still reference deleted assets; only the written archive is pruned.
        p.prune_unused_assets();
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        let rev = self.revision;
        std::thread::spawn(move || {
            let result = if recovery {
                std::fs::create_dir_all(path.parent().unwrap())
                    .map_err(|e| e.to_string())
                    .and_then(|_| storage::save(&path, &p))
            } else {
                storage::save(&path, &p)
            };
            let _ = tx.send(WorkEvent::Saved(result, path, rev, recovery));
            ctx.request_repaint();
        });
        true
    }
    pub(super) fn replace(&mut self, action: Replace, ctx: &egui::Context) {
        match action {
            Replace::New => {
                self.project = Project {
                    machine: self.project.machine.clone(),
                    ..Default::default()
                };
                self.selection.clear();
                self.textures.clear();
                self.masks.clear();
                self.plan = None;
                self.preview_gcode.clear();
                self.path = None;
                self.undo.clear();
                self.redo.clear();
                self.dirty = false;
                self.zoom = 1.;
                self.pan = Vec2::ZERO;
            }
            Replace::Open(path) => {
                if self.snapshot.state != State::Disconnected {
                    self.machine.send(Command::Disconnect);
                }
                self.start_work(tr("Opening project"));
                let tx = self.tx.clone();
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    let result = storage::load(&path);
                    let _ = tx.send(WorkEvent::Loaded(result, path));
                    ctx.request_repaint();
                });
            }
            Replace::Close => {
                self.machine.send(Command::Cancel);
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
    pub(super) fn request_replace(&mut self, action: Replace, ctx: &egui::Context) {
        if self.dirty {
            self.pending = Some(action);
        } else {
            self.replace(action, ctx);
        }
    }
    pub(super) fn plan(&mut self, ctx: &egui::Context) {
        let cancel = self.start_work(tr("Calculating toolpaths"));
        let p = self.project.clone();
        let rev = self.revision;
        self.preview_smax = self.snapshot.smax.unwrap_or(1000.);
        let smax = self.preview_smax;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = planner::plan(&p, &cancel, |v| {
                let _ = tx.send(WorkEvent::Progress(v));
                ctx.request_repaint();
            })
            .and_then(|plan| plan.gcode(smax).map(|code| (plan, code)));
            let _ = tx.send(WorkEvent::Planned(result, rev));
            ctx.request_repaint();
        });
    }
    pub(super) fn undo(&mut self) {
        if let Some(mut p) = self.undo.pop_back() {
            p.machine = self.project.machine.clone();
            self.redo.push_back(std::mem::replace(&mut self.project, p));
            self.selection.clear();
            self.plan = None;
            self.preview_gcode.clear();
            self.dirty = true;
            self.revision += 1;
        }
    }
    pub(super) fn redo(&mut self) {
        if let Some(mut p) = self.redo.pop_back() {
            p.machine = self.project.machine.clone();
            self.undo.push_back(std::mem::replace(&mut self.project, p));
            self.selection.clear();
            self.plan = None;
            self.preview_gcode.clear();
            self.dirty = true;
            self.revision += 1;
        }
    }
    pub(super) fn delete(&mut self) {
        self.project
            .objects
            .retain(|o| !self.selection.contains(&o.id) || o.locked);
        self.selection.clear();
    }
    pub(super) fn duplicate(&mut self) {
        let objects: Vec<_> = self
            .project
            .objects
            .iter()
            .filter(|o| self.selection.contains(&o.id))
            .cloned()
            .collect();
        self.selection.clear();
        for mut o in objects {
            o.id = self.project.id();
            o.x += 5.;
            o.y += 5.;
            o.name = trf!("{name} copy", name = o.name);
            o.locked = false;
            self.selection.insert(o.id);
            self.project.objects.push(o);
        }
    }
    pub(super) fn pick_import(&mut self, ctx: &egui::Context) {
        if self.locked() {
            return;
        }
        if let Some(paths) = rfd::FileDialog::new()
            .add_filter(tr("SVG and images"), &["svg", "png", "jpg", "jpeg"])
            .pick_files()
        {
            self.import_files(paths, ctx);
        }
    }
    pub(super) fn start_job(&mut self) {
        if self.locked() || self.snapshot.state != State::Ready || !self.snapshot.referenced {
            return;
        }
        if let Some(plan) = self.plan.clone() {
            self.job = Some(JobMonitor {
                started: Instant::now(),
                finished: None,
                observed: false,
                accepted: 0,
                total: self.preview_gcode.len(),
                estimate: plan.estimated_seconds,
                status: tr("Starting job…").into(),
            });
            self.machine.send(Command::Start(plan));
            self.snapshot.state = State::Running;
            self.mode = Mode::Device;
        }
    }
}
