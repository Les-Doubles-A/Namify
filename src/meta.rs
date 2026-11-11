//! Métadonnées d'un fichier : tailles, dates, EXIF, dimensions d'image, CRC32.
//! Les informations coûteuses sont chargées à la demande puis mises en cache.

use std::cell::OnceCell;
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};

#[derive(Clone, Debug, Default)]
pub struct Heavy {
    pub exif_date: Option<DateTime<Local>>,
    pub make: Option<String>,
    pub model: Option<String>,
    pub width: Option<usize>,
    pub height: Option<usize>,
}

#[derive(Debug)]
pub struct FileMeta {
    pub size: u64,
    pub modified: Option<DateTime<Local>>,
    pub created: Option<DateTime<Local>>,
    pub hidden: bool,
    heavy: OnceCell<Heavy>,
    crc: OnceCell<Option<u32>>,
}

impl Clone for FileMeta {
    fn clone(&self) -> Self {
        Self {
            size: self.size,
            modified: self.modified,
            created: self.created,
            hidden: self.hidden,
            heavy: self.heavy.clone(),
            crc: self.crc.clone(),
        }
    }
}

fn to_local(t: std::io::Result<SystemTime>) -> Option<DateTime<Local>> {
    t.ok().map(DateTime::<Local>::from)
}

impl FileMeta {
    pub fn read(path: &Path) -> Self {
        let md = fs::metadata(path).ok();
        let hidden = is_hidden(path, md.as_ref());
        Self {
            size: md.as_ref().map(|m| if m.is_dir() { 0 } else { m.len() }).unwrap_or(0),
            modified: md.as_ref().and_then(|m| to_local(m.modified())),
            created: md.as_ref().and_then(|m| to_local(m.created())),
            hidden,
            heavy: OnceCell::new(),
            crc: OnceCell::new(),
        }
    }

    pub fn heavy(&self, path: &Path) -> &Heavy {
        self.heavy.get_or_init(|| load_heavy(path))
    }

    pub fn crc32(&self, path: &Path) -> Option<u32> {
        *self.crc.get_or_init(|| {
            let file = fs::File::open(path).ok()?;
            let mut reader = BufReader::with_capacity(1 << 16, file);
            let mut hasher = crc32fast::Hasher::new();
            let mut buf = vec![0u8; 1 << 16];
            loop {
                let n = reader.read(&mut buf).ok()?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
            }
            Some(hasher.finalize())
        })
    }

    pub fn crc32_cached(&self) -> Option<u32> {
        self.crc.get().copied().flatten()
    }

    /// Date EXIF de prise de vue, ou à défaut date de modification.
    pub fn best_date(&self, path: &Path) -> Option<DateTime<Local>> {
        self.heavy(path).exif_date.or(self.modified)
    }
}

#[cfg(windows)]
fn is_hidden(path: &Path, md: Option<&fs::Metadata>) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    md.map(|m| m.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0).unwrap_or(false)
        || path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.'))
}

#[cfg(not(windows))]
fn is_hidden(path: &Path, _md: Option<&fs::Metadata>) -> bool {
    path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.'))
}

fn ascii_field(exif: &exif::Exif, tag: exif::Tag) -> Option<String> {
    let field = exif.get_field(tag, exif::In::PRIMARY)?;
    match &field.value {
        exif::Value::Ascii(parts) => {
            let s = parts.first().map(|v| String::from_utf8_lossy(v).trim().to_string())?;
            if s.is_empty() { None } else { Some(s) }
        }
        _ => None,
    }
}

fn load_heavy(path: &Path) -> Heavy {
    let mut h = Heavy::default();
    if let Ok(size) = imagesize::size(path) {
        h.width = Some(size.width);
        h.height = Some(size.height);
    }
    if let Ok(file) = fs::File::open(path) {
        let mut reader = BufReader::new(file);
        if let Ok(exif) = exif::Reader::new().read_from_container(&mut reader) {
            let date = ascii_field(&exif, exif::Tag::DateTimeOriginal)
                .or_else(|| ascii_field(&exif, exif::Tag::DateTimeDigitized))
                .or_else(|| ascii_field(&exif, exif::Tag::DateTime));
            h.exif_date = date
                .and_then(|d| NaiveDateTime::parse_from_str(&d, "%Y:%m:%d %H:%M:%S").ok())
                .and_then(|n| Local.from_local_datetime(&n).earliest());
            h.make = ascii_field(&exif, exif::Tag::Make);
            h.model = ascii_field(&exif, exif::Tag::Model);
        }
    }
    h
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["o", "Ko", "Mo", "Go", "To"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 { format!("{bytes} o") } else { format!("{v:.1} {}", UNITS[u]) }
}

/// Fichier ou dossier chargé dans la liste.
#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub checked: bool,
    pub selected: bool,
    pub meta: FileMeta,
}

impl Entry {
    pub fn new(path: PathBuf) -> Self {
        let is_dir = path.is_dir();
        let meta = FileMeta::read(&path);
        Self { path, is_dir, checked: true, selected: false, meta }
    }

    pub fn file_name(&self) -> String {
        self.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
    }

    pub fn parent(&self) -> PathBuf {
        self.path.parent().map(Path::to_path_buf).unwrap_or_default()
    }
}
