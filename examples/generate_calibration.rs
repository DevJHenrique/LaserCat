// SPDX-License-Identifier: AGPL-3.0-only
//! Generate portable templates; the physical machine must be configured on opening.
use lasercat::{calibration::Calibration, domain::MachineProfile, storage};
fn main() -> Result<(), String> {
    let directory = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "calibration".into()),
    );
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    for cut in [false, true] {
        let config = Calibration {
            cut,
            ..Default::default()
        };
        let machine = MachineProfile {
            configured: true,
            width: 100.,
            height: 100.,
            ..Default::default()
        };
        let mut project = config.build(machine)?;
        project.machine = MachineProfile::default();
        storage::save(
            &directory.join(if cut {
                "cutting-calibration.lcc"
            } else {
                "engraving-calibration.lcc"
            }),
            &project,
        )?;
    }
    Ok(())
}
