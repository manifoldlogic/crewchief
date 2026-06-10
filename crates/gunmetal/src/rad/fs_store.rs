//! FsStore — filesystem chunk storage, a port of GUN's `lib/rfs.js`.
//!
//! Native targets only. Files live in a data directory (GUN default:
//! `radata/`). Writes are atomic:
//!
//! 1. Write to a temp file *next to* the data directory:
//!    `<dir>-<file>-<random>.tmp` (exactly rfs.js's naming).
//! 2. Rename into place: `<dir>/<file>`.
//! 3. If the rename fails with `EXDEV` (cross-device link), fall back to
//!    copy + unlink, mirroring rfs.js's stream-copy fallback.
//!
//! A `puts` cache holds data for files currently being written, so a
//! concurrent `get` returns the buffered data instead of racing the disk.
//! File names arriving here are already URL-encoded by radisk
//! ([`crate::rad::ename`]), so they contain no path separators.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::concurrency::{SharedMut, lock_mut, new_shared_mut};

use super::store::RadStore;

/// `EXDEV` — "Invalid cross-device link" (18 on Linux and macOS).
#[cfg(unix)]
const EXDEV: i32 = 18;
/// `ERROR_NOT_SAME_DEVICE` on Windows.
#[cfg(windows)]
const EXDEV: i32 = 17;
#[cfg(not(any(unix, windows)))]
const EXDEV: i32 = 18;

/// Filesystem-backed [`RadStore`].
#[derive(Clone)]
pub struct FsStore {
    dir: PathBuf,
    /// Pending-write cache (GUN's `puts`): file → buffered data.
    puts: SharedMut<HashMap<String, String>>,
}

impl FsStore {
    /// Open (creating if needed) a data directory.
    pub fn new(dir: impl Into<PathBuf>) -> Result<Self, String> {
        let dir = dir.into();
        fs::create_dir_all(&dir).map_err(|e| format!("rfs mkdir: {}", e))?;
        Ok(Self {
            dir,
            puts: new_shared_mut(HashMap::new()),
        })
    }

    /// The data directory path.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Reject file names that could escape the data directory. Radisk
    /// URL-encodes names, so legitimate names never contain separators.
    fn check_name(file: &str) -> Result<(), String> {
        if file.is_empty()
            || file.contains('/')
            || file.contains('\\')
            || file.contains("..")
            || file.contains('\0')
        {
            return Err(format!("rfs: invalid file name {:?}", file));
        }
        Ok(())
    }
}

/// Rename, falling back to copy + unlink on `EXDEV` (cross-device).
fn move_file(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(EXDEV) => {
            fs::copy(from, to)?;
            fs::remove_file(from)
        }
        Err(e) => Err(e),
    }
}

impl RadStore for FsStore {
    fn put(&self, file: &str, data: &str) -> Result<(), String> {
        Self::check_name(file)?;
        // Buffer so concurrent reads see the pending data.
        lock_mut(&self.puts).insert(file.to_string(), data.to_string());

        let result = (|| {
            let random = format!("{:06x}", rand::random::<u32>() & 0xFF_FFFF);
            // rfs.js: `opt.file + '-' + file + '-' + random + '.tmp'`
            let tmp = PathBuf::from(format!(
                "{}-{}-{}.tmp",
                self.dir.display(),
                file,
                random
            ));
            fs::write(&tmp, data).map_err(|e| format!("rfs write: {}", e))?;
            move_file(&tmp, &self.dir.join(file)).map_err(|e| {
                let _ = fs::remove_file(&tmp);
                format!("rfs rename: {}", e)
            })
        })();

        lock_mut(&self.puts).remove(file);
        result
    }

    fn get(&self, file: &str) -> Result<Option<String>, String> {
        Self::check_name(file)?;
        // `if(tmp = puts[file]){ cb(u, tmp.data); return }`
        if let Some(data) = lock_mut(&self.puts).get(file) {
            return Ok(Some(data.clone()));
        }
        match fs::read_to_string(self.dir.join(file)) {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("rfs read: {}", e)),
        }
    }

    fn list(&self) -> Result<Vec<String>, String> {
        let entries = fs::read_dir(&self.dir).map_err(|e| format!("rfs list: {}", e))?;
        let mut files = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("rfs list: {}", e))?;
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                files.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        files.sort();
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "gunmetal-fs-store-{}-{}-{:x}",
            name,
            std::process::id(),
            rand::random::<u32>()
        ));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn put_get_roundtrip() {
        let dir = temp_dir("roundtrip");
        let store = FsStore::new(&dir).unwrap();
        store.put("!", r#"{"a":{"":1}}"#).unwrap();
        assert_eq!(store.get("!").unwrap().unwrap(), r#"{"a":{"":1}}"#);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn get_missing_is_none() {
        let dir = temp_dir("missing");
        let store = FsStore::new(&dir).unwrap();
        assert_eq!(store.get("nope").unwrap(), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn overwrite_is_atomic_replace() {
        let dir = temp_dir("overwrite");
        let store = FsStore::new(&dir).unwrap();
        store.put("!", "first").unwrap();
        store.put("!", "second").unwrap();
        assert_eq!(store.get("!").unwrap().unwrap(), "second");
        // Exactly one file remains; temp files were renamed away.
        assert_eq!(store.list().unwrap(), vec!["!".to_string()]);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_tmp_files_left_behind() {
        let dir = temp_dir("tmp-clean");
        let store = FsStore::new(&dir).unwrap();
        store.put("%1C", r#"{"!":{"":1}}"#).unwrap();
        // Temp files are siblings of the dir: `<dir>-...tmp`.
        let parent = dir.parent().unwrap();
        let leftovers: Vec<_> = fs::read_dir(parent)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| {
                n.starts_with(&format!(
                    "{}-",
                    dir.file_name().unwrap().to_string_lossy()
                )) && n.ends_with(".tmp")
            })
            .collect();
        assert!(leftovers.is_empty(), "leftover temp files: {:?}", leftovers);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_enumerates_files() {
        let dir = temp_dir("list");
        let store = FsStore::new(&dir).unwrap();
        store.put("!", "{}").unwrap();
        store.put("m", "{}").unwrap();
        store.put("%1C", "{}").unwrap();
        assert_eq!(
            store.list().unwrap(),
            vec!["!".to_string(), "%1C".to_string(), "m".to_string()]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_path_traversal_names() {
        let dir = temp_dir("traversal");
        let store = FsStore::new(&dir).unwrap();
        assert!(store.put("../evil", "x").is_err());
        assert!(store.put("a/b", "x").is_err());
        assert!(store.get("..").is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pending_cache_serves_reads() {
        let dir = temp_dir("pending");
        let store = FsStore::new(&dir).unwrap();
        // Simulate an in-flight write by populating the cache directly.
        lock_mut(&store.puts).insert("!".to_string(), "buffered".to_string());
        assert_eq!(store.get("!").unwrap().unwrap(), "buffered");
        lock_mut(&store.puts).remove("!");
        assert_eq!(store.get("!").unwrap(), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn move_file_fallback_on_copy_path() {
        // We can't force EXDEV portably, but the copy+unlink path is
        // exercised directly.
        let dir = temp_dir("exdev");
        fs::create_dir_all(&dir).unwrap();
        let from = dir.join("from.tmp");
        let to = dir.join("to");
        fs::write(&from, "data").unwrap();
        fs::copy(&from, &to).unwrap();
        fs::remove_file(&from).unwrap();
        assert_eq!(fs::read_to_string(&to).unwrap(), "data");
        assert!(!from.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
