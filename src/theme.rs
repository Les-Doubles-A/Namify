//! Thème visuel : palette claire/sombre, couleur d'accent, polices.

use std::sync::Arc;

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Shadow, Stroke, TextStyle, Theme,
    Visuals,
};

use crate::storage::{Settings, ThemeChoice};

pub const ACCENTS: [([u8; 3], &str); 8] = [
    ([124, 92, 255], "Violet"),
    ([59, 130, 246], "Bleu"),
    ([6, 182, 212], "Cyan"),
    ([16, 185, 129], "Émeraude"),
    ([132, 204, 22], "Citron"),
    ([245, 158, 11], "Ambre"),
    ([239, 68, 68], "Rouge"),
    ([236, 72, 153], "Rose"),
];

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color32,
    pub panel: Color32,
    pub card: Color32,
    pub card_hover: Color32,
    pub border: Color32,
    pub text: Color32,
    pub weak: Color32,
    pub accent: Color32,
    pub accent_soft: Color32,
    pub on_accent: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
    pub removed: Color32,
}

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn soft(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

pub fn palette(dark: bool, accent: [u8; 3]) -> Palette {
    let accent = rgb(accent);
    if dark {
        Palette {
            dark,
            bg: Color32::from_rgb(13, 15, 21),
            panel: Color32::from_rgb(19, 22, 30),
            card: Color32::from_rgb(26, 30, 40),
            card_hover: Color32::from_rgb(33, 38, 51),
            border: Color32::from_rgb(42, 47, 62),
            text: Color32::from_rgb(230, 232, 239),
            weak: Color32::from_rgb(139, 145, 163),
            accent,
            accent_soft: soft(accent, 48),
            on_accent: Color32::WHITE,
            success: Color32::from_rgb(52, 211, 153),
            warning: Color32::from_rgb(251, 191, 36),
            error: Color32::from_rgb(248, 113, 113),
            removed: Color32::from_rgb(248, 113, 113),
        }
    } else {
        Palette {
            dark,
            bg: Color32::from_rgb(243, 244, 248),
            panel: Color32::from_rgb(250, 250, 252),
            card: Color32::WHITE,
            card_hover: Color32::from_rgb(246, 246, 250),
            border: Color32::from_rgb(225, 227, 235),
            text: Color32::from_rgb(27, 30, 39),
            weak: Color32::from_rgb(107, 113, 131),
            accent,
            accent_soft: soft(accent, 36),
            on_accent: Color32::WHITE,
            success: Color32::from_rgb(5, 150, 105),
            warning: Color32::from_rgb(217, 119, 6),
            error: Color32::from_rgb(220, 38, 38),
            removed: Color32::from_rgb(220, 38, 38),
        }
    }
}

fn visuals(p: &Palette) -> Visuals {
    let mut v = if p.dark { Visuals::dark() } else { Visuals::light() };
    let r8 = CornerRadius::same(8);
    v.panel_fill = p.panel;
    v.window_fill = p.card;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(14);
    v.menu_corner_radius = CornerRadius::same(10);
    v.window_shadow = Shadow { offset: [0, 8], blur: 28, spread: 0, color: Color32::from_black_alpha(if p.dark { 120 } else { 40 }) };
    v.popup_shadow = Shadow { offset: [0, 4], blur: 16, spread: 0, color: Color32::from_black_alpha(if p.dark { 100 } else { 30 }) };
    v.extreme_bg_color = if p.dark { Color32::from_rgb(16, 18, 25) } else { Color32::WHITE };
    v.text_edit_bg_color = Some(if p.dark { Color32::from_rgb(16, 18, 25) } else { Color32::WHITE });
    v.faint_bg_color = if p.dark { Color32::from_rgb(22, 25, 34) } else { Color32::from_rgb(247, 248, 251) };
    v.code_bg_color = p.card_hover;
    v.hyperlink_color = p.accent;
    v.selection.bg_fill = soft(p.accent, if p.dark { 90 } else { 70 });
    v.selection.stroke = Stroke::new(1.0, p.text);
    v.warn_fg_color = p.warning;
    v.error_fg_color = p.error;
    v.override_text_color = None;

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = p.card;
    w.noninteractive.weak_bg_fill = p.card;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = r8;

    w.inactive.bg_fill = p.card_hover;
    w.inactive.weak_bg_fill = p.card_hover;
    w.inactive.bg_stroke = Stroke::new(1.0, p.border);
    w.inactive.fg_stroke = Stroke::new(1.0, p.text);
    w.inactive.corner_radius = r8;

    w.hovered.bg_fill = soft(p.accent, 60);
    w.hovered.weak_bg_fill = if p.dark { Color32::from_rgb(40, 46, 62) } else { Color32::from_rgb(236, 237, 244) };
    w.hovered.bg_stroke = Stroke::new(1.0, soft(p.accent, 160));
    w.hovered.fg_stroke = Stroke::new(1.5, p.text);
    w.hovered.corner_radius = r8;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = p.accent;
    w.active.weak_bg_fill = soft(p.accent, 90);
    w.active.bg_stroke = Stroke::new(1.0, p.accent);
    w.active.fg_stroke = Stroke::new(2.0, p.text);
    w.active.corner_radius = r8;
    w.active.expansion = 0.0;

    w.open.bg_fill = p.card_hover;
    w.open.weak_bg_fill = p.card_hover;
    w.open.bg_stroke = Stroke::new(1.0, soft(p.accent, 160));
    w.open.fg_stroke = Stroke::new(1.0, p.text);
    w.open.corner_radius = r8;
    v
}

/// Applique le thème, la couleur d'accent et le zoom.
pub fn apply(ctx: &egui::Context, settings: &Settings) {
    ctx.set_theme(match settings.theme {
        ThemeChoice::System => egui::ThemePreference::System,
        ThemeChoice::Dark => egui::ThemePreference::Dark,
        ThemeChoice::Light => egui::ThemePreference::Light,
    });
    ctx.set_visuals_of(Theme::Dark, visuals(&palette(true, settings.accent)));
    ctx.set_visuals_of(Theme::Light, visuals(&palette(false, settings.accent)));
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
        style.spacing.interact_size.y = 26.0;
        style.spacing.menu_margin = egui::Margin::same(8);
        style.spacing.combo_width = 160.0;
        style.spacing.scroll = egui::style::ScrollStyle::thin();
        style.text_styles = [
            (TextStyle::Heading, FontId::new(20.0, FontFamily::Name("bold".into()))),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
        ]
        .into();
    });
}

