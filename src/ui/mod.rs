// SPDX-License-Identifier: AGPL-3.0-only
use crate::{
    domain::*,
    grbl::{self, Command, Event, MachineSession, Snapshot, State},
    i18n::{self, tr, Language},
    import::{self, Shape},
    planner::{self, JobPlan},
    storage, trf,
};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
mod actions;
mod device;
mod dialogs;
mod icons;
mod preview;
mod shell;
mod theme;
mod workspace;
use theme::*;
mod transform;
use icons::Icon;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    time::{Duration, Instant},
};

const ACCENT: Color32 = Color32::from_rgb(36, 115, 214);
const MUTED: Color32 = Color32::from_rgb(103, 113, 116);
const ORANGE: Color32 = Color32::from_rgb(213, 112, 55);
const HEADER: Color32 = Color32::from_rgb(48, 60, 79);
const CANVAS: Color32 = Color32::from_rgb(229, 234, 241);
const PALETTE: [Color32; 5] = [
    Color32::from_rgb(76, 205, 180),
    Color32::from_rgb(118, 164, 255),
    Color32::from_rgb(213, 147, 250),
    ORANGE,
    Color32::from_rgb(251, 133, 148),
];

enum WorkEvent {
    CalibrationSaved(Result<PathBuf, String>),
    Ports(Result<Vec<String>, String>),
    Imported(Result<(Asset, image::RgbaImage), String>),
    Loaded(Result<Project, String>, PathBuf),
    Saved(Result<(), String>, PathBuf, u64, bool),
    Planned(Result<(JobPlan, Vec<String>), String>, u64),
    Progress(f32),
    Exported(Result<(), String>),
    Texture(u64, Result<image::RgbaImage, String>),
    Finished,
}
struct Busy {
    label: String,
    progress: f32,
    cancel: Arc<AtomicBool>,
}
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Prepare,
    Preview,
    Device,
}
enum Replace {
    New,
    Open(PathBuf),
    Close,
}

struct JobMonitor {
    started: Instant,
    finished: Option<Instant>,
    observed: bool,
    accepted: usize,
    total: usize,
    estimate: f64,
    status: String,
}

enum TransformDrag {
    Resize {
        original: SceneObject,
        corner: usize,
    },
    Rotate {
        original: SceneObject,
        angle: f64,
    },
}
pub struct LaserApp {
    project: Project,
    selection: HashSet<u64>,
    textures: HashMap<u64, egui::TextureHandle>,
    masks: HashMap<u64, egui::TextureHandle>,
    undo: VecDeque<Project>,
    redo: VecDeque<Project>,
    revision: u64,
    dirty: bool,
    last_edit: Instant,
    path: Option<PathBuf>,
    mode: Mode,
    plan: Option<JobPlan>,
    busy: Option<Busy>,
    tx: Sender<WorkEvent>,
    rx: Receiver<WorkEvent>,
    machine: MachineSession,
    snapshot: Snapshot,
    logs: VecDeque<String>,
    ports: Vec<String>,
    ports_refreshing: bool,
    last_ports_scan: Instant,
    ports_error: Option<String>,
    simulator: bool,
    config_open: bool,
    license_open: bool,
    draft: MachineProfile,
    text_open: bool,
    text_input: String,
    zoom: f32,
    pan: Vec2,
    snap: bool,
    show_rapid: bool,
    jog_step: f64,
    export_smax: f64,
    pending: Option<Replace>,
    recovery_available: bool,
    recovery_enabled: bool,
    /// Autosave target; `None` for demo/test sessions so they never touch the operator's files.
    recovery_file: Option<PathBuf>,
    /// Where the language choice is saved; `None` for demo/test sessions.
    preferences_file: Option<PathBuf>,
    last_recovery: Instant,
    recovery_revision: u64,
    export_open: bool,
    after_save: Option<Replace>,
    allow_close: bool,
    cancel_to_edit: bool,
    cancel_acknowledged: bool,
    preview_gcode: Vec<String>,
    preview_smax: f64,
    last_view: Option<(Pos2, f32)>,
    position_settings: bool,
    show_grid: bool,
    sidebar_collapsed: bool,
    object_search: String,
    device_expanded: bool,
    objects_expanded: bool,
    process_expanded: bool,
    transform_open: bool,
    transform_drag: Option<TransformDrag>,
    transform_history: Option<Project>,
    transform_undo_committed: bool,
    calibration_open: bool,
    calibration: crate::calibration::Calibration,
    saved_machine: MachineProfile,
    persist_device: bool,
    device_settings_path: PathBuf,
    job: Option<JobMonitor>,
}

