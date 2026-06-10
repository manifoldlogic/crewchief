//! Radix — in-memory radix (patricia) tree, a port of GUN's `lib/radix.js`.
//!
//! GUN stores the tree as nested plain objects. Each node maps a prefix
//! string to either a subtree (object) or a terminal value stored at the
//! empty-string key `""`:
//!
//! ```json
//! { "a": { "lex": { "": 27, "andria": { "": "library" } }, "ndrew": { "": true } } }
//! ```
//!
//! We mirror that exactly using `serde_json::Map<String, Value>` so that
//! `to_json()` produces byte-for-byte the same format as GUN's
//! `JSON.stringify(radix.$)` (modulo key ordering, which is irrelevant to
//! JSON semantics — and sorted order matches GUN's insertion order in the
//! common cases). Leaf values are arbitrary JSON values; RAD stores
//! `{":": value, ">": state}` envelopes as leaves.
//!
//! Source mapping:
//! - `Radix()` write path  → [`Radix::insert`]
//! - `Radix()` read path   → [`Radix::get`] (returns leaf or subtree)
//! - `Radix.map`           → [`Radix::map`] / [`map_tree`]
//! - `radix.$`             → [`Radix::tree`]
//! - `radix.last`          → [`Radix::last`]

use serde_json::{Map, Value};

/// Result of a radix lookup.
///
/// Mirrors GUN's `radix(key)` return: either an exact leaf value
/// (`radix.unit = 1`) or a subtree of everything under the key/prefix.
#[derive(Debug, Clone, PartialEq)]
pub enum RadixGet {
    /// An exact value was found at the key.
    Leaf(Value),
    /// The key is a prefix; the subtree's keys are the remaining suffixes.
    Tree(Map<String, Value>),
}

/// Iteration options for [`Radix::map`]. Port of `Radix.map`'s `opt`.
#[derive(Debug, Default, Clone)]
pub struct MapOpt {
    /// Iterate in reverse lexicographic order.
    pub reverse: bool,
    /// Skip keys before this value (inclusive).
    pub start: Option<String>,
    /// Skip keys after this value (inclusive).
    pub end: Option<String>,
}

/// An in-memory radix tree with JSON-native leaves.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Radix {
    tree: Map<String, Value>,
    last: String,
}

impl Radix {
    /// Create a new empty tree. Port of `Radix()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Wrap an already-parsed nested radix object (e.g. from `JSON.parse`).
    pub fn from_tree(tree: Map<String, Value>) -> Self {
        Self {
            tree,
            last: String::new(),
        }
    }

    /// The raw nested object backing the tree — GUN's `radix.$`.
    pub fn tree(&self) -> &Map<String, Value> {
        &self.tree
    }

    /// The lexicographically greatest key ever inserted — GUN's `radix.last`.
    pub fn last(&self) -> &str {
        &self.last
    }

    /// True if no keys have been inserted.
    pub fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    /// Insert a value at a key. Port of `radix(key, val)`.
    pub fn insert(&mut self, key: &str, val: Value) {
        if self.last.as_str() < key {
            self.last = key.to_string();
        }
        insert_into(&mut self.tree, key, val);
    }

    /// Look up a key. Returns an exact leaf, a subtree for prefix matches,
    /// or `None`. Port of `radix(key)` (read path).
    pub fn get(&self, key: &str) -> Option<RadixGet> {
        get_in(&self.tree, key)
    }

    /// Walk the tree in sorted key order, calling `cb(value, full_key)` for
    /// each leaf. Return `Some(r)` from the callback to stop early; the
    /// value propagates out. Port of `Radix.map`.
    pub fn map<R, F>(&self, opt: &MapOpt, cb: &mut F) -> Option<R>
    where
        F: FnMut(&Value, &str) -> Option<R>,
    {
        map_tree(&self.tree, opt, "", cb)
    }

    /// Visit every leaf in sorted order (no early termination).
    pub fn each<F>(&self, mut cb: F)
    where
        F: FnMut(&Value, &str),
    {
        let _ = self.map::<(), _>(&MapOpt::default(), &mut |v, k| {
            cb(v, k);
            None
        });
    }

    /// Total number of leaves in the tree.
    pub fn count(&self) -> usize {
        let mut n = 0;
        self.each(|_, _| n += 1);
        n
    }

