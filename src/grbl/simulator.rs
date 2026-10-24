// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

/// In-process GRBL protocol simulator: asynchronous status, delayed completion, pause and reset.
pub(super) struct Simulator {
    output: VecDeque<u8>,
    input: Vec<u8>,
    until: Instant,
    paused: bool,
    pos: [f64; 3],
}
impl Simulator {
    pub(super) fn new() -> Self {
        let mut s = Self {
            output: VecDeque::new(),
            input: vec![],
            until: Instant::now(),
            paused: false,
            pos: [0.; 3],
        };
        s.say("Grbl 1.1h ['$' for help]");
        s
    }
    fn say(&mut self, s: &str) {
        self.output.extend(s.bytes());
        self.output.extend(b"\r\n");
    }
    fn line(&mut self, s: String) {
        if s == "$$" {
            self.say("$22=1");
            self.say("$30=1000");
            self.say("$31=0");
            self.say("$32=1");
        }
        if s == "$I" {
            self.say("[VER:1.1h:SIMULATOR]");
        }
        if s == "$G" {
            self.say("[GC:G0 G54 G17 G21 G90 G94 M5 M9 T0 F0 S0]");
        }
        if s.starts_with("G0 ") || s.starts_with("G1 ") || s.starts_with("$J=") {
            for word in s.split_whitespace() {
                if let Some(v) = word.strip_prefix('X').and_then(|v| v.parse::<f64>().ok()) {
                    if s.contains("G91") {
                        self.pos[0] += v
                    } else {
                        self.pos[0] = v
                    }
                }
                if let Some(v) = word.strip_prefix('Y').and_then(|v| v.parse::<f64>().ok()) {
                    if s.contains("G91") {
                        self.pos[1] += v
                    } else {
                        self.pos[1] = v
                    }
                }
            }
            self.until = Instant::now() + Duration::from_millis(120);
        }
        if s.starts_with("G10") {
            self.pos = [0.; 3];
        }
        self.say("ok");
    }
}
impl Wire for Simulator {
    fn write(&mut self, b: &[u8]) -> Result<(), String> {
        for &b in b {
            match b {
                b'?' => {
                    let state = if self.paused {
                        "Hold:0"
                    } else if Instant::now() < self.until {
                        "Run"
                    } else {
                        "Idle"
                    };
                    self.say(&format!(
                        "<{state}|WPos:{:.3},{:.3},{:.3}|WCO:0,0,0>",
                        self.pos[0], self.pos[1], self.pos[2]
                    ));
                }
                b'!' => self.paused = true,
                b'~' => self.paused = false,
                0x18 => {
                    self.output.clear();
                    self.input.clear();
                    self.paused = false;
                    self.until = Instant::now();
                    self.say("Grbl 1.1h ['$' for help]");
                }
                b'\n' | b'\r' => {
                    if !self.input.is_empty() {
                        let s =
                            String::from_utf8_lossy(&std::mem::take(&mut self.input)).into_owned();
                        self.line(s);
                    }
                }
                _ => self.input.push(b),
            }
        }
        Ok(())
    }
    fn read(&mut self, b: &mut [u8]) -> Result<usize, String> {
        let n = b.len().min(self.output.len());
        for byte in &mut b[..n] {
            *byte = self.output.pop_front().unwrap();
        }
        Ok(n)
    }
}
