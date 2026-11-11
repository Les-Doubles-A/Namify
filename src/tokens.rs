//! Balises dynamiques utilisables dans les règles « Insérer », « Remplacer »
//! et « Modèle » : {name}, {n:3}, {mdate:%Y-%m-%d}, {exif}, {parent}...

use std::path::Path;

use chrono::{DateTime, Local};
use chrono::format::StrftimeItems;

use crate::meta::{FileMeta, human_size};
use crate::rules::Name;

pub struct TokenContext<'a> {
    /// Position (0-based) dans la liste des fichiers cochés.
    pub index: usize,
    /// Position (0-based) parmi les fichiers cochés du même dossier.
    pub folder_index: usize,
    pub total: usize,
    pub is_dir: bool,
    pub original: Option<&'a Name>,
    pub path: Option<&'a Path>,
    pub meta: Option<&'a FileMeta>,
}

impl TokenContext<'static> {
    pub fn test(index: usize) -> Self {
        Self { index, folder_index: index, total: 10, is_dir: false, original: None, path: None, meta: None }
    }
}

/// Description des balises pour l'aide intégrée : (balise, description, exemple).
pub const HELP: &[(&str, &str)] = &[
    ("{name}", "Nom d'origine sans extension"),
    ("{name:0:5}", "Sous-chaîne du nom d'origine (début, longueur)"),
    ("{cur}", "Nom courant, après les règles précédentes"),
    ("{ext}", "Extension d'origine (sans le point)"),
    ("{full}", "Nom d'origine complet"),
    ("{n} / {n:3}", "Compteur global (1, 2, 3...) avec remplissage"),
    ("{fn:2}", "Compteur remis à zéro pour chaque dossier"),
    ("{total}", "Nombre de fichiers cochés"),
    ("{parent}", "Nom du dossier parent ({parent:2} = grand-parent)"),
    ("{date:%Y-%m-%d}", "Date du jour (format strftime)"),
    ("{mdate:%Y%m%d}", "Date de modification du fichier"),
    ("{cdate:%Y%m%d}", "Date de création du fichier"),
    ("{exif:%Y-%m-%d_%H%M%S}", "Date de prise de vue EXIF (sinon modification)"),
    ("{make} / {model}", "Marque / modèle de l'appareil photo (EXIF)"),
    ("{width} / {height} / {dims}", "Dimensions de l'image (px)"),
    ("{size} / {bytes}", "Taille lisible (1.2 Mo) / taille en octets"),
    ("{crc}", "Empreinte CRC32 du contenu (8 caractères hexadécimaux)"),
    ("{{ et }}", "Accolades littérales"),
];

const DEFAULT_DATE: &str = "%Y-%m-%d";

fn fmt_date(date: Option<DateTime<Local>>, fmt: Option<&str>) -> Result<String, String> {
    let fmt = fmt.filter(|f| !f.is_empty()).unwrap_or(DEFAULT_DATE);
    let items = StrftimeItems::new(fmt).parse().map_err(|_| format!("Format de date invalide : {fmt}"))?;
    Ok(match date {
        Some(d) => d.format_with_items(items.into_iter()).to_string(),
        None => String::new(),
    })
}

fn pad_num(n: usize, arg: Option<&str>) -> Result<String, String> {
    let width = match arg {
        Some(a) if !a.is_empty() => a.parse::<usize>().map_err(|_| format!("Remplissage invalide : {a}"))?,
        _ => 1,
    };
    Ok(format!("{:0>width$}", n, width = width.min(32)))
}

