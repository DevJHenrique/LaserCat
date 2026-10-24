// SPDX-License-Identifier: AGPL-3.0-only
use crate::{domain::*, import};
use crate::{i18n::tr, trf};

#[derive(Clone)]
pub struct Calibration {
    pub min_power: f64,
    pub max_power: f64,
    pub min_speed: f64,
    pub max_speed: f64,
    pub steps: u32,
    pub cell: f64,
    pub passes: u32,
    pub cut: bool,
}
impl Default for Calibration {
    fn default() -> Self {
        Self {
            min_power: 10.,
            max_power: 100.,
            min_speed: 500.,
            max_speed: 3000.,
            steps: 5,
            cell: 8.,
            passes: 1,
            cut: false,
        }
    }
}
impl Calibration {
    pub fn build(&self, machine: MachineProfile) -> Result<Project, String> {
        machine.validate()?;
        if !(2..=10).contains(&self.steps)
            || !self.cell.is_finite()
            || !(2. ..=30.).contains(&self.cell)
            || !self.min_power.is_finite()
            || !self.max_power.is_finite()
            || !(0. ..=100.).contains(&self.min_power)
            || !(self.min_power..=100.).contains(&self.max_power)
            || !self.min_speed.is_finite()
            || !self.max_speed.is_finite()
            || self.min_speed <= 0.
            || self.max_speed < self.min_speed
            || self.max_speed > 100000.
            || !(1..=100).contains(&self.passes)
        {
            return Err(tr("Check the power, speed, and matrix size ranges.").into());
        }
        let size = 25. + self.steps as f64 * (self.cell + 3.);
        if size > machine.width || size > machine.height {
            return Err(trf!(
                "The matrix needs {size} × {size} mm of work area.",
                size = format!("{size:.0}")
            ));
        }
        let mut project = Project {
            machine,
            ..Default::default()
        };
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{0}mm" height="{0}mm" viewBox="0 0 {0} {0}"><rect width="{0}" height="{0}" fill="black"/></svg>"#,
            self.cell
        );
        let cell = import::from_bytes(format!("{}.svg", tr("Sample")), svg.into_bytes())?;
        for row in 0..self.steps {
            let speed = self.min_speed
                + (self.max_speed - self.min_speed) * row as f64 / (self.steps - 1) as f64;
            for col in 0..self.steps {
                let power = self.min_power
                    + (self.max_power - self.min_power) * col as f64 / (self.steps - 1) as f64;
                project.add_asset(cell.clone());
                let o = project.objects.last_mut().unwrap();
                o.x = 20. + col as f64 * (self.cell + 3.);
                o.y = 10. + row as f64 * (self.cell + 3.);
                o.name = format!("P{power:.0}% · V{speed:.0} mm/min");
                let mut op = Operation::new(if self.cut {
                    OperationKind::Cut
                } else {
                    OperationKind::Fill
                });
                op.power = power;
                op.speed = speed;
                op.passes = self.passes;
                o.operations = vec![op];
            }
        }
        // Labels are embedded outlines; the saved file remains portable.
        let mut labels = String::new();
        for i in 0..self.steps {
            let x = 20. + i as f64 * (self.cell + 3.);
            let power = self.min_power
                + (self.max_power - self.min_power) * i as f64 / (self.steps - 1) as f64;
            let speed = self.min_speed
                + (self.max_speed - self.min_speed) * i as f64 / (self.steps - 1) as f64;
            labels.push_str(&format!(
                r#"<text x="{x}" y="5">{power:.0}%</text><text x="1" y="{}">{speed:.0}</text>"#,
                size - 10. - i as f64 * (self.cell + 3.) - self.cell / 2.
            ));
        }
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{size}mm" height="{size}mm" viewBox="0 0 {size} {size}"><g font-family="Arial" font-size="2.5" fill="black">{labels}</g></svg>"#
        );
        project.add_asset(import::from_bytes(
            format!("{}.svg", tr("Legend · power % / speed mm-min")),
            svg.into_bytes(),
        )?);
        let labels = project.objects.last_mut().unwrap();
        labels.x = 0.;
        labels.y = 0.;
        labels.operations[0].power = self.min_power;
        labels.operations[0].speed = self.max_speed;
        project.validate()?;
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calibration_cells_keep_independent_parameters_and_fit() {
        let machine = MachineProfile {
            configured: true,
            width: 100.,
            height: 100.,
            ..Default::default()
        };
        for cut in [false, true] {
            let p = Calibration {
                cut,
                ..Default::default()
            }
            .build(machine.clone())
            .unwrap();
            assert_eq!(p.objects.len(), 26);
            assert_eq!(p.objects[0].operations[0].power, 10.);
            assert_eq!(p.objects[24].operations[0].power, 100.);
            assert_eq!(p.objects[0].operations[0].speed, 500.);
            assert_eq!(p.objects[24].operations[0].speed, 3000.);
            assert_eq!(
                p.objects[0].operations[0].kind,
                if cut {
                    OperationKind::Cut
                } else {
                    OperationKind::Fill
                }
            );
            let plan = crate::planner::plan(&p, &std::sync::atomic::AtomicBool::new(false), |_| {})
                .unwrap();
            let code = plan.gcode(1000.).unwrap().join("\n");
            assert!(code.contains("F500.0") && code.contains("F3000.0"));
            let dir = tempfile::tempdir_in("target").unwrap();
            let path = dir.path().join("matrix.lcc");
            crate::storage::save(&path, &p).unwrap();
            assert_eq!(crate::storage::load(&path).unwrap(), p);
        }
        assert!(Calibration::default()
            .build(MachineProfile {
                width: 40.,
                ..machine
            })
            .is_err());
    }
}
