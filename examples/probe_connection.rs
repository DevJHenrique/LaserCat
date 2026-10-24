// SPDX-License-Identifier: AGPL-3.0-only
//! Connection-only diagnostic: identifies GRBL without jogging, homing or emitting laser commands.
use lasercat::{
    domain::MachineProfile,
    grbl::{Command, Event, MachineSession, State},
};
use std::time::{Duration, Instant};
fn main() {
    let Some(port) = std::env::args().nth(1) else {
        eprintln!("Usage: cargo run --example probe_connection -- COM3 [baud]");
        std::process::exit(2);
    };
    let baud = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(115200);
    let session = MachineSession::new();
    session.send(Command::Connect(
        MachineProfile {
            port,
            baud,
            ..Default::default()
        },
        false,
    ));
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut success = false;
    while Instant::now() < deadline {
        match session.rx.recv_timeout(Duration::from_millis(250)) {
            Ok(Event::Snapshot(s)) if s.state == State::Fault => {
                eprintln!("{}", s.message);
                break;
            }
            Ok(Event::Snapshot(s))
                if s.verified && matches!(s.state, State::Ready | State::Alarm) =>
            {
                println!(
                    "Identified: {}\nState: {}\n$30={:?}, $31={:?}, $32={}",
                    s.controller,
                    s.state.label(),
                    s.smax,
                    s.smin,
                    s.laser
                );
                success = true;
                break;
            }
            Ok(Event::Log(s)) => println!("{s}"),
            _ => {}
        }
    }
    session.send(Command::Disconnect);
    // Wait until transport is closed before dropping the worker.
    let close_deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < close_deadline {
        if matches!(session.rx.recv_timeout(Duration::from_millis(100)),Ok(Event::Snapshot(s))if s.state==State::Disconnected)
        {
            break;
        }
    }
    drop(session);
    if !success {
        std::process::exit(1);
    }
}