    /// Serialize as GUN does: `JSON.stringify(radix.$)`.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(&Value::Object(self.tree.clone()))
            .map_err(|e| format!("Cannot radisk! {}", e))
    }

    /// Parse a chunk previously produced by [`Radix::to_json`] (or GUN.js).
    pub fn from_json(data: &str) -> Result<Self, String> {
        let val: Value =
            serde_json::from_str(data).map_err(|e| format!("JSON error! {}", e))?;
        match val {
            Value::Object(tree) => Ok(Self::from_tree(tree)),
            _ => Err("JSON error! root is not an object".to_string()),
        }
    }
}

// ── Insert (port of radix.js write path) ────────────────────────────

/// Char-boundary prefix lengths of `key`, shortest first (1 char .. all).
fn prefix_lengths(key: &str) -> impl Iterator<Item = usize> + '_ {
    key.char_indices().skip(1).map(|(i, _)| i).chain(std::iter::once(key.len()))
}

/// Longest common prefix of two strings, on char boundaries.
fn common_prefix<'a>(a: &'a str, b: &str) -> &'a str {
    let mut end = 0;
    for ((ia, ca), (_, cb)) in a.char_indices().zip(b.char_indices()) {
        if ca != cb {
            break;
        }
        end = ia + ca.len_utf8();
    }
    &a[..end]
}

fn insert_into(t: &mut Map<String, Value>, key: &str, val: Value) {
    if key.is_empty() {
        t.insert(String::new(), val);
        return;
    }

    // `while(!(at = t[k]) && i < l){ k += key[++i] }` — find an existing
    // edge equal to a prefix of `key`, shortest first.
    let found = prefix_lengths(key).find(|&end| t.contains_key(&key[..end]));

    match found {
        // Exact edge match for the whole key: store the leaf at "".
        Some(end) if end == key.len() => match t.get_mut(key) {
            Some(Value::Object(at)) => {
                at.insert(String::new(), val);
            }
            _ => {
                let mut leaf = Map::new();
                leaf.insert(String::new(), val);
                t.insert(key.to_string(), Value::Object(leaf));
            }
        },
        // Edge matches a prefix: recurse with the remainder.
        Some(end) => {
            let k = key[..end].to_string();
            let rest = &key[end..];
            match t.get_mut(&k) {
                Some(Value::Object(at)) => insert_into(at, rest, val),
                _ => {
                    let mut sub = Map::new();
                    insert_into(&mut sub, rest, val);
                    t.insert(k, Value::Object(sub));
                }
            }
        }
        // No edge is a prefix of key: split an edge sharing a common
        // prefix, or add a brand new edge.
        None => {
            let split = t.keys().find_map(|s| {
                if s.is_empty() {
                    return None;
                }
                let kk = common_prefix(s, key);
                if kk.is_empty() {
                    None
                } else {
                    Some((s.clone(), kk.to_string()))
                }
            });

            if let Some((s, kk)) = split {
                // `__[s.slice(ii)] = r; t[kk] = __; delete t[s];`
                let old = t.remove(&s).unwrap_or(Value::Null);
                let mut node = Map::new();
                node.insert(s[kk.len()..].to_string(), old);
                let rest = &key[kk.len()..];
                if rest.is_empty() {
                    node.insert(String::new(), val);
                } else {
                    let mut leaf = Map::new();
                    leaf.insert(String::new(), val);
                    node.insert(rest.to_string(), Value::Object(leaf));
                }
                t.insert(kk, Value::Object(node));
            } else {
                // `(t[k] || (t[k] = {}))[''] = val`
                let mut leaf = Map::new();
                leaf.insert(String::new(), val);
                t.insert(key.to_string(), Value::Object(leaf));
            }
        }
    }
}

// ── Lookup (port of radix.js read path) ─────────────────────────────

