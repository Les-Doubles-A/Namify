//! Persistance JSON dans %APPDATA%\Les-Doubles-A\Namify\config :
//! réglages, préréglages, historique des lots et dernière session.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::engine::Batch;
use crate::rules::{CaseMode, ExtMode, NumStyle, Occurrence, Position, Rule, RuleItem, Scope};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    System,
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortKey {
    Manual,
    Name,
    Natural,
    Extension,
    Size,
    Modified,
    Created,
    Folder,
}

impl SortKey {
    pub const ALL: [SortKey; 8] = [
        SortKey::Manual,
        SortKey::Natural,
        SortKey::Name,
        SortKey::Extension,
        SortKey::Size,
        SortKey::Modified,
        SortKey::Created,
        SortKey::Folder,
    ];
    pub fn label(self) -> &'static str {
        match self {
            SortKey::Manual => "Ordre manuel",
            SortKey::Name => "Nom (alphabétique)",
            SortKey::Natural => "Nom (naturel)",
            SortKey::Extension => "Extension",
            SortKey::Size => "Taille",
            SortKey::Modified => "Date de modification",
            SortKey::Created => "Date de création",
            SortKey::Folder => "Dossier",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeChoice,
    pub accent: [u8; 3],
    pub zoom: f32,
    pub confirm_before_apply: bool,
    pub auto_resolve: bool,
    pub recursive: bool,
    pub include_hidden: bool,
    pub folders_as_items: bool,
    pub show_folder_column: bool,
    pub show_size_column: bool,
    pub show_date_column: bool,
    pub keep_files_after_apply: bool,
    pub restore_session: bool,
    pub sort: SortKey,
    pub sort_desc: bool,
    pub inspector_open: bool,
    pub ext_filter: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::System,
            accent: [124, 92, 255],
            zoom: 1.0,
            confirm_before_apply: true,
            auto_resolve: false,
            recursive: false,
            include_hidden: false,
            folders_as_items: false,
            show_folder_column: false,
            show_size_column: true,
            show_date_column: false,
            keep_files_after_apply: true,
            restore_session: true,
            sort: SortKey::Manual,
            sort_desc: false,
            inspector_open: true,
            ext_filter: String::new(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Session {
    pub rules: Vec<RuleItem>,
}

pub type Presets = BTreeMap<String, Vec<RuleItem>>;

pub fn config_dir() -> Option<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "Les-Doubles-A", "Namify")?;
    let dir = dirs.config_dir().to_path_buf();
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn load<T: DeserializeOwned + Default>(file: &str) -> T {
    config_dir()
        .and_then(|d| fs::read_to_string(d.join(file)).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save<T: Serialize>(file: &str, value: &T) -> Result<(), String> {
    let dir = config_dir().ok_or("Dossier de configuration introuvable")?;
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp = dir.join(format!("{file}.tmp"));
    fs::write(&tmp, json).map_err(|e| e.to_string())?;
    fs::rename(&tmp, dir.join(file)).map_err(|e| e.to_string())
}

pub fn load_settings() -> Settings {
    load("settings.json")
}
pub fn save_settings(s: &Settings) -> Result<(), String> {
    save("settings.json", s)
}
pub fn load_session() -> Session {
    load("session.json")
}
pub fn save_session(s: &Session) -> Result<(), String> {
    save("session.json", s)
}
pub fn load_presets() -> Presets {
    let p: Presets = load("presets.json");
    if p.is_empty() { builtin_presets() } else { p }
}
pub fn save_presets(p: &Presets) -> Result<(), String> {
    save("presets.json", p)
}
pub fn load_history() -> Vec<Batch> {
    load("history.json")
}
pub fn save_history(h: &[Batch]) -> Result<(), String> {
    save("history.json", &h)
}

/// Préréglages proposés au premier lancement.
pub fn builtin_presets() -> Presets {
    let mut p = Presets::new();
    p.insert(
        "Photos par date de prise de vue".into(),
        vec![
            RuleItem::new(Rule::Template { pattern: "{exif:%Y-%m-%d_%H%M%S}".into() }),
            RuleItem::new(Rule::Extension { mode: ExtMode::Lower, value: String::new() }),
        ],
    );
    p.insert(
        "Noms propres pour le web".into(),
        vec![
            RuleItem::new(Rule::Clean {
                trim: true,
                collapse_spaces: true,
                separators_to_spaces: true,
                spaces_to: String::new(),
                remove_accents: true,
                remove_special: true,
                ascii_only: true,
                remove_brackets: false,
                fix_invalid: true,
            }),
            RuleItem::new(Rule::Case { mode: CaseMode::Kebab, scope: Scope::Name }),
            RuleItem::new(Rule::Extension { mode: ExtMode::Lower, value: String::new() }),
        ],
    );
    p.insert(
        "Épisodes de série (S01E01)".into(),
        vec![
            RuleItem::new(Rule::Template { pattern: "{parent} - S01E{n:2}".into() }),
        ],
    );
    p.insert(
        "Numéroter en préfixe (001 - nom)".into(),
        vec![RuleItem::new(Rule::Numbering {
            start: 1,
            step: 1,
            padding: 3,
            position: Position::Start,
            index: 0,
            separator: " - ".into(),
            style: NumStyle::Decimal,
            reset_per_folder: false,
        })],
    );
    p.insert(
        "Supprimer « (1) », « - Copie »".into(),
        vec![
            RuleItem::new(Rule::Replace {
                find: r"\s*(\(\d+\)|- Copie|- Copy)$".into(),
                replace: String::new(),
                regex: true,
                case_sensitive: false,
                scope: Scope::Name,
                occurrence: Occurrence::All,
            }),
        ],
    );
    p
}
