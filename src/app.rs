//! Application principale : état, actions et mise en page.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Color32, CornerRadius, Key, KeyboardShortcut, Layout, Modifiers, RichText, Sense, Stroke, Ui,
};
use egui_extras::{Column, TableBuilder};

use crate::engine::{self, Batch, Op, Preview, PreviewOptions, Stats, Status};
use crate::icons as ic;
use crate::meta::{Entry, human_size};
use crate::rules::{self, Name, RuleItem, RuleKind};
use crate::rules_ui::{self, CardAction};
use crate::storage::{self, Presets, Settings, SortKey, ThemeChoice};
use crate::theme::{self, ACCENTS, Palette};
use crate::tokens::{self, TokenContext};
use crate::widgets::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPO_URL: &str = "https://github.com/Les-Doubles-A/Namify";
const HISTORY_MAX: usize = 100;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

struct Toast {
    text: String,
    kind: ToastKind,
    born: Instant,
}

enum Dialog {
    ConfirmApply,
    Settings,
    SavePreset(String),
    History,
    Help,
    Report { title: String, lines: Vec<String> },
    ConfirmClear,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StatusFilter {
    All,
    Changed,
    Errors,
    Unchanged,
}

impl StatusFilter {
    const ALL: [StatusFilter; 4] = [StatusFilter::All, StatusFilter::Changed, StatusFilter::Errors, StatusFilter::Unchanged];
    fn label(self) -> &'static str {
        match self {
            StatusFilter::All => "Tous",
            StatusFilter::Changed => "Modifiés",
            StatusFilter::Errors => "Erreurs",
            StatusFilter::Unchanged => "Inchangés",
        }
    }
}

pub struct NamifyApp {
    entries: Vec<Entry>,
    previews: Vec<Preview>,
    stats: Stats,
    rules: Vec<RuleItem>,
    last_rules: Vec<RuleItem>,
    rule_errors: Vec<Option<String>>,
    dirty: bool,
    settings: Settings,
    saved_settings: Settings,
    presets: Presets,
    history: Vec<Batch>,
    filter: String,
    status_filter: StatusFilter,
    anchor: Option<usize>,
    focused: Option<usize>,
    dialog: Option<Dialog>,
    toasts: Vec<Toast>,
    logo: Option<egui::TextureHandle>,
    session_changed_at: Option<Instant>,
    focus_filter: bool,
    crc_requested: bool,
}