fn resolve(key: &str, arg: Option<&str>, ctx: &TokenContext, current: &Name) -> Result<String, String> {
    let original = ctx.original.unwrap_or(current);
    let path = ctx.path;
    let meta = ctx.meta;
    let heavy = || match (meta, path) {
        (Some(m), Some(p)) => Some(m.heavy(p)),
        _ => None,
    };
    Ok(match key {
        "name" => match arg {
            Some(a) => {
                let mut parts = a.splitn(2, ':');
                let start: usize = parts.next().unwrap_or("0").trim().parse().map_err(|_| format!("Début invalide : {a}"))?;
                let len: Option<usize> = match parts.next() {
                    Some(l) => Some(l.trim().parse().map_err(|_| format!("Longueur invalide : {a}"))?),
                    None => None,
                };
                let it = original.stem.chars().skip(start);
                match len {
                    Some(l) => it.take(l).collect(),
                    None => it.collect(),
                }
            }
            None => original.stem.clone(),
        },
        "cur" => current.stem.clone(),
        "ext" => original.ext.clone(),
        "full" => original.join(),
        "n" => pad_num(ctx.index + 1, arg)?,
        "fn" => pad_num(ctx.folder_index + 1, arg)?,
        "total" => ctx.total.to_string(),
        "parent" => {
            let level: usize = arg.and_then(|a| a.parse().ok()).unwrap_or(1).max(1);
            let mut p = path.and_then(Path::parent);
            for _ in 1..level {
                p = p.and_then(Path::parent);
            }
            p.and_then(Path::file_name).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        }
        "date" => fmt_date(Some(Local::now()), arg)?,
        "mdate" => fmt_date(meta.and_then(|m| m.modified), arg)?,
        "cdate" => fmt_date(meta.and_then(|m| m.created), arg)?,
        "exif" => {
            let d = match (meta, path) {
                (Some(m), Some(p)) => m.best_date(p),
                _ => None,
            };
            fmt_date(d, arg)?
        }
        "make" => heavy().and_then(|h| h.make.clone()).unwrap_or_default(),
        "model" => heavy().and_then(|h| h.model.clone()).unwrap_or_default(),
        "width" => heavy().and_then(|h| h.width).map(|v| v.to_string()).unwrap_or_default(),
        "height" => heavy().and_then(|h| h.height).map(|v| v.to_string()).unwrap_or_default(),
        "dims" => heavy()
            .and_then(|h| Some(format!("{}x{}", h.width?, h.height?)))
            .unwrap_or_default(),
        "size" => meta.map(|m| human_size(m.size)).unwrap_or_default(),
        "bytes" => meta.map(|m| m.size.to_string()).unwrap_or_default(),
        "crc" => match (meta, path) {
            (Some(m), Some(p)) if !ctx.is_dir => m.crc32(p).map(|c| format!("{c:08x}")).unwrap_or_default(),
            _ => String::new(),
        },
        other => return Err(format!("Balise inconnue : {{{other}}}")),
    })
}

/// Remplace les balises d'un texte.
pub fn expand(template: &str, ctx: &TokenContext, current: &Name) -> Result<String, String> {
    if !template.contains('{') && !template.contains('}') {
        return Ok(template.to_string());
    }
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(pos) = rest.find(['{', '}']) {
        out.push_str(&rest[..pos]);
        let c = rest.as_bytes()[pos];
        let after = &rest[pos + 1..];
        if c == b'}' {
            // « }} » ou « } » isolée : accolade littérale.
            out.push('}');
            rest = after.strip_prefix('}').unwrap_or(after);
            continue;
        }
        if let Some(stripped) = after.strip_prefix('{') {
            out.push('{');
            rest = stripped;
            continue;
        }
        let Some(end) = after.find('}') else {
            return Err("Accolade « { » non fermée".into());
        };
        let inner = &after[..end];
        let (key, arg) = match inner.find(':') {
            Some(i) => (&inner[..i], Some(&inner[i + 1..])),
            None => (inner, None),
        };
        out.push_str(&resolve(key.trim(), arg, ctx, current)?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_basic_tokens() {
        let ctx = TokenContext::test(4);
        let n = Name::split("Holiday.JPG", false);
        assert_eq!(expand("{name}_{n:3}.{ext}", &ctx, &n).unwrap(), "Holiday_005.JPG");
        assert_eq!(expand("{name:0:3}", &ctx, &n).unwrap(), "Hol");
        assert_eq!(expand("{{x}}", &ctx, &n).unwrap(), "{x}");
        assert!(expand("{nope}", &ctx, &n).is_err());
        assert!(expand("{name", &ctx, &n).is_err());
        assert!(expand("{date:%Q}", &ctx, &n).is_err());
        assert_eq!(expand("{total}", &ctx, &n).unwrap(), "10");
    }
}
