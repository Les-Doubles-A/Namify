//! Règles de renommage : chaque règle transforme le couple (nom, extension)
//! d'un fichier. Les règles sont appliquées dans l'ordre du pipeline.

use std::borrow::Cow;

use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use crate::tokens::{self, TokenContext};

/// Partie du nom concernée par une règle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    Name,
    Extension,
    Full,
}

impl Scope {
    pub const ALL: [Scope; 3] = [Scope::Name, Scope::Extension, Scope::Full];
    pub fn label(self) -> &'static str {
        match self {
            Scope::Name => "Nom",
            Scope::Extension => "Extension",
            Scope::Full => "Nom + extension",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Occurrence {
    All,
    First,
    Last,
}

impl Occurrence {
    pub const ALL: [Occurrence; 3] = [Occurrence::All, Occurrence::First, Occurrence::Last];
    pub fn label(self) -> &'static str {
        match self {
            Occurrence::All => "Toutes",
            Occurrence::First => "Première",
            Occurrence::Last => "Dernière",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Position {
    Start,
    End,
    Index,
    BeforeText,
    AfterText,
}

impl Position {
    pub const ALL: [Position; 5] = [
        Position::Start,
        Position::End,
        Position::Index,
        Position::BeforeText,
        Position::AfterText,
    ];
    pub const SIMPLE: [Position; 3] = [Position::Start, Position::End, Position::Index];
    pub fn label(self) -> &'static str {
        match self {
            Position::Start => "Au début",
            Position::End => "À la fin",
            Position::Index => "À la position",
            Position::BeforeText => "Avant le texte",
            Position::AfterText => "Après le texte",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoveMode {
    FirstN,
    LastN,
    Range,
    Text,
    Digits,
    Letters,
    Between,
    BeforeText,
    AfterText,
}

impl RemoveMode {
    pub const ALL: [RemoveMode; 9] = [
        RemoveMode::FirstN,
        RemoveMode::LastN,
        RemoveMode::Range,
        RemoveMode::Text,
        RemoveMode::Digits,
        RemoveMode::Letters,
        RemoveMode::Between,
        RemoveMode::BeforeText,
        RemoveMode::AfterText,
    ];
    pub fn label(self) -> &'static str {
        match self {
            RemoveMode::FirstN => "N premiers caractères",
            RemoveMode::LastN => "N derniers caractères",
            RemoveMode::Range => "Plage de caractères",
            RemoveMode::Text => "Un texte précis",
            RemoveMode::Digits => "Tous les chiffres",
            RemoveMode::Letters => "Toutes les lettres",
            RemoveMode::Between => "Entre deux délimiteurs",
            RemoveMode::BeforeText => "Tout avant un texte",
            RemoveMode::AfterText => "Tout après un texte",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseMode {
    Lower,
    Upper,
    Title,
    Sentence,
    Invert,
    Camel,
    Pascal,
    Snake,
    Kebab,
    ScreamingSnake,
}

impl CaseMode {
    pub const ALL: [CaseMode; 10] = [
        CaseMode::Lower,
        CaseMode::Upper,
        CaseMode::Title,
        CaseMode::Sentence,
        CaseMode::Invert,
        CaseMode::Camel,
        CaseMode::Pascal,
        CaseMode::Snake,
        CaseMode::Kebab,
        CaseMode::ScreamingSnake,
    ];
    pub fn label(self) -> &'static str {
        match self {
            CaseMode::Lower => "minuscules",
            CaseMode::Upper => "MAJUSCULES",
            CaseMode::Title => "Titre Pour Chaque Mot",
            CaseMode::Sentence => "Phrase avec majuscule",
            CaseMode::Invert => "iNVERSER lA cASSE",
            CaseMode::Camel => "camelCase",
            CaseMode::Pascal => "PascalCase",
            CaseMode::Snake => "snake_case",
            CaseMode::Kebab => "kebab-case",
            CaseMode::ScreamingSnake => "SCREAMING_SNAKE",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NumStyle {
    Decimal,
    AlphaLower,
    AlphaUpper,
    RomanUpper,
    RomanLower,
    Hex,
}

impl NumStyle {
    pub const ALL: [NumStyle; 6] = [
        NumStyle::Decimal,
        NumStyle::AlphaLower,
        NumStyle::AlphaUpper,
        NumStyle::RomanUpper,
        NumStyle::RomanLower,
        NumStyle::Hex,
    ];
    pub fn label(self) -> &'static str {
        match self {
            NumStyle::Decimal => "1, 2, 3",
            NumStyle::AlphaLower => "a, b, c",
            NumStyle::AlphaUpper => "A, B, C",
            NumStyle::RomanUpper => "I, II, III",
            NumStyle::RomanLower => "i, ii, iii",
            NumStyle::Hex => "Hexadécimal",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtMode {
    Lower,
    Upper,
    Set,
    Remove,
    Append,
}

impl ExtMode {
    pub const ALL: [ExtMode; 5] = [ExtMode::Lower, ExtMode::Upper, ExtMode::Set, ExtMode::Remove, ExtMode::Append];
    pub fn label(self) -> &'static str {
        match self {
            ExtMode::Lower => "En minuscules",
            ExtMode::Upper => "En majuscules",
            ExtMode::Set => "Remplacer par",
            ExtMode::Remove => "Supprimer",
            ExtMode::Append => "Ajouter une extension",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Rule {
    Replace {
        find: String,
        replace: String,
        regex: bool,
        case_sensitive: bool,
        scope: Scope,
        occurrence: Occurrence,
    },
    Insert {
        text: String,
        position: Position,
        index: i64,
        anchor: String,
        scope_ext: bool,
    },
    Remove {
        mode: RemoveMode,
        count: usize,
        from: usize,
        text: String,
        text2: String,
        case_sensitive: bool,
    },
    Case {
        mode: CaseMode,
        scope: Scope,
    },
    Numbering {
        start: i64,
        step: i64,
        padding: usize,
        position: Position,
        index: i64,
        separator: String,
        style: NumStyle,
        reset_per_folder: bool,
    },
    Clean {
        trim: bool,
        collapse_spaces: bool,
        separators_to_spaces: bool,
        spaces_to: String,
        remove_accents: bool,
        remove_special: bool,
        ascii_only: bool,
        remove_brackets: bool,
        fix_invalid: bool,
    },
    Extension {
        mode: ExtMode,
        value: String,
    },
    Template {
        pattern: String,
    },
    Truncate {
        max: usize,
        keep_end: bool,
    },
    Swap {
        separator: String,
    },
    PadNumbers {
        width: usize,
    },
}

/// Catégorie affichable d'une règle (menu « Ajouter une règle »).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleKind {
    Replace,
    Insert,
    Remove,
    Case,
    Numbering,
    Clean,
    Extension,
    Template,
    Truncate,
    Swap,
    PadNumbers,
}

impl RuleKind {
    pub const ALL: [RuleKind; 11] = [
        RuleKind::Replace,
        RuleKind::Insert,
        RuleKind::Remove,
        RuleKind::Case,
        RuleKind::Numbering,
        RuleKind::Template,
        RuleKind::Clean,
        RuleKind::Extension,
        RuleKind::Truncate,
        RuleKind::Swap,
        RuleKind::PadNumbers,
    ];

    pub fn title(self) -> &'static str {
        match self {
            RuleKind::Replace => "Rechercher et remplacer",
            RuleKind::Insert => "Insérer du texte",
            RuleKind::Remove => "Supprimer",
            RuleKind::Case => "Casse",
            RuleKind::Numbering => "Numérotation",
            RuleKind::Clean => "Nettoyage",
            RuleKind::Extension => "Extension",
            RuleKind::Template => "Modèle (balises)",
            RuleKind::Truncate => "Tronquer",
            RuleKind::Swap => "Permuter",
            RuleKind::PadNumbers => "Compléter les nombres",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            RuleKind::Replace => "Texte simple ou expression régulière, avec groupes $1",
            RuleKind::Insert => "Préfixe, suffixe ou position précise, balises acceptées",
            RuleKind::Remove => "Caractères, plages, chiffres, texte entre délimiteurs",
            RuleKind::Case => "minuscules, MAJUSCULES, Titre, camelCase, snake_case...",
            RuleKind::Numbering => "Compteur décimal, lettres, chiffres romains, par dossier",
            RuleKind::Clean => "Espaces, accents, caractères spéciaux, crochets",
            RuleKind::Extension => "Changer, supprimer ou modifier la casse de l'extension",
            RuleKind::Template => "Reconstruit le nom: {name}, {n:3}, {exif:%Y-%m-%d}...",
            RuleKind::Truncate => "Limite la longueur du nom",
            RuleKind::Swap => "« Artiste - Titre » devient « Titre - Artiste »",
            RuleKind::PadNumbers => "img7 devient img007 pour un tri naturel",
        }
    }

    pub fn icon(self) -> &'static str {
        use crate::icons as i;
        match self {
            RuleKind::Replace => i::ARROWS_LEFT_RIGHT,
            RuleKind::Insert => i::CURSOR_TEXT,
            RuleKind::Remove => i::SCISSORS,
            RuleKind::Case => i::TEXT_AA,
            RuleKind::Numbering => i::LIST_NUMBERS,
            RuleKind::Clean => i::BROOM,
            RuleKind::Extension => i::FILE_TEXT,
            RuleKind::Template => i::BRACKETS_CURLY,
            RuleKind::Truncate => i::RULER,
            RuleKind::Swap => i::SWAP,
            RuleKind::PadNumbers => i::HASH,
        }
    }

    pub fn default_rule(self) -> Rule {
        match self {
            RuleKind::Replace => Rule::Replace {
                find: String::new(),
                replace: String::new(),
                regex: false,
                case_sensitive: false,
                scope: Scope::Name,
                occurrence: Occurrence::All,
            },
            RuleKind::Insert => Rule::Insert {
                text: String::new(),
                position: Position::Start,
                index: 0,
                anchor: String::new(),
                scope_ext: false,
            },
            RuleKind::Remove => Rule::Remove {
                mode: RemoveMode::FirstN,
                count: 1,
                from: 0,
                text: String::new(),
                text2: String::new(),
                case_sensitive: false,
            },
            RuleKind::Case => Rule::Case { mode: CaseMode::Title, scope: Scope::Name },
            RuleKind::Numbering => Rule::Numbering {
                start: 1,
                step: 1,
                padding: 3,
                position: Position::End,
                index: 0,
                separator: "_".into(),
                style: NumStyle::Decimal,
                reset_per_folder: false,
            },
            RuleKind::Clean => Rule::Clean {
                trim: true,
                collapse_spaces: true,
                separators_to_spaces: false,
                spaces_to: String::new(),
                remove_accents: false,
                remove_special: false,
                ascii_only: false,
                remove_brackets: false,
                fix_invalid: true,
            },
            RuleKind::Extension => Rule::Extension { mode: ExtMode::Lower, value: String::new() },
            RuleKind::Template => Rule::Template { pattern: "{name}".into() },
            RuleKind::Truncate => Rule::Truncate { max: 40, keep_end: false },
            RuleKind::Swap => Rule::Swap { separator: " - ".into() },
            RuleKind::PadNumbers => Rule::PadNumbers { width: 3 },
        }
    }
}

impl Rule {
    pub fn kind(&self) -> RuleKind {
        match self {
            Rule::Replace { .. } => RuleKind::Replace,
            Rule::Insert { .. } => RuleKind::Insert,
            Rule::Remove { .. } => RuleKind::Remove,
            Rule::Case { .. } => RuleKind::Case,
            Rule::Numbering { .. } => RuleKind::Numbering,
            Rule::Clean { .. } => RuleKind::Clean,
            Rule::Extension { .. } => RuleKind::Extension,
            Rule::Template { .. } => RuleKind::Template,
            Rule::Truncate { .. } => RuleKind::Truncate,
            Rule::Swap { .. } => RuleKind::Swap,
            Rule::PadNumbers { .. } => RuleKind::PadNumbers,
        }
    }

    /// Résumé court affiché dans l'en-tête de la carte de règle.
    pub fn summary(&self) -> String {
        match self {
            Rule::Replace { find, replace, regex, .. } => {
                if find.is_empty() {
                    "(rien à rechercher)".into()
                } else {
                    format!("{}« {} » → « {} »", if *regex { "regex " } else { "" }, find, replace)
                }
            }
            Rule::Insert { text, position, index, anchor, .. } => match position {
                Position::Index => format!("« {text} » à la position {index}"),
                Position::BeforeText | Position::AfterText => {
                    format!("« {text} » {} « {anchor} »", position.label().to_lowercase())
                }
                _ => format!("« {text} » {}", position.label().to_lowercase()),
            },
            Rule::Remove { mode, count, from, text, text2, .. } => match mode {
                RemoveMode::FirstN => format!("{count} premier(s)"),
                RemoveMode::LastN => format!("{count} dernier(s)"),
                RemoveMode::Range => format!("{count} à partir de {from}"),
                RemoveMode::Text => format!("« {text} »"),
                RemoveMode::Between => format!("entre « {text} » et « {text2} »"),
                RemoveMode::BeforeText => format!("avant « {text} »"),
                RemoveMode::AfterText => format!("après « {text} »"),
                m => m.label().to_string(),
            },
            Rule::Case { mode, .. } => mode.label().to_string(),
            Rule::Numbering { start, step, padding, style, .. } => {
                format!("{} dès {start}, pas {step}, {padding} chiffres", style.label())
            }
            Rule::Clean { .. } => "espaces, caractères, accents".into(),
            Rule::Extension { mode, value } => match mode {
                ExtMode::Set | ExtMode::Append => format!("{} .{value}", mode.label().to_lowercase()),
                m => m.label().to_lowercase(),
            },
            Rule::Template { pattern } => pattern.clone(),
            Rule::Truncate { max, .. } => format!("{max} caractères max"),
            Rule::Swap { separator } => format!("autour de « {separator} »"),
            Rule::PadNumbers { width } => format!("{width} chiffres"),
        }
    }

}

/// Une règle dans le pipeline, avec son état d'activation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RuleItem {
    pub enabled: bool,
    #[serde(default)]
    pub collapsed: bool,
    pub rule: Rule,
}

impl RuleItem {
    pub fn new(rule: Rule) -> Self {
        Self { enabled: true, collapsed: false, rule }
    }
}

/// Nom de fichier en cours de transformation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name {
    pub stem: String,
    /// Extension sans le point. Vide si aucune.
    pub ext: String,
}

impl Name {
    pub fn split(file_name: &str, is_dir: bool) -> Self {
        if is_dir {
            return Self { stem: file_name.to_string(), ext: String::new() };
        }
        match file_name.rfind('.') {
            Some(pos) if pos > 0 && pos + 1 < file_name.len() => Self {
                stem: file_name[..pos].to_string(),
                ext: file_name[pos + 1..].to_string(),
            },
            _ => Self { stem: file_name.to_string(), ext: String::new() },
        }
    }

    pub fn join(&self) -> String {
        if self.ext.is_empty() {
            self.stem.clone()
        } else {
            format!("{}.{}", self.stem, self.ext)
        }
    }

    fn get(&self, scope: Scope) -> String {
        match scope {
            Scope::Name => self.stem.clone(),
            Scope::Extension => self.ext.clone(),
            Scope::Full => self.join(),
        }
    }

    fn set(&mut self, scope: Scope, value: String, is_dir: bool) {
        match scope {
            Scope::Name => self.stem = value,
            Scope::Extension => self.ext = value,
            Scope::Full => *self = Name::split(&value, is_dir),
        }
    }
}

/// Applique tout le pipeline à un nom de fichier.
pub fn apply_all(rules: &[RuleItem], original: &Name, ctx: &TokenContext) -> Result<Name, String> {
    let mut name = original.clone();
    for item in rules.iter().filter(|r| r.enabled) {
        apply(&item.rule, &mut name, ctx)?;
    }
    Ok(name)
}

pub fn apply(rule: &Rule, name: &mut Name, ctx: &TokenContext) -> Result<(), String> {
    let is_dir = ctx.is_dir;
    match rule {
        Rule::Replace { find, replace, regex, case_sensitive, scope, occurrence } => {
            if find.is_empty() {
                return Ok(());
            }
            let target = name.get(*scope);
            let replacement = tokens::expand(replace, ctx, name)?;
            let pattern = if *regex { find.clone() } else { regex::escape(find) };
            let re = RegexBuilder::new(&pattern)
                .case_insensitive(!case_sensitive)
                .build()
                .map_err(|e| format!("Expression régulière invalide : {}", first_line(&e.to_string())))?;
            // En mode texte simple, « $ » doit rester littéral.
            let rep: Cow<str> = if *regex { Cow::Borrowed(&replacement) } else { Cow::Owned(replacement.replace('$', "$$")) };
            let out = match occurrence {
                Occurrence::All => re.replace_all(&target, rep.as_ref()).into_owned(),
                Occurrence::First => re.replace(&target, rep.as_ref()).into_owned(),
                Occurrence::Last => match re.find_iter(&target).last() {
                    Some(m) => {
                        let tail = &target[m.start()..];
                        let replaced = re.replace(tail, rep.as_ref());
                        format!("{}{}", &target[..m.start()], replaced)
                    }
                    None => target,
                },
            };
            name.set(*scope, out, is_dir);
        }
        Rule::Insert { text, position, index, anchor, scope_ext } => {
            let text = tokens::expand(text, ctx, name)?;
            if text.is_empty() {
                return Ok(());
            }
            let scope = if *scope_ext { Scope::Extension } else { Scope::Name };
            let target = name.get(scope);
            let out = insert_at(&target, &text, *position, *index, anchor);
            name.set(scope, out, is_dir);
        }
        Rule::Remove { mode, count, from, text, text2, case_sensitive } => {
            name.stem = remove(&name.stem, *mode, *count, *from, text, text2, *case_sensitive);
        }
        Rule::Case { mode, scope } => {
            let target = name.get(*scope);
            name.set(*scope, change_case(&target, *mode), is_dir);
        }
        Rule::Numbering { start, step, padding, position, index, separator, style, reset_per_folder } => {
            let i = if *reset_per_folder { ctx.folder_index } else { ctx.index } as i64;
            let value = start.saturating_add(step.saturating_mul(i));
            let num = format_number(value, *style, *padding);
            name.stem = match position {
                Position::Start => format!("{num}{separator}{}", name.stem),
                Position::End => format!("{}{separator}{num}", name.stem),
                _ => insert_at(&name.stem, &num, *position, *index, ""),
            };
        }
        Rule::Clean {
            trim,
            collapse_spaces,
            separators_to_spaces,
            spaces_to,
            remove_accents,
            remove_special,
            ascii_only,
            remove_brackets,
            fix_invalid,
        } => {
            let mut s = name.stem.clone();
            if *remove_brackets {
                s = strip_brackets(&s);
            }
            if *separators_to_spaces {
                s = s.replace(['_', '.'], " ");
            }
            if *remove_accents {
                s = strip_accents(&s);
            }
            if *remove_special {
                s = s
                    .chars()
                    .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '(' | ')'))
                    .collect();
            }
            if *ascii_only {
                s = s.chars().filter(|c| c.is_ascii() && !c.is_ascii_control()).collect();
            }
            if *fix_invalid {
                s = s.chars().map(|c| if is_invalid_char(c) { '_' } else { c }).collect();
            }
            if *collapse_spaces {
                s = s.split_whitespace().collect::<Vec<_>>().join(" ");
            }
            if *trim {
                s = s.trim().trim_end_matches('.').trim().to_string();
            }
            if !spaces_to.is_empty() {
                s = s.replace(' ', spaces_to);
            }
            name.stem = s;
        }
        Rule::Extension { mode, value } => {
            let value = value.trim_start_matches('.');
            match mode {
                ExtMode::Lower => name.ext = name.ext.to_lowercase(),
                ExtMode::Upper => name.ext = name.ext.to_uppercase(),
                ExtMode::Set => name.ext = value.to_string(),
                ExtMode::Remove => name.ext.clear(),
                ExtMode::Append => {
                    if !value.is_empty() {
                        if name.ext.is_empty() {
                            name.ext = value.to_string();
                        } else {
                            name.ext = format!("{}.{}", name.ext, value);
                        }
                    }
                }
            }
        }
        Rule::Template { pattern } => {
            name.stem = tokens::expand(pattern, ctx, name)?;
        }
        Rule::Truncate { max, keep_end } => {
            let n = name.stem.chars().count();
            if n > *max {
                name.stem = if *keep_end {
                    name.stem.chars().skip(n - max).collect()
                } else {
                    name.stem.chars().take(*max).collect()
                };
            }
        }
        Rule::Swap { separator } => {
            if !separator.is_empty() {
                if let Some(pos) = name.stem.find(separator.as_str()) {
                    let (a, b) = (&name.stem[..pos], &name.stem[pos + separator.len()..]);
                    name.stem = format!("{b}{separator}{a}");
                }
            }
        }
        Rule::PadNumbers { width } => {
            let re = regex::Regex::new(r"\d+").expect("regex statique");
            name.stem = re
                .replace_all(&name.stem, |c: &regex::Captures| format!("{:0>width$}", &c[0], width = *width))
                .into_owned();
        }
    }
    Ok(())
}

fn first_line(s: &str) -> String {
    s.lines().last().unwrap_or(s).trim().to_string()
}

/// Convertit un index de caractères (éventuellement négatif = depuis la fin) en index d'octet.
fn char_to_byte(s: &str, index: i64) -> usize {
    let len = s.chars().count() as i64;
    let idx = if index < 0 { (len + index).max(0) } else { index.min(len) } as usize;
    s.char_indices().nth(idx).map(|(b, _)| b).unwrap_or(s.len())
}

fn insert_at(target: &str, text: &str, position: Position, index: i64, anchor: &str) -> String {
    match position {
        Position::Start => format!("{text}{target}"),
        Position::End => format!("{target}{text}"),
        Position::Index => {
            let b = char_to_byte(target, index);
            format!("{}{}{}", &target[..b], text, &target[b..])
        }
        Position::BeforeText | Position::AfterText => {
            if anchor.is_empty() {
                return target.to_string();
            }
            match target.find(anchor) {
                Some(pos) => {
                    let b = if position == Position::BeforeText { pos } else { pos + anchor.len() };
                    format!("{}{}{}", &target[..b], text, &target[b..])
                }
                None => target.to_string(),
            }
        }
    }
}

fn find_ci(hay: &str, needle: &str, case_sensitive: bool) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return None;
    }
    let re = RegexBuilder::new(&regex::escape(needle)).case_insensitive(!case_sensitive).build().ok()?;
    re.find(hay).map(|m| (m.start(), m.end()))
}

fn remove(s: &str, mode: RemoveMode, count: usize, from: usize, text: &str, text2: &str, cs: bool) -> String {
    let chars: Vec<char> = s.chars().collect();
    match mode {
        RemoveMode::FirstN => chars.iter().skip(count).collect(),
        RemoveMode::LastN => chars.iter().take(chars.len().saturating_sub(count)).collect(),
        RemoveMode::Range => chars
            .iter()
            .enumerate()
            .filter(|(i, _)| *i < from || *i >= from.saturating_add(count))
            .map(|(_, c)| *c)
            .collect(),
        RemoveMode::Text => {
            if text.is_empty() {
                return s.to_string();
            }
            match RegexBuilder::new(&regex::escape(text)).case_insensitive(!cs).build() {
                Ok(re) => re.replace_all(s, "").into_owned(),
                Err(_) => s.to_string(),
            }
        }
        RemoveMode::Digits => chars.iter().filter(|c| !c.is_ascii_digit()).collect(),
        RemoveMode::Letters => chars.iter().filter(|c| !c.is_alphabetic()).collect(),
        RemoveMode::Between => {
            if text.is_empty() || text2.is_empty() {
                return s.to_string();
            }
            let mut out = s.to_string();
            // Supprime chaque occurrence « début ... fin » (délimiteurs inclus).
            loop {
                let Some((a, a_end)) = find_ci(&out, text, cs) else { break };
                let Some((_, b_end)) = find_ci(&out[a_end..], text2, cs) else { break };
                out.replace_range(a..a_end + b_end, "");
            }
            out
        }
        RemoveMode::BeforeText => match find_ci(s, text, cs) {
            Some((a, _)) => s[a..].to_string(),
            None => s.to_string(),
        },
        RemoveMode::AfterText => match find_ci(s, text, cs) {
            Some((_, e)) => s[..e].to_string(),
            None => s.to_string(),
        },
    }
}

/// Découpe un nom en mots (espaces, séparateurs et transitions minuscule→majuscule).
fn words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev: Option<char> = None;
    for c in s.chars() {
        if c.is_whitespace() || matches!(c, '_' | '-' | '.') {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev = None;
            continue;
        }
        if let Some(p) = prev {
            if (p.is_lowercase() && c.is_uppercase()) || (p.is_alphabetic() != c.is_alphabetic() && (p.is_ascii_digit() || c.is_ascii_digit())) {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
        }
        cur.push(c);
        prev = Some(c);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn capitalize(w: &str) -> String {
    let mut it = w.chars();
    match it.next() {
        Some(f) => f.to_uppercase().collect::<String>() + &it.as_str().to_lowercase(),
        None => String::new(),
    }
}

pub fn change_case(s: &str, mode: CaseMode) -> String {
    match mode {
        CaseMode::Lower => s.to_lowercase(),
        CaseMode::Upper => s.to_uppercase(),
        CaseMode::Title => {
            let mut out = String::with_capacity(s.len());
            let mut start = true;
            for c in s.chars() {
                if c.is_alphanumeric() || c == '\'' {
                    if start {
                        out.extend(c.to_uppercase());
                    } else {
                        out.extend(c.to_lowercase());
                    }
                    start = false;
                } else {
                    out.push(c);
                    start = true;
                }
            }
            out
        }
        CaseMode::Sentence => capitalize(s),
        CaseMode::Invert => s
            .chars()
            .map(|c| {
                if c.is_uppercase() {
                    c.to_lowercase().collect::<String>()
                } else {
                    c.to_uppercase().collect::<String>()
                }
            })
            .collect(),
        CaseMode::Camel => {
            let w = words(s);
            w.iter()
                .enumerate()
                .map(|(i, w)| if i == 0 { w.to_lowercase() } else { capitalize(w) })
                .collect()
        }
        CaseMode::Pascal => words(s).iter().map(|w| capitalize(w)).collect(),
        CaseMode::Snake => words(s).iter().map(|w| w.to_lowercase()).collect::<Vec<_>>().join("_"),
        CaseMode::Kebab => words(s).iter().map(|w| w.to_lowercase()).collect::<Vec<_>>().join("-"),
        CaseMode::ScreamingSnake => words(s).iter().map(|w| w.to_uppercase()).collect::<Vec<_>>().join("_"),
    }
}

pub fn format_number(value: i64, style: NumStyle, padding: usize) -> String {
    match style {
        NumStyle::Decimal => {
            if value < 0 {
                format!("-{:0>width$}", value.unsigned_abs(), width = padding)
            } else {
                format!("{:0>width$}", value, width = padding)
            }
        }
        NumStyle::Hex => format!("{:0>width$x}", value.max(0), width = padding),
        NumStyle::AlphaLower | NumStyle::AlphaUpper => {
            // 1 -> a, 26 -> z, 27 -> aa (numérotation bijective base 26)
            let mut n = value.max(1) as u64;
            let mut s = Vec::new();
            while n > 0 {
                n -= 1;
                s.push(b'a' + (n % 26) as u8);
                n /= 26;
            }
            s.reverse();
            let s = String::from_utf8(s).unwrap_or_default();
            if style == NumStyle::AlphaUpper { s.to_uppercase() } else { s }
        }
        NumStyle::RomanUpper | NumStyle::RomanLower => {
            let s = to_roman(value);
            if style == NumStyle::RomanLower { s.to_lowercase() } else { s }
        }
    }
}

fn to_roman(value: i64) -> String {
    if !(1..=3999).contains(&value) {
        return value.to_string();
    }
    const TABLE: [(i64, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut n = value;
    let mut out = String::new();
    for (v, s) in TABLE {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    out
}

pub fn strip_accents(s: &str) -> String {
    s.nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .map(|c| match c {
            'ß' => "ss".to_string(),
            'æ' => "ae".to_string(),
            'Æ' => "AE".to_string(),
            'œ' => "oe".to_string(),
            'Œ' => "OE".to_string(),
            'ø' => "o".to_string(),
            'Ø' => "O".to_string(),
            'đ' => "d".to_string(),
            'ł' => "l".to_string(),
            'Ł' => "L".to_string(),
            c => c.to_string(),
        })
        .collect()
}

fn strip_brackets(s: &str) -> String {
    let re = regex::Regex::new(r"\s*(\([^()]*\)|\[[^\[\]]*\]|\{[^{}]*\})").expect("regex statique");
    let mut out = s.to_string();
    // Plusieurs passes pour gérer les crochets imbriqués.
    for _ in 0..4 {
        let next = re.replace_all(&out, "").into_owned();
        if next == out {
            break;
        }
        out = next;
    }
    out
}

pub fn is_invalid_char(c: char) -> bool {
    matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::TokenContext;

    fn ctx(index: usize) -> TokenContext<'static> {
        TokenContext::test(index)
    }

    fn run(rule: Rule, file: &str) -> String {
        let mut n = Name::split(file, false);
        apply(&rule, &mut n, &ctx(0)).unwrap();
        n.join()
    }

    #[test]
    fn split_and_join() {
        assert_eq!(Name::split("a.tar.gz", false), Name { stem: "a.tar".into(), ext: "gz".into() });
        assert_eq!(Name::split(".gitignore", false).ext, "");
        assert_eq!(Name::split("noext", false).join(), "noext");
        assert_eq!(Name::split("dir.v2", true).ext, "");
    }

    #[test]
    fn replace_plain_and_regex() {
        let r = Rule::Replace {
            find: "img".into(),
            replace: "Photo$".into(),
            regex: false,
            case_sensitive: false,
            scope: Scope::Name,
            occurrence: Occurrence::All,
        };
        assert_eq!(run(r, "IMG_01.jpg"), "Photo$_01.jpg");
        let r = Rule::Replace {
            find: r"(\w+) - (\w+)".into(),
            replace: "$2 - $1".into(),
            regex: true,
            case_sensitive: true,
            scope: Scope::Name,
            occurrence: Occurrence::All,
        };
        assert_eq!(run(r, "Artist - Song.mp3"), "Song - Artist.mp3");
        let r = Rule::Replace {
            find: "a".into(),
            replace: "X".into(),
            regex: false,
            case_sensitive: true,
            scope: Scope::Name,
            occurrence: Occurrence::Last,
        };
        assert_eq!(run(r, "banana.txt"), "bananX.txt");
    }

    #[test]
    fn insert_positions() {
        let mk = |position, index| Rule::Insert {
            text: "X".into(),
            position,
            index,
            anchor: "b".into(),
            scope_ext: false,
        };
        assert_eq!(run(mk(Position::Start, 0), "abc.t"), "Xabc.t");
        assert_eq!(run(mk(Position::End, 0), "abc.t"), "abcX.t");
        assert_eq!(run(mk(Position::Index, 1), "abc.t"), "aXbc.t");
        assert_eq!(run(mk(Position::Index, -1), "abc.t"), "abXc.t");
        assert_eq!(run(mk(Position::AfterText, 0), "abc.t"), "abXc.t");
    }

    #[test]
    fn remove_modes() {
        let mk = |mode, text: &str, text2: &str| Rule::Remove {
            mode,
            count: 2,
            from: 1,
            text: text.into(),
            text2: text2.into(),
            case_sensitive: false,
        };
        assert_eq!(run(mk(RemoveMode::FirstN, "", ""), "abcdef.x"), "cdef.x");
        assert_eq!(run(mk(RemoveMode::LastN, "", ""), "abcdef.x"), "abcd.x");
        assert_eq!(run(mk(RemoveMode::Range, "", ""), "abcdef.x"), "adef.x");
        assert_eq!(run(mk(RemoveMode::Digits, "", ""), "a1b2.x"), "ab.x");
        assert_eq!(run(mk(RemoveMode::Between, "[", "]"), "a[1]b[2]c.x"), "abc.x");
        assert_eq!(run(mk(RemoveMode::AfterText, "-", ""), "a-b-c.x"), "a-.x");
    }

    #[test]
    fn cases() {
        assert_eq!(change_case("hello WORLD foo", CaseMode::Title), "Hello World Foo");
        assert_eq!(change_case("my file_name-here", CaseMode::Camel), "myFileNameHere");
        assert_eq!(change_case("MyFileName", CaseMode::Snake), "my_file_name");
        assert_eq!(change_case("My File", CaseMode::Kebab), "my-file");
        assert_eq!(change_case("aB", CaseMode::Invert), "Ab");
    }

    #[test]
    fn numbering_styles() {
        assert_eq!(format_number(7, NumStyle::Decimal, 3), "007");
        assert_eq!(format_number(27, NumStyle::AlphaLower, 0), "aa");
        assert_eq!(format_number(14, NumStyle::RomanUpper, 0), "XIV");
        assert_eq!(format_number(255, NumStyle::Hex, 4), "00ff");
        let r = RuleKind::Numbering.default_rule();
        let mut n = Name::split("photo.jpg", false);
        apply(&r, &mut n, &ctx(4)).unwrap();
        assert_eq!(n.join(), "photo_005.jpg");
    }

    #[test]
    fn clean_rule() {
        let r = Rule::Clean {
            trim: true,
            collapse_spaces: true,
            separators_to_spaces: true,
            spaces_to: "-".into(),
            remove_accents: true,
            remove_special: false,
            ascii_only: false,
            remove_brackets: true,
            fix_invalid: true,
        };
        assert_eq!(run(r, "  Été__à  la plage (copie) [2].jpg"), "Ete-a-la-plage.jpg");
    }

    #[test]
    fn misc_rules() {
        assert_eq!(run(Rule::Swap { separator: " - ".into() }, "A - B.mp3"), "B - A.mp3");
        assert_eq!(run(Rule::PadNumbers { width: 3 }, "ep7 part12.mkv"), "ep007 part012.mkv");
        assert_eq!(run(Rule::Truncate { max: 3, keep_end: true }, "abcdef.x"), "def.x");
        assert_eq!(run(Rule::Extension { mode: ExtMode::Set, value: ".png".into() }, "a.JPG"), "a.png");
        assert_eq!(run(Rule::Extension { mode: ExtMode::Remove, value: String::new() }, "a.JPG"), "a");
    }

    #[test]
    fn bad_regex_is_error() {
        let r = Rule::Replace {
            find: "(".into(),
            replace: String::new(),
            regex: true,
            case_sensitive: false,
            scope: Scope::Name,
            occurrence: Occurrence::All,
        };
        let mut n = Name::split("a.b", false);
        assert!(apply(&r, &mut n, &ctx(0)).is_err());
    }
}