fn get_in(t: &Map<String, Value>, key: &str) -> Option<RadixGet> {
    if key.is_empty() {
        // `if(!key && Object.keys(t).length){ return t }`
        if t.is_empty() {
            return None;
        }
        return Some(RadixGet::Tree(t.clone()));
    }

    let found = prefix_lengths(key).find(|&end| t.contains_key(&key[..end]));

    match found {
        Some(end) if end == key.len() => {
            let Some(Value::Object(at)) = t.get(key) else {
                return None;
            };
            // `(u === (tmp = at['']))? at : ((radix.unit = 1) && tmp)`
            match at.get("") {
                Some(v) => Some(RadixGet::Leaf(v.clone())),
                None => Some(RadixGet::Tree(at.clone())),
            }
        }
        Some(end) => {
            let Some(Value::Object(at)) = t.get(&key[..end]) else {
                return None;
            };
            get_in(at, &key[end..])
        }
        None => {
            // Prefix query ending inside an edge: synthesize a subtree
            // holding the remainder of the matching edge.
            for (s, r) in t.iter() {
                if s.is_empty() {
                    continue;
                }
                let kk = common_prefix(s, key);
                if kk.len() == key.len() {
                    let mut sub = Map::new();
                    sub.insert(s[kk.len()..].to_string(), r.clone());
                    return Some(RadixGet::Tree(sub));
                }
            }
            None
        }
    }
}

// ── Iteration (port of Radix.map) ───────────────────────────────────

