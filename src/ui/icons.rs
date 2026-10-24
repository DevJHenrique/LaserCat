// SPDX-License-Identifier: AGPL-3.0-only
//! Original line icons drawn with egui primitives, keeping the editor independent of external assets.
use eframe::egui::{self, Color32, Pos2, Rect, Response, Sense, Stroke, Vec2};

#[derive(Clone, Copy)]
pub enum Icon {
    File,
    Save,
    Undo,
    Redo,
    Box,
    Layers,
    Device,
    Settings,
    Import,
    Move,
    Rotate,
    Scale,
    Mirror,
    Duplicate,
    Delete,
    Align,
    Rectangle,
    Circle,
    Text,
    Fit,
    Grid,
    Origin,
    Lock,
}

pub fn paint(p: &egui::Painter, r: Rect, icon: Icon, color: Color32) {
    let r = r.shrink(5.);
    let a = r.min;
    let w = r.width();
    let h = r.height();
    let q = |x: f32, y: f32| Pos2::new(a.x + x * w, a.y + y * h);
    let s = Stroke::new(1.25_f32, color);
    let line = |a: Pos2, b: Pos2| {
        p.line_segment([a, b], s);
    };
    let path = |pts: Vec<Pos2>| {
        p.add(egui::Shape::line(pts, s));
    };
    let rect = |a: Pos2, b: Pos2| {
        p.rect_stroke(Rect::from_two_pos(a, b), 1., s, egui::StrokeKind::Inside);
    };
    match icon {
        Icon::File => {
            path(vec![
                q(0.2, 0.05),
                q(0.65, 0.05),
                q(0.85, 0.25),
                q(0.85, 0.95),
                q(0.2, 0.95),
                q(0.2, 0.05),
            ]);
            path(vec![q(0.65, 0.05), q(0.65, 0.25), q(0.85, 0.25)]);
        }
        Icon::Save => {
            rect(q(0.1, 0.05), q(0.9, 0.95));
            rect(q(0.3, 0.05), q(0.7, 0.4));
            rect(q(0.25, 0.6), q(0.75, 0.95));
        }
        Icon::Undo | Icon::Redo => {
            let flip = matches!(icon, Icon::Redo);
            let x = |v: f32| if flip { 1. - v } else { v };
            path(vec![
                q(x(0.25), 0.25),
                q(x(0.65), 0.25),
                q(x(0.85), 0.45),
                q(x(0.85), 0.7),
                q(x(0.65), 0.9),
                q(x(0.25), 0.9),
            ]);
            path(vec![q(x(0.45), 0.05), q(x(0.2), 0.25), q(x(0.45), 0.45)]);
        }
        Icon::Box => {
            path(vec![
                q(0.5, 0.),
                q(1., 0.25),
                q(1., 0.75),
                q(0.5, 1.),
                q(0., 0.75),
                q(0., 0.25),
                q(0.5, 0.),
            ]);
            path(vec![q(0., 0.25), q(0.5, 0.5), q(1., 0.25)]);
            line(q(0.5, 0.5), q(0.5, 1.));
        }
        Icon::Layers => {
            for y in [0.1, 0.35, 0.6] {
                path(vec![
                    q(0., y + 0.2),
                    q(0.5, y),
                    q(1., y + 0.2),
                    q(0.5, y + 0.4),
                    q(0., y + 0.2),
                ]);
            }
        }
        Icon::Device => {
            rect(q(0.1, 0.1), q(0.9, 0.8));
            line(q(0.5, 0.8), q(0.5, 1.));
            line(q(0.25, 1.), q(0.75, 1.));
            rect(q(0.3, 0.35), q(0.7, 0.65));
        }
        Icon::Settings => {
            for (x, y) in [(0.2, 0.25), (0.5, 0.65), (0.8, 0.4)] {
                line(q(x, 0.), q(x, 1.));
                rect(q(x - 0.12, y - 0.08), q(x + 0.12, y + 0.08));
            }
        }
        Icon::Import => {
            path(vec![
                q(0.15, 0.5),
                q(0.15, 0.95),
                q(0.85, 0.95),
                q(0.85, 0.5),
            ]);
            line(q(0.5, 0.), q(0.5, 0.7));
            path(vec![q(0.3, 0.5), q(0.5, 0.7), q(0.7, 0.5)]);
        }
        Icon::Move => {
            line(q(0., 0.5), q(1., 0.5));
            line(q(0.5, 0.), q(0.5, 1.));
            for (x, y, dx, dy) in [
                (0., 0.5, 0.18, 0.15),
                (1., 0.5, -0.18, 0.15),
                (0.5, 0., 0.15, 0.18),
                (0.5, 1., 0.15, -0.18),
            ] {
                if x == 0.5 {
                    path(vec![q(x - dx, y + dy), q(x, y), q(x + dx, y + dy)]);
                } else {
                    path(vec![q(x + dx, y - dy), q(x, y), q(x + dx, y + dy)]);
                }
            }
        }
        Icon::Rotate => {
            path(vec![
                q(0.85, 0.3),
                q(0.6, 0.1),
                q(0.3, 0.1),
                q(0.1, 0.35),
                q(0.1, 0.65),
                q(0.35, 0.9),
                q(0.7, 0.9),
                q(0.9, 0.65),
            ]);
            path(vec![q(0.85, 0.), q(0.85, 0.3), q(0.55, 0.3)]);
        }
        Icon::Scale => {
            rect(q(0.05, 0.35), q(0.65, 0.95));
            path(vec![q(0.4, 0.6), q(0.95, 0.05), q(0.55, 0.05)]);
            line(q(0.95, 0.05), q(0.95, 0.45));
        }
        Icon::Mirror => {
            line(q(0.5, 0.), q(0.5, 1.));
            path(vec![
                q(0.3, 0.15),
                q(0.05, 0.85),
                q(0.3, 0.85),
                q(0.3, 0.15),
            ]);
            path(vec![
                q(0.7, 0.15),
                q(0.95, 0.85),
                q(0.7, 0.85),
                q(0.7, 0.15),
            ]);
        }
        Icon::Duplicate => {
            rect(q(0.05, 0.05), q(0.7, 0.7));
            rect(q(0.3, 0.3), q(0.95, 0.95));
        }
        Icon::Delete => {
            line(q(0.1, 0.2), q(0.9, 0.2));
            line(q(0.35, 0.05), q(0.65, 0.05));
            path(vec![q(0.2, 0.2), q(0.25, 0.95), q(0.75, 0.95), q(0.8, 0.2)]);
            line(q(0.4, 0.35), q(0.4, 0.8));
            line(q(0.6, 0.35), q(0.6, 0.8));
        }
        Icon::Align => {
            line(q(0.05, 0.), q(0.05, 1.));
            for (y, x) in [(0.15, 0.8), (0.45, 0.55), (0.75, 1.)] {
                rect(q(0.2, y), q(x, y + 0.15));
            }
        }
        Icon::Rectangle => rect(q(0.05, 0.15), q(0.95, 0.85)),
        Icon::Circle => {
            p.circle_stroke(r.center(), w.min(h) * 0.45, s);
        }
        Icon::Text => {
            line(q(0.1, 0.1), q(0.9, 0.1));
            line(q(0.5, 0.1), q(0.5, 0.95));
            line(q(0.25, 0.95), q(0.75, 0.95));
        }
        Icon::Fit => {
            for (x, y, sx, sy) in [
                (0., 0., 1., 1.),
                (1., 0., -1., 1.),
                (1., 1., -1., -1.),
                (0., 1., 1., -1.),
            ] {
                path(vec![q(x, y + 0.3 * sy), q(x, y), q(x + 0.3 * sx, y)]);
            }
        }
        Icon::Grid => {
            rect(q(0., 0.), q(1., 1.));
            for x in [0.33, 0.66] {
                line(q(x, 0.), q(x, 1.));
                line(q(0., x), q(1., x));
            }
        }
        Icon::Origin => {
            p.circle_stroke(r.center(), w * 0.3, s);
            line(q(0., 0.5), q(1., 0.5));
            line(q(0.5, 0.), q(0.5, 1.));
        }
        Icon::Lock => {
            rect(q(0.15, 0.45), q(0.85, 0.95));
            path(vec![
                q(0.3, 0.45),
                q(0.3, 0.15),
                q(0.5, 0.02),
                q(0.7, 0.15),
                q(0.7, 0.45),
            ]);
        }
    }
}
pub fn button(
    ui: &mut egui::Ui,
    icon: Icon,
    tooltip: &str,
    enabled: bool,
    selected: bool,
) -> Response {
    let enabled = enabled && ui.is_enabled();
    let (r, response) = ui.allocate_exact_size(
        Vec2::splat(32.),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    let color = if !enabled {
        Color32::from_gray(175)
    } else if selected {
        super::ACCENT
    } else {
        Color32::from_rgb(65, 76, 78)
    };
    if selected || response.hovered() {
        ui.painter().rect_filled(
            r.shrink(1.),
            3.,
            if selected {
                Color32::from_rgb(210, 235, 230)
            } else {
                Color32::from_gray(216)
            },
        );
    }
    paint(ui.painter(), r, icon, color);
    response.on_hover_text(tooltip)
}