impl LaserApp {
    pub fn new(ctx: &egui::Context) -> Self {
        Self::with_device_path(ctx, storage::device_path())
    }
    fn with_device_path(ctx: &egui::Context, device_settings_path: PathBuf) -> Self {
        let loaded = if device_settings_path == storage::device_path() {
            storage::load_device_with_legacy(&device_settings_path, &storage::legacy_device_path())
        } else {
            storage::load_device(&device_settings_path)
        };
        let operator_files = device_settings_path == storage::device_path();
        // Apply the saved language first so startup messages already use it.
        let preferences = operator_files.then(|| {
            let loaded = storage::load_preferences(&storage::preferences_path());
            if let Ok(p) = &loaded {
                i18n::set_language(p.language);
            }
            loaded
        });
        let recovery_file = operator_files.then(storage::recovery_path);
        let recovery_available = recovery_file.is_some() && storage::recovery_source().is_some();
        let mut app = Self::with_settings(
            ctx,
            device_settings_path,
            loaded,
            recovery_file,
            recovery_available,
        );
        if let Some(loaded) = preferences {
            app.preferences_file = Some(storage::preferences_path());
            if let Err(e) = loaded {
                app.log(e);
            }
        }
        app
    }
    fn with_settings(
        ctx: &egui::Context,
        device_settings_path: PathBuf,
        loaded: Result<Option<MachineProfile>, String>,
        recovery_file: Option<PathBuf>,
        recovery_available: bool,
    ) -> Self {
        theme(ctx);
        let (tx, rx) = mpsc::channel();
        let machine = loaded
            .as_ref()
            .ok()
            .and_then(|p| p.clone())
            .unwrap_or_default();
        let mut logs = VecDeque::from([tr("Welcome to LaserCat.").into()]);
        if let Err(error) = loaded {
            logs.push_back(error);
        }
        Self {
            project: Project {
                machine: machine.clone(),
                ..Default::default()
            },
            selection: HashSet::new(),
            textures: HashMap::new(),
            masks: HashMap::new(),
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            revision: 0,
            dirty: false,
            last_edit: Instant::now() - Duration::from_secs(2),
            path: None,
            mode: Mode::Prepare,
            plan: None,
            busy: None,
            tx,
            rx,
            machine: MachineSession::new(),
            snapshot: Snapshot::default(),
            logs,
            ports: vec![],
            ports_refreshing: false,
            last_ports_scan: Instant::now() - Duration::from_secs(5),
            ports_error: None,
            simulator: false,
            config_open: !machine.configured,
            license_open: false,
            draft: machine.clone(),
            text_open: false,
            text_input: "LaserCat".into(),
            zoom: 1.,
            pan: Vec2::ZERO,
            snap: false,
            show_rapid: true,
            jog_step: 1.,
            export_smax: 1000.,
            pending: None,
            recovery_available,
            // A pending recovery copy is never overwritten before the operator decides.
            recovery_enabled: recovery_file.is_some() && !recovery_available,
            recovery_file,
            preferences_file: None,
            last_recovery: Instant::now(),
            recovery_revision: 0,
            export_open: false,
            after_save: None,
            allow_close: false,
            cancel_to_edit: false,
            cancel_acknowledged: false,
            preview_gcode: vec![],
            preview_smax: 1000.,
            last_view: None,
            position_settings: false,
            show_grid: true,
            sidebar_collapsed: false,
            object_search: String::new(),
            device_expanded: true,
            objects_expanded: true,
            process_expanded: true,
            transform_open: false,
            transform_drag: None,
            transform_history: None,
            transform_undo_committed: false,
            calibration_open: false,
            calibration: Default::default(),
            saved_machine: machine,
            persist_device: !cfg!(test),
            device_settings_path,
            job: None,
        }
    }
    /// Explicit demonstration profile; never used as the default machine configuration.
    pub fn demo(ctx: &egui::Context) -> Self {
        // Demo and UI tests never load the operator's profile or recovery files.
        let mut app = Self::with_settings(ctx, PathBuf::new(), Ok(None), None, false);
        app.persist_device = false;
        app.project.machine = MachineProfile {
            name: tr("Demo bed · simulator").into(),
            width: 300.,
            height: 200.,
            configured: true,
            ..Default::default()
        };
        let specs = [
            (Shape::Rectangle, "", 25., 30., 90., 60., OperationKind::Cut),
            (
                Shape::Text,
                "LASER",
                40.,
                48.,
                65.,
                22.,
                OperationKind::Fill,
            ),
            (Shape::Circle, "", 155., 90., 70., 60., OperationKind::Line),
        ];
        for (kind, text, x, y, w, h, operation) in specs {
            if let Ok(asset) = import::shape(kind, text) {
                let id = app.project.add_asset(asset);
                let o = app.project.objects.last_mut().unwrap();
                o.x = x;
                o.y = y;
                o.width = w;
                o.height = h;
                o.operations[0].kind = operation;
                let a = app
                    .project
                    .asset(app.project.objects.last().unwrap().asset_id)
                    .unwrap();
                if let Ok(img) = import::preview(a, 1024) {
                    app.texture(ctx, a.id, img);
                }
                app.selection.clear();
                app.selection.insert(id);
            }
        }
        app.config_open = false;
        app.recovery_available = false;
        app.simulator = true;
        app.draft = app.project.machine.clone();
        app.log(tr("Demo example. The connection is set to the simulator."));
        app
    }
    pub fn open_initial(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.config_open = false;
        self.recovery_available = false;
        self.replace(Replace::Open(path), ctx);
    }
    /// Renderable examples of each workspace; used by the standalone visual QA renderer.
    pub fn demo_with_view(ctx: &egui::Context, view: &str) -> Self {
        let mut app = Self::demo(ctx);
        if matches!(view, "preview" | "device") {
            if let Ok(plan) = planner::plan(&app.project, &AtomicBool::new(false), |_| {}) {
                app.preview_gcode = plan.gcode(app.preview_smax).unwrap_or_default();
                app.plan = Some(plan);
                app.mode = if view == "device" {
                    Mode::Device
                } else {
                    Mode::Preview
                };
            }
        }
        if view == "license" {
            app.license_open = true;
        }
        if view == "transform" {
            app.transform_open = true;
        }
        if view == "calibration" {
            app.calibration_open = true;
        }
        if view == "collapsed" {
            app.device_expanded = false;
            app.objects_expanded = false;
        }
        app
    }
}

