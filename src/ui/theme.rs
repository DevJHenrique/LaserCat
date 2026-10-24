// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

pub(super) fn field(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    suffix: &str,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::DragValue::new(value)
                .range(range)
                .speed(0.1)
                .suffix(suffix),
        );
    });
}
pub(super) fn theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(7., 7.);
    style.spacing.button_padding = Vec2::new(9., 5.);
    style.spacing.interact_size = Vec2::new(32., 26.);
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(13.));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(13.));
    let mut v = egui::Visuals::light();
    v.panel_fill = Color32::from_rgb(248, 248, 248);
    v.window_fill = Color32::from_rgb(248, 248, 248);
    v.extreme_bg_color = Color32::WHITE;
    v.faint_bg_color = Color32::from_gray(241);
    v.override_text_color = Some(Color32::from_rgb(35, 45, 62));
    v.selection.bg_fill = Color32::from_rgb(206, 225, 248);
    v.selection.stroke = Stroke::new(1.0_f32, Color32::from_rgb(24, 85, 161));
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_gray(207));
    v.widgets.inactive.bg_fill = Color32::from_rgb(238, 238, 238);
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(238, 238, 238);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_gray(202));
    v.widgets.inactive.corner_radius = egui::CornerRadius::same(3);
    v.widgets.hovered.bg_fill = Color32::from_rgb(222, 233, 249);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(222, 233, 249);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(35, 45, 62));
    v.widgets.active.bg_fill = Color32::from_rgb(204, 224, 249);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(204, 224, 249);
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    v.hyperlink_color = ACCENT;
    style.visuals = v;
    ctx.set_style(style);
}
pub(super) fn pill(label: &str, width: f32) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(label).color(Color32::WHITE))
        .fill(ACCENT)
        .stroke(Stroke::NONE)
        .corner_radius(14.)
        .min_size(Vec2::new(width, 27.))
}
pub(super) fn dark_icon(ui: &mut egui::Ui, icon: Icon, tip: &str, enabled: bool) -> egui::Response {
    let (r, response) = ui.allocate_exact_size(
        Vec2::new(29., 29.),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if response.hovered() && enabled {
        ui.painter()
            .rect_filled(r, 2., Color32::from_rgb(79, 91, 94));
    }
    icons::paint(
        ui.painter(),
        r,
        icon,
        if enabled {
            Color32::from_gray(235)
        } else {
            Color32::from_gray(123)
        },
    );
    response.on_hover_text(tip)
}
pub(super) fn nav_tab(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    selected: bool,
    width: f32,
) -> egui::Response {
    let (r, response) = ui.allocate_exact_size(Vec2::new(width, 39.), Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(
            r,
            0.,
            if selected {
                ACCENT
            } else {
                Color32::from_rgb(76, 86, 89)
            },
        );
    }
    let icon_rect = Rect::from_min_size(r.left_top() + Vec2::new(7., 5.), Vec2::splat(27.));
    icons::paint(ui.painter(), icon_rect, icon, Color32::from_gray(245));
    if !label.is_empty() {
        ui.painter().text(
            r.left_center() + Vec2::new(39., 0.),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(14.),
            Color32::WHITE,
        );
    }
    response
}
pub(super) fn section_toggle(ui: &mut egui::Ui, title: &str, expanded: &mut bool) {
    let (r, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().min(230.), 26.),
        Sense::click(),
    );
    if response.hovered() {
        ui.painter().rect_filled(r, 3., ACCENT.gamma_multiply(0.08));
    }
    let c = r.left_center() + Vec2::new(6., 0.);
    let triangle = if *expanded {
        vec![
            c + Vec2::new(-4., -2.),
            c + Vec2::new(4., -2.),
            c + Vec2::new(0., 3.),
        ]
    } else {
        vec![
            c + Vec2::new(-2., -4.),
            c + Vec2::new(-2., 4.),
            c + Vec2::new(3., 0.),
        ]
    };
    ui.painter()
        .add(egui::Shape::convex_polygon(triangle, MUTED, Stroke::NONE));
    ui.painter().text(
        r.left_center() + Vec2::new(19., 0.),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(13.),
        HEADER,
    );
    if response.clicked() {
        *expanded = !*expanded;
    }
}
pub(super) fn section_header(ui: &mut egui::Ui, icon: Icon, title: &str) {
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(Vec2::new(23., 27.), Sense::hover());
        icons::paint(ui.painter(), r, icon, HEADER);
        ui.strong(title);
    });
}
pub(super) fn paint_machine(p: &egui::Painter, r: Rect) {
    let q = |x: f32, y: f32| r.min + Vec2::new(r.width() * x, r.height() * y);
    let s = Stroke::new(1.3_f32, HEADER);
    p.add(egui::Shape::convex_polygon(
        vec![q(0.1, 0.65), q(0.65, 0.48), q(0.95, 0.7), q(0.38, 0.93)],
        Color32::from_gray(224),
        s,
    ));
    for (x, y) in [(0.15, 0.65), (0.85, 0.7)] {
        p.line_segment([q(x, y), q(x, 0.13)], s);
    }
    p.line_segment([q(0.15, 0.13), q(0.85, 0.13)], s);
    p.rect_filled(Rect::from_two_pos(q(0.43, 0.12), q(0.63, 0.34)), 2., ACCENT);
    p.line_segment(
        [q(0.53, 0.34), q(0.53, 0.65)],
        Stroke::new(1.2_f32, Color32::from_rgb(223, 85, 85)),
    );
}
pub(super) fn format_time(seconds: f64) -> String {
    let seconds = seconds.max(0.).round() as u64;
    if seconds >= 3600 {
        format!(
            "{}h {:02}min {:02}s",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}min {:02}s", seconds / 60, seconds % 60)
    }
}
pub(super) fn nice_step(target: f32) -> f32 {
    let power = 10.0_f32.powf(target.log10().floor());
    [1., 2., 5., 10.]
        .into_iter()
        .map(|n| n * power)
        .find(|n| *n >= target)
        .unwrap_or(power * 10.)
}
