//! Calcul de l'aperçu, détection des conflits et exécution sécurisée du renommage.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::meta::Entry;
use crate::rules::{self, Name, RuleItem};
use crate::tokens::TokenContext;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Fichier non coché : ignoré.
    Skipped,
    Unchanged,
    Ok,
    /// Conflit résolu automatiquement par un suffixe « (2) ».
    Resolved,
    Duplicate,
    Exists,
    Invalid(String),
    Missing,
}

impl Status {
    pub fn is_error(&self) -> bool {
        matches!(self, Status::Duplicate | Status::Exists | Status::Invalid(_) | Status::Missing)
    }
    pub fn will_rename(&self) -> bool {
        matches!(self, Status::Ok | Status::Resolved)
    }
    pub fn label(&self) -> String {
        match self {
            Status::Skipped => "Ignoré".into(),
            Status::Unchanged => "Inchangé".into(),
            Status::Ok => "Prêt".into(),
            Status::Resolved => "Conflit résolu".into(),
            Status::Duplicate => "Doublon dans le lot".into(),
            Status::Exists => "Existe déjà sur le disque".into(),
            Status::Invalid(m) => m.clone(),
            Status::Missing => "Fichier introuvable".into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Preview {
    pub new_name: String,
    pub status: Status,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub total: usize,
    pub checked: usize,
    pub to_rename: usize,
    pub unchanged: usize,
    pub errors: usize,
    pub resolved: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct PreviewOptions {
    pub auto_resolve: bool,
}

const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1",
    "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Vérifie qu'un nom est valide pour Windows.
pub fn validate_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("Nom vide".into());
    }
    if let Some(c) = name.chars().find(|c| rules::is_invalid_char(*c)) {
        let shown = if c.is_control() { "contrôle".to_string() } else { c.to_string() };
        return Err(format!("Caractère interdit : {shown}"));
    }
    if name.ends_with(' ') || name.ends_with('.') {
        return Err("Ne peut pas finir par un espace ou un point".into());
    }
    let base = name.split('.').next().unwrap_or("").trim().to_uppercase();
    if RESERVED.contains(&base.as_str()) {
        return Err(format!("Nom réservé par Windows : {base}"));
    }
    if name.encode_utf16().count() > 255 {
        return Err("Nom trop long (plus de 255 caractères)".into());
    }
    Ok(())
}

fn key(p: &Path) -> String {
    p.to_string_lossy().to_lowercase()
}

fn with_suffix(name: &str, is_dir: bool, k: usize) -> String {
    let n = Name::split(name, is_dir);
    let stem = format!("{} ({k})", n.stem);
    Name { stem, ext: n.ext }.join()
}

/// Calcule le nouveau nom et le statut de chaque entrée.
pub fn compute(entries: &[Entry], rules: &[RuleItem], opts: PreviewOptions) -> (Vec<Preview>, Stats) {
    let total_checked = entries.iter().filter(|e| e.checked).count();
    let mut folder_counters: HashMap<PathBuf, usize> = HashMap::new();
    let mut previews = Vec::with_capacity(entries.len());
    let mut index = 0usize;

    for e in entries {
        if !e.checked {
            previews.push(Preview { new_name: e.file_name(), status: Status::Skipped });
            continue;
        }
        let parent = e.parent();
        let folder_index = {
            let c = folder_counters.entry(parent).or_insert(0);
            let v = *c;
            *c += 1;
            v
        };
        let original = Name::split(&e.file_name(), e.is_dir);
        let ctx = TokenContext {
            index,
            folder_index,
            total: total_checked,
            is_dir: e.is_dir,
            original: Some(&original),
            path: Some(&e.path),
            meta: Some(&e.meta),
        };
        index += 1;
        let preview = match rules::apply_all(rules, &original, &ctx) {
            Ok(n) => {
                let new_name = n.join();
                let status = if new_name == e.file_name() {
                    Status::Unchanged
                } else if let Err(m) = validate_name(&new_name) {
                    Status::Invalid(m)
                } else {
                    Status::Ok
                };
                Preview { new_name, status }
            }
            Err(m) => Preview { new_name: e.file_name(), status: Status::Invalid(m) },
        };
        previews.push(preview);
    }

    // Chemins qui seront libérés par le lot (sources renommées).
    let sources: HashSet<String> = entries
        .iter()
        .zip(&previews)
        .filter(|(_, p)| p.status.will_rename())
        .map(|(e, _)| key(&e.path))
        .collect();
    // Chemins occupés par des fichiers qui ne bougent pas.
    let mut taken: HashSet<String> = HashSet::new();
    for (e, p) in entries.iter().zip(&previews) {
        if !p.status.will_rename() {
            taken.insert(key(&e.path));
        }
    }

    for (i, e) in entries.iter().enumerate() {
        if !previews[i].status.will_rename() {
            continue;
        }
        if !e.path.exists() {
            previews[i].status = Status::Missing;
            continue;
        }
        let parent = e.parent();
        let mut candidate = previews[i].new_name.clone();
        let mut k = 2;
        loop {
            let target = parent.join(&candidate);
            let tk = key(&target);
            let on_disk = target.exists() && !sources.contains(&tk);
            if !taken.contains(&tk) && !on_disk {
                taken.insert(tk);
                if k > 2 {
                    previews[i].new_name = candidate;
                    previews[i].status = Status::Resolved;
                }
                break;
            }
            if !opts.auto_resolve || k > 9999 {
                previews[i].status = if on_disk { Status::Exists } else { Status::Duplicate };
                break;
            }
            candidate = with_suffix(&previews[i].new_name, e.is_dir, k);
            k += 1;
        }
    }

    // Marque aussi l'autre membre d'un doublon (le premier a pris la place).
    if !opts.auto_resolve {
        let mut first: HashMap<String, usize> = HashMap::new();
        for (i, e) in entries.iter().enumerate() {
            let p = &previews[i];
            if p.status.will_rename() || p.status == Status::Duplicate {
                let tk = key(&e.parent().join(&p.new_name));
                if p.status == Status::Duplicate {
                    if let Some(&j) = first.get(&tk) {
                        previews[j].status = Status::Duplicate;
                    }
                } else {
                    first.insert(tk, i);
                }
            }
        }
    }

    let mut stats = Stats { total: entries.len(), checked: total_checked, ..Default::default() };
    for p in &previews {
        match &p.status {
            Status::Ok => stats.to_rename += 1,
            Status::Resolved => {
                stats.to_rename += 1;
                stats.resolved += 1;
            }
            Status::Unchanged => stats.unchanged += 1,
            s if s.is_error() => stats.errors += 1,
            _ => {}
        }
    }
    (previews, stats)
}

/// Un renommage effectué (pour l'annulation).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Op {
    pub from: PathBuf,
    pub to: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Batch {
    pub timestamp: String,
    pub ops: Vec<Op>,
    #[serde(default)]
    pub undone: bool,
}

#[derive(Debug, Default)]
pub struct Outcome {
    pub done: Vec<Op>,
    pub errors: Vec<String>,
}

fn depth(p: &Path) -> usize {
    p.components().count()
}

fn temp_path(src: &Path, i: usize) -> PathBuf {
    let parent = src.parent().unwrap_or(Path::new("."));
    let pid = std::process::id();
    let mut k = 0;
    loop {
        let candidate = parent.join(format!(".namify-{pid}-{i}-{k}.tmp"));
        if !candidate.exists() {
            return candidate;
        }
        k += 1;
    }
}

/// Exécute un plan (source -> cible) en deux passes, niveau de profondeur par niveau
/// de profondeur (les plus profonds d'abord, pour que renommer un dossier
/// n'invalide pas les chemins de son contenu).
pub fn execute(plan: &[Op]) -> Outcome {
    let mut out = Outcome::default();
    let mut by_depth: Vec<&Op> = plan.iter().collect();
    by_depth.sort_by_key(|op| std::cmp::Reverse(depth(&op.from)));

    let mut start = 0;
    while start < by_depth.len() {
        let d = depth(&by_depth[start].from);
        let end = by_depth[start..].iter().position(|op| depth(&op.from) != d).map_or(by_depth.len(), |p| start + p);
        let level = &by_depth[start..end];

        // Passe 1 : vers des noms temporaires uniques.
        let mut staged: Vec<(PathBuf, &Op)> = Vec::with_capacity(level.len());
        for (i, op) in level.iter().enumerate() {
            let tmp = temp_path(&op.from, start + i);
            match fs::rename(&op.from, &tmp) {
                Ok(()) => staged.push((tmp, op)),
                Err(e) => out.errors.push(format!("{} : {e}", op.from.display())),
            }
        }
        // Passe 2 : vers les noms définitifs, avec retour arrière en cas d'échec.
        for (tmp, op) in staged {
            let dst_taken = op.to.exists() && key(&op.to) != key(&op.from);
            let res = if dst_taken {
                Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "la cible existe déjà"))
            } else {
                fs::rename(&tmp, &op.to)
            };
            match res {
                Ok(()) => out.done.push(op.clone()),
                Err(e) => {
                    let _ = fs::rename(&tmp, &op.from);
                    out.errors.push(format!("{} : {e}", op.from.display()));
                }
            }
        }
        start = end;
    }
    out
}

