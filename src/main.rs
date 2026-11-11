//! Namify - renommage de fichiers par lot.
//!
//! Utilisation : `Namify.exe [fichiers ou dossiers...]`
//! Les chemins passés en argument (par ex. via « Envoyer vers ») sont chargés au démarrage.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod engine;
mod icons;
mod meta;
mod rules;
mod rules_ui;
mod storage;
mod theme;
mod tokens;
mod widgets;

use std::path::PathBuf;

use eframe::egui;

fn window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("../Namify.png")).ok()?;
    let img = img.resize(256, 256, image::imageops::FilterType::Lanczos3).to_rgba8();
    Some(egui::IconData { width: img.width(), height: img.height(), rgba: img.into_raw() })
}

fn main() -> eframe::Result {
    let initial: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).filter(|p| p.exists()).collect();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Namify")
        .with_app_id("Namify")
        .with_inner_size([1320.0, 820.0])
        .with_min_inner_size([980.0, 600.0])
        .with_drag_and_drop(true);
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport, centered: true, ..Default::default() };
    eframe::run_native("Namify", options, Box::new(move |cc| Ok(Box::new(app::NamifyApp::new(cc, initial)))))
}