impl NamifyApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial: Vec<PathBuf>) -> Self {
        let settings = storage::load_settings();
        theme::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, &settings);
        cc.egui_ctx.set_zoom_factor(settings.zoom.clamp(0.6, 2.5));

        let logo = load_logo(&cc.egui_ctx);
        let rules = if settings.restore_session {
            storage::load_session().rules
        } else {
            Vec::new()
        };
        let rules = if rules.is_empty() {
            vec![RuleItem::new(RuleKind::Replace.default_rule()), RuleItem::new(RuleKind::Numbering.default_rule())]
                .into_iter()
                .map(|mut r| {
                    if r.rule.kind() == RuleKind::Numbering {
                        r.enabled = false;
                    }
                    r
                })
                .collect()
        } else {
            rules
        };

        let mut app = Self {
            entries: Vec::new(),
            previews: Vec::new(),
            stats: Stats::default(),
            last_rules: Vec::new(),
            rule_errors: Vec::new(),
            rules,
            dirty: true,
            saved_settings: settings.clone(),
            settings,
            presets: storage::load_presets(),
            history: storage::load_history(),
            filter: String::new(),
            status_filter: StatusFilter::All,
            anchor: None,
            focused: None,
            dialog: None,
            toasts: Vec::new(),
            logo,
            session_changed_at: None,
            focus_filter: false,
            crc_requested: false,
        };
        if !initial.is_empty() {
            app.add_paths(initial);
        }
        app
    }

    // ------------------------------------------------------------------
    // Actions
    // ------------------------------------------------------------------

    fn toast(&mut self, kind: ToastKind, text: impl Into<String>) {
        self.toasts.push(Toast { text: text.into(), kind, born: Instant::now() });
        if self.toasts.len() > 5 {
            self.toasts.remove(0);
        }
    }

    fn add_paths(&mut self, paths: Vec<PathBuf>) {
        let mut known: HashSet<String> = self.entries.iter().map(|e| e.path.to_string_lossy().to_lowercase()).collect();
        let exts: Vec<String> = self
            .settings
            .ext_filter
            .split([',', ';', ' '])
            .map(|s| s.trim().trim_start_matches('.').trim_start_matches("*.").to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let before = self.entries.len();
        let mut skipped = 0usize;
        let mut push = |entries: &mut Vec<Entry>, path: PathBuf, from_folder: bool| {
            let k = path.to_string_lossy().to_lowercase();
            if known.contains(&k) {
                return;
            }
            let e = Entry::new(path);
            if from_folder && !self.settings.include_hidden && e.meta.hidden {
                skipped += 1;
                return;
            }
            if from_folder && !e.is_dir && !exts.is_empty() {
                let ext = e.path.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
                if !exts.contains(&ext) {
                    skipped += 1;
                    return;
                }
            }
            known.insert(k);
            entries.push(e);
        };

        for path in paths {
            if path.is_dir() && !self.settings.folders_as_items {
                let depth = if self.settings.recursive { usize::MAX } else { 1 };
                let mut items: Vec<PathBuf> = walkdir::WalkDir::new(&path)
                    .min_depth(1)
                    .max_depth(depth)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|d| d.file_type().is_file())
                    .map(|d| d.into_path())
                    .collect();
                items.sort_by(|a, b| natord::compare_ignore_case(&a.to_string_lossy(), &b.to_string_lossy()));
                for item in items {
                    push(&mut self.entries, item, true);
                }
            } else if path.exists() {
                push(&mut self.entries, path, false);
            }
        }
        let added = self.entries.len() - before;
        if self.settings.sort != SortKey::Manual {
            self.sort_entries();
        }
        self.dirty = true;
        match (added, skipped) {
            (0, 0) => self.toast(ToastKind::Info, "Aucun nouvel élément ajouté"),
            (a, 0) => self.toast(ToastKind::Success, format!("{a} élément(s) ajouté(s)")),
            (a, s) => self.toast(ToastKind::Info, format!("{a} élément(s) ajouté(s), {s} ignoré(s) par les filtres")),
        }
    }

    fn pick_files(&mut self) {
        if let Some(files) = rfd::FileDialog::new().set_title("Ajouter des fichiers").pick_files() {
            self.add_paths(files);
        }
    }

    fn pick_folder(&mut self) {
        if let Some(dirs) = rfd::FileDialog::new().set_title("Ajouter des dossiers").pick_folders() {
            self.add_paths(dirs);
        }
    }

    fn sort_entries(&mut self) {
        let key = self.settings.sort;
        let desc = self.settings.sort_desc;
        if key == SortKey::Manual {
            return;
        }
        self.entries.sort_by(|a, b| {
            let o = match key {
                SortKey::Manual => std::cmp::Ordering::Equal,
                SortKey::Name => a.file_name().to_lowercase().cmp(&b.file_name().to_lowercase()),
                SortKey::Natural => natord::compare_ignore_case(&a.file_name(), &b.file_name()),
                SortKey::Extension => {
                    let ea = a.path.extension().map(|x| x.to_string_lossy().to_lowercase());
                    let eb = b.path.extension().map(|x| x.to_string_lossy().to_lowercase());
                    ea.cmp(&eb).then_with(|| natord::compare_ignore_case(&a.file_name(), &b.file_name()))
                }
                SortKey::Size => a.meta.size.cmp(&b.meta.size),
                SortKey::Modified => a.meta.modified.cmp(&b.meta.modified),
                SortKey::Created => a.meta.created.cmp(&b.meta.created),
                SortKey::Folder => natord::compare_ignore_case(&a.parent().to_string_lossy(), &b.parent().to_string_lossy())
                    .then_with(|| natord::compare_ignore_case(&a.file_name(), &b.file_name())),
            };
            if desc { o.reverse() } else { o }
        });
        self.dirty = true;
    }

    fn recompute(&mut self) {
        let (previews, stats) =
            engine::compute(&self.entries, &self.rules, PreviewOptions { auto_resolve: self.settings.auto_resolve });
        self.previews = previews;
        self.stats = stats;
        let sample = Name { stem: "Exemple 01".into(), ext: "txt".into() };
        let ctx = TokenContext::test(0);
        self.rule_errors = self
            .rules
            .iter()
            .map(|r| {
                if !r.enabled {
                    return None;
                }
                let mut n = sample.clone();
                rules::apply(&r.rule, &mut n, &ctx).err()
            })
            .collect();
        self.last_rules = self.rules.clone();
        self.dirty = false;
    }

    fn plan(&self) -> Vec<Op> {
        self.entries
            .iter()
            .zip(&self.previews)
            .filter(|(_, p)| p.status.will_rename())
            .map(|(e, p)| Op { from: e.path.clone(), to: e.parent().join(&p.new_name) })
            .collect()
    }

    /// Met à jour les chemins des entrées après un renommage (dossiers compris).
    fn remap_entries(&mut self, ops: &[Op]) {
        for op in ops {
            for e in self.entries.iter_mut() {
                if e.path == op.from {
                    e.path = op.to.clone();
                    e.meta = crate::meta::FileMeta::read(&e.path);
                } else if let Ok(rel) = e.path.strip_prefix(&op.from) {
                    e.path = op.to.join(rel);
                }
            }
        }
    }

    fn apply(&mut self) {
        let plan = self.plan();
        if plan.is_empty() {
            self.toast(ToastKind::Info, "Rien à renommer");
            return;
        }
        let outcome = engine::execute(&plan);
        let done = outcome.done.len();
        if done > 0 {
            self.history.push(Batch {
                timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                ops: outcome.done.clone(),
                undone: false,
            });
            if self.history.len() > HISTORY_MAX {
                let extra = self.history.len() - HISTORY_MAX;
                self.history.drain(..extra);
            }
            if let Err(e) = storage::save_history(&self.history) {
                self.toast(ToastKind::Warning, format!("Historique non enregistré : {e}"));
            }
        }
        // Remap du plus profond au moins profond, comme l'exécution.
        let mut ops = outcome.done.clone();
        ops.sort_by_key(|o| std::cmp::Reverse(o.from.components().count()));
        self.remap_entries(&ops);
        if !self.settings.keep_files_after_apply && outcome.errors.is_empty() {
            self.entries.clear();
        }
        self.dirty = true;
        if outcome.errors.is_empty() {
            self.toast(ToastKind::Success, format!("{done} élément(s) renommé(s). Ctrl+Z pour annuler."));
        } else {
            self.dialog = Some(Dialog::Report {
                title: format!("{done} renommé(s), {} erreur(s)", outcome.errors.len()),
                lines: outcome.errors,
            });
        }
    }

    fn undo_batch(&mut self, index: usize) {
        let Some(batch) = self.history.get(index).cloned() else { return };
        if batch.undone {
            return;
        }
        let outcome = engine::undo(&batch);
        self.history[index].undone = true;
        let _ = storage::save_history(&self.history);
        let mut reverse: Vec<Op> = outcome.done.clone();
        reverse.sort_by_key(|o| std::cmp::Reverse(o.from.components().count()));
        self.remap_entries(&reverse);
        self.dirty = true;
        if outcome.errors.is_empty() {
            self.toast(ToastKind::Success, format!("Annulé : {} élément(s) restauré(s)", outcome.done.len()));
        } else {
            self.dialog = Some(Dialog::Report {
                title: format!("Annulation partielle : {} erreur(s)", outcome.errors.len()),
                lines: outcome.errors,
            });
        }
    }

    fn undo_last(&mut self) {
        match self.history.iter().rposition(|b| !b.undone) {
            Some(i) => self.undo_batch(i),
            None => self.toast(ToastKind::Info, "Rien à annuler"),
        }
    }

    fn request_apply(&mut self) {
        if self.stats.to_rename == 0 {
            self.toast(ToastKind::Info, "Aucun fichier à renommer");
            return;
        }
        if self.settings.confirm_before_apply || self.stats.errors > 0 {
            self.dialog = Some(Dialog::ConfirmApply);
        } else {
            self.apply();
        }
    }

    fn export_csv(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Exporter l'aperçu")
            .add_filter("CSV", &["csv"])
            .set_file_name("namify-apercu.csv")
            .save_file()
        else {
            return;
        };
        let esc = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
        let mut out = String::from("\u{feff}dossier;ancien nom;nouveau nom;statut\r\n");
        for (e, p) in self.entries.iter().zip(&self.previews) {
            out.push_str(&format!(
                "{};{};{};{}\r\n",
                esc(&e.parent().to_string_lossy()),
                esc(&e.file_name()),
                esc(&p.new_name),
                esc(&p.status.label())
            ));
        }
        match std::fs::write(&path, out) {
            Ok(()) => self.toast(ToastKind::Success, "Aperçu exporté en CSV"),
            Err(e) => self.toast(ToastKind::Error, format!("Export impossible : {e}")),
        }
    }

    fn export_presets(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Exporter les préréglages")
            .add_filter("JSON", &["json"])
            .set_file_name("namify-prereglages.json")
            .save_file()
        else {
            return;
        };
        let res = serde_json::to_string_pretty(&self.presets).map_err(|e| e.to_string()).and_then(|s| std::fs::write(&path, s).map_err(|e| e.to_string()));
        match res {
            Ok(()) => self.toast(ToastKind::Success, "Préréglages exportés"),
            Err(e) => self.toast(ToastKind::Error, format!("Export impossible : {e}")),
        }
    }

    fn import_presets(&mut self) {
        let Some(path) = rfd::FileDialog::new().set_title("Importer des préréglages").add_filter("JSON", &["json"]).pick_file() else {
            return;
        };
        let parsed: Result<Presets, String> = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()));
        match parsed {
            Ok(p) => {
                let n = p.len();
                self.presets.extend(p);
                let _ = storage::save_presets(&self.presets);
                self.toast(ToastKind::Success, format!("{n} préréglage(s) importé(s)"));
            }
            Err(e) => self.toast(ToastKind::Error, format!("Fichier invalide : {e}")),
        }
    }

    fn remove_selected(&mut self) {
        let before = self.entries.len();
        self.entries.retain(|e| !e.selected);
        let n = before - self.entries.len();
        if n > 0 {
            self.focused = None;
            self.anchor = None;
            self.dirty = true;
            self.toast(ToastKind::Info, format!("{n} élément(s) retiré(s) de la liste"));
        }
    }

    fn move_selected(&mut self, delta: i32) {
        if self.settings.sort != SortKey::Manual {
            self.settings.sort = SortKey::Manual;
        }
        let n = self.entries.len();
        if delta < 0 {
            for i in 1..n {
                if self.entries[i].selected && !self.entries[i - 1].selected {
                    self.entries.swap(i, i - 1);
                }
            }
        } else {
            for i in (0..n.saturating_sub(1)).rev() {
                if self.entries[i].selected && !self.entries[i + 1].selected {
                    self.entries.swap(i, i + 1);
                }
            }
        }
        self.focused = self.entries.iter().position(|e| e.selected);
        self.dirty = true;
    }

    fn move_selected_to(&mut self, top: bool) {
        self.settings.sort = SortKey::Manual;
        let (sel, rest): (Vec<Entry>, Vec<Entry>) = std::mem::take(&mut self.entries).into_iter().partition(|e| e.selected);
        self.entries = if top { sel.into_iter().chain(rest).collect() } else { rest.into_iter().chain(sel).collect() };
        self.focused = self.entries.iter().position(|e| e.selected);
        self.dirty = true;
    }

    fn visible_indices(&self) -> Vec<usize> {
        let f = self.filter.to_lowercase();
        (0..self.entries.len())
            .filter(|&i| {
                let e = &self.entries[i];
                let p = self.previews.get(i);
                let text_ok = f.is_empty()
                    || e.file_name().to_lowercase().contains(&f)
                    || p.is_some_and(|p| p.new_name.to_lowercase().contains(&f));
                let status_ok = match (self.status_filter, p) {
                    (StatusFilter::All, _) => true,
                    (StatusFilter::Changed, Some(p)) => p.status.will_rename(),
                    (StatusFilter::Errors, Some(p)) => p.status.is_error(),
                    (StatusFilter::Unchanged, Some(p)) => matches!(p.status, Status::Unchanged | Status::Skipped),
                    _ => true,
                };
                text_ok && status_ok
            })
            .collect()
    }

    // ------------------------------------------------------------------
    // Raccourcis clavier et glisser-déposer
    // ------------------------------------------------------------------

    fn handle_input(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).filter(|p| !p.as_os_str().is_empty()).collect());
        if !dropped.is_empty() {
            self.add_paths(dropped);
        }
        if self.dialog.is_some() {
            return;
        }
        let sc = |m: Modifiers, k: Key| KeyboardShortcut::new(m, k);
        let shift_cmd = Modifiers::COMMAND | Modifiers::SHIFT;
        let typing = ctx.egui_wants_keyboard_input();
        let pressed = |s: KeyboardShortcut| ctx.input_mut(|i| i.consume_shortcut(&s));

        if pressed(sc(shift_cmd, Key::O)) {
            self.pick_folder();
        } else if pressed(sc(Modifiers::COMMAND, Key::O)) {
            self.pick_files();
        }
        if pressed(sc(Modifiers::COMMAND, Key::Enter)) {
            self.request_apply();
        }
        if pressed(sc(Modifiers::COMMAND, Key::F)) {
            self.focus_filter = true;
        }
        if pressed(sc(Modifiers::COMMAND, Key::S)) {
            self.dialog = Some(Dialog::SavePreset(String::new()));
        }
        if pressed(sc(Modifiers::COMMAND, Key::E)) {
            self.export_csv();
        }
        if pressed(sc(Modifiers::NONE, Key::F1)) {
            self.dialog = Some(Dialog::Help);
        }
        if pressed(sc(Modifiers::NONE, Key::F5)) {
            for e in self.entries.iter_mut() {
                e.meta = crate::meta::FileMeta::read(&e.path);
            }
            self.dirty = true;
            self.toast(ToastKind::Info, "Liste actualisée");
        }
        if !typing {
            if pressed(sc(Modifiers::COMMAND, Key::Z)) {
                self.undo_last();
            }
            if pressed(sc(Modifiers::COMMAND, Key::A)) {
                for i in self.visible_indices() {
                    self.entries[i].selected = true;
                }
            }
            if pressed(sc(Modifiers::NONE, Key::Delete)) {
                self.remove_selected();
            }
            if pressed(sc(Modifiers::ALT, Key::ArrowUp)) {
                self.move_selected(-1);
            }
            if pressed(sc(Modifiers::ALT, Key::ArrowDown)) {
                self.move_selected(1);
            }
            if pressed(sc(Modifiers::NONE, Key::Space)) {
                let any_unchecked = self.entries.iter().any(|e| e.selected && !e.checked);
                for e in self.entries.iter_mut().filter(|e| e.selected) {
                    e.checked = any_unchecked;
                }
                self.dirty = true;
            }
        }
    }

    // ------------------------------------------------------------------
    // Interface
    // ------------------------------------------------------------------

    fn top_bar(&mut self, ui: &mut Ui, p: &Palette) {
        let frame = egui::Frame::new().fill(p.bg).inner_margin(egui::Margin::symmetric(14, 10));
        egui::Panel::top("top").frame(frame).show_separator_line(false).show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(logo) = &self.logo {
                    ui.add(egui::Image::new(logo).fit_to_exact_size(egui::vec2(30.0, 30.0)).corner_radius(CornerRadius::same(7)));
                }
                ui.label(RichText::new("Namify").font(theme::bold(21.0)).color(p.text));
                chip(ui, &format!("v{VERSION}"), p.accent);
                ui.add_space(14.0);

                if ui.button(format!("{}  Fichiers", ic::FILE_PLUS)).on_hover_text("Ajouter des fichiers (Ctrl+O)").clicked() {
                    self.pick_files();
                }
                if ui.button(format!("{}  Dossier", ic::FOLDER_SIMPLE_PLUS)).on_hover_text("Ajouter le contenu d'un dossier (Ctrl+Maj+O)").clicked() {
                    self.pick_folder();
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, ic::GEAR, "Réglages").clicked() {
                        self.dialog = Some(Dialog::Settings);
                    }
                    if icon_button(ui, ic::QUESTION, "Aide et raccourcis (F1)").clicked() {
                        self.dialog = Some(Dialog::Help);
                    }
                    let (icon, next, tip) = match self.settings.theme {
                        ThemeChoice::System => (ic::MONITOR, ThemeChoice::Dark, "Thème : système"),
                        ThemeChoice::Dark => (ic::MOON, ThemeChoice::Light, "Thème : sombre"),
                        ThemeChoice::Light => (ic::SUN, ThemeChoice::System, "Thème : clair"),
                    };
                    if icon_button(ui, icon, tip).clicked() {
                        self.settings.theme = next;
                    }
                    ui.separator();
                    if icon_button(ui, ic::CLOCK_COUNTER_CLOCKWISE, "Historique des renommages").clicked() {
                        self.dialog = Some(Dialog::History);
                    }
                    let can_undo = self.history.iter().any(|b| !b.undone);
                    if ui
                        .add_enabled(can_undo, egui::Button::new(format!("{}  Annuler", ic::ARROW_COUNTER_CLOCKWISE)))
                        .on_hover_text("Annuler le dernier renommage (Ctrl+Z)")
                        .clicked()
                    {
                        self.undo_last();
                    }
                });
            });
        });
    }

    fn rules_panel(&mut self, ui: &mut Ui, p: &Palette) {
        let frame = egui::Frame::new().fill(p.bg).inner_margin(egui::Margin { left: 14, right: 6, top: 2, bottom: 12 });
        egui::Panel::left("rules")
            .frame(frame)
            .resizable(true)
            .default_size(390.0)
            .size_range(320.0..=640.0)
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Règles").font(theme::bold(17.0)));
                    let active = self.rules.iter().filter(|r| r.enabled).count();
                    chip(ui, &format!("{active}/{}", self.rules.len()), p.weak);
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        self.presets_menu(ui, p);
                        ui.menu_button(RichText::new(format!("{}  Ajouter", ic::PLUS)).color(p.accent).strong(), |ui| {
                            ui.set_min_width(330.0);
                            for kind in RuleKind::ALL {
                                let btn = egui::Button::new(
                                    RichText::new(format!("{}   {}", kind.icon(), kind.title())).strong(),
                                )
                                .frame_when_inactive(false)
                                .min_size(egui::vec2(ui.available_width(), 0.0));
                                let r = ui.add(btn).on_hover_text(kind.description());
                                if r.clicked() {
                                    self.rules.push(RuleItem::new(kind.default_rule()));
                                    ui.close();
                                }
                            }
                        });
                    });
                });
                ui.add_space(6.0);

                if self.rules.is_empty() {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(ic::MAGIC_WAND).size(40.0).color(p.weak));
                        ui.label(RichText::new("Aucune règle").font(theme::bold(15.0)));
                        ui.label(RichText::new("Ajoutez une règle ou chargez un préréglage.").color(p.weak));
                    });
                }

                let mut action: Option<(usize, CardAction)> = None;
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_width(ui.available_width() - 8.0);
                    let count = self.rules.len();
                    for (i, item) in self.rules.iter_mut().enumerate() {
                        let err = self.rule_errors.get(i).cloned().flatten();
                        let a = rules_ui::rule_card(ui, p, item, i, count, err.as_deref());
                        if !matches!(a, CardAction::None) {
                            action = Some((i, a));
                        }
                        ui.add_space(6.0);
                    }
                    if !self.rules.is_empty() {
                        ui.horizontal(|ui| {
                            if ghost_button(ui, &format!("{}  Tout replier", ic::CARET_RIGHT)).clicked() {
                                self.rules.iter_mut().for_each(|r| r.collapsed = true);
                            }
                            if ghost_button(ui, &format!("{}  Tout déplier", ic::CARET_DOWN)).clicked() {
                                self.rules.iter_mut().for_each(|r| r.collapsed = false);
                            }
                            if ghost_button(ui, &format!("{}  Vider", ic::TRASH)).clicked() {
                                self.rules.clear();
                            }
                        });
                    }
                });
                if let Some((i, a)) = action {
                    match a {
                        CardAction::Up if i > 0 => self.rules.swap(i, i - 1),
                        CardAction::Down if i + 1 < self.rules.len() => self.rules.swap(i, i + 1),
                        CardAction::Duplicate => {
                            let c = self.rules[i].clone();
                            self.rules.insert(i + 1, c);
                        }
                        CardAction::Delete => {
                            self.rules.remove(i);
                        }
                        _ => {}
                    }
                }
            });
    }

    fn presets_menu(&mut self, ui: &mut Ui, _p: &Palette) {
        ui.menu_button(RichText::new(ic::BOOKMARK_SIMPLE).size(16.0), |ui| {
            ui.set_min_width(280.0);
            ui.label(RichText::new("Préréglages").strong());
            ui.separator();
            let names: Vec<String> = self.presets.keys().cloned().collect();
            if names.is_empty() {
                ui.label(RichText::new("Aucun préréglage").weak());
            }
            for name in &names {
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new(name).frame_when_inactive(false).min_size(egui::vec2(200.0, 0.0)))
                        .on_hover_text("Remplacer les règles actuelles")
                        .clicked()
                    {
                        self.rules = self.presets[name].clone();
                        self.toast(ToastKind::Success, format!("Préréglage « {name} » chargé"));
                        ui.close();
                    }
                    if small_icon_button(ui, ic::PLUS, "Ajouter à la suite des règles actuelles").clicked() {
                        let extra = self.presets[name].clone();
                        self.rules.extend(extra);
                        ui.close();
                    }
                    if small_icon_button(ui, ic::TRASH, "Supprimer ce préréglage").clicked() {
                        self.presets.remove(name);
                        let _ = storage::save_presets(&self.presets);
                    }
                });
            }
            ui.separator();
            if ui.button(format!("{}  Enregistrer les règles actuelles...", ic::FLOPPY_DISK)).clicked() {
                self.dialog = Some(Dialog::SavePreset(String::new()));
                ui.close();
            }
            if ui.button(format!("{}  Importer un fichier...", ic::DOWNLOAD_SIMPLE)).clicked() {
                ui.close();
                self.import_presets();
            }
            if ui.button(format!("{}  Exporter tous les préréglages...", ic::UPLOAD_SIMPLE)).clicked() {
                ui.close();
                self.export_presets();
            }
        })
        .response
        .on_hover_text("Préréglages");
    }

    fn inspector_panel(&mut self, ui: &mut Ui, p: &Palette) {
        if !self.settings.inspector_open {
            return;
        }
        let frame = egui::Frame::new().fill(p.bg).inner_margin(egui::Margin { left: 6, right: 14, top: 2, bottom: 12 });
        egui::Panel::right("inspector")
            .frame(frame)
            .resizable(true)
            .default_size(270.0)
            .size_range(240.0..=480.0)
            .show_separator_line(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Détails").font(theme::bold(17.0)));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if small_icon_button(ui, ic::X, "Masquer le panneau").clicked() {
                            self.settings.inspector_open = false;
                        }
                    });
                });
                ui.add_space(6.0);
                let Some(i) = self.focused.filter(|&i| i < self.entries.len()) else {
                    card_frame(p).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            ui.add_space(12.0);
                            ui.label(RichText::new(ic::INFO).size(30.0).color(p.weak));
                            ui.label(RichText::new("Sélectionnez un fichier pour voir ses métadonnées et les valeurs des balises.").color(p.weak));
                            ui.add_space(12.0);
                        });
                    });
                    return;
                };
                let crc_requested = std::mem::take(&mut self.crc_requested);
                let e = &self.entries[i];
                let preview = self.previews.get(i);
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    card_frame(p).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(if e.is_dir { ic::FOLDER } else { ic::FILE }).size(22.0).color(p.accent));
                            ui.add(egui::Label::new(RichText::new(e.file_name()).font(theme::bold(14.0))).wrap());
                        });
                        if let Some(pv) = preview {
                            if pv.new_name != e.file_name() {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(ic::ARROW_RIGHT).color(p.accent));
                                    ui.add(egui::Label::new(RichText::new(&pv.new_name).color(p.accent).strong()).wrap());
                                });
                            }
                            ui.add_space(2.0);
                            let (c, _) = status_style(&pv.status, p);
                            chip(ui, &pv.status.label(), c);
                        }
                        ui.add_space(4.0);
                        ui.add(egui::Label::new(RichText::new(e.parent().to_string_lossy()).small().color(p.weak)).wrap());
                        ui.horizontal(|ui| {
                            if ui.small_button(format!("{}  Explorateur", ic::FOLDER_OPEN)).clicked() {
                                reveal_in_explorer(&e.path);
                            }
                            if ui.small_button(format!("{}  Copier le chemin", ic::COPY)).clicked() {
                                ui.ctx().copy_text(e.path.to_string_lossy().into_owned());
                            }
                        });
                    });
                    ui.add_space(8.0);
                    section_label(ui, p, "Métadonnées");
                    card_frame(p).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let fmt = |d: Option<chrono::DateTime<chrono::Local>>| d.map(|d| d.format("%d/%m/%Y %H:%M:%S").to_string()).unwrap_or_else(|| "-".into());
                        let heavy = e.meta.heavy(&e.path).clone();
                        egui::Grid::new("meta").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
                            let mut row = |k: &str, v: String| {
                                ui.label(RichText::new(k).color(p.weak));
                                ui.add(egui::Label::new(v).truncate());
                                ui.end_row();
                            };
                            row("Type", if e.is_dir { "Dossier".into() } else { "Fichier".into() });
                            if !e.is_dir {
                                row("Taille", format!("{} ({} octets)", human_size(e.meta.size), e.meta.size));
                            }
                            row("Modifié", fmt(e.meta.modified));
                            row("Créé", fmt(e.meta.created));
                            if let Some(d) = heavy.exif_date {
                                row("Prise de vue", d.format("%d/%m/%Y %H:%M:%S").to_string());
                            }
                            if let (Some(w), Some(h)) = (heavy.width, heavy.height) {
                                row("Dimensions", format!("{w} x {h} px"));
                            }
                            if let Some(m) = heavy.make.as_ref().or(heavy.model.as_ref()) {
                                let model = heavy.model.clone().unwrap_or_default();
                                row("Appareil", if heavy.make.is_some() { format!("{m} {model}") } else { model });
                            }
                            if e.meta.hidden {
                                row("Attribut", "Caché".into());
                            }
                        });
                    });
                    ui.add_space(8.0);
                    section_label(ui, p, "Valeurs des balises");
                    card_frame(p).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let original = Name::split(&e.file_name(), e.is_dir);
                        let checked_before = self.entries[..i].iter().filter(|x| x.checked).count();
                        let folder_before = self.entries[..i].iter().filter(|x| x.checked && x.parent() == e.parent()).count();
                        let ctx = TokenContext {
                            index: checked_before,
                            folder_index: folder_before,
                            total: self.stats.checked,
                            is_dir: e.is_dir,
                            original: Some(&original),
                            path: Some(&e.path),
                            meta: Some(&e.meta),
                        };
                        egui::Grid::new("tokens").num_columns(2).spacing([10.0, 3.0]).show(ui, |ui| {
                            let keys = [
                                "{name}", "{ext}", "{n}", "{fn}", "{parent}", "{mdate}", "{cdate}", "{exif}", "{dims}", "{model}", "{size}",
                            ];
                            for k in keys {
                                let v = tokens::expand(k, &ctx, &original).unwrap_or_default();
                                let r = ui.add(egui::Label::new(RichText::new(k).monospace().color(p.accent)).sense(Sense::click()));
                                if r.on_hover_text("Cliquer pour copier la balise").clicked() {
                                    ui.ctx().copy_text(k.to_string());
                                }
                                ui.add(egui::Label::new(if v.is_empty() { RichText::new("vide").weak().italics() } else { RichText::new(v) }).truncate());
                                ui.end_row();
                            }
                            ui.label(RichText::new("{crc}").monospace().color(p.accent));
                            if crc_requested || e.meta.crc32_cached().is_some() {
                                let v = tokens::expand("{crc}", &ctx, &original).unwrap_or_default();
                                ui.label(v);
                            } else if ui.small_button("Calculer").clicked() {
                                self.crc_requested = true;
                            }
                            ui.end_row();
                        });
                    });
                });
            });
    }

    fn files_toolbar(&mut self, ui: &mut Ui, p: &Palette) {
        ui.horizontal(|ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.menu_button(RichText::new(format!("{}  Liste", ic::DOTS_SIX_VERTICAL)), |ui| {
                    ui.set_min_width(260.0);
                    let has_sel = self.entries.iter().any(|e| e.selected);
                    if ui.button(format!("{}  Tout cocher", ic::CHECK_SQUARE)).clicked() {
                        self.entries.iter_mut().for_each(|e| e.checked = true);
                        self.dirty = true;
                    }
                    if ui.button(format!("{}  Tout décocher", ic::SQUARE)).clicked() {
                        self.entries.iter_mut().for_each(|e| e.checked = false);
                        self.dirty = true;
                    }
                    if ui.button(format!("{}  Inverser les coches", ic::SHUFFLE)).clicked() {
                        self.entries.iter_mut().for_each(|e| e.checked = !e.checked);
                        self.dirty = true;
                    }
                    if ui.button(format!("{}  Cocher uniquement les visibles", ic::FUNNEL)).clicked() {
                        let vis: HashSet<usize> = self.visible_indices().into_iter().collect();
                        self.entries.iter_mut().enumerate().for_each(|(i, e)| e.checked = vis.contains(&i));
                        self.dirty = true;
                    }
                    ui.separator();
                    ui.add_enabled_ui(has_sel, |ui| {
                        if ui.button(format!("{}  Monter la sélection (Alt+Haut)", ic::ARROW_UP)).clicked() {
                            self.move_selected(-1);
                        }
                        if ui.button(format!("{}  Descendre la sélection (Alt+Bas)", ic::ARROW_DOWN)).clicked() {
                            self.move_selected(1);
                        }
                        if ui.button(format!("{}  Sélection en haut", ic::ARROW_UP)).clicked() {
                            self.move_selected_to(true);
                        }
                        if ui.button(format!("{}  Sélection en bas", ic::ARROW_DOWN)).clicked() {
                            self.move_selected_to(false);
                        }
                        if ui.button(format!("{}  Retirer la sélection (Suppr)", ic::X)).clicked() {
                            self.remove_selected();
                        }
                    });
                    ui.separator();
                    if ui.button(format!("{}  Exporter l'aperçu en CSV (Ctrl+E)", ic::EXPORT)).clicked() {
                        ui.close();
                        self.export_csv();
                    }
                    if ui.button(format!("{}  Actualiser les métadonnées (F5)", ic::ARROWS_CLOCKWISE)).clicked() {
                        for e in self.entries.iter_mut() {
                            e.meta = crate::meta::FileMeta::read(&e.path);
                        }
                        self.dirty = true;
                    }
                    if ui.button(RichText::new(format!("{}  Vider la liste", ic::TRASH)).color(p.error)).clicked() {
                        ui.close();
                        self.dialog = Some(Dialog::ConfirmClear);
                    }
                });
                let desc_icon = if self.settings.sort_desc { ic::SORT_DESCENDING } else { ic::SORT_ASCENDING };
                if self.settings.sort != SortKey::Manual && icon_button(ui, desc_icon, "Inverser l'ordre").clicked() {
                    self.settings.sort_desc = !self.settings.sort_desc;
                    self.sort_entries();
                }
                if enum_combo(ui, "sort", &mut self.settings.sort, &SortKey::ALL, SortKey::label, 170.0) {
                    self.sort_entries();
                }
                if !self.settings.inspector_open && icon_button(ui, ic::INFO, "Afficher le panneau de détails").clicked() {
                    self.settings.inspector_open = true;
                }
                ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                    let w = ui.available_width().clamp(80.0, 320.0);
                    let search = ui.add(
                        egui::TextEdit::singleline(&mut self.filter)
                            .hint_text(format!("{}  Filtrer (Ctrl+F)", ic::MAGNIFYING_GLASS))
                            .desired_width(w),
                    );
                    if std::mem::take(&mut self.focus_filter) {
                        search.request_focus();
                    }
                });
            });
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            for f in StatusFilter::ALL {
                let count = match f {
                    StatusFilter::All => self.stats.total,
                    StatusFilter::Changed => self.stats.to_rename,
                    StatusFilter::Errors => self.stats.errors,
                    StatusFilter::Unchanged => self.stats.total - self.stats.to_rename - self.stats.errors,
                };
                let txt = format!("{} {count}", f.label());
                let sel = self.status_filter == f;
                let color = if f == StatusFilter::Errors && count > 0 { p.error } else if sel { p.accent } else { p.weak };
                let b = egui::Button::selectable(sel, RichText::new(txt).color(color).size(12.5)).corner_radius(CornerRadius::same(255));
                if ui.add(b).clicked() {
                    self.status_filter = f;
                }
            }
        });
    }

    fn files_table(&mut self, ui: &mut Ui, p: &Palette) {
        let visible = self.visible_indices();
        if visible.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new(ic::FUNNEL).size(36.0).color(p.weak));
                ui.label(RichText::new("Aucun fichier ne correspond au filtre").color(p.weak));
            });
            return;
        }
        let row_h = 30.0;
        let s = self.settings.clone();
        let mut clicked: Option<(usize, Modifiers)> = None;
        let mut toggled_check = false;
        let mut context_action: Option<(usize, &'static str)> = None;

        let mut table = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .sense(Sense::click())
            .cell_layout(Layout::left_to_right(Align::Center))
            .column(Column::exact(26.0))
            .column(Column::exact(38.0))
            .column(Column::remainder().at_least(90.0).clip(true))
            .column(Column::exact(22.0))
            .column(Column::remainder().at_least(90.0).clip(true));
        if s.show_size_column {
            table = table.column(Column::initial(70.0).at_least(44.0).clip(true));
        }
        if s.show_date_column {
            table = table.column(Column::initial(118.0).at_least(60.0).clip(true));
        }
        if s.show_folder_column {
            table = table.column(Column::initial(160.0).at_least(60.0).clip(true));
        }
        table = table.column(Column::initial(130.0).at_least(60.0).clip(true));

        let all_checked = self.entries.iter().all(|e| e.checked);
        let entries = &mut self.entries;
        let previews = &self.previews;
        table
            .header(26.0, |mut h| {
                h.col(|ui| {
                    let mut c = all_checked;
                    if ui.checkbox(&mut c, "").on_hover_text("Tout cocher / décocher").changed() {
                        entries.iter_mut().for_each(|e| e.checked = c);
                        toggled_check = true;
                    }
                });
                let head = |ui: &mut Ui, t: &str| {
                    ui.label(RichText::new(t).small().strong().color(p.weak));
                };
                h.col(|ui| head(ui, "#"));
                h.col(|ui| head(ui, "NOM ACTUEL"));
                h.col(|_| {});
                h.col(|ui| head(ui, "NOUVEAU NOM"));
                if s.show_size_column {
                    h.col(|ui| head(ui, "TAILLE"));
                }
                if s.show_date_column {
                    h.col(|ui| head(ui, "MODIFIÉ"));
                }
                if s.show_folder_column {
                    h.col(|ui| head(ui, "DOSSIER"));
                }
                h.col(|ui| head(ui, "STATUT"));
            })
            .body(|body| {
                body.rows(row_h, visible.len(), |mut row| {
                    let i = visible[row.index()];
                    let e = &mut entries[i];
                    let pv = previews.get(i);
                    row.set_selected(e.selected);
                    let old = e.file_name();
                    let dim = !e.checked;
                    row.col(|ui| {
                        if ui.checkbox(&mut e.checked, "").changed() {
                            toggled_check = true;
                        }
                    });
                    row.col(|ui| {
                        ui.label(RichText::new(format!("{}", i + 1)).small().color(p.weak));
                    });
                    let changed = pv.is_some_and(|pv| pv.new_name != old && e.checked);
                    let (jo, jn) = match pv {
                        Some(pv) if changed => diff_jobs(&old, &pv.new_name, p, 14.0),
                        _ => {
                            let c = if dim { p.weak } else { p.text };
                            (plain_job(&old, c, 14.0), plain_job(pv.map_or(&old, |p| &p.new_name), p.weak, 14.0))
                        }
                    };
                    row.col(|ui| {
                        ui.label(RichText::new(if e.is_dir { ic::FOLDER } else { ic::FILE }).color(p.weak));
                        ui.add(egui::Label::new(jo).truncate().selectable(false)).on_hover_text(e.path.to_string_lossy());
                    });
                    row.col(|ui| {
                        ui.label(RichText::new(ic::ARROW_RIGHT).color(if changed { p.accent } else { p.weak }));
                    });
                    row.col(|ui| {
                        let r = ui.add(egui::Label::new(jn).truncate().selectable(false));
                        if let Some(pv) = pv {
                            r.on_hover_text(&pv.new_name);
                        }
                    });
                    if s.show_size_column {
                        row.col(|ui| {
                            if !e.is_dir {
                                ui.label(RichText::new(human_size(e.meta.size)).small().color(p.weak));
                            }
                        });
                    }
                    if s.show_date_column {
                        row.col(|ui| {
                            let d = e.meta.modified.map(|d| d.format("%d/%m/%Y %H:%M").to_string()).unwrap_or_default();
                            ui.label(RichText::new(d).small().color(p.weak));
                        });
                    }
                    if s.show_folder_column {
                        row.col(|ui| {
                            ui.add(egui::Label::new(RichText::new(e.parent().to_string_lossy()).small().color(p.weak)).truncate());
                        });
                    }
                    row.col(|ui| {
                        if let Some(pv) = pv {
                            let (c, icon) = status_style(&pv.status, p);
                            ui.label(RichText::new(icon).color(c));
                            ui.add(egui::Label::new(RichText::new(pv.status.label()).small().color(c)).truncate());
                        }
                    });
                    let resp = row.response();
                    if resp.clicked() {
                        clicked = Some((i, resp.ctx.input(|inp| inp.modifiers)));
                    }
                    if resp.double_clicked() {
                        e.checked = !e.checked;
                        toggled_check = true;
                    }
                    resp.context_menu(|ui| {
                        for (label, key) in [
                            (format!("{}  Ouvrir", ic::FILE_TEXT), "open"),
                            (format!("{}  Afficher dans l'explorateur", ic::FOLDER_OPEN), "reveal"),
                            (format!("{}  Copier le chemin", ic::COPY), "copy_path"),
                            (format!("{}  Copier le nouveau nom", ic::COPY), "copy_new"),
                            (format!("{}  Cocher / décocher", ic::CHECK_SQUARE), "toggle"),
                            (format!("{}  Retirer de la liste", ic::X), "remove"),
                        ] {
                            if ui.button(label).clicked() {
                                context_action = Some((i, key));
                                ui.close();
                            }
                        }
                    });
                });
            });

        if toggled_check {
            self.dirty = true;
        }
        if let Some((i, m)) = clicked {
            self.select_row(i, m, &visible);
        }
        if let Some((i, action)) = context_action {
            let path = self.entries[i].path.clone();
            match action {
                "open" => open_path(&path),
                "reveal" => reveal_in_explorer(&path),
                "copy_path" => ui.ctx().copy_text(path.to_string_lossy().into_owned()),
                "copy_new" => {
                    if let Some(pv) = self.previews.get(i) {
                        ui.ctx().copy_text(pv.new_name.clone());
                    }
                }
                "toggle" => {
                    self.entries[i].checked = !self.entries[i].checked;
                    self.dirty = true;
                }
                "remove" => {
                    self.entries.remove(i);
                    self.focused = None;
                    self.anchor = None;
                    self.dirty = true;
                }
                _ => {}
            }
        }
    }

    fn select_row(&mut self, i: usize, m: Modifiers, visible: &[usize]) {
        if m.shift {
            if let Some(a) = self.anchor {
                let pa = visible.iter().position(|&x| x == a);
                let pi = visible.iter().position(|&x| x == i);
                if let (Some(pa), Some(pi)) = (pa, pi) {
                    if !m.command {
                        self.entries.iter_mut().for_each(|e| e.selected = false);
                    }
                    for &k in &visible[pa.min(pi)..=pa.max(pi)] {
                        self.entries[k].selected = true;
                    }
                    self.focused = Some(i);
                    return;
                }
            }
        }
        if m.command {
            self.entries[i].selected = !self.entries[i].selected;
        } else {
            self.entries.iter_mut().for_each(|e| e.selected = false);
            self.entries[i].selected = true;
        }
        self.anchor = Some(i);
        self.focused = Some(i);
    }

    fn empty_state(&mut self, ui: &mut Ui, p: &Palette) {
        let avail = ui.available_rect_before_wrap();
        let rect = avail.shrink(8.0);
        let painter = ui.painter();
        painter.rect_filled(rect, 18.0, p.card);
        let dash = egui::Shape::dashed_line(
            &[rect.left_top(), rect.right_top(), rect.right_bottom(), rect.left_bottom(), rect.left_top()],
            Stroke::new(1.5, p.border),
            8.0,
            6.0,
        );
        painter.extend(dash);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::Center)));
        child.add_space((rect.height() / 2.0 - 130.0).max(10.0));
        child.label(RichText::new(ic::FILES).size(64.0).color(p.accent));
        child.add_space(6.0);
        child.label(RichText::new("Glissez-déposez vos fichiers ou dossiers ici").font(theme::bold(20.0)));
        child.label(RichText::new("ou utilisez les boutons ci-dessous. Tout se prévisualise avant d'être appliqué.").color(p.weak));
        child.add_space(16.0);
        child.horizontal(|ui| {
            let w = 380.0;
            ui.add_space((ui.available_width() - w).max(0.0) / 2.0);
            if primary_button(ui, p, &format!("{}  Ajouter des fichiers", ic::FILE_PLUS), true).clicked() {
                self.pick_files();
            }
            if ui
                .add(egui::Button::new(RichText::new(format!("{}  Ajouter un dossier", ic::FOLDER_SIMPLE_PLUS)).size(14.5)).min_size(egui::vec2(0.0, 36.0)).corner_radius(CornerRadius::same(10)))
                .clicked()
            {
                self.pick_folder();
            }
        });
        child.add_space(18.0);
        child.label(RichText::new(format!("{}  Astuce : clic droit sur des fichiers dans l'explorateur > Envoyer vers > Namify", ic::LIGHTNING)).small().color(p.weak));
        ui.allocate_rect(avail, Sense::hover());
    }

    fn bottom_bar(&mut self, ui: &mut Ui, p: &Palette) {
        let frame = egui::Frame::new().fill(p.bg).inner_margin(egui::Margin::symmetric(14, 10));
        egui::Panel::bottom("bottom").frame(frame).show_separator_line(false).show(ui, |ui| {
            ui.horizontal(|ui| {
                let st = self.stats;
                chip(ui, &format!("{}  {} élément(s)", ic::FILES, st.total), p.weak);
                chip(ui, &format!("{}  {} à renommer", ic::PENCIL_SIMPLE, st.to_rename), p.accent);
                if st.resolved > 0 {
                    chip(ui, &format!("{}  {} conflit(s) résolu(s)", ic::SPARKLE, st.resolved), p.warning);
                }
                if st.errors > 0 {
                    chip(ui, &format!("{}  {} erreur(s)", ic::WARNING, st.errors), p.error);
                }
                if st.checked < st.total {
                    chip(ui, &format!("{} ignoré(s)", st.total - st.checked), p.weak);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let label = if st.to_rename > 0 {
                        format!("{}  Renommer {} élément(s)", ic::PLAY, st.to_rename)
                    } else {
                        format!("{}  Renommer", ic::PLAY)
                    };
                    if primary_button(ui, p, &label, st.to_rename > 0).on_hover_text("Ctrl+Entrée").clicked() {
                        self.request_apply();
                    }
                    ui.add_space(4.0);
                    ui.add(egui::Label::new(RichText::new(format!("{}  Annulation possible à tout moment depuis l'historique", ic::INFO)).small().color(p.weak)).truncate());
                });
            });
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context, p: &Palette) {
        let Some(dialog) = self.dialog.take() else { return };
        let mut keep = true;
        let modal_frame = egui::Frame::new()
            .fill(p.card)
            .stroke(Stroke::new(1.0, p.border))
            .corner_radius(CornerRadius::same(16))
            .inner_margin(egui::Margin::same(20));
        let mut next: Option<Dialog> = None;
        let mut dialog = dialog;
        let resp = egui::Modal::new(egui::Id::unique("dialog")).frame(modal_frame).show(ctx, |ui| match &mut dialog {
            Dialog::ConfirmApply => {
                ui.set_width(440.0);
                ui.label(RichText::new(format!("{}  Confirmer le renommage", ic::PLAY)).font(theme::bold(18.0)));
                ui.add_space(8.0);
                ui.label(format!("{} élément(s) vont être renommés.", self.stats.to_rename));
                if self.stats.errors > 0 {
                    ui.label(RichText::new(format!("{}  {} élément(s) en erreur seront ignorés.", ic::WARNING, self.stats.errors)).color(p.warning));
                }
                if self.stats.resolved > 0 {
                    ui.label(RichText::new(format!("{} conflit(s) résolu(s) par un suffixe numéroté.", self.stats.resolved)).color(p.weak));
                }
                ui.label(RichText::new("L'opération peut être annulée avec Ctrl+Z ou depuis l'historique.").color(p.weak));
                ui.add_space(6.0);
                ui.checkbox(&mut self.settings.confirm_before_apply, "Toujours demander confirmation");
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if primary_button(ui, p, "Renommer", true).clicked() {
                            keep = false;
                            next = None;
                            self.apply();
                        }
                        if ui.button("Annuler").clicked() {
                            keep = false;
                        }
                    });
                });
            }
            Dialog::ConfirmClear => {
                ui.set_width(380.0);
                ui.label(RichText::new("Vider la liste ?").font(theme::bold(18.0)));
                ui.label(RichText::new("Les fichiers ne sont pas supprimés du disque, seulement retirés de Namify.").color(p.weak));
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if primary_button(ui, p, "Vider", true).clicked() {
                        self.entries.clear();
                        self.focused = None;
                        self.anchor = None;
                        self.dirty = true;
                        keep = false;
                    }
                    if ui.button("Annuler").clicked() {
                        keep = false;
                    }
                });
            }
            Dialog::SavePreset(name) => {
                ui.set_width(400.0);
                ui.label(RichText::new(format!("{}  Enregistrer un préréglage", ic::FLOPPY_DISK)).font(theme::bold(18.0)));
                ui.add_space(8.0);
                let r = ui.add(egui::TextEdit::singleline(name).hint_text("Nom du préréglage").desired_width(f32::INFINITY));
                r.request_focus();
                let exists = self.presets.contains_key(name.trim());
                if exists {
                    ui.label(RichText::new("Un préréglage porte déjà ce nom : il sera remplacé.").color(p.warning).small());
                }
                ui.add_space(12.0);
                let enter = ui.input(|i| i.key_pressed(Key::Enter));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let ok = !name.trim().is_empty();
                    if primary_button(ui, p, "Enregistrer", ok).clicked() || (ok && enter) {
                        self.presets.insert(name.trim().to_string(), self.rules.clone());
                        match storage::save_presets(&self.presets) {
                            Ok(()) => self.toast(ToastKind::Success, format!("Préréglage « {} » enregistré", name.trim())),
                            Err(e) => self.toast(ToastKind::Error, e),
                        }
                        keep = false;
                    }
                    if ui.button("Annuler").clicked() {
                        keep = false;
                    }
                });
            }
            Dialog::Report { title, lines } => {
                ui.set_width(520.0);
                ui.label(RichText::new(format!("{}  {title}", ic::WARNING)).font(theme::bold(18.0)).color(p.warning));
                ui.add_space(8.0);
                egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                    for l in lines.iter() {
                        ui.label(RichText::new(l).small());
                    }
                });
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if primary_button(ui, p, "Fermer", true).clicked() {
                        keep = false;
                    }
                    if ui.button(format!("{}  Copier", ic::COPY)).clicked() {
                        ui.ctx().copy_text(lines.join("\n"));
                    }
                });
            }
            Dialog::History => {
                ui.set_width(560.0);
                ui.label(RichText::new(format!("{}  Historique", ic::CLOCK_COUNTER_CLOCKWISE)).font(theme::bold(18.0)));
                ui.label(RichText::new("Chaque lot peut être annulé, même après redémarrage de Namify.").color(p.weak));
                ui.add_space(8.0);
                let mut undo: Option<usize> = None;
                egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                    if self.history.is_empty() {
                        ui.label(RichText::new("Aucun renommage pour l'instant.").weak());
                    }
                    for (i, b) in self.history.iter().enumerate().rev() {
                        card_frame(p).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&b.timestamp).strong());
                                chip(ui, &format!("{} élément(s)", b.ops.len()), p.accent);
                                if b.undone {
                                    chip(ui, "annulé", p.weak);
                                }
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if !b.undone && ui.button(format!("{}  Annuler", ic::ARROW_COUNTER_CLOCKWISE)).clicked() {
                                        undo = Some(i);
                                    }
                                });
                            });
                            egui::CollapsingHeader::new(RichText::new("Détails").small()).id_salt(("hist", i)).show(ui, |ui| {
                                for op in b.ops.iter().take(200) {
                                    let f = op.from.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                                    let t = op.to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                                    ui.label(RichText::new(format!("{f}  {}  {t}", ic::ARROW_RIGHT)).small());
                                }
                                if b.ops.len() > 200 {
                                    ui.label(RichText::new(format!("... et {} autres", b.ops.len() - 200)).small().weak());
                                }
                            });
                        });
                        ui.add_space(4.0);
                    }
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if !self.history.is_empty() && ui.button(RichText::new(format!("{}  Effacer l'historique", ic::TRASH)).color(p.error)).clicked() {
                        self.history.clear();
                        let _ = storage::save_history(&self.history);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if primary_button(ui, p, "Fermer", true).clicked() {
                            keep = false;
                        }
                    });
                });
                if let Some(i) = undo {
                    self.undo_batch(i);
                    if self.dialog.is_some() {
                        next = self.dialog.take();
                        keep = false;
                    }
                }
            }
            Dialog::Help => {
                ui.set_width(600.0);
                ui.horizontal(|ui| {
                    if let Some(logo) = &self.logo {
                        ui.add(egui::Image::new(logo).fit_to_exact_size(egui::vec2(44.0, 44.0)).corner_radius(CornerRadius::same(10)));
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("Namify {VERSION}")).font(theme::bold(20.0)));
                        ui.hyperlink_to(REPO_URL, REPO_URL);
                    });
                });
                ui.add_space(8.0);
                egui::ScrollArea::vertical().max_height(440.0).show(ui, |ui| {
                    section_label(ui, p, "Raccourcis clavier");
                    egui::Grid::new("keys").num_columns(2).spacing([16.0, 4.0]).striped(true).show(ui, |ui| {
                        for (k, d) in [
                            ("Ctrl+O", "Ajouter des fichiers"),
                            ("Ctrl+Maj+O", "Ajouter un dossier"),
                            ("Ctrl+Entrée", "Renommer"),
                            ("Ctrl+Z", "Annuler le dernier renommage"),
                            ("Ctrl+F", "Filtrer la liste"),
                            ("Ctrl+A", "Tout sélectionner"),
                            ("Ctrl+S", "Enregistrer un préréglage"),
                            ("Ctrl+E", "Exporter l'aperçu en CSV"),
                            ("Espace", "Cocher / décocher la sélection"),
                            ("Suppr", "Retirer la sélection de la liste"),
                            ("Alt+Haut / Bas", "Déplacer la sélection"),
                            ("Ctrl + / Ctrl -", "Zoom de l'interface"),
                            ("F5", "Actualiser les métadonnées"),
                            ("F1", "Cette aide"),
                        ] {
                            ui.label(RichText::new(k).monospace().color(p.accent));
                            ui.label(d);
                            ui.end_row();
                        }
                    });
                    ui.add_space(10.0);
                    section_label(ui, p, "Balises dynamiques");
                    egui::Grid::new("toks").num_columns(2).spacing([16.0, 4.0]).striped(true).show(ui, |ui| {
                        for (k, d) in tokens::HELP {
                            ui.label(RichText::new(*k).monospace().color(p.accent));
                            ui.label(*d);
                            ui.end_row();
                        }
                    });
                    ui.add_space(10.0);
                    section_label(ui, p, "Bon à savoir");
                    for t in [
                        "Les règles s'appliquent dans l'ordre, de haut en bas. Chaque règle voit le résultat de la précédente.",
                        "Le renommage passe par des noms temporaires : permuter des noms (a <-> b) fonctionne.",
                        "Les conflits sont détectés avant l'application ; activez la résolution automatique dans les réglages pour ajouter « (2) ».",
                        "Les expressions régulières suivent la syntaxe Rust (regex) ; utilisez $1, $2 ou ${nom} dans le remplacement.",
                        "Namify s'ajoute au menu « Envoyer vers » de l'explorateur Windows à l'installation.",
                    ] {
                        ui.label(format!("•  {t}"));
                    }
                });
                ui.add_space(10.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if primary_button(ui, p, "Fermer", true).clicked() {
                        keep = false;
                    }
                });
            }
            Dialog::Settings => {
                ui.set_width(520.0);
                ui.label(RichText::new(format!("{}  Réglages", ic::GEAR)).font(theme::bold(18.0)));
                ui.add_space(8.0);
                egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
                    section_label(ui, p, "Apparence");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.settings.theme, ThemeChoice::System, format!("{}  Système", ic::MONITOR));
                        ui.selectable_value(&mut self.settings.theme, ThemeChoice::Light, format!("{}  Clair", ic::SUN));
                        ui.selectable_value(&mut self.settings.theme, ThemeChoice::Dark, format!("{}  Sombre", ic::MOON));
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label("Accent");
                        for (c, name) in ACCENTS {
                            let col = Color32::from_rgb(c[0], c[1], c[2]);
                            let (rect, r) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::click());
                            ui.painter().circle_filled(rect.center(), 10.0, col);
                            if self.settings.accent == c {
                                ui.painter().circle_stroke(rect.center(), 10.0, Stroke::new(2.5, p.text));
                            }
                            if r.on_hover_text(name).clicked() {
                                self.settings.accent = c;
                            }
                        }
                        let mut rgb = self.settings.accent;
                        if ui.color_edit_button_srgb(&mut rgb).changed() {
                            self.settings.accent = rgb;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Zoom");
                        ui.add(egui::Slider::new(&mut self.settings.zoom, 0.7..=1.8).step_by(0.05).custom_formatter(|v, _| format!("{:.0} %", v * 100.0)));
                    });
                    ui.add_space(10.0);
                    section_label(ui, p, "Colonnes de la liste");
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.settings.show_size_column, "Taille");
                        ui.checkbox(&mut self.settings.show_date_column, "Date");
                        ui.checkbox(&mut self.settings.show_folder_column, "Dossier");
                    });
                    ui.add_space(10.0);
                    section_label(ui, p, "Ajout de dossiers");
                    ui.checkbox(&mut self.settings.recursive, "Inclure les sous-dossiers");
                    ui.checkbox(&mut self.settings.include_hidden, "Inclure les fichiers cachés");
                    ui.checkbox(&mut self.settings.folders_as_items, "Renommer les dossiers eux-mêmes (au lieu de leur contenu)");
                    ui.horizontal(|ui| {
                        ui.label("Extensions autorisées");
                        ui.add(egui::TextEdit::singleline(&mut self.settings.ext_filter).hint_text("toutes (ex : jpg, png, mp4)").desired_width(220.0));
                    });
                    ui.add_space(10.0);
                    section_label(ui, p, "Renommage");
                    ui.checkbox(&mut self.settings.confirm_before_apply, "Demander confirmation avant de renommer");
                    ui.checkbox(&mut self.settings.auto_resolve, "Résoudre automatiquement les conflits avec « (2) », « (3) »...");
                    ui.checkbox(&mut self.settings.keep_files_after_apply, "Garder les fichiers dans la liste après renommage");
                    ui.checkbox(&mut self.settings.restore_session, "Restaurer les règles au démarrage");
                    ui.add_space(10.0);
                    section_label(ui, p, "Données");
                    ui.horizontal(|ui| {
                        if ui.button(format!("{}  Ouvrir le dossier de configuration", ic::FOLDER_OPEN)).clicked() {
                            if let Some(d) = storage::config_dir() {
                                open_path(&d);
                            }
                        }
                        if ui.button(format!("{}  Restaurer les préréglages d'origine", ic::ARROWS_CLOCKWISE)).clicked() {
                            self.presets.extend(storage::builtin_presets());
                            let _ = storage::save_presets(&self.presets);
                            self.toast(ToastKind::Success, "Préréglages d'origine restaurés");
                        }
                    });
                    if ui.button(format!("{}  Réinitialiser les réglages", ic::ARROW_COUNTER_CLOCKWISE)).clicked() {
                        self.settings = Settings::default();
                    }
                });
                ui.add_space(12.0);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if primary_button(ui, p, "Fermer", true).clicked() {
                        keep = false;
                    }
                });
            }
        });
        if resp.should_close() {
            keep = false;
        }
        if keep {
            self.dialog = Some(dialog);
        } else if next.is_some() {
            self.dialog = next;
        } else if self.dialog.is_none() {
            self.dialog = None;
        }
    }

    fn toasts_ui(&mut self, ctx: &egui::Context, p: &Palette) {
        const LIFE: f32 = 4.0;
        self.toasts.retain(|t| t.born.elapsed().as_secs_f32() < LIFE);
        if self.toasts.is_empty() {
            return;
        }
        egui::Area::new(egui::Id::unique("toasts"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-18.0, -70.0))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                for t in &self.toasts {
                    let age = t.born.elapsed().as_secs_f32();
                    let alpha = ((LIFE - age) / 0.4).clamp(0.0, 1.0) * (age / 0.15).clamp(0.0, 1.0);
                    let (c, icon) = match t.kind {
                        ToastKind::Info => (p.accent, ic::INFO),
                        ToastKind::Success => (p.success, ic::CHECK_CIRCLE),
                        ToastKind::Warning => (p.warning, ic::WARNING),
                        ToastKind::Error => (p.error, ic::X_CIRCLE),
                    };
                    egui::Frame::new()
                        .fill(p.card)
                        .stroke(Stroke::new(1.0, p.border))
                        .corner_radius(CornerRadius::same(12))
                        .inner_margin(egui::Margin::symmetric(14, 10))
                        .shadow(ui.visuals().popup_shadow)
                        .multiply_with_opacity(alpha)
                        .show(ui, |ui| {
                            ui.set_max_width(380.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(icon).size(18.0).color(c.gamma_multiply(alpha)));
                                ui.label(RichText::new(&t.text).color(p.text.gamma_multiply(alpha)));
                            });
                        });
                    ui.add_space(6.0);
                }
            });
        ctx.request_repaint_after(Duration::from_millis(50));
    }

    fn drop_overlay(&self, ctx: &egui::Context, p: &Palette) {
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        if !hovering {
            return;
        }
        let rect = ctx.content_rect();
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("drop")));
        painter.rect_filled(rect, 0.0, Color32::from_black_alpha(140));
        let inner = rect.shrink(30.0);
        painter.rect_stroke(inner, 20.0, Stroke::new(3.0, p.accent), egui::StrokeKind::Inside);
        painter.text(
            inner.center() - egui::vec2(0.0, 30.0),
            egui::Align2::CENTER_CENTER,
            ic::DOWNLOAD_SIMPLE,
            egui::FontId::proportional(64.0),
            Color32::WHITE,
        );
        painter.text(
            inner.center() + egui::vec2(0.0, 30.0),
            egui::Align2::CENTER_CENTER,
            "Déposez pour ajouter",
            theme::bold(24.0),
            Color32::WHITE,
        );
    }

    fn persist(&mut self, ctx: &egui::Context) {
        let zoom = ctx.zoom_factor();
        if (zoom - self.settings.zoom).abs() > 0.001 && (self.settings.zoom - self.saved_settings.zoom).abs() < 0.001 {
            // Zoom modifié au clavier (Ctrl +/-).
            self.settings.zoom = zoom;
        } else if (zoom - self.settings.zoom).abs() > 0.001 {
            ctx.set_zoom_factor(self.settings.zoom);
        }
        if !settings_eq(&self.settings, &self.saved_settings) {
            if self.settings.theme != self.saved_settings.theme || self.settings.accent != self.saved_settings.accent {
                theme::apply(ctx, &self.settings);
            }
            if self.settings.auto_resolve != self.saved_settings.auto_resolve {
                self.dirty = true;
            }
            let _ = storage::save_settings(&self.settings);
            self.saved_settings = self.settings.clone();
        }
        if self.rules != self.last_rules {
            self.session_changed_at = Some(Instant::now());
        }
        if let Some(t) = self.session_changed_at {
            if t.elapsed() > Duration::from_millis(800) {
                let _ = storage::save_session(&storage::Session { rules: self.rules.clone() });
                self.session_changed_at = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(900));
            }
        }
    }
}

