//! Petits composants d'interface réutilisables.

use eframe::egui::{
    self, Color32, CornerRadius, Response, RichText, Sense, Stroke, TextFormat, Ui, text::LayoutJob,
};

use crate::theme::{self, Palette};

pub fn primary_button(ui: &mut Ui, p: &Palette, text: &str, enabled: bool) -> Response {
    let (fill, fg) = if enabled { (p.accent, p.on_accent) } else { (p.card_hover, p.weak) };
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(text).font(theme::bold(14.5)).color(fg))
            .fill(fill)
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(10))
            .min_size(egui::vec2(0.0, 36.0)),
    )
}

pub fn ghost_button(ui: &mut Ui, text: &str) -> Response {
    ui.add(egui::Button::new(text).frame_when_inactive(false).corner_radius(CornerRadius::same(8)))
}

pub fn icon_button(ui: &mut Ui, icon: &str, tip: &str) -> Response {
    ui.add(egui::Button::new(RichText::new(icon).size(16.0)).frame_when_inactive(false).min_size(egui::vec2(28.0, 28.0)))
        .on_hover_text(tip)
}

pub fn small_icon_button_enabled(ui: &mut Ui, enabled: bool, icon: &str, tip: &str) -> Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(icon).size(14.0)).frame_when_inactive(false).min_size(egui::vec2(22.0, 22.0)),
    )
    .on_hover_text(tip)
}

pub fn small_icon_button(ui: &mut Ui, icon: &str, tip: &str) -> Response {
    ui.add(egui::Button::new(RichText::new(icon).size(14.0)).frame_when_inactive(false).min_size(egui::vec2(22.0, 22.0)))
        .on_hover_text(tip)
}

pub fn toggle(ui: &mut Ui, p: &Palette, on: &mut bool) -> Response {
    let size = egui::vec2(34.0, 18.0);
    let (rect, mut response) = ui.allocate_exact_size(size, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool_responsive(response.id, *on);
        let radius = 0.5 * rect.height();
        let off = if p.dark { Color32::from_rgb(55, 61, 79) } else { Color32::from_rgb(203, 207, 219) };
        let bg = lerp_color(off, p.accent, t);
        ui.painter().rect_filled(rect, radius, bg);
        let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        ui.painter().circle_filled(egui::pos2(x, rect.center().y), radius - 3.0, Color32::WHITE);
    }
    response
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

pub fn chip(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let bg = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 34);
    egui::Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(255))
        .inner_margin(egui::Margin::symmetric(9, 3))
        .show(ui, |ui| ui.label(RichText::new(text).color(color).size(12.5)))
        .response
}

pub fn card_frame(p: &Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(10))
}

pub fn section_label(ui: &mut Ui, p: &Palette, text: &str) {
    ui.label(RichText::new(text.to_uppercase()).size(11.0).color(p.weak).strong());
}

pub fn enum_combo<T: Copy + PartialEq>(
    ui: &mut Ui,
    id: impl egui::AsIdSalt,
    value: &mut T,
    all: &[T],
    label: fn(T) -> &'static str,
    width: f32,
) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id).width(width).selected_text(label(*value)).show_ui(ui, |ui| {
        for v in all {
            if ui.selectable_value(value, *v, label(*v)).changed() {
                changed = true;
            }
        }
    });
    changed
}

pub fn text_field(ui: &mut Ui, value: &mut String, hint: &str) -> Response {
    ui.add(egui::TextEdit::singleline(value).hint_text(hint).desired_width(f32::INFINITY))
}

/// Met en évidence la partie modifiée entre l'ancien et le nouveau nom.
pub fn diff_jobs(old: &str, new: &str, p: &Palette, size: f32) -> (LayoutJob, LayoutJob) {
    let o: Vec<char> = old.chars().collect();
    let n: Vec<char> = new.chars().collect();
    let mut pre = 0;
    while pre < o.len() && pre < n.len() && o[pre] == n[pre] {
        pre += 1;
    }
    let mut suf = 0;
    while suf < o.len() - pre && suf < n.len() - pre && o[o.len() - 1 - suf] == n[n.len() - 1 - suf] {
        suf += 1;
    }
    let font = egui::FontId::proportional(size);
    let plain = |c: Color32| TextFormat { font_id: font.clone(), color: c, ..Default::default() };
    let s = |v: &[char]| v.iter().collect::<String>();

    let mut jo = LayoutJob::default();
    jo.append(&s(&o[..pre]), 0.0, plain(p.text));
    let mid_old = TextFormat {
        font_id: font.clone(),
        color: p.removed,
        strikethrough: Stroke::new(1.0, p.removed),
        ..Default::default()
    };
    jo.append(&s(&o[pre..o.len() - suf]), 0.0, mid_old);
    jo.append(&s(&o[o.len() - suf..]), 0.0, plain(p.text));

    let mut jn = LayoutJob::default();
    jn.append(&s(&n[..pre]), 0.0, plain(p.text));
    let mid_new = TextFormat {
        font_id: font.clone(),
        color: p.accent,
        background: p.accent_soft,
        ..Default::default()
    };
    jn.append(&s(&n[pre..n.len() - suf]), 0.0, mid_new);
    jn.append(&s(&n[n.len() - suf..]), 0.0, plain(p.text));
    for j in [&mut jo, &mut jn] {
        j.wrap.max_rows = 1;
        j.wrap.break_anywhere = true;
        j.wrap.overflow_character = Some('…');
    }
    (jo, jn)
}

pub fn plain_job(text: &str, color: Color32, size: f32) -> LayoutJob {
    let mut j = LayoutJob::simple_singleline(text.to_string(), egui::FontId::proportional(size), color);
    j.wrap.max_rows = 1;
    j.wrap.break_anywhere = true;
    j.wrap.overflow_character = Some('…');
    j
}
