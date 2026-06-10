//! Radisk — the core RAD engine, a port of GUN's `lib/radisk.js`.
//!
//! Responsibilities (see `sources/docs/rad.md`):
//! - **Batching** — writes are buffered per chunk file and flushed after
//!   `until_ms` (default 250 ms) or when `batch` pending writes accumulate.
//!   All callbacks queued during a batch fire after the single flush.
//! - **Chunking** — when a chunk's serialized JSON exceeds `chunk` bytes,
//!   the lexicographically-last half is split into a new file (`f.split`).
//! - **Directory** — a radix of file names persisted under `\x1C`
//!   (`String.fromCharCode(28)`); reads walk it in reverse to find the
//!   greatest file name <= the key (`r.find`). The first chunk is `!`.
//! - **Memory** — chunks are evicted after flush when no writes are
//!   pending (`if(!rad.Q){ delete r.disk[file] }`); values larger than
//!   `max` bytes are rejected with `"Data too big!"`; oversized chunk
//!   reads abort with `"Chunk too big!"`.
//! - **Self-healing** — chunks that are missing or fail to parse are
//!   dropped from the directory and the operation retries (`r.find.bad`).
//! - **Serialization** — JSON of the raw radix tree, exactly GUN's
//!   `JSON.stringify(rad.$)`. The legacy `\x1F`-delimited RAD binary
//!   format is *not* supported for reading; such chunks (data not
//!   starting with `{`) produce an error rather than being healed away.
//!
//! Differences from the JS source: synchronous `Result`-returning store
//! backends instead of callback adapters, and a deterministic [`Radisk::flush`]
//! for tests / shutdown. The 250 ms timer flush still runs in the
//! background (a thread on native, a `setTimeout`-backed task on WASM).

use std::collections::HashMap;
use std::time::Duration;

use serde_json::Value;

use crate::concurrency::{SharedMut, lock_mut, new_shared_mut};

use super::radix::{MapOpt, Radix, RadixGet, map_tree};
use super::store::RadStore;

// ── Constants ───────────────────────────────────────────────────────

/// File key of the directory radix — `String.fromCharCode(28)`.
pub const DIR_FILE: &str = "\u{1C}";

/// Name of the initial/root chunk file — `opt.code.from`.
pub const FROM: &str = "!";

// ── File-name encoding ──────────────────────────────────────────────