fn settings_eq(a: &Settings, b: &Settings) -> bool {
    serde_json::to_string(a).ok() == serde_json::to_string(b).ok()
}

fn status_style(s: &Status, p: &Palette) -> (Color32, &'static str) {
    match s {
        Status::Ok => (p.success, ic::CHECK_CIRCLE),
        Status::Resolved => (p.warning, ic::SPARKLE),
        Status::Unchanged => (p.weak, ic::MINUS),
        Status::Skipped => (p.weak, ic::EYE_SLASH),
        _ => (p.error, ic::WARNING),
    }
}

fn load_logo(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(include_bytes!("../Namify.png")).ok()?;
    let img = img.resize(128, 128, image::imageops::FilterType::Lanczos3).to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture("logo", color, egui::TextureOptions::LINEAR))
}

pub fn open_path(path: &Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(path).spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
    }
}

pub fn reveal_in_explorer(path: &Path) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("explorer").raw_arg(format!("/select,\"{}\"", path.display())).spawn();
    }
    #[cfg(not(windows))]
    {
        if let Some(parent) = path.parent() {
            open_path(parent);
        }
    }
}

impl eframe::App for NamifyApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let p = theme::palette(ui.visuals().dark_mode, self.settings.accent);
        self.handle_input(&ctx);
        if self.dirty || self.rules != self.last_rules {
            self.recompute();
        }

        self.top_bar(ui, &p);
        self.bottom_bar(ui, &p);
        self.rules_panel(ui, &p);
        if !self.entries.is_empty() {
            self.inspector_panel(ui, &p);
        }
        let central = egui::Frame::new().fill(p.bg).inner_margin(egui::Margin { left: 6, right: 6, top: 2, bottom: 12 });
        egui::CentralPanel::default().frame(central).show(ui, |ui| {
            if self.entries.is_empty() {
                self.empty_state(ui, &p);
            } else {
                card_frame(&p).inner_margin(egui::Margin::same(12)).show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    self.files_toolbar(ui, &p);
                    ui.add_space(6.0);
                    self.files_table(ui, &p);
                });
            }
        });

        self.dialogs(&ctx, &p);
        self.toasts_ui(&ctx, &p);
        self.drop_overlay(&ctx, &p);
        if self.dirty || self.rules != self.last_rules {
            ctx.request_repaint();
        }
        self.persist(&ctx);
    }
}