impl LaserApp {
    pub fn draw(&mut self, ctx: &egui::Context) {
        self.transform_undo_committed = false;
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_close
            && (self.dirty || self.locked())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Replace::Close);
        }
        let before = self.project.clone();
        let revision = self.revision;
        self.poll(ctx);
        if self.last_ports_scan.elapsed() > Duration::from_secs(3)
            && matches!(self.snapshot.state, State::Disconnected | State::Fault)
        {
            self.refresh_ports(ctx);
        }
        self.shortcuts(ctx);
        self.top(ctx);
        self.status_bar(ctx);
        match self.mode {
            Mode::Prepare => {
                if !self.sidebar_collapsed {
                    self.left(ctx);
                }
                self.canvas(ctx);
            }
            Mode::Preview => {
                if !self.sidebar_collapsed {
                    self.left(ctx);
                }
                self.preview_panel(ctx);
                self.canvas(ctx);
            }
            Mode::Device => self.device(ctx),
        }
        self.dialogs(ctx);
        if self.persist_device
            && (self.project.machine.port != self.saved_machine.port
                || self.project.machine.baud != self.saved_machine.baud)
        {
            let mut candidate = self.saved_machine.clone();
            candidate.port = self.project.machine.port.clone();
            candidate.baud = self.project.machine.baud;
            if self.last_edit.elapsed() > Duration::from_millis(600) {
                match storage::save_device(&self.device_settings_path, &candidate) {
                    Ok(()) => self.saved_machine = candidate,
                    Err(error) => {
                        self.log(trf!(
                            "Could not save the connection: {error}",
                            error = error
                        ));
                        self.persist_device = false;
                    }
                }
            }
        }
        if !self.locked() && self.mode == Mode::Prepare {
            let paths = ctx.input(|i| {
                i.raw
                    .dropped_files
                    .iter()
                    .filter_map(|f| f.path.clone())
                    .collect::<Vec<_>>()
            });
            self.import_files(paths, ctx);
        }
        if self.project != before && self.revision == revision {
            if self.transform_history.is_none()
                && !self.transform_undo_committed
                && (self.last_edit.elapsed() > Duration::from_millis(450) || self.undo.is_empty())
            {
                self.undo.push_back(before);
                while self.undo.len() > 30 {
                    self.undo.pop_front();
                }
            }
            self.last_edit = Instant::now();
            self.redo.clear();
            self.revision += 1;
            self.dirty = true;
            self.plan = None;
            self.preview_gcode.clear();
            self.snapshot.physical_complete = false;
        }
        if self.recovery_enabled
            && self.recovery_file.is_some()
            && !self.recovery_available
            && self.dirty
            && self.revision != self.recovery_revision
            && self.last_recovery.elapsed() > Duration::from_secs(30)
            && !self.locked()
        {
            self.last_recovery = Instant::now();
            self.save(ctx, true);
        }
        ctx.request_repaint_after(Duration::from_millis(
            if self.snapshot.state == State::Running || self.busy.is_some() {
                33
            } else {
                150
            },
        ));
    }
}
impl eframe::App for LaserApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.draw(ctx);
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.persist_device {
            let mut candidate = self.saved_machine.clone();
            candidate.port = self.project.machine.port.clone();
            candidate.baud = self.project.machine.baud;
            if candidate != self.saved_machine {
                let _ = storage::save_device(&self.device_settings_path, &candidate);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn frame(
        app: &mut LaserApp,
        ctx: &egui::Context,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1440., 900.))),
                events,
                ..Default::default()
            },
            |ctx| app.draw(ctx),
        )
    }
    #[test]
    fn clean_save_discards_recovery_and_prunes_deleted_assets() {
        let dir = tempfile::tempdir_in("target").unwrap();
        let recovery = dir.path().join("recovery.lcc");
        let project_path = dir.path().join("work.lcc");
        let ctx = egui::Context::default();
        let mut app = LaserApp::with_settings(
            &ctx,
            dir.path().join("device.json"),
            Ok(None),
            Some(recovery.clone()),
            false,
        );
        // Without a pending copy, normal sessions autosave; demo sessions never do.
        assert!(app.recovery_enabled);
        assert!(!LaserApp::demo(&ctx).recovery_enabled);
        let demo = LaserApp::demo(&ctx);
        app.project = demo.project.clone();
        app.dirty = true;
        assert!(app.save(&ctx, true));
        pump(&mut app, &ctx, |a| a.busy.is_none());
        assert!(recovery.exists());
        let deleted = app.project.objects[0].clone();
        app.project.objects.remove(0);
        app.path = Some(project_path.clone());
        assert!(app.save(&ctx, false));
        pump(&mut app, &ctx, |a| a.busy.is_none());
        assert!(!app.dirty, "{:?}", app.logs);
        assert!(!recovery.exists());
        let saved = storage::load(&project_path).unwrap();
        assert!(saved.asset(deleted.asset_id).is_none());
        assert_eq!(saved.objects, app.project.objects);
        // The in-memory project keeps the asset so undo can restore the object.
        assert!(app.project.asset(deleted.asset_id).is_some());
    }
    #[test]
    fn stale_planner_result_cannot_replace_a_newer_preview() {
        let ctx = egui::Context::default();
        let mut app = LaserApp::demo_with_view(&ctx, "preview");
        let original = app.preview_gcode.clone();
        app.revision = 10;
        let plan = app.plan.as_ref().unwrap().clone();
        app.tx
            .send(WorkEvent::Planned(Ok((plan, vec!["STALE".into()])), 9))
            .unwrap();
        app.poll(&ctx);
        assert_eq!(app.preview_gcode, original);
        assert!(app.plan.is_some());
    }
    #[test]
    fn loading_new_project_clears_preview_and_generated_code() {
        let ctx = egui::Context::default();
        let mut app = LaserApp::demo_with_view(&ctx, "preview");
        assert!(!app.preview_gcode.is_empty());
        app.tx
            .send(WorkEvent::Loaded(
                Ok(Project::default()),
                PathBuf::from("new.lcc"),
            ))
            .unwrap();
        app.poll(&ctx);
        assert!(app.plan.is_none());
        assert!(app.preview_gcode.is_empty());
        assert!(app.project.objects.is_empty());
    }
    #[test]
    fn undoing_an_operation_change_restores_parameters_and_invalidates_preview() {
        let ctx = egui::Context::default();
        let mut app = LaserApp::demo_with_view(&ctx, "preview");
        app.mode = Mode::Prepare;
        frame(&mut app, &ctx, vec![]);
        let before = app.project.clone();
        let old = before.objects[0].operations[0].power;
        app.project.objects[0].operations[0].power = old + 1.;
        // History snapshots contain editable parameters; undo must discard the old plan.
        app.undo.push_back(before);
        app.undo();
        assert_eq!(app.project.objects[0].operations[0].power, old);
        assert!(app.plan.is_none());
        assert!(app.preview_gcode.is_empty());
    }
    #[test]
    fn device_settings_restore_without_project_or_physical_reference() {
        let dir = tempfile::tempdir_in("target").unwrap();
        let path = dir.path().join("device.json");
        let machine = MachineProfile {
            configured: true,
            width: 400.,
            height: 300.,
            port: "COM3".into(),
            ..Default::default()
        };
        storage::save_device(&path, &machine).unwrap();
        let ctx = egui::Context::default();
        let mut app = LaserApp::with_device_path(&ctx, path.clone());
        assert_eq!(app.project.machine, machine);
        assert!(!app.config_open);
        assert_eq!(app.snapshot.state, State::Disconnected);
        assert!(!app.snapshot.referenced);
        assert!(!app.snapshot.homed);
        let mut imported = Project::default();
        imported.machine.width = 999.;
        app.tx
            .send(WorkEvent::Loaded(
                Ok(imported),
                dir.path().join("project.lcc"),
            ))
            .unwrap();
        app.poll(&ctx);
        assert_eq!(app.project.machine, machine);
        app.persist_device = true;
        app.project.machine.port = "COM4".into();
        eframe::App::on_exit(&mut app, None);
        let restarted = LaserApp::with_device_path(&ctx, path);
        assert_eq!(restarted.project.machine.port, "COM4");
        assert_eq!(restarted.project.machine.width, 400.);
    }
    #[test]
    fn full_command_acceptance_does_not_mark_physical_completion() {
        let ctx = egui::Context::default();
        let mut app = LaserApp::demo_with_view(&ctx, "preview");
        app.snapshot.state = State::Ready;
        app.snapshot.referenced = true;
        app.start_job();
        // Isolate presentation events from the background simulator connection.
        let (tx, rx) = mpsc::channel();
        app.machine.rx = rx;
        tx.send(Event::Snapshot(Snapshot {
            state: State::Running,
            accepted: 20,
            total: 20,
            ..Default::default()
        }))
        .unwrap();
        app.poll(&ctx);
        let job = app.job.as_ref().unwrap();
        assert_eq!(job.accepted, 20);
        assert!(job.finished.is_none());
        tx.send(Event::Snapshot(Snapshot {
            state: State::Ready,
            accepted: 20,
            total: 20,
            physical_complete: true,
            ..Default::default()
        }))
        .unwrap();
        app.poll(&ctx);
        assert!(app.job.as_ref().unwrap().finished.is_some());
        assert!(app.job.as_ref().unwrap().status.starts_with("Complete"));
    }
    #[test]
    fn corners_resize_and_rotation_handle_edit_on_the_bed() {
        let ctx = egui::Context::default();
        let mut app = LaserApp::demo(&ctx);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        let original = app.project.objects[2].clone();
        let (origin, scale) = app.last_view.unwrap();
        let map =
            |p: Point| Pos2::new(origin.x + p.x as f32 * scale, origin.y - p.y as f32 * scale);
        let pointer = |pressed, pos| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let corner = map(transform::corners(&original)[2]);
        let end = corner + Vec2::new(45., -35.);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(corner), pointer(true, corner)],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        frame(&mut app, &ctx, vec![pointer(false, end)]);
        let resized = &app.project.objects[2];
        assert!(resized.width > original.width);
        assert!((resized.width / resized.height - original.width / original.height).abs() < 1e-8);
        assert!(transform::corners(resized)[0].distance(transform::corners(&original)[0]) < 1e-8);
        app.undo();
        assert_eq!(app.project.objects[2], original);
        app.selection.insert(original.id);
        frame(&mut app, &ctx, vec![]);
        let corners = transform::corners(&original).map(map);
        let mid = corners[2].lerp(corners[3], 0.5);
        let handle = mid - Vec2::new(0., 28.);
        let center = map(Point::new(
            original.x + original.width / 2.,
            original.y + original.height / 2.,
        ));
        let end = center + Vec2::new(100., 0.);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(handle), pointer(true, handle)],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(end)]);
        frame(&mut app, &ctx, vec![pointer(false, end)]);
        assert!((app.project.objects[2].rotation - 270.).abs() < 0.01);
        assert_eq!(app.project.objects[2].width, original.width);
        app.undo();
        assert_eq!(app.project.objects[2], original);
    }
    #[test]
    fn native_layout_and_async_planner_smoke_test() {
        let ctx = egui::Context::default();
        let mut app = LaserApp::demo(&ctx);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(out.shapes.len() > 100);
        app.plan(&ctx);
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.busy.is_some() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
            app.poll(&ctx);
        }
        assert!(app.plan.is_some(), "{:?}", app.logs);
        assert!(app.mode == Mode::Preview);
        let out = frame(&mut app, &ctx, vec![]);
        assert!(out.shapes.len() > 100);
        app.config_open = true;
        app.draft = app.project.machine.clone();
        frame(&mut app, &ctx, vec![]);
    }
    #[test]
    fn dragging_an_object_invalidates_the_preview_and_can_be_undone() {
        let ctx = egui::Context::default();
        let mut app = LaserApp::demo(&ctx);
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        app.plan = Some(planner::plan(&app.project, &AtomicBool::new(false), |_| {}).unwrap());
        let before = app.project.objects[2].x;
        let pointer = |pressed, pos| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let (origin, scale) = app.last_view.unwrap();
        let object = &app.project.objects[2];
        let a = Pos2::new(
            origin.x + (object.x + object.width / 2.) as f32 * scale,
            origin.y - (object.y + object.height / 2.) as f32 * scale,
        );
        let b = a + Vec2::new(30., 0.);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::PointerMoved(a), pointer(true, a)],
        );
        frame(&mut app, &ctx, vec![egui::Event::PointerMoved(b)]);
        frame(&mut app, &ctx, vec![pointer(false, b)]);
        assert!(app.project.objects[2].x > before);
        assert!(app.plan.is_none());
        app.undo();
        assert!((app.project.objects[2].x - before).abs() < 0.001);
    }
    fn pump(app: &mut LaserApp, ctx: &egui::Context, condition: impl Fn(&LaserApp) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(6);
        while !condition(app) && Instant::now() < deadline {
            frame(app, ctx, vec![]);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            condition(app),
            "State: {:?}; messages: {:?}",
            app.snapshot,
            app.logs
        );
    }
    #[test]
    fn stop_simulation_returns_to_prepare_without_losing_objects() {
        let ctx = egui::Context::default();
        let mut app = LaserApp::demo(&ctx);
        let original = app.project.clone();
        app.connect();
        pump(&mut app, &ctx, |a| a.snapshot.state == State::Ready);
        app.machine.send(Command::Zero);
        pump(&mut app, &ctx, |a| a.snapshot.referenced);
        app.plan(&ctx);
        pump(&mut app, &ctx, |a| a.plan.is_some());
        assert!(!app.preview_gcode.is_empty());
        app.mode = Mode::Device;
        app.machine
            .send(Command::Start(app.plan.as_ref().unwrap().clone()));
        pump(&mut app, &ctx, |a| a.snapshot.state == State::Running);
        app.machine.send(Command::Pause);
        pump(&mut app, &ctx, |a| a.snapshot.state == State::Paused);
        app.stop_and_edit();
        pump(&mut app, &ctx, |a| {
            !a.cancel_to_edit && a.snapshot.state == State::Ready
        });
        assert!(app.mode == Mode::Prepare);
        assert!(!app.locked());
        assert!(!app.snapshot.referenced);
        assert_eq!(app.project, original);
        // A second preparation/preview cycle works without reopening the project.
        app.plan(&ctx);
        pump(&mut app, &ctx, |a| a.busy.is_none());
        assert!(app.mode == Mode::Preview);
        assert!(app.plan.is_some());
    }
    #[test]
    fn preview_is_read_only_and_switching_tabs_preserves_the_project() {
        let ctx = egui::Context::default();
        let mut app = LaserApp::demo(&ctx);
        let original = app.project.clone();
        app.plan(&ctx);
        pump(&mut app, &ctx, |a| a.plan.is_some());
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: egui::Key::Delete,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
        );
        assert_eq!(app.project, original);
        app.mode = Mode::Prepare;
        frame(&mut app, &ctx, vec![]);
        assert!(app.plan.is_some());
        app.mode = Mode::Device;
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.project, original);
    }
}