/// URL-encode a file name as GUN does:
/// `encodeURIComponent(t).replace(/\*/g, '%2A')`.
pub fn ename(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    for b in t.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~'
            | b'\'' | b'(' | b')' => out.push(b as char),
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

/// Decode a file name produced by [`ename`] (decodeURIComponent).
pub fn dename(t: &str) -> String {
    let bytes = t.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h = (bytes[i + 1] as char).to_digit(16);
            let l = (bytes[i + 2] as char).to_digit(16);
            if let (Some(h), Some(l)) = (h, l) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ── Options ─────────────────────────────────────────────────────────

/// Configuration, mirroring radisk.js defaults.
#[derive(Debug, Clone)]
pub struct RadiskOptions {
    /// Max bytes per chunk file before splitting (`opt.chunk`).
    pub chunk: usize,
    /// Milliseconds to coalesce writes before flushing (`opt.until`).
    pub until_ms: u64,
    /// Max pending write count before a forced early flush (`opt.batch`).
    pub batch: usize,
    /// Max byte size of a single value / chunk (`opt.max`).
    pub max: usize,
}

impl Default for RadiskOptions {
    fn default() -> Self {
        Self {
            chunk: 1024 * 1024,           // 1 MB
            until_ms: 250,                // 250 ms
            batch: 10_000,                // 10,000 writes
            max: 90_000_000,              // ~90 MB (300000000 * 0.3)
        }
    }
}

/// Read options for range queries (subset of GUN's lex `o`).
#[derive(Debug, Default, Clone)]
pub struct ReadOpt {
    /// Inclusive lower bound.
    pub start: Option<String>,
    /// Inclusive upper bound.
    pub end: Option<String>,
    /// Iterate in reverse lexicographic order.
    pub reverse: bool,
}

// ── Callback type (platform-gated Send) ─────────────────────────────

/// A write acknowledgment callback, fired once after the batch flushes.
#[cfg(not(target_arch = "wasm32"))]
pub type RadAck = Box<dyn FnOnce(Result<(), String>) + Send>;
#[cfg(target_arch = "wasm32")]
pub type RadAck = Box<dyn FnOnce(Result<(), String>)>;

// ── Internal state ──────────────────────────────────────────────────

struct Inner {
    opt: RadiskOptions,
    store: Box<dyn RadStore>,
    /// Directory radix: file name → 1 (present) / 0 (deleted). GUN's `dir`.
    dir: Radix,
    dir_loaded: bool,
    /// In-memory chunk cache, keyed by (decoded) file name. GUN's `r.disk`.
    disk: HashMap<String, Radix>,
    /// Pending flush callbacks per dirty file. Presence marks dirty. `disk.Q`.
    queue: HashMap<String, Vec<RadAck>>,
    /// Total pending writes since last flush.
    pending: usize,
    /// Bumped on every flush; invalidates scheduled timers.
    flush_gen: u64,
    timer_scheduled: bool,
}

type AckList = Vec<(RadAck, Result<(), String>)>;

/// The RAD storage engine. Cheap to clone (shared internal state).
pub struct Radisk {
    inner: SharedMut<Inner>,
}

impl Clone for Radisk {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl Radisk {
    /// Create a radisk over a store backend.
    pub fn new(store: Box<dyn RadStore>, opt: RadiskOptions) -> Self {
        Self {
            inner: new_shared_mut(Inner {
                opt,
                store,
                dir: Radix::new(),
                dir_loaded: false,
                disk: HashMap::new(),
                queue: HashMap::new(),
                pending: 0,
                flush_gen: 0,
                timer_scheduled: false,
            }),
        }
    }

    /// Convenience constructor with default options.
    pub fn with_store(store: Box<dyn RadStore>) -> Self {
        Self::new(store, RadiskOptions::default())
    }

    // ── Public API ──────────────────────────────────────────────────

    /// Buffer a write of `value` at `key`. Returns `Err("Data too big!")`
    /// for oversized values. `cb` (if any) fires after the batch flushes,
    /// with the flush result. Port of `r.save`.
    pub fn put(&self, key: &str, value: Value, cb: Option<RadAck>) -> Result<(), String> {
        let mut acks: AckList = Vec::new();
        let result = {
            let mut g = lock_mut(&self.inner);
            Self::put_inner(&mut g, key, value, cb, &mut acks).map(|need_timer| {
                if need_timer {
                    let generation = g.flush_gen;
                    let ms = g.opt.until_ms;
                    drop(g);
                    schedule_timer(&self.inner, generation, ms);
                }
            })
        };
        fire(acks);
        result
    }

    /// Read the exact value at `key`, loading (and caching) the owning
    /// chunk on demand. Buffered (unflushed) writes are visible.
    /// Port of `r.read` for an exact key.
    pub fn get(&self, key: &str) -> Result<Option<Value>, String> {
        let mut g = lock_mut(&self.inner);
        let file = Self::locate(&mut g, key)?;
        match Self::chunk_get(&mut g, &file, key)? {
            Some(RadixGet::Leaf(v)) => Ok(Some(v)),
            _ => Ok(None),
        }
    }

    /// Range iteration across chunk files. Calls `cb(value, key)` for every
    /// leaf within `[start, end]` (both bounds inclusive), in lexicographic
    /// order (reversed if `opt.reverse`). Return `Some(r)` from the callback
    /// to stop early. Port of the cross-chunk `r.read` loop.
    pub fn each<R, F>(&self, opt: &ReadOpt, cb: &mut F) -> Result<Option<R>, String>
    where
        F: FnMut(&Value, &str) -> Option<R>,
    {
        let mut g = lock_mut(&self.inner);
        Self::ensure_dir(&mut g)?;

        // Candidate chunk files: every dir entry whose covered key range
        // [file, next_file) intersects [start, end].
        let mut files = Self::dir_files(&g);
        if !files.contains(&FROM.to_string()) {
            files.insert(0, FROM.to_string());
        }

        let mut candidates: Vec<String> = Vec::new();
        for i in 0..files.len() {
            let lo = &files[i];
            let hi = files.get(i + 1);
            if let Some(end) = &opt.end {
                // The root chunk also holds keys below `!`, so it is
                // never excluded by the upper bound.
                if lo.as_str() > end.as_str() && lo != FROM {
                    continue; // whole chunk after range
                }
            }
            if let (Some(start), Some(hi)) = (&opt.start, hi)
                && hi.as_str() <= start.as_str()
            {
                continue; // whole chunk before range
            }
            candidates.push(lo.clone());
        }
        if opt.reverse {
            candidates.reverse();
        }

        let map_opt = MapOpt {
            reverse: opt.reverse,
            start: opt.start.clone(),
            end: opt.end.clone(),
        };

        for file in candidates {
            let rad = match Self::load_chunk(&mut g, &file) {
                Ok(Some(rad)) => rad,
                Ok(None) => continue,
                Err(e) => return Err(e),
            };
            if let Some(r) = map_tree(rad.tree(), &map_opt, "", cb) {
                return Ok(Some(r));
            }
        }
        Ok(None)
    }

    /// Flush all pending writes now. Deterministic alternative to the
    /// 250 ms timer — tests and shutdown paths call this directly.
    pub fn flush(&self) -> Result<(), String> {
        let (result, acks) = {
            let mut g = lock_mut(&self.inner);
            let mut acks: AckList = Vec::new();
            let result = Self::flush_inner(&mut g, &mut acks);
            (result, acks)
        };
        fire(acks);
        result
    }

    /// Number of buffered (unflushed) writes.
    pub fn pending(&self) -> usize {
        lock_mut(&self.inner).pending
    }

    /// Number of chunks currently held in memory (eviction tests).
    pub fn cached_chunks(&self) -> usize {
        lock_mut(&self.inner).disk.len()
    }

    // ── Write path (r.save / r.write) ───────────────────────────────

    /// Returns Ok(true) when a flush timer should be scheduled.
    fn put_inner(
        g: &mut Inner,
        key: &str,
        value: Value,
        cb: Option<RadAck>,
        acks: &mut AckList,
    ) -> Result<bool, String> {
        // `opt.max` guard — reject oversized values up front.
        let size = serde_json::to_string(&value)
            .map_err(|e| format!("Cannot radisk! {}", e))?
            .len();
        if size >= g.opt.max {
            return Err("Data too big!".to_string());
        }

        // Locate the owning chunk (with corrupt-file self-healing).
        // `locate` caches every non-root chunk it visits, so afterwards
        // only the root `!` chunk may still need loading.
        let file = Self::locate(g, key)?;
        if !g.disk.contains_key(&file) {
            let rad = match Self::parse_chunk(g, &file) {
                Ok(Some(rad)) => rad,
                Ok(None) => Radix::new(),
                Err(ChunkError::Corrupt) => {
                    if file != FROM {
                        Self::find_bad(g, &file)?;
                    }
                    Radix::new()
                }
                Err(ChunkError::Fatal(e)) => return Err(e),
            };
            g.disk.insert(file.clone(), rad);
        }
        let rad = g.disk.get_mut(&file).expect("chunk cached above");
        rad.insert(key, value);

        let q = g.queue.entry(file).or_default();
        if let Some(cb) = cb {
            q.push(cb);
        }
        g.pending += 1;

        // Forced early flush once `batch` writes accumulate.
        if g.pending >= g.opt.batch {
            let _ = Self::flush_inner(g, acks);
            return Ok(false);
        }
        // Otherwise make sure a timer flush is scheduled.
        if g.timer_scheduled {
            return Ok(false);
        }
        g.timer_scheduled = true;
        Ok(true)
    }

    /// Flush every dirty chunk; queued callbacks are *collected* into
    /// `acks` (fired by the caller after the lock is released).
    fn flush_inner(g: &mut Inner, acks: &mut AckList) -> Result<(), String> {
        g.flush_gen += 1;
        g.timer_scheduled = false;
        g.pending = 0;

        let mut files: Vec<String> = g.queue.keys().cloned().collect();
        files.sort();

        let mut first_err: Option<String> = None;
        for file in files {
            let cbs = g.queue.remove(&file).unwrap_or_default();
            let rad = match g.disk.remove(&file) {
                Some(rad) => rad,
                None => {
                    // Shouldn't happen; report to callbacks.
                    let err = format!("No radix for chunk {:?}!", file);
                    for cb in cbs {
                        acks.push((cb, Err(err.clone())));
                    }
                    first_err.get_or_insert(err);
                    continue;
                }
            };
            let result = Self::write_chunk(g, &file, rad, false);
            if let Err(e) = &result {
                first_err.get_or_insert(e.clone());
            }
            // VERY IMPORTANT! Clean up memory (eviction happens because we
            // `remove`d the chunk above and only re-insert on later access).
            for cb in cbs {
                acks.push((cb, result.clone()));
            }
        }
        match first_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Serialize and persist one chunk, splitting it if oversized.
    /// Port of `r.write` + `r.write.jsonify` + `f.split`.
    fn write_chunk(g: &mut Inner, file: &str, rad: Radix, force: bool) -> Result<(), String> {
        let raw = rad.to_json()?;

        if !force && g.opt.chunk < raw.len() {
            let count = rad.count();
            // Single-entry chunks are never split.
            if count > 1 {
                return Self::split_chunk(g, file, rad, count);
            }
        }

        Self::find_add(g, file)?;
        g.store.put(&ename(file), &raw)
    }

    /// Move the lexicographically-last half of `rad` to a new chunk file
    /// (named after its smallest key), rewrite the first half in place.
    fn split_chunk(g: &mut Inner, file: &str, rad: Radix, count: usize) -> Result<(), String> {
        let limit = count.div_ceil(2); // Math.ceil(f.count / 2)

        // IMPORTANT: walk in reverse so the last half of the data is moved
        // to the new file before being dropped from the current file.
        let mut sub = Radix::new();
        let mut end_key = String::new();
        let mut moved = 0usize;
        let _ = rad.map::<(), _>(
            &MapOpt {
                reverse: true,
                ..Default::default()
            },
            &mut |v, k| {
                sub.insert(k, v.clone());
                end_key = k.to_string();
                moved += 1;
                if limit <= moved { Some(()) } else { None }
            },
        );

        // Write the new (last-half) chunk first.
        Self::write_chunk(g, &end_key, sub, false)?;

        // Then rewrite the remaining first half under the original name.
        let mut hub = Radix::new();
        let _ = rad.map::<(), _>(&MapOpt::default(), &mut |v, k| {
            if k >= end_key.as_str() {
                return Some(());
            }
            hub.insert(k, v.clone());
            None
        });
        Self::write_chunk(g, file, hub, false)
    }

    // ── Directory (r.find) ──────────────────────────────────────────

    /// Lazily load the directory chunk and import `store.list()`.
    fn ensure_dir(g: &mut Inner) -> Result<(), String> {
        if g.dir_loaded {
            return Ok(());
        }
        g.dir_loaded = true;

        if let Some(raw) = g.store.get(&ename(DIR_FILE))? {
            match Radix::from_json(&raw) {
                Ok(dir) => g.dir = dir,
                Err(_) => g.dir = Radix::new(), // corrupt dir: rebuild from list()
            }
        }

        // Startup import: add any stored file the directory doesn't know.
        let mut changed = false;
        let dir_name = ename(DIR_FILE);
        for encoded in g.store.list()? {
            if encoded == dir_name || encoded.ends_with(".tmp") {
                continue;
            }
            let file = dename(&encoded);
            if !Self::dir_has(&g.dir, &file) {
                g.dir.insert(&file, Value::from(1));
                changed = true;
            }
        }
        if changed {
            Self::write_dir(g)?;
        }
        Ok(())
    }

    fn dir_has(dir: &Radix, file: &str) -> bool {
        matches!(dir.get(file), Some(RadixGet::Leaf(v)) if truthy(&v))
    }

    /// All present chunk files, sorted ascending.
    fn dir_files(g: &Inner) -> Vec<String> {
        let mut files = Vec::new();
        g.dir.each(|v, k| {
            if truthy(v) {
                files.push(k.to_string());
            }
        });
        files
    }

    /// Greatest file name <= `key`, defaulting to `!`. Port of `r.find`.
    fn find(g: &mut Inner, key: &str) -> Result<String, String> {
        Self::ensure_dir(g)?;
        let hit = g.dir.map(
            &MapOpt {
                reverse: true,
                end: Some(key.to_string()),
                ..Default::default()
            },
            &mut |v, k| {
                if truthy(v) {
                    Some(k.to_string())
                } else {
                    None
                }
            },
        );
        Ok(hit.unwrap_or_else(|| FROM.to_string()))
    }

    /// Add a file to the directory (persisting it) if missing.
    /// Port of `r.find.add` — the directory itself is written with
    /// `force` so it never splits.
    fn find_add(g: &mut Inner, file: &str) -> Result<(), String> {
        if file == DIR_FILE {
            return Ok(());
        }
        Self::ensure_dir(g)?;
        if Self::dir_has(&g.dir, file) {
            return Ok(());
        }
        g.dir.insert(file, Value::from(1));
        Self::write_dir(g)
    }

    /// Mark a file deleted in the directory and drop its cache entry.
    /// Port of `r.find.bad` (corrupt-file self-healing).
    fn find_bad(g: &mut Inner, file: &str) -> Result<(), String> {
        g.dir.insert(file, Value::from(0));
        g.disk.remove(file);
        g.queue.remove(file);
        Self::write_dir(g)
    }

    fn write_dir(g: &mut Inner) -> Result<(), String> {
        let raw = g.dir.to_json()?;
        g.store.put(&ename(DIR_FILE), &raw)
    }

    // ── Chunk loading (r.parse) with self-healing ───────────────────

    /// Find the chunk file owning `key`, healing corrupt/missing files by
    /// dropping them from the directory and retrying (`r.find.bad`).
    fn locate(g: &mut Inner, key: &str) -> Result<String, String> {
        // Each retry removes one file from the directory, so this
        // terminates; the guard is belt-and-braces.
        for _ in 0..10_000 {
            let file = Self::find(g, key)?;
            if file == FROM || g.disk.contains_key(&file) {
                return Ok(file);
            }
            match Self::parse_chunk(g, &file) {
                Ok(Some(rad)) => {
                    g.disk.insert(file.clone(), rad);
                    return Ok(file);
                }
                // Listed in the directory but missing or corrupt → heal.
                Ok(None) => Self::find_bad(g, &file)?,
                Err(ChunkError::Corrupt) => Self::find_bad(g, &file)?,
                Err(ChunkError::Fatal(e)) => return Err(e),
            }
        }
        Err("RAD directory healing did not converge!".to_string())
    }

    /// Load (cache-first) a chunk; corrupt files are healed to `None`.
    fn load_chunk<'a>(g: &'a mut Inner, file: &str) -> Result<Option<&'a Radix>, String> {
        if !g.disk.contains_key(file) {
            match Self::parse_chunk(g, file) {
                Ok(Some(rad)) => {
                    g.disk.insert(file.to_string(), rad);
                }
                Ok(None) => {
                    if file != FROM {
                        Self::find_bad(g, file)?;
                    }
                    return Ok(None);
                }
                Err(ChunkError::Corrupt) => {
                    Self::find_bad(g, file)?;
                    return Ok(None);
                }
                Err(ChunkError::Fatal(e)) => return Err(e),
            }
        }
        Ok(g.disk.get(file))
    }

    /// Read + lookup in one step (used by `get`).
    fn chunk_get(g: &mut Inner, file: &str, key: &str) -> Result<Option<RadixGet>, String> {
        match Self::load_chunk(g, file)? {
            Some(rad) => Ok(rad.get(key)),
            None => Ok(None),
        }
    }

    /// Fetch + parse raw chunk data from the store. Port of `r.parse`.
    fn parse_chunk(g: &mut Inner, file: &str) -> Result<Option<Radix>, ChunkError> {
        let raw = g
            .store
            .get(&ename(file))
            .map_err(ChunkError::Fatal)?;
        let Some(data) = raw else {
            return Ok(None);
        };
        if g.opt.max <= data.len() {
            return Err(ChunkError::Fatal("Chunk too big!".to_string()));
        }
        if !data.starts_with('{') {
            // Legacy \x1F-delimited RAD binary format: reading it is not
            // supported (documented limitation). Don't heal it away —
            // surface an error so the data is preserved on disk.
            return Err(ChunkError::Fatal(format!(
                "File {:?} uses the legacy RAD binary format, which gunmetal does not read.",
                file
            )));
        }
        match Radix::from_json(&data) {
            Ok(rad) => Ok(Some(rad)),
            Err(_) => Err(ChunkError::Corrupt),
        }
    }
}

enum ChunkError {
    /// Unparseable chunk — eligible for self-healing.
    Corrupt,
    /// Store failure or policy error — propagate to the caller.
    Fatal(String),
}

impl Drop for Radisk {
    fn drop(&mut self) {
        // Best-effort durability: flush pending writes when the last
        // handle is dropped (timer tasks only hold weak references).
        let last = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::sync::Arc::strong_count(&self.inner) == 1
            }
            #[cfg(target_arch = "wasm32")]
            {
                std::rc::Rc::strong_count(&self.inner) == 1
            }
        };
        if last {
            let _ = self.flush();
        }
    }
}