/// Polices : Segoe UI (si présente) + police d'icônes Phosphor.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("phosphor".into(), Arc::new(FontData::from_static(include_bytes!("../assets/Phosphor.ttf"))));

    let fonts_dir = std::env::var_os("WINDIR")
        .map(|w| std::path::PathBuf::from(w).join("Fonts"))
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows\Fonts"));
    let regular = std::fs::read(fonts_dir.join("segoeui.ttf")).ok();
    let bold = std::fs::read(fonts_dir.join("seguisb.ttf")).or_else(|_| std::fs::read(fonts_dir.join("segoeuib.ttf"))).ok();

    let mut base: Vec<String> = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    if let Some(bytes) = regular {
        fonts.font_data.insert("segoe".into(), Arc::new(FontData::from_owned(bytes)));
        base.insert(0, "segoe".into());
    }
    base.push("phosphor".into());
    fonts.families.insert(FontFamily::Proportional, base.clone());

    let mut bold_family = base.clone();
    if let Some(bytes) = bold {
        fonts.font_data.insert("segoe-bold".into(), Arc::new(FontData::from_owned(bytes)));
        bold_family.insert(0, "segoe-bold".into());
    }
    fonts.families.insert(FontFamily::Name("bold".into()), bold_family);

    if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        mono.push("phosphor".into());
    }
    ctx.set_fonts(fonts);
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}
