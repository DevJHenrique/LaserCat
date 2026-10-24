// SPDX-License-Identifier: AGPL-3.0-only
fn main() {
    match lasercat::grbl::available_ports() {
        Ok(ports) => {
            println!("Serial ports detected: {}", ports.len());
            for port in ports {
                println!("{port}");
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