// ── Timer flush ─────────────────────────────────────────────────────

fn fire(acks: AckList) {
    for (cb, result) in acks {
        cb(result);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn schedule_timer(inner: &SharedMut<Inner>, generation: u64, ms: u64) {
    let weak = std::sync::Arc::downgrade(inner);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(ms));
        if let Some(inner) = weak.upgrade() {
            let mut acks: AckList = Vec::new();
            {
                let mut g = lock_mut(&inner);
                if g.timer_scheduled && g.flush_gen == generation {
                    let _ = Radisk::flush_inner(&mut g, &mut acks);
                }
            }
            fire(acks);
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn schedule_timer(inner: &SharedMut<Inner>, generation: u64, ms: u64) {
    let weak = std::rc::Rc::downgrade(inner);
    crate::runtime::spawn_async(async move {
        crate::runtime::sleep_async(Duration::from_millis(ms)).await;
        if let Some(inner) = weak.upgrade() {
            let mut acks: AckList = Vec::new();
            {
                let mut g = lock_mut(&inner);
                if g.timer_scheduled && g.flush_gen == generation {
                    let _ = Radisk::flush_inner(&mut g, &mut acks);
                }
            }
            fire(acks);
        }
    });
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concurrency::{lock_mut as lk, new_shared_mut as shared};
    use crate::rad::store::MemoryRadStore;
    use serde_json::json;

    fn radisk(store: &MemoryRadStore, opt: RadiskOptions) -> Radisk {
        Radisk::new(Box::new(store.clone()), opt)
    }

    fn tiny_opts() -> RadiskOptions {
        RadiskOptions {
            chunk: 100,
            until_ms: 10_000, // effectively "no timer" — tests flush manually
            ..Default::default()
        }
    }

    fn collect_range(rad: &Radisk, opt: &ReadOpt) -> Vec<String> {
        let mut keys = Vec::new();
        rad.each::<(), _>(opt, &mut |_, k| {
            keys.push(k.to_string());
            None
        })
        .unwrap();
        keys
    }

    // ── ename / dename ──────────────────────────────────────────────

    #[test]
    fn ename_matches_encode_uri_component() {
        assert_eq!(ename("!"), "!");
        assert_eq!(ename("users/alice"), "users%2Falice");
        assert_eq!(ename("a*b"), "a%2Ab"); // '*' also escaped
        assert_eq!(ename("\u{1C}"), "%1C");
        assert_eq!(ename("soul\u{1B}key"), "soul%1Bkey");
        assert_eq!(ename("héllo"), "h%C3%A9llo");
    }

    #[test]
    fn dename_roundtrip() {
        for s in ["!", "users/alice", "a*b", "\u{1C}", "soul\u{1B}key", "héllo"] {
            assert_eq!(dename(&ename(s)), s);
        }
    }

    // ── Basic put/get + buffering ───────────────────────────────────

    #[test]
    fn put_get_before_flush_sees_buffered_data() {
        let store = MemoryRadStore::new();
        let rad = radisk(&store, tiny_opts());
        rad.put("alex", json!(27), None).unwrap();
        // Nothing on "disk" yet…
        assert_eq!(store.put_count(), 0);
        // …but reads see the buffered write.
        assert_eq!(rad.get("alex").unwrap(), Some(json!(27)));
    }

    #[test]
    fn flush_persists_and_reads_back() {
        let store = MemoryRadStore::new();
        {
            let rad = radisk(&store, RadiskOptions::default());
            rad.put("alex", json!(27), None).unwrap();
            rad.put("alexandria", json!("library"), None).unwrap();
            rad.put("andrew", json!(true), None).unwrap();
            rad.flush().unwrap();
        }
        // Root chunk written exactly as GUN would.
        assert_eq!(
            store.raw("!").unwrap(),
            r#"{"a":{"lex":{"":27,"andria":{"":"library"}},"ndrew":{"":true}}}"#
        );
        // Re-open and read back.
        let rad2 = radisk(&store, RadiskOptions::default());
        assert_eq!(rad2.get("alex").unwrap(), Some(json!(27)));
        assert_eq!(rad2.get("alexandria").unwrap(), Some(json!("library")));
        assert_eq!(rad2.get("andrew").unwrap(), Some(json!(true)));
        assert_eq!(rad2.get("missing").unwrap(), None);
    }

    #[test]
    fn directory_file_written_under_x1c() {
        let store = MemoryRadStore::new();
        let rad = radisk(&store, RadiskOptions::default());
        rad.put("a", json!(1), None).unwrap();
        rad.flush().unwrap();
        assert_eq!(store.raw("%1C").unwrap(), r#"{"!":{"":1}}"#);
    }

    // ── Batching / callbacks ────────────────────────────────────────

    #[test]
    fn all_callbacks_fire_after_single_flush() {
        let store = MemoryRadStore::new();
        // Default (1 MB) chunk so everything stays in one file.
        let rad = radisk(
            &store,
            RadiskOptions {
                until_ms: 10_000,
                ..Default::default()
            },
        );
        let fired = shared(0usize);

        for i in 0..50 {
            let fired = fired.clone();
            rad.put(
                &format!("key{:02}", i),
                json!(i),
                Some(Box::new(move |r| {
                    assert!(r.is_ok());
                    *lk(&fired) += 1;
                })),
            )
            .unwrap();
        }
        assert_eq!(*lk(&fired), 0, "no callbacks before flush");
        assert_eq!(store.put_count(), 0, "no I/O before flush");

        rad.flush().unwrap();
        assert_eq!(*lk(&fired), 50, "every queued callback fires once");
        // One chunk write + one directory write — not 50.
        assert_eq!(store.put_count(), 2);
    }

    #[test]
    fn batch_count_triggers_early_flush() {
        let store = MemoryRadStore::new();
        let rad = radisk(
            &store,
            RadiskOptions {
                batch: 5,
                until_ms: 10_000,
                ..Default::default()
            },
        );
        let fired = shared(0usize);
        for i in 0..5 {
            let fired = fired.clone();
            rad.put(
                &format!("k{}", i),
                json!(i),
                Some(Box::new(move |_| *lk(&fired) += 1)),
            )
            .unwrap();
        }
        // 5th write hit the batch limit → flushed without flush()/timer.
        assert_eq!(*lk(&fired), 5);
        assert!(store.put_count() >= 1);
        assert_eq!(rad.pending(), 0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn timer_flushes_after_interval() {
        let store = MemoryRadStore::new();
        let rad = radisk(
            &store,
            RadiskOptions {
                until_ms: 30,
                ..Default::default()
            },
        );
        let fired = shared(false);
        let f2 = fired.clone();
        rad.put("alex", json!(27), Some(Box::new(move |_| *lk(&f2) = true)))
            .unwrap();
        assert_eq!(store.put_count(), 0);

        // Wait for the 30ms timer (generously).
        let mut waited = 0;
        while !*lk(&fired) && waited < 2_000 {
            std::thread::sleep(Duration::from_millis(10));
            waited += 10;
        }
        assert!(*lk(&fired), "timer flush fired the callback");
        assert_eq!(store.raw("!").unwrap(), r#"{"alex":{"":27}}"#);
    }

    // ── Chunk splitting ─────────────────────────────────────────────

    #[test]
    fn oversized_chunk_splits_into_files() {
        let store = MemoryRadStore::new();
        let rad = radisk(&store, tiny_opts()); // 100-byte chunks
        for i in 0..20 {
            rad.put(&format!("key{:02}", i), json!(i), None).unwrap();
        }
        rad.flush().unwrap();

        let files = store.files();
        let chunks: Vec<&String> = files.iter().filter(|f| *f != "%1C").collect();
        assert!(
            chunks.len() > 1,
            "expected a split, got files {:?}",
            files
        );
        // First chunk is always "!".
        assert!(files.contains(&"!".to_string()));
        // Every chunk respects the size limit (single-entry chunks excepted).
        for c in &chunks {
            let raw = store.raw(c).unwrap();
            let parsed = Radix::from_json(&raw).unwrap();
            assert!(
                raw.len() <= 100 || parsed.count() == 1,
                "chunk {:?} too big: {} bytes",
                c,
                raw.len()
            );
        }

        // Reads find every key across chunks.
        let rad2 = radisk(&store, tiny_opts());
        for i in 0..20 {
            assert_eq!(
                rad2.get(&format!("key{:02}", i)).unwrap(),
                Some(json!(i)),
                "key{:02}",
                i
            );
        }
    }

    #[test]
    fn single_entry_chunk_never_splits() {
        let store = MemoryRadStore::new();
        let rad = radisk(&store, tiny_opts());
        let big: String = "x".repeat(500); // bigger than chunk size
        rad.put("big", json!(big), None).unwrap();
        rad.flush().unwrap();
        let chunks: Vec<String> = store.files().into_iter().filter(|f| f != "%1C").collect();
        assert_eq!(chunks, vec!["!".to_string()]);
    }

    #[test]
    fn range_read_spans_chunks() {
        let store = MemoryRadStore::new();
        let rad = radisk(&store, tiny_opts());
        for i in 0..20 {
            rad.put(&format!("key{:02}", i), json!(i), None).unwrap();
        }
        rad.flush().unwrap();
        assert!(store.files().len() > 2, "expected multiple chunks");

        let rad2 = radisk(&store, tiny_opts());
        let keys = collect_range(
            &rad2,
            &ReadOpt {
                start: Some("key03".into()),
                end: Some("key15".into()),
                ..Default::default()
            },
        );
        let expected: Vec<String> = (3..=15).map(|i| format!("key{:02}", i)).collect();
        assert_eq!(keys, expected);
    }

    #[test]
    fn reverse_range_read_spans_chunks() {
        let store = MemoryRadStore::new();
        let rad = radisk(&store, tiny_opts());
        for i in 0..20 {
            rad.put(&format!("key{:02}", i), json!(i), None).unwrap();
        }
        rad.flush().unwrap();

        let rad2 = radisk(&store, tiny_opts());
        let keys = collect_range(
            &rad2,
            &ReadOpt {
                start: Some("key03".into()),
                end: Some("key15".into()),
                reverse: true,
            },
        );
        let expected: Vec<String> = (3..=15).rev().map(|i| format!("key{:02}", i)).collect();
        assert_eq!(keys, expected);
    }

    #[test]
    fn each_early_termination() {
        let store = MemoryRadStore::new();
        let rad = radisk(&store, tiny_opts());
        for i in 0..20 {
            rad.put(&format!("key{:02}", i), json!(i), None).unwrap();
        }
        rad.flush().unwrap();

        let rad2 = radisk(&store, tiny_opts());
        let mut seen = 0;
        let hit = rad2
            .each(&ReadOpt::default(), &mut |_, k| {
                seen += 1;
                if k == "key05" { Some(k.to_string()) } else { None }
            })
            .unwrap();
        assert_eq!(hit, Some("key05".to_string()));
        assert_eq!(seen, 6);
    }

    // ── Memory management ───────────────────────────────────────────

    #[test]
    fn data_too_big_rejected() {
        let store = MemoryRadStore::new();
        let rad = radisk(
            &store,
            RadiskOptions {
                max: 10,
                ..Default::default()
            },
        );
        let err = rad.put("k", json!("a very long string value"), None);
        assert_eq!(err, Err("Data too big!".to_string()));
    }

    #[test]
    fn chunk_too_big_on_read() {
        let store = MemoryRadStore::new();
        store.inject("!", &format!("{{\"a\":{{\"\":\"{}\"}}}}", "x".repeat(64)));
        let rad = radisk(
            &store,
            RadiskOptions {
                max: 20,
                ..Default::default()
            },
        );
        assert_eq!(rad.get("a"), Err("Chunk too big!".to_string()));
    }

    #[test]
    fn chunks_evicted_after_flush() {
        let store = MemoryRadStore::new();
        let rad = radisk(&store, RadiskOptions::default());
        rad.put("alex", json!(27), None).unwrap();
        assert_eq!(rad.cached_chunks(), 1);
        rad.flush().unwrap();
        assert_eq!(rad.cached_chunks(), 0, "chunk evicted after flush");
        // Read re-caches on demand.
        assert_eq!(rad.get("alex").unwrap(), Some(json!(27)));
        assert_eq!(rad.cached_chunks(), 1);
    }

    // ── Self-healing ────────────────────────────────────────────────

    #[test]
    fn corrupt_chunk_is_healed_on_read() {
        let store = MemoryRadStore::new();
        let rad = radisk(&store, tiny_opts());
        for i in 0..20 {
            rad.put(&format!("key{:02}", i), json!(i), None).unwrap();
        }
        rad.flush().unwrap();
        let chunks: Vec<String> = store.files().into_iter().filter(|f| f != "%1C" && f != "!").collect();
        assert!(!chunks.is_empty());

        // Corrupt a non-root chunk.
        let victim = &chunks[0];
        store.inject(victim, "{corrupt json!!");

        // Reads heal: drop from directory, retry, return None / remaining data.
        let rad2 = radisk(&store, tiny_opts());
        let victim_key = dename(victim);
        assert_eq!(rad2.get(&victim_key).unwrap(), None);

        // Directory no longer lists the corrupt file as present.
        let dir = Radix::from_json(&store.raw("%1C").unwrap()).unwrap();
        assert_eq!(dir.get(&victim_key), Some(RadixGet::Leaf(json!(0))));

        // Root chunk data is still readable.
        assert_eq!(rad2.get("key00").unwrap(), Some(json!(0)));
    }

    #[test]
    fn legacy_binary_chunk_is_an_error_not_healed() {
        let store = MemoryRadStore::new();
        store.inject("!", "\u{1F}\"alex\u{1F}:\u{1F}+27\u{1F}\n");
        let rad = radisk(&store, RadiskOptions::default());
        let err = rad.get("alex").unwrap_err();
        assert!(err.contains("legacy RAD binary format"), "{}", err);
        // The data was NOT destroyed.
        assert!(store.raw("!").is_some());
    }

    #[test]
    fn startup_list_imports_unknown_files() {
        // Simulate a data directory with chunks but no directory file.
        let store = MemoryRadStore::new();
        store.inject("!", r#"{"alex":{"":27}}"#);
        store.inject("m", r#"{"mike":{"":1}}"#);

        let rad = radisk(&store, RadiskOptions::default());
        assert_eq!(rad.get("mike").unwrap(), Some(json!(1)));
        assert_eq!(rad.get("alex").unwrap(), Some(json!(27)));
        // Directory was rebuilt and persisted.
        let dir = Radix::from_json(&store.raw("%1C").unwrap()).unwrap();
        assert!(Radisk::dir_has(&dir, "!"));
        assert!(Radisk::dir_has(&dir, "m"));
    }

    #[test]
    fn drop_flushes_pending_writes() {
        let store = MemoryRadStore::new();
        {
            let rad = radisk(&store, tiny_opts());
            rad.put("alex", json!(27), None).unwrap();
            assert_eq!(store.put_count(), 0);
        } // dropped here
        assert_eq!(store.raw("!").unwrap(), r#"{"alex":{"":27}}"#);
    }
}
