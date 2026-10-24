// SPDX-License-Identifier: AGPL-3.0-only
use crate::{
    domain::{MachineProfile, Positioning},
    planner::JobPlan,
};
use crate::{i18n::tr, trf};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant},
};

pub fn normalize_port(port: &str) -> String {
    let trimmed = port.trim();
    if cfg!(windows) {
        trimmed
            .strip_prefix(r"\\.\")
            .unwrap_or(trimmed)
            .to_ascii_uppercase()
    } else {
        trimmed.into()
    }
}

pub fn available_ports() -> Result<Vec<String>, String> {
    let enumerated = serialport::available_ports()
        .map(|ports| ports.into_iter().map(|p| p.port_name).collect::<Vec<_>>())
        .map_err(|e| e.to_string());
    #[cfg(windows)]
    let fallback = windows_registry_ports();
    #[cfg(not(windows))]
    let fallback: Result<Vec<String>, String> = Ok(vec![]);
    if let (Err(a), Err(b)) = (&enumerated, &fallback) {
        return Err(trf!(
            "Could not list ports: {first}; {second}. Enter the port manually.",
            first = a,
            second = b
        ));
    }
    let mut ports: Vec<_> = enumerated
        .unwrap_or_default()
        .into_iter()
        .chain(fallback.unwrap_or_default())
        .map(|p| normalize_port(&p))
        .collect();
    ports.sort_by(|a, b| a.len().cmp(&b.len()).then(a.cmp(b)));
    ports.dedup();
    Ok(ports)
}

#[cfg(windows)]
fn windows_registry_ports() -> Result<Vec<String>, String> {
    use windows_sys::Win32::{
        Foundation::{ERROR_NO_MORE_ITEMS, ERROR_SUCCESS},
        System::Registry::*,
    };
    let path: Vec<u16> = "HARDWARE\\DEVICEMAP\\SERIALCOMM\0".encode_utf16().collect();
    let mut key = std::ptr::null_mut();
    // Read-only fallback for ports that SetupAPI/USB enumeration does not report.
    let result = unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, path.as_ptr(), 0, KEY_READ, &mut key) };
    if result != ERROR_SUCCESS {
        return Err(trf!(
            "Port registry unavailable (Windows {code})",
            code = result
        ));
    }
    let result = (|| {
        let mut ports = vec![];
        for index in 0..4096 {
            let mut name = [0u16; 512];
            let mut name_len = name.len() as u32;
            let mut bytes = [0u8; 2048];
            let mut bytes_len = bytes.len() as u32;
            let mut kind = 0;
            let status = unsafe {
                RegEnumValueW(
                    key,
                    index,
                    name.as_mut_ptr(),
                    &mut name_len,
                    std::ptr::null_mut(),
                    &mut kind,
                    bytes.as_mut_ptr(),
                    &mut bytes_len,
                )
            };
            if status == ERROR_NO_MORE_ITEMS {
                break;
            }
            if status != ERROR_SUCCESS {
                return Err(trf!("Failed to read ports (Windows {code})", code = status));
            }
            if kind == REG_SZ {
                let utf16: Vec<_> = bytes[..bytes_len as usize]
                    .chunks_exact(2)
                    .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    .take_while(|c| *c != 0)
                    .collect();
                let port = String::from_utf16_lossy(&utf16);
                if port.starts_with("COM") {
                    ports.push(port);
                }
            }
        }
        Ok(ports)
    })();
    unsafe {
        RegCloseKey(key);
    }
    result
}

fn validate_connection(profile: &MachineProfile, simulator: bool) -> Result<(), String> {
    if !(9600..=921_600).contains(&profile.baud) {
        return Err(tr("Select a valid serial speed (GRBL default: 115200 baud).").into());
    }
    if !simulator && profile.port.is_empty() {
        return Err(tr("Select or enter the machine's port, for example COM3.").into());
    }
    Ok(())
}