/// Inverse un lot : chaque cible reprend son nom d'origine.
pub fn undo(batch: &Batch) -> Outcome {
    let plan: Vec<Op> = batch.ops.iter().map(|op| Op { from: op.to.clone(), to: op.from.clone() }).collect();
    // Un dossier renommé a pu changer le chemin de ses enfants : on recalcule
    // les chemins sources en appliquant les renommages de dossiers ancêtres.
    let dir_ops: Vec<&Op> = batch.ops.iter().filter(|op| op.to.is_dir()).collect();
    let fixed: Vec<Op> = plan
        .into_iter()
        .map(|op| {
            let mut from = op.from.clone();
            let mut to = op.to.clone();
            for d in &dir_ops {
                if from != d.to {
                    if let Ok(rel) = from.strip_prefix(&d.from) {
                        from = d.to.join(rel);
                    }
                }
                if to != d.from {
                    if let Ok(rel) = to.strip_prefix(&d.from) {
                        to = d.to.join(rel);
                    }
                }
            }
            Op { from, to }
        })
        .collect();
    execute(&fixed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::{Rule, RuleKind};

    fn tmpdir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("namify-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn validates_names() {
        assert!(validate_name("ok.txt").is_ok());
        assert!(validate_name("a?b").is_err());
        assert!(validate_name("CON.txt").is_err());
        assert!(validate_name("trail.").is_err());
        assert!(validate_name("   ").is_err());
    }

    #[test]
    fn swap_names_and_undo() {
        let d = tmpdir("swap");
        fs::write(d.join("a.txt"), "A").unwrap();
        fs::write(d.join("b.txt"), "B").unwrap();
        let rules = vec![RuleItem::new(Rule::Template { pattern: "{n}".into() })];
        let entries = vec![Entry::new(d.join("b.txt")), Entry::new(d.join("a.txt"))];
        // b -> 1, a -> 2 : pas de conflit
        let (prev, stats) = compute(&entries, &rules, PreviewOptions { auto_resolve: false });
        assert_eq!(stats.to_rename, 2);
        assert_eq!(prev[0].new_name, "1.txt");

        // Permutation a <-> b via un plan explicite
        let plan = vec![
            Op { from: d.join("a.txt"), to: d.join("b.txt") },
            Op { from: d.join("b.txt"), to: d.join("a.txt") },
        ];
        let out = execute(&plan);
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        assert_eq!(fs::read_to_string(d.join("a.txt")).unwrap(), "B");
        let batch = Batch { timestamp: String::new(), ops: out.done, undone: false };
        let out = undo(&batch);
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        assert_eq!(fs::read_to_string(d.join("a.txt")).unwrap(), "A");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn detects_duplicates_and_resolves() {
        let d = tmpdir("dup");
        fs::write(d.join("x1.txt"), "").unwrap();
        fs::write(d.join("x2.txt"), "").unwrap();
        fs::write(d.join("keep.txt"), "").unwrap();
        let rules = vec![RuleItem::new(Rule::Template { pattern: "keep".into() })];
        let entries = vec![Entry::new(d.join("x1.txt")), Entry::new(d.join("x2.txt"))];
        let (prev, stats) = compute(&entries, &rules, PreviewOptions { auto_resolve: false });
        assert_eq!(prev[0].status, Status::Exists);
        assert_eq!(stats.errors, 2);
        let (prev, _) = compute(&entries, &rules, PreviewOptions { auto_resolve: true });
        assert_eq!(prev[0].new_name, "keep (2).txt");
        assert_eq!(prev[1].new_name, "keep (3).txt");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn renames_folder_and_its_content() {
        let d = tmpdir("tree");
        fs::create_dir_all(d.join("dir")).unwrap();
        fs::write(d.join("dir").join("f.txt"), "F").unwrap();
        let rules = vec![RuleItem::new(RuleKind::Case.default_rule()), RuleItem::new(Rule::Case {
            mode: crate::rules::CaseMode::Upper,
            scope: crate::rules::Scope::Name,
        })];
        let entries = vec![Entry::new(d.join("dir")), Entry::new(d.join("dir").join("f.txt"))];
        let (prev, _) = compute(&entries, &rules, PreviewOptions { auto_resolve: false });
        let plan: Vec<Op> = entries
            .iter()
            .zip(&prev)
            .filter(|(_, p)| p.status.will_rename())
            .map(|(e, p)| Op { from: e.path.clone(), to: e.parent().join(&p.new_name) })
            .collect();
        let out = execute(&plan);
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        assert!(d.join("DIR").join("F.txt").exists());
        let out = undo(&Batch { timestamp: String::new(), ops: out.done, undone: false });
        assert!(out.errors.is_empty(), "{:?}", out.errors);
        assert!(d.join("dir").join("f.txt").exists());
        let _ = fs::remove_dir_all(&d);
    }
}
