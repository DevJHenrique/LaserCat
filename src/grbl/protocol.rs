// SPDX-License-Identifier: AGPL-3.0-only
use crate::i18n::tr;
use std::collections::VecDeque;

/// Line-oriented response decoder. Real-time status messages do not consume acknowledgements.
#[derive(Default)]
pub struct Decoder {
    pub(super) bytes: Vec<u8>,
}
impl Decoder {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, String> {
        let mut lines = vec![];
        for &b in bytes {
            if b == b'\r' || b == b'\n' {
                if !self.bytes.is_empty() {
                    lines.push(
                        String::from_utf8_lossy(&std::mem::take(&mut self.bytes)).into_owned(),
                    );
                }
            } else {
                self.bytes.push(b);
                if self.bytes.len() > 4096 {
                    self.bytes.clear();
                    return Err(tr("The serial response exceeded the limit.").into());
                }
            }
        }
        Ok(lines)
    }
}

#[derive(Default)]
pub struct Stream {
    pub(super) queue: VecDeque<String>,
    pub(super) pending: VecDeque<usize>,
    pub(super) used: usize,
    pub accepted: usize,
    pub total: usize,
    pub waiting_idle: bool,
    pub queried_after_ack: bool,
}
impl Stream {
    pub fn begin(&mut self, lines: Vec<String>) -> Result<(), String> {
        if self.active() {
            return Err(tr("An operation is already in progress.").into());
        }
        if lines.is_empty()
            || lines
                .iter()
                .any(|l| l.len() + 1 > 127 || !l.is_ascii() || l.contains(['\r', '\n']))
        {
            return Err(tr("Invalid serial program.").into());
        }
        *self = Self {
            total: lines.len(),
            queue: lines.into(),
            ..Default::default()
        };
        Ok(())
    }
    pub fn next_line(&mut self) -> Option<String> {
        let line = self.queue.front()?;
        if self.used + line.len() + 1 > 127 {
            return None;
        }
        let line = self.queue.pop_front().unwrap();
        let n = line.len() + 1;
        self.used += n;
        self.pending.push_back(n);
        Some(line + "\n")
    }
    pub fn acknowledge(&mut self) -> Result<(), String> {
        let n = self
            .pending
            .pop_front()
            .ok_or(tr("GRBL sent an acknowledgement with no pending command."))?;
        self.used -= n;
        self.accepted += 1;
        if self.queue.is_empty() && self.pending.is_empty() {
            self.waiting_idle = true;
        }
        Ok(())
    }
    pub fn active(&self) -> bool {
        !self.queue.is_empty() || !self.pending.is_empty() || self.waiting_idle
    }
    pub fn status(&mut self, idle: bool) -> bool {
        if idle && self.waiting_idle && self.queried_after_ack {
            self.waiting_idle = false;
            return true;
        }
        false
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