/// Walk a raw radix subtree. `pre` is the accumulated key prefix.
pub fn map_tree<R, F>(t: &Map<String, Value>, opt: &MapOpt, pre: &str, cb: &mut F) -> Option<R>
where
    F: FnMut(&Value, &str) -> Option<R>,
{
    // serde_json's Map is sorted (BTreeMap) by default, but sort
    // explicitly so behavior never depends on the map implementation.
    let mut keys: Vec<&String> = t.keys().collect();
    keys.sort();
    if opt.reverse {
        keys.reverse();
    }

    for key in keys {
        if key.is_empty() {
            continue;
        }
        let Some(Value::Object(tree)) = t.get(key.as_str()) else {
            continue; // malformed child — skip, mirroring `if(!tree){ continue }`
        };
        let pt = format!("{}{}", pre, key);

        // Subtree pruning, as in radix.js:
        // `if(u !== start && pt < (start||'').slice(0,pt.length)){ continue }`
        if let Some(start) = &opt.start {
            let cut: String = start.chars().take(pt.chars().count()).collect();
            if pt.as_str() < cut.as_str() {
                continue;
            }
        }
        // `if(u !== end && (end || END) < pt){ continue }`
        if let Some(end) = &opt.end
            && end.as_str() < pt.as_str()
        {
            continue;
        }

        let visit_leaf = |cb: &mut F| -> Option<R> {
            if let Some(v) = tree.get("") {
                let mut yes = true;
                if let Some(start) = &opt.start
                    && pt.as_str() < start.as_str()
                {
                    yes = false;
                }
                if let Some(end) = &opt.end
                    && pt.as_str() > end.as_str()
                {
                    yes = false;
                }
                if yes {
                    return cb(v, &pt);
                }
            }
            None
        };

        if opt.reverse {
            // Children must be checked first when going in reverse.
            if let Some(r) = map_tree(tree, opt, &pt, cb) {
                return Some(r);
            }
            if let Some(r) = visit_leaf(cb) {
                return Some(r);
            }
        } else {
            if let Some(r) = visit_leaf(cb) {
                return Some(r);
            }
            if let Some(r) = map_tree(tree, opt, &pt, cb) {
                return Some(r);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn demo_tree() -> Radix {
        let mut r = Radix::new();
        r.insert("alex", json!(27));
        r.insert("alexandria", json!("library"));
        r.insert("andrew", json!(true));
        r
    }

    fn collect(r: &Radix, opt: &MapOpt) -> Vec<(String, Value)> {
        let mut out = Vec::new();
        let _ = r.map::<(), _>(opt, &mut |v, k| {
            out.push((k.to_string(), v.clone()));
            None
        });
        out
    }

    // ── Insert + lookup ─────────────────────────────────────────────

    #[test]
    fn insert_and_exact_lookup() {
        let r = demo_tree();
        assert_eq!(r.get("alex"), Some(RadixGet::Leaf(json!(27))));
        assert_eq!(r.get("alexandria"), Some(RadixGet::Leaf(json!("library"))));
        assert_eq!(r.get("andrew"), Some(RadixGet::Leaf(json!(true))));
    }

    #[test]
    fn lookup_missing_key() {
        let r = demo_tree();
        assert_eq!(r.get("zelda"), None);
        assert_eq!(Radix::new().get("anything"), None);
    }

    #[test]
    fn null_and_false_are_valid_leaves() {
        let mut r = Radix::new();
        r.insert("bob", json!(null));
        r.insert("charlie", json!(false));
        assert_eq!(r.get("bob"), Some(RadixGet::Leaf(json!(null))));
        assert_eq!(r.get("charlie"), Some(RadixGet::Leaf(json!(false))));
    }

    #[test]
    fn overwrite_value() {
        let mut r = demo_tree();
        r.insert("alex", json!(28));
        assert_eq!(r.get("alex"), Some(RadixGet::Leaf(json!(28))));
        assert_eq!(r.count(), 3);
    }

    #[test]
    fn insert_shorter_key_after_longer() {
        // Splits the "alexandria" edge with an interior key.
        let mut r = Radix::new();
        r.insert("alexandria", json!("library"));
        r.insert("alex", json!(27));
        assert_eq!(r.get("alex"), Some(RadixGet::Leaf(json!(27))));
        assert_eq!(r.get("alexandria"), Some(RadixGet::Leaf(json!("library"))));
    }

    #[test]
    fn prefix_lookup_returns_subtree() {
        let r = demo_tree();
        match r.get("ale") {
            Some(RadixGet::Tree(sub)) => {
                // Subtree keyed by remaining suffix "x".
                let mut found = Vec::new();
                let _ = map_tree::<(), _>(&sub, &MapOpt::default(), "ale", &mut |v, k| {
                    found.push((k.to_string(), v.clone()));
                    None
                });
                assert_eq!(
                    found,
                    vec![
                        ("alex".to_string(), json!(27)),
                        ("alexandria".to_string(), json!("library")),
                    ]
                );
            }
            other => panic!("expected subtree, got {:?}", other),
        }
    }

    #[test]
    fn root_read_returns_whole_tree() {
        let r = demo_tree();
        match r.get("") {
            Some(RadixGet::Tree(t)) => assert_eq!(&t, r.tree()),
            other => panic!("expected tree, got {:?}", other),
        }
        assert_eq!(Radix::new().get(""), None);
    }

    #[test]
    fn last_tracks_greatest_key() {
        let mut r = Radix::new();
        r.insert("m", json!(1));
        assert_eq!(r.last(), "m");
        r.insert("z", json!(1));
        assert_eq!(r.last(), "z");
        r.insert("a", json!(1));
        assert_eq!(r.last(), "z");
    }

    // ── Iteration ───────────────────────────────────────────────────

    #[test]
    fn map_is_lexicographically_ordered() {
        let mut r = demo_tree();
        r.insert("bob", json!(null));
        r.insert("charlie", json!(false));
        let keys: Vec<String> = collect(&r, &MapOpt::default())
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(keys, vec!["alex", "alexandria", "andrew", "bob", "charlie"]);
    }

    #[test]
    fn map_reverse() {
        let r = demo_tree();
        let keys: Vec<String> = collect(
            &r,
            &MapOpt {
                reverse: true,
                ..Default::default()
            },
        )
        .into_iter()
        .map(|(k, _)| k)
        .collect();
        assert_eq!(keys, vec!["andrew", "alexandria", "alex"]);
    }

    #[test]
    fn map_range_inclusive_both_bounds() {
        let mut r = Radix::new();
        for k in ["alice", "bob", "carl", "dave", "fred"] {
            r.insert(k, json!(1));
        }
        let keys: Vec<String> = collect(
            &r,
            &MapOpt {
                start: Some("bob".into()),
                end: Some("dave".into()),
                ..Default::default()
            },
        )
        .into_iter()
        .map(|(k, _)| k)
        .collect();
        assert_eq!(keys, vec!["bob", "carl", "dave"]);
    }

    #[test]
    fn map_single_direction_ranges() {
        let mut r = Radix::new();
        for k in ["a", "b", "c", "d"] {
            r.insert(k, json!(1));
        }
        let after: Vec<String> = collect(
            &r,
            &MapOpt {
                start: Some("c".into()),
                ..Default::default()
            },
        )
        .into_iter()
        .map(|(k, _)| k)
        .collect();
        assert_eq!(after, vec!["c", "d"]);

        let before: Vec<String> = collect(
            &r,
            &MapOpt {
                end: Some("b".into()),
                ..Default::default()
            },
        )
        .into_iter()
        .map(|(k, _)| k)
        .collect();
        assert_eq!(before, vec!["a", "b"]);
    }

    #[test]
    fn map_reverse_range() {
        let mut r = Radix::new();
        for k in ["alice", "bob", "carl", "dave"] {
            r.insert(k, json!(1));
        }
        let keys: Vec<String> = collect(
            &r,
            &MapOpt {
                reverse: true,
                start: Some("bob".into()),
                end: Some("dave".into()),
            },
        )
        .into_iter()
        .map(|(k, _)| k)
        .collect();
        assert_eq!(keys, vec!["dave", "carl", "bob"]);
    }

    #[test]
    fn map_early_termination() {
        let r = demo_tree();
        let mut visited = Vec::new();
        let first = r.map(&MapOpt::default(), &mut |v, k| {
            visited.push(k.to_string());
            Some((k.to_string(), v.clone()))
        });
        assert_eq!(first, Some(("alex".to_string(), json!(27))));
        assert_eq!(visited, vec!["alex"]);
    }

    #[test]
    fn map_range_with_shared_prefixes() {
        let mut r = Radix::new();
        for k in ["users/alice", "users/bob", "users/carl", "zoo"] {
            r.insert(k, json!(1));
        }
        let keys: Vec<String> = collect(
            &r,
            &MapOpt {
                start: Some("users/b".into()),
                end: Some("users/z".into()),
                ..Default::default()
            },
        )
        .into_iter()
        .map(|(k, _)| k)
        .collect();
        assert_eq!(keys, vec!["users/bob", "users/carl"]);
    }

    // ── Serialization ───────────────────────────────────────────────

    #[test]
    fn json_matches_gun_format_exactly() {
        // From rad.md: inserting alex=27, alexandria="library", andrew=true.
        let r = demo_tree();
        assert_eq!(
            r.to_json().unwrap(),
            r#"{"a":{"lex":{"":27,"andria":{"":"library"}},"ndrew":{"":true}}}"#
        );
    }

    #[test]
    fn json_roundtrip() {
        let mut r = demo_tree();
        r.insert("bob", json!(null));
        r.insert("users/alice", json!({":": "admin", ">": 1700000000000.0}));

        let raw = r.to_json().unwrap();
        let parsed = Radix::from_json(&raw).unwrap();
        assert_eq!(collect(&parsed, &MapOpt::default()), collect(&r, &MapOpt::default()));
    }

    #[test]
    fn parses_gun_written_chunk() {
        // Hand-crafted chunk exactly as GUN.js would write it.
        let gun_chunk = r#"{"a":{"lex":{"":27,"andria":{"":"library"}},"ndrew":{"":true}}}"#;
        let r = Radix::from_json(gun_chunk).unwrap();
        assert_eq!(r.get("alex"), Some(RadixGet::Leaf(json!(27))));
        assert_eq!(r.get("alexandria"), Some(RadixGet::Leaf(json!("library"))));
        assert_eq!(r.get("andrew"), Some(RadixGet::Leaf(json!(true))));
        assert_eq!(r.count(), 3);
    }

    #[test]
    fn from_json_rejects_non_object() {
        assert!(Radix::from_json("[1,2,3]").is_err());
        assert!(Radix::from_json("not json").is_err());
    }

    #[test]
    fn envelope_leaves_are_not_subtrees() {
        // A `{":": v, ">": s}` envelope at "" must be yielded as a leaf,
        // never descended into.
        let mut r = Radix::new();
        r.insert("soul\u{1B}name", json!({":": "Mark", ">": 100.0}));
        let entries = collect(&r, &MapOpt::default());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "soul\u{1B}name");
        assert_eq!(entries[0].1, json!({":": "Mark", ">": 100.0}));
    }

    #[test]
    fn empty_string_key_is_a_root_leaf() {
        let mut r = Radix::new();
        r.insert("", json!("root"));
        // Stored at the root's "" slot; iteration skips it (as GUN does).
        assert_eq!(r.tree().get(""), Some(&json!("root")));
    }

    #[test]
    fn unicode_keys() {
        let mut r = Radix::new();
        r.insert("héllo", json!(1));
        r.insert("hélp", json!(2));
        assert_eq!(r.get("héllo"), Some(RadixGet::Leaf(json!(1))));
        assert_eq!(r.get("hélp"), Some(RadixGet::Leaf(json!(2))));
        let keys: Vec<String> = collect(&r, &MapOpt::default())
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(keys, vec!["héllo", "hélp"]);
    }
}
