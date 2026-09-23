//! One job: showing a person the directories they can choose a project from.
//!
//! Spec §1: choosing or creating a project directory is a daemon route, because
//! only the daemon can see the machine's disk. This is the disk side of those
//! routes — listing one directory's subdirectories, the roots to start from,
//! and creating one new directory to choose. It blocks on the filesystem, so
//! callers on an async runtime run it on a blocking thread.
//!
//! Paths cross to the client as UTF-8 strings. A directory whose name is not
//! valid UTF-8 cannot be shown or sent back faithfully, so a listing leaves it
//! out, like any entry it cannot read.

use std::path::{Path, PathBuf};

use super::directory::{DirectoryError, canonical_dir, utf8};

/// One directory a person could open or choose.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct DirectoryEntry {
    pub name: String,
    /// Absolute and canonical; sending it back names exactly this directory.
    pub path: String,
    /// Hidden or system on Windows, or named with a leading dot. Flagged rather
    /// than filtered, so the client decides what to show and nothing is
    /// withheld without anyone saying so.
    pub hidden: bool,
}

/// A directory's immediate subdirectories, or the roots when `path` is `None`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, utoipa::ToSchema)]
pub struct DirectoryListing {
    /// The directory listed, canonical. `None` for the roots.
    pub path: Option<String>,
    /// Where "up" leads. `None` at a root, and for the roots themselves.
    pub parent: Option<String>,
    /// Sorted case-insensitively by name, which is how a person scans a list
    /// of folders; ties are broken by the exact name so the order is total.
    pub entries: Vec<DirectoryEntry>,
}

/// `path`'s subdirectories, or the roots when it is `None`. `path` must be an
/// absolute path to a readable directory; entries inside it that cannot be
/// read are skipped rather than failing the whole listing.
pub fn list(path: Option<&Path>) -> Result<DirectoryListing, DirectoryError> {
    let Some(path) = path else {
        return Ok(DirectoryListing {
            path: None,
            parent: None,
            entries: roots(),
        });
    };
    let dir = canonical_dir(path)?;
    let reader = std::fs::read_dir(&dir).map_err(|e| DirectoryError::from_io(&dir, e))?;
    let mut entries: Vec<DirectoryEntry> = reader
        .filter_map(Result::ok)
        .filter_map(|entry| subdirectory(&entry))
        .collect();
    entries.sort_by(|a, b| (a.name.to_lowercase(), &a.name).cmp(&(b.name.to_lowercase(), &b.name)));
    Ok(DirectoryListing {
        parent: dir.parent().and_then(|p| utf8(p.to_path_buf()).ok()),
        path: Some(utf8(dir)?),
        entries,
    })
}

/// Creates `name` inside `parent` and returns it. `name` must be one path
/// component that Windows would also accept, so a project folder made here can
/// be opened on any machine the project reaches. An existing directory is
/// `AlreadyExists`, never silently reused: the person asked for a new one.
pub fn create_subdirectory(parent: &Path, name: &str) -> Result<DirectoryEntry, DirectoryError> {
    check_name(name)?;
    let parent = canonical_dir(parent)?;
    let path = parent.join(name);
    std::fs::create_dir(&path).map_err(|e| DirectoryError::from_io(&path, e))?;
    Ok(DirectoryEntry {
        name: name.to_string(),
        path: utf8(path)?,
        hidden: name.starts_with('.'),
    })
}

/// A listing entry for `entry` if it is a directory — or a link to one — that
/// can be read and named in UTF-8. Files are never listed.
fn subdirectory(entry: &std::fs::DirEntry) -> Option<DirectoryEntry> {
    let path = entry.path();
    // Follows a symlink or junction, so a link to a directory is listed and a
    // broken one is skipped.
    let metadata = std::fs::metadata(&path).ok()?;
    if !metadata.is_dir() {
        return None;
    }
    let name = entry.file_name().into_string().ok()?;
    Some(DirectoryEntry {
        hidden: name.starts_with('.') || hidden_by_attribute(entry),
        path: utf8(path).ok()?,
        name,
    })
}

#[cfg(windows)]
fn hidden_by_attribute(entry: &std::fs::DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;
    const HIDDEN: u32 = 0x2;
    const SYSTEM: u32 = 0x4;
    entry
        .metadata()
        .is_ok_and(|m| m.file_attributes() & (HIDDEN | SYSTEM) != 0)
}

#[cfg(not(windows))]
fn hidden_by_attribute(_: &std::fs::DirEntry) -> bool {
    false
}

/// Where browsing starts: every drive that answers on Windows; `/` and the
/// user's home elsewhere.
#[cfg(windows)]
fn roots() -> Vec<DirectoryEntry> {
    (b'A'..=b'Z')
        .map(|letter| PathBuf::from(format!("{}:\\", letter as char)))
        .filter(|root| std::fs::metadata(root).is_ok_and(|m| m.is_dir()))
        .filter_map(root_entry)
        .collect()
}

#[cfg(not(windows))]
fn roots() -> Vec<DirectoryEntry> {
    let mut roots = vec![PathBuf::from("/")];
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from)
        && home.is_absolute()
        && home.is_dir()
        && home != Path::new("/")
    {
        roots.push(home);
    }
    roots.into_iter().filter_map(root_entry).collect()
}

fn root_entry(root: PathBuf) -> Option<DirectoryEntry> {
    let path = utf8(root).ok()?;
    Some(DirectoryEntry {
        name: path.clone(),
        path,
        hidden: false,
    })
}

/// Refuses anything that is not exactly one new name: empty, `.`, `..`, a
/// separator, or what Windows forbids in a file name — its reserved
/// characters, control characters, a trailing dot or space (which Windows
/// silently strips, creating a different name), and its device names.
fn check_name(name: &str) -> Result<(), DirectoryError> {
    let refuse = |why| {
        Err(DirectoryError::InvalidName {
            name: name.to_string(),
            why,
        })
    };
    if name.is_empty() {
        return refuse("it is empty");
    }
    if name == "." || name == ".." {
        return refuse("it names an existing directory, not a new one");
    }
    if name.contains(['/', '\\']) {
        return refuse("it contains a path separator");
    }
    if name.contains(['<', '>', ':', '"', '|', '?', '*']) || name.chars().any(char::is_control) {
        return refuse("it contains a character Windows forbids in a name");
    }
    if name.ends_with(['.', ' ']) {
        return refuse("it ends with a dot or a space");
    }
    let stem = name.split('.').next().unwrap_or(name).to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit()
            && stem.as_bytes()[3] != b'0');
    if device {
        return refuse("it is a name Windows reserves for a device");
    }
    Ok(())
}