fn connection_error(port: &str, baud: u32, error: serialport::Error) -> String {
    let explanation=match error.kind(){
        serialport::ErrorKind::Io(std::io::ErrorKind::PermissionDenied)=>tr("The port may be in use by LaserGRBL or another program, or access was denied. Disconnect LaserGRBL and try again."),
        serialport::ErrorKind::NoDevice=>tr("The port is unavailable. Disconnect LaserGRBL or any other program using this port; also check the USB cable, power, and the selected port."),
        _=>tr("Check the USB cable, the driver, and whether another program is using this port."),
    };
    trf!(
        "Could not connect to {port} at {baud} baud. {explanation} Detail: {error}",
        port = port,
        baud = baud,
        explanation = explanation,
        error = error
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Disconnected,
    Connecting,
    Ready,
    Running,
    Paused,
    Alarm,
    Fault,
}
impl State {
    pub fn label(self) -> &'static str {
        match self {
            Self::Disconnected => tr("Disconnected"),
            Self::Connecting => tr("Identifying GRBL"),
            Self::Ready => tr("Ready"),
            Self::Running => tr("Running"),
            Self::Paused => tr("Paused"),
            Self::Alarm => tr("Alarm"),
            Self::Fault => tr("Fault"),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub state: State,
    pub referenced: bool,
    pub homed: bool,
    pub homing_enabled: Option<bool>,
    pub verified: bool,
    pub smax: Option<f64>,
    pub smin: Option<f64>,
    pub laser: bool,
    pub controller: String,
    pub position: Option<[f64; 3]>,
    pub accepted: usize,
    pub total: usize,
    pub physical_complete: bool,
    pub message: String,
}
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            state: State::Disconnected,
            referenced: false,
            homed: false,
            homing_enabled: None,
            verified: false,
            smax: None,
            smin: None,
            laser: false,
            controller: String::new(),
            position: None,
            accepted: 0,
            total: 0,
            physical_complete: false,
            message: String::new(),
        }
    }
}
pub enum Command {
    Connect(MachineProfile, bool),
    Disconnect,
    Start(JobPlan),
    Frame(JobPlan),
    Jog(f64, f64),
    Zero,
    ReferenceTable,
    Home,
    ReturnOrigin,
    Unlock,
    Pause,
    Resume,
    Cancel,
    Shutdown,
}
pub enum Event {
    CancellationStarted,
    Snapshot(Snapshot),
    Log(String),
}

mod protocol;
pub use protocol::{Decoder, Stream};

trait Wire: Send {
    fn write(&mut self, b: &[u8]) -> Result<(), String>;
    fn read(&mut self, b: &mut [u8]) -> Result<usize, String>;
}
struct Serial(Box<dyn serialport::SerialPort>);
impl Wire for Serial {
    fn write(&mut self, b: &[u8]) -> Result<(), String> {
        self.0.write_all(b).map_err(|e| e.to_string())
    }
    fn read(&mut self, b: &mut [u8]) -> Result<usize, String> {
        match self.0.read(b) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Ok(0),
            Err(e) => Err(e.to_string()),
        }
    }
}

mod simulator;
use simulator::Simulator;

pub struct MachineSession {
    pub tx: Sender<Command>,
    pub rx: Receiver<Event>,
    handle: Option<thread::JoinHandle<()>>,
}
impl Default for MachineSession {
    fn default() -> Self {
        Self::new()
    }
}
impl MachineSession {
    pub fn new() -> Self {
        let (tx, commands) = mpsc::channel();
        let (events, rx) = mpsc::channel();
        let handle = thread::spawn(move || Worker::new(events).run(commands));
        Self {
            tx,
            rx,
            handle: Some(handle),
        }
    }
    pub fn send(&self, c: Command) {
        let _ = self.tx.send(c);
    }
}
impl Drop for MachineSession {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Task {
    Handshake,
    Job,
    Frame,
    Jog,
    Zero,
    Home,
    Unlock,
    ReturnOrigin,
}
struct Worker {
    wire: Option<Box<dyn Wire>>,
    events: Sender<Event>,
    snapshot: Snapshot,
    stream: Stream,
    decoder: Decoder,
    profile: MachineProfile,
    task: Option<Task>,
    last_rx: Instant,
    last_poll: Instant,
    wco: Option<[f64; 3]>,
    mpos: Option<[f64; 3]>,
    status: String,
    query_epochs: VecDeque<bool>,
    resuming: bool,
    task_started: Instant,
    zero_on_connect: bool,
}
impl Worker {
    fn new(events: Sender<Event>) -> Self {
        Self {
            wire: None,
            events,
            snapshot: Snapshot::default(),
            stream: Stream::default(),
            decoder: Decoder::default(),
            profile: MachineProfile::default(),
            task: None,
            last_rx: Instant::now(),
            last_poll: Instant::now(),
            wco: None,
            mpos: None,
            status: String::new(),
            query_epochs: VecDeque::new(),
            resuming: false,
            task_started: Instant::now(),
            zero_on_connect: false,
        }
    }
    fn publish(&self) {
        let _ = self.events.send(Event::Snapshot(self.snapshot.clone()));
    }
    fn log(&self, s: impl Into<String>) {
        let _ = self.events.send(Event::Log(s.into()));
    }
    fn write(&mut self, b: &[u8]) -> Result<(), String> {
        self.wire.as_mut().ok_or(tr("Not connected."))?.write(b)
    }
    fn fail(&mut self, s: String) {
        let _ = self.write(&[0x18]);
        self.stream.clear();
        self.task = None;
        self.snapshot.state = State::Fault;
        self.snapshot.referenced = false;
        self.snapshot.homed = false;
        self.snapshot.physical_complete = false;
        self.snapshot.message = s.clone();
        self.wire = None;
        self.log(s);
        self.publish();
    }
    fn run(mut self, commands: Receiver<Command>) {
        loop {
            match commands.recv_timeout(Duration::from_millis(5)) {
                Ok(Command::Shutdown) => {
                    let _ = self.write(&[0x18]);
                    break;
                }
                Ok(c) => {
                    let connecting = matches!(&c, Command::Connect(..));
                    if let Err(e) = self.command(c) {
                        if connecting && self.wire.is_none() {
                            self.snapshot.state = State::Fault;
                            self.snapshot.message = e.clone();
                            self.publish();
                        }
                        self.log(e);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let _ = self.write(&[0x18]);
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if self.wire.is_some() {
                if let Err(e) = self.tick() {
                    self.fail(e);
                }
            }
        }
    }
    fn begin(&mut self, lines: Vec<String>, task: Task) -> Result<(), String> {
        self.stream.begin(lines)?;
        self.task = Some(task);
        self.task_started = Instant::now();
        self.snapshot.accepted = 0;
        self.snapshot.total = self.stream.total;
        self.snapshot.physical_complete = false;
        self.snapshot.state = if task == Task::Handshake {
            State::Connecting
        } else {
            State::Running
        };
        self.publish();
        Ok(())
    }
    fn idle(&self) -> Result<(), String> {
        if self.snapshot.state != State::Ready
            || !self.snapshot.verified
            || self.task.is_some()
            || self.status != "Idle"
        {
            return Err(tr("The machine must be identified and idle.").into());
        }
        Ok(())
    }
    fn reference(&self) -> Result<(), String> {
        self.profile.validate()?;
        self.idle()?;
        if !self.snapshot.referenced {
            return Err(tr("Set the bed reference before running.").into());
        }
        Ok(())
    }
    fn command(&mut self, c: Command) -> Result<(), String> {
        match c {
            Command::Connect(mut profile, simulator) => {
                if self.wire.is_some() {
                    return Err(tr("Disconnect before changing the machine.").into());
                }
                profile.port = normalize_port(&profile.port);
                validate_connection(&profile, simulator)?;
                self.wire = Some(if simulator {
                    Box::new(Simulator::new())
                } else {
                    Box::new(Serial(
                        serialport::new(&profile.port, profile.baud)
                            .data_bits(serialport::DataBits::Eight)
                            .parity(serialport::Parity::None)
                            .stop_bits(serialport::StopBits::One)
                            .flow_control(serialport::FlowControl::None)
                            .timeout(Duration::from_millis(10))
                            .open()
                            .map_err(|e| connection_error(&profile.port, profile.baud, e))?,
                    ))
                });
                self.zero_on_connect = profile.positioning == Positioning::UserOrigin;
                self.profile = profile;
                self.snapshot = Snapshot {
                    state: State::Connecting,
                    ..Default::default()
                };
                self.decoder = Decoder::default();
                self.stream.clear();
                self.task = None;
                self.wco = None;
                self.mpos = None;
                self.status.clear();
                self.query_epochs.clear();
                self.resuming = false;
                self.last_rx = Instant::now();
                self.last_poll = Instant::now();
                self.publish();
            }
            Command::Disconnect => {
                if self.task.is_some() {
                    self.write(&[0x18])?;
                }
                self.wire = None;
                self.task = None;
                self.stream.clear();
                self.snapshot = Snapshot::default();
                self.publish();
            }
            Command::Start(plan) => {
                self.reference()?;
                if plan.machine != self.profile {
                    return Err(tr(
                        "The profile changed; reconnect and generate the toolpaths again.",
                    )
                    .into());
                }
                if !self.snapshot.laser || self.snapshot.smin != Some(0.) {
                    return Err(tr("Requires GRBL laser mode ($32=1) and minimum power $31=0. Change these deliberately in the firmware.").into());
                }
                let lines = plan.gcode(self.snapshot.smax.ok_or(tr("Could not read $30."))?)?;
                self.begin(lines, Task::Job)?;
            }
            Command::Frame(plan) => {
                self.reference()?;
                if plan.machine != self.profile {
                    return Err(tr("The profile changed; reconnect.").into());
                }
                self.begin(plan.frame(), Task::Frame)?;
            }
            Command::Jog(x, y) => {
                self.profile.validate()?;
                self.idle()?;
                if !x.is_finite() || !y.is_finite() || x.abs() > 10. || y.abs() > 10. {
                    return Err(tr("Invalid jog step.").into());
                }
                if self.snapshot.referenced {
                    let p = self.snapshot.position.ok_or(tr("Unknown work position."))?;
                    if p[0] + x < 0.
                        || p[1] + y < 0.
                        || p[0] + x > self.profile.width
                        || p[1] + y > self.profile.height
                    {
                        return Err(tr("Movement outside the bed.").into());
                    }
                }
                self.begin(
                    vec!["M5".into(), format!("$J=G91 G21 X{x:.3} Y{y:.3} F600")],
                    Task::Jog,
                )?;
            }
            Command::Zero | Command::ReferenceTable => {
                self.profile.validate()?;
                self.idle()?;
                if self.profile.positioning == Positioning::Absolute && !self.snapshot.homed {
                    return Err(tr("Run homing before referencing the absolute bed.").into());
                }
                self.snapshot.referenced = false;
                self.wco = None;
                self.begin(
                    vec!["M5".into(), "G54".into(), "G10 L20 P1 X0 Y0".into()],
                    Task::Zero,
                )?;
            }
            Command::Home => {
                self.profile.validate()?;
                if !(self.snapshot.state == State::Alarm
                    && self.snapshot.verified
                    && !self.stream.active())
                {
                    self.idle()?;
                }
                if !self.profile.homing {
                    return Err(tr("Homing is not configured in this profile.").into());
                }
                if self.snapshot.homing_enabled != Some(true) {
                    return Err(tr("Homing is not enabled on the controller ($22=0 or not reported). Use Return to origin after setting the work zero.").into());
                }
                self.snapshot.referenced = false;
                self.snapshot.homed = false;
                self.begin(vec!["$H".into()], Task::Home)?;
            }
            Command::ReturnOrigin => {
                self.reference()?;
                // Work origin (G54), deliberately independent of homing and display-axis inversion.
                self.begin(
                    vec![
                        "M5".into(),
                        "G21".into(),
                        "G90".into(),
                        "G54".into(),
                        "G1 X0.000 Y0.000 F1000 S0".into(),
                        "M5".into(),
                    ],
                    Task::ReturnOrigin,
                )?;
            }
            Command::Unlock => {
                if self.snapshot.state != State::Alarm
                    || !self.snapshot.verified
                    || self.stream.active()
                {
                    return Err(
                        tr("Unlock is only available after GRBL is identified in alarm.").into(),
                    );
                }
                self.snapshot.referenced = false;
                self.snapshot.homed = false;
                self.begin(vec!["$X".into()], Task::Unlock)?;
            }
            Command::Pause => {
                if self.snapshot.state == State::Running {
                    self.write(b"!")?;
                    self.snapshot.state = State::Paused;
                    self.publish();
                }
            }
            Command::Resume => {
                if self.snapshot.state == State::Paused {
                    self.write(b"~")?;
                    self.resuming = true;
                    self.snapshot.state = State::Running;
                    self.publish();
                }
            }
            Command::Cancel => {
                if self.wire.is_some() {
                    self.write(&[0x18])?;
                    let _ = self.events.send(Event::CancellationStarted);
                    self.stream.clear();
                    self.decoder = Decoder::default();
                    self.query_epochs.clear();
                    self.resuming = false;
                    self.task = None;
                    self.snapshot = Snapshot {
                        state: State::Connecting,
                        message: tr("Cancelled. The reference must be set again.").into(),
                        ..Default::default()
                    };
                    self.last_rx = Instant::now();
                    self.publish();
                }
            }
            Command::Shutdown => {}
        }
        Ok(())
    }
    fn tick(&mut self) -> Result<(), String> {
        if self.task == Some(Task::Handshake)
            && self.task_started.elapsed() > Duration::from_secs(10)
        {
            return Err(tr("Could not finish identification after interrupting/connecting. Reconnect the device; the project can still be edited.").into());
        }
        let mut buf = [0; 4096];
        let n = self.wire.as_mut().unwrap().read(&mut buf)?;
        if n > 0 {
            self.last_rx = Instant::now();
            for line in self.decoder.push(&buf[..n])? {
                self.receive(&line)?;
            }
        }
        if self.task == Some(Task::Handshake)
            || (self.snapshot.state != State::Paused && self.snapshot.state != State::Alarm)
        {
            while let Some(line) = self.stream.next_line() {
                self.write(line.as_bytes())?;
            }
        }
        if self.last_poll.elapsed() > Duration::from_millis(200) {
            self.write(b"?")?;
            self.query_epochs.push_back(self.stream.waiting_idle);
            if self.query_epochs.len() > 40 {
                return Err(tr("No response to GRBL status queries.").into());
            }
            self.last_poll = Instant::now();
            // Some USB controllers do not re-emit their startup banner when an existing port is opened.
            if self.snapshot.state == State::Connecting && self.task.is_none() {
                self.begin(vec!["$I".into(), "$$".into(), "$G".into()], Task::Handshake)?;
            }
        }
        if self.last_rx.elapsed() > Duration::from_secs(6) {
            return Err(
                tr("No response from GRBL. Execution invalidated; check the connection.").into(),
            );
        }
        Ok(())
    }
    fn receive(&mut self, line: &str) -> Result<(), String> {
        if line.starts_with("Grbl ") {
            if self.task.is_some() && self.task != Some(Task::Handshake) {
                return Err(tr("The controller restarted during the operation.").into());
            }
            self.stream.clear();
            self.query_epochs.clear();
            self.resuming = false;
            self.task = None;
            self.snapshot.referenced = false;
            self.snapshot.homed = false;
            self.snapshot.verified = line.starts_with("Grbl 1.1");
            self.snapshot.controller = line.into();
            if !self.snapshot.verified {
                return Err(tr("This version only supports GRBL 1.1.").into());
            }
            self.begin(vec!["$I".into(), "$$".into(), "$G".into()], Task::Handshake)?;
        } else if line.starts_with("[VER:") {
            self.snapshot.verified = line.starts_with("[VER:1.1");
            self.snapshot.controller = line.into();
            if !self.snapshot.verified {
                return Err(tr("Firmware is not compatible with GRBL 1.1.").into());
            }
        } else if line == "ok" {
            // Controllers may acknowledge startup blank lines or retain an initial response.
            // Only tolerate these while identifying; job acknowledgements remain strictly counted.
            if self.stream.pending.is_empty()
                && (self.snapshot.state == State::Connecting || self.task == Some(Task::Handshake))
            {
                self.log(tr(
                    "Initial acknowledgement received; identifying the controller.",
                ));
                return Ok(());
            }
            self.stream.acknowledge()?;
            self.snapshot.accepted = self.stream.accepted;
        } else if line.starts_with("error:") {
            return Err(trf!(
                "GRBL rejected a command ({line}). Job stopped.",
                line = line
            ));
        } else if line.starts_with("ALARM:") {
            if self.task != Some(Task::Handshake) {
                self.write(b"!")?;
                self.stream.clear();
                self.task = None;
            }
            self.snapshot.state = State::Alarm;
            self.snapshot.referenced = false;
            self.snapshot.homed = false;
            self.snapshot.physical_complete = false;
            self.snapshot.message = line.into();
            self.log(line);
        } else if let Some(v) = line.strip_prefix("$30=") {
            self.snapshot.smax = v.parse().ok();
        } else if let Some(v) = line.strip_prefix("$31=") {
            self.snapshot.smin = v.parse().ok();
        } else if let Some(v) = line.strip_prefix("$32=") {
            self.snapshot.laser = v == "1";
        } else if let Some(v) = line.strip_prefix("$22=") {
            self.snapshot.homing_enabled = match v {
                "1" => Some(true),
                "0" => Some(false),
                _ => None,
            };
        } else if line.starts_with('$') && line.contains('=') {
            // Vendor-specific firmware can include network credentials in $$ responses.
            // Unused settings are deliberately omitted from UI/diagnostic logs.
        } else if line.starts_with('<') && line.ends_with('>') {
            let fields: Vec<_> = line[1..line.len() - 1].split('|').collect();
            self.status = fields[0].into();
            self.stream.queried_after_ack = self.query_epochs.pop_front().unwrap_or(false);
            let mut wpos = None;
            for f in &fields[1..] {
                if let Some(v) = f.strip_prefix("WPos:") {
                    wpos = parse_position(v);
                }
                if let Some(v) = f.strip_prefix("MPos:") {
                    self.mpos = parse_position(v);
                }
                if let Some(v) = f.strip_prefix("WCO:") {
                    self.wco = parse_position(v);
                }
            }
            self.snapshot.position = wpos.or_else(|| {
                self.mpos
                    .zip(self.wco)
                    .map(|(p, o)| [p[0] - o[0], p[1] - o[1], p[2] - o[2]])
            });
            if self.status.starts_with("Alarm")
                && self.task != Some(Task::Home)
                && self.task != Some(Task::Unlock)
            {
                if self.task != Some(Task::Handshake) {
                    self.stream.clear();
                    self.task = None;
                }
                self.snapshot.state = State::Alarm;
                self.snapshot.referenced = false;
                self.snapshot.homed = false;
                self.snapshot.physical_complete = false;
            }
            if (self.status.starts_with("Hold")
                && !self.resuming
                && self.task != Some(Task::Handshake)
                && self.snapshot.state != State::Connecting)
                || self.status.starts_with("Door")
            {
                self.snapshot.state = State::Paused;
            }
            if self.status == "Run" {
                self.resuming = false;
                self.snapshot.state = State::Running;
            }
            let alarm_handshake =
                self.task == Some(Task::Handshake) && self.status.starts_with("Alarm");
            if self.stream.status(self.status == "Idle" || alarm_handshake) {
                let task = self.task.take();
                if task == Some(Task::Handshake)
                    && (!self.snapshot.verified
                        || self.snapshot.smax.is_none()
                        || self.snapshot.smin.is_none())
                {
                    return Err(tr("GRBL identification or settings are incomplete.").into());
                }
                if task == Some(Task::Handshake) && self.zero_on_connect {
                    self.zero_on_connect = false;
                    if !alarm_handshake {
                        self.wco = None;
                        self.begin(
                            vec!["M5".into(), "G54".into(), "G10 L20 P1 X0 Y0".into()],
                            Task::Zero,
                        )?;
                        self.snapshot.message =
                            tr("Setting the current position as the work zero, without movement.")
                                .into();
                        self.publish();
                        return Ok(());
                    }
                }
                if task == Some(Task::Zero) {
                    let p = self.snapshot.position.ok_or(tr(
                        "Could not confirm the work origin. Wait and set the zero again.",
                    ))?;
                    if p[0].abs() > 0.005 || p[1].abs() > 0.005 {
                        return Err(tr("GRBL did not confirm the bed zero.").into());
                    }
                    self.snapshot.referenced = true;
                    self.snapshot.message = tr("Current position set as work X0 Y0.").into();
                }
                if task == Some(Task::Home) {
                    self.snapshot.homed = true;
                    self.snapshot.message=tr("Homing complete. Move to the usable lower-left corner and reference the bed.").into();
                }
                if task == Some(Task::ReturnOrigin) {
                    self.snapshot.message =
                        tr("Returned to the work origin with the laser off.").into();
                    self.log(&self.snapshot.message);
                }
                self.snapshot.state = if alarm_handshake {
                    State::Alarm
                } else {
                    State::Ready
                };
                self.snapshot.physical_complete = task == Some(Task::Job);
                if self.snapshot.physical_complete {
                    self.snapshot.message = tr("Job complete: controller idle.").into();
                    self.log(&self.snapshot.message);
                }
            }
        } else {
            self.log(line);
        }
        self.publish();
        Ok(())
    }
}
fn parse_position(s: &str) -> Option<[f64; 3]> {
    let v: Vec<f64> = s
        .split(',')
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if v.len() != 3 || v.iter().any(|v| !v.is_finite()) {
        return None;
    }
    Some([v[0], v[1], v[2]])
}

#[cfg(test)]
mod tests {
    use super::*;
    fn worker() -> Worker {
        let (tx, _) = mpsc::channel();
        let mut w = Worker::new(tx);
        w.wire = Some(Box::new(Simulator::new()));
        w.snapshot.verified = true;
        w.snapshot.smax = Some(1000.);
        w.snapshot.smin = Some(0.);
        w.snapshot.homing_enabled = Some(true);
        w.profile = MachineProfile {
            configured: true,
            width: 300.,
            height: 200.,
            ..Default::default()
        };
        w
    }
    fn acknowledge_all(w: &mut Worker) {
        while w.stream.next_line().is_some() {}
        let n = w.stream.pending.len();
        for _ in 0..n {
            w.receive("ok").unwrap();
        }
    }
    #[test]
    fn stale_idle_cannot_complete_a_job() {
        let mut w = worker();
        w.begin(vec!["M5".into()], Task::Job).unwrap();
        w.query_epochs.push_back(false);
        acknowledge_all(&mut w);
        w.receive("<Idle|WPos:0,0,0>").unwrap();
        assert!(!w.snapshot.physical_complete);
        assert!(w.task.is_some());
        w.query_epochs.push_back(true);
        w.receive("<Idle|WPos:0,0,0>").unwrap();
        assert!(w.snapshot.physical_complete);
    }
    #[test]
    fn initial_connection_zeros_without_motion_and_requires_confirmation() {
        let mut w = worker();
        w.zero_on_connect = true;
        w.begin(vec!["$I".into()], Task::Handshake).unwrap();
        acknowledge_all(&mut w);
        w.query_epochs.push_back(true);
        w.receive("<Idle|WPos:25,-9,0>").unwrap();
        assert!(w.task == Some(Task::Zero));
        assert!(!w.snapshot.referenced);
        assert!(!w.zero_on_connect);
        assert_eq!(
            w.stream
                .queue
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["M5", "G54", "G10 L20 P1 X0 Y0"]
        );
        acknowledge_all(&mut w);
        w.query_epochs.push_back(true);
        w.receive("<Idle|WPos:0,0,0>").unwrap();
        assert!(w.snapshot.referenced);
        assert!(!w.snapshot.homed);
        assert_eq!(w.snapshot.state, State::Ready);
    }
    #[test]
    fn errors_and_reset_during_job_interrupt_execution() {
        let mut w = worker();
        w.begin(vec!["M5".into()], Task::Job).unwrap();
        assert!(w.receive("error:2").is_err());
        assert!(w.receive("Grbl 1.1h ['$' for help]").is_err());
        w.fail("Test failure".into());
        assert_eq!(w.snapshot.state, State::Fault);
        assert!(!w.snapshot.referenced);
        assert!(!w.stream.active());
        assert!(w.wire.is_none());
    }
    #[test]
    fn alarm_discards_queue_and_reference() {
        let mut w = worker();
        w.snapshot.referenced = true;
        w.begin(vec!["M5".into()], Task::Job).unwrap();
        w.receive("ALARM:1").unwrap();
        assert_eq!(w.snapshot.state, State::Alarm);
        assert!(!w.stream.active());
        assert!(!w.snapshot.referenced);
        assert!(!w.snapshot.physical_complete);
    }
    #[test]
    fn asynchronous_status_does_not_consume_serial_acknowledgements() {
        let mut w = worker();
        w.begin(vec!["M5".into()], Task::Job).unwrap();
        w.stream.next_line().unwrap();
        w.receive("<Run|MPos:3,4,0|WCO:1,2,0>").unwrap();
        assert_eq!(w.stream.accepted, 0);
        assert_eq!(w.snapshot.position, Some([2., 2., 0.]));
        w.receive("ok").unwrap();
        assert_eq!(w.stream.accepted, 1);
    }
    #[test]
    fn stale_hold_after_resume_does_not_pause_sender_again() {
        let mut w = worker();
        w.snapshot.state = State::Paused;
        w.command(Command::Resume).unwrap();
        w.receive("<Hold:0|WPos:0,0,0>").unwrap();
        assert_eq!(w.snapshot.state, State::Running);
        w.receive("<Run|WPos:0,0,0>").unwrap();
        assert!(!w.resuming);
    }
    #[test]
    fn origin_requires_actual_zero_confirmation() {
        let mut w = worker();
        w.begin(vec!["G10 L20 P1 X0 Y0".into()], Task::Zero)
            .unwrap();
        acknowledge_all(&mut w);
        w.query_epochs.push_back(true);
        assert!(w.receive("<Idle|WPos:10,0,0>").is_err());
        assert!(!w.snapshot.referenced);
    }
    #[test]
    fn manual_commands_cannot_be_interleaved() {
        let mut w = worker();
        w.begin(vec!["M5".into()], Task::Job).unwrap();
        assert!(w.command(Command::Jog(1., 0.)).is_err());
        assert!(w.command(Command::Zero).is_err());
        assert!(w.command(Command::Home).is_err());
        assert!(w.command(Command::ReturnOrigin).is_err());
    }
    #[test]
    fn startup_alarm_allows_identification_and_explicit_homing() {
        let mut w = worker();
        w.profile.homing = true;
        w.begin(vec!["$I".into(), "$$".into(), "$G".into()], Task::Handshake)
            .unwrap();
        acknowledge_all(&mut w);
        w.query_epochs.push_back(true);
        w.receive("<Alarm|WPos:0,0,0>").unwrap();
        assert_eq!(w.snapshot.state, State::Alarm);
        assert!(!w.stream.active());
        w.command(Command::Home).unwrap();
        assert_eq!(w.stream.next_line().unwrap(), "$H\n");
        w.receive("<Alarm|WPos:0,0,0>").unwrap();
        assert!(w.task == Some(Task::Home));
        w.receive("ok").unwrap();
        w.query_epochs.push_back(true);
        w.receive("<Idle|WPos:0,0,0>").unwrap();
        assert!(w.snapshot.homed);
        assert!(!w.snapshot.referenced);
    }
    #[test]
    fn connecting_does_not_require_a_configured_bed() {
        let profile = MachineProfile {
            port: "COM3".into(),
            ..Default::default()
        };
        assert!(profile.validate().is_err());
        assert!(validate_connection(&profile, false).is_ok());
        assert!(validate_connection(&MachineProfile::default(), false).is_err());
        assert!(validate_connection(&MachineProfile::default(), true).is_ok());
    }
    #[test]
    fn busy_port_error_explains_how_to_release_lasergrbl() {
        let error = serialport::Error::new(
            serialport::ErrorKind::Io(std::io::ErrorKind::PermissionDenied),
            "Access denied",
        );
        let message = connection_error("COM3", 115200, error);
        assert!(message.contains("COM3"));
        assert!(message.contains("LaserGRBL"));
        assert!(message.contains("115200"));
    }
    #[test]
    fn windows_port_input_is_normalized() {
        if cfg!(windows) {
            assert_eq!(normalize_port(" com3 "), "COM3");
            assert_eq!(normalize_port(r"\\.\COM3"), "COM3");
        }
    }
    #[test]
    fn startup_ok_is_tolerated_but_job_acknowledgements_remain_strict() {
        let mut w = worker();
        w.snapshot.state = State::Connecting;
        assert!(w.receive("ok").is_ok());
        w.begin(vec!["$I".into(), "$$".into()], Task::Handshake)
            .unwrap();
        assert!(w.receive("ok").is_ok());
        assert_eq!(w.stream.accepted, 0);
        w.stream.clear();
        w.task = None;
        w.begin(vec!["M5".into()], Task::Job).unwrap();
        assert!(w.receive("ok").is_err());
    }
    #[test]
    fn vendor_settings_are_not_written_to_logs() {
        let (tx, rx) = mpsc::channel();
        let mut w = Worker::new(tx);
        w.receive("$52=vendor-private-setting").unwrap();
        assert!(!rx.try_iter().any(|e| matches!(e, Event::Log(_))));
    }
    #[test]
    fn return_origin_uses_work_coordinates_and_laser_off() {
        let mut w = worker();
        w.snapshot.state = State::Ready;
        w.snapshot.referenced = true;
        w.status = "Idle".into();
        w.profile.invert_x = true;
        w.command(Command::ReturnOrigin).unwrap();
        let lines: Vec<_> = std::iter::from_fn(|| w.stream.next_line()).collect();
        assert_eq!(lines.first().unwrap(), "M5\n");
        assert_eq!(lines.last().unwrap(), "M5\n");
        assert!(lines.iter().any(|l| l == "G54\n"));
        assert!(lines.iter().any(|l| l == "G1 X0.000 Y0.000 F1000 S0\n"));
        assert!(!lines
            .iter()
            .any(|l| l.starts_with("M3") || l.starts_with("M4") || l.contains("$H")));
    }
    #[test]
    fn origin_return_requires_reference_and_home_requires_firmware_support() {
        let mut w = worker();
        w.snapshot.state = State::Ready;
        w.status = "Idle".into();
        assert!(w.command(Command::ReturnOrigin).is_err());
        w.profile.homing = true;
        w.receive("$22=0").unwrap();
        assert_eq!(w.snapshot.homing_enabled, Some(false));
        assert!(w.command(Command::Home).unwrap_err().contains("$22"));
    }
}
