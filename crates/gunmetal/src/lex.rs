//! LEX — Lexical expression matching for GUN queries.
//!
//! GUN uses lexical expressions (LEX) for key filtering and range queries.
//! A LEX expression can match keys by exact value, prefix, or range.
//!
//! From the source (`./shim`, `String.match`):
//! ```js
//! String.match = function(t, o){
//!     if('string' !== typeof t){ return false }
//!     if('string' == typeof o){ o = {'=': o} }
//!     o = o || {};
//!     tmp = (o['='] || o['*'] || o['>'] || o['<']);
//!     if(t === tmp){ return true }
//!     if(u !== o['=']){ return false }
//!     tmp = (o['*'] || o['>']);
//!     if(t.slice(0, (tmp||'').length) === tmp){ return true }
//!     if(u !== o['*']){ return false }
//!     if(u !== o['>'] && u !== o['<']){
//!         return (t >= o['>'] && t <= o['<'])? true : false;
//!     }
//!     if(u !== o['>'] && t >= o['>']){ return true }
//!     if(u !== o['<'] && t <= o['<']){ return true }
//!     return false;
//! }
//! ```
//!
//! Match hierarchy (cascading specificity):
//! 1. `=` — exact match (prevents `*`, `>`, `<` from matching)
//! 2. `*` — prefix match (prevents `>`, `<` from matching)
//! 3. `>` AND `<` — range match (inclusive on both bounds)
//! 4. `>` OR `<` — single-direction match

/// A lexical expression for matching keys.
///
/// Corresponds to the `LEX` type in GUN's TypeScript definitions:
/// ```typescript
/// type LEX = {
///     '='?: string;  // exact match
///     '*'?: string;  // prefix match
///     '>'?: string;  // gte match
///     '<'?: string;  // lte match
///     '-'?: number;  // 1 for reverse
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct Lex {
    /// Exact match.
    pub exact: Option<String>,
    /// Prefix match.
    pub prefix: Option<String>,
    /// Greater-than-or-equal (inclusive lower bound).
    pub gte: Option<String>,
    /// Less-than-or-equal (inclusive upper bound).
    pub lte: Option<String>,
    /// If true, iterate in reverse lexical order.
    pub reverse: bool,
}

impl Lex {
    /// Create a LEX for exact match.
    pub fn exact(value: impl Into<String>) -> Self {
        Self {
            exact: Some(value.into()),
            ..Default::default()
        }
    }

    /// Create a LEX for prefix match.
    pub fn prefix(value: impl Into<String>) -> Self {
        Self {
            prefix: Some(value.into()),
            ..Default::default()
        }
    }

    /// Create a LEX for a range query (inclusive on both bounds).
    pub fn range(gte: impl Into<String>, lte: impl Into<String>) -> Self {
        Self {
            gte: Some(gte.into()),
            lte: Some(lte.into()),
            ..Default::default()
        }
    }

    /// Test whether a key matches this lexical expression.
    ///
    /// Follows the exact matching logic from GUN's `String.match`:
    /// 1. If `exact` is set: only exact equality matches
    /// 2. If `prefix` is set: key must start with the prefix
    /// 3. If both `gte` and `lte`: key must be in [gte, lte] inclusive
    /// 4. If only `gte`: key must be >= gte
    /// 5. If only `lte`: key must be <= lte
    pub fn matches(&self, key: &str) -> bool {
        // Try the "fast path" — any of the set values as exact match first
        // From source: `tmp = (o['='] || o['*'] || o['>'] || o['<']);`
        //              `if(t === tmp){ return true }`
        let first = self
            .exact
            .as_deref()
            .or(self.prefix.as_deref())
            .or(self.gte.as_deref())
            .or(self.lte.as_deref());
        if let Some(val) = first {
            if key == val {
                return true;
            }
        }

        // 1. Exact match mode — if '=' is set, nothing else matches
        if self.exact.is_some() {
            return false;
        }

        // Prefix check: `tmp = (o['*'] || o['>']);`
        //               `if(t.slice(0, (tmp||'').length) === tmp){ return true }`
        let prefix_or_gte = self.prefix.as_deref().or(self.gte.as_deref());
        if let Some(pfx) = prefix_or_gte {
            if key.starts_with(pfx) {
                return true;
            }
        }

        // 2. Prefix match mode — if '*' is set, nothing else matches
        if self.prefix.is_some() {
            return false;
        }

        // 3. Range match
        match (self.gte.as_deref(), self.lte.as_deref()) {
            (Some(gte), Some(lte)) => key >= gte && key <= lte,
            (Some(gte), None) => key >= gte,
            (None, Some(lte)) => key <= lte,
            (None, None) => false,
        }
    }
}

/// A full lexical query, including a field-level LEX and an optional limit.
///
/// Corresponds to `LEXQuery` in GUN's TypeScript:
/// ```typescript
/// type LEXQuery = { '.': LEX; ':'?: number };
/// ```
#[derive(Debug, Clone)]
pub struct LexQuery {
    /// The field-level lexical expression.
    pub lex: Lex,
    /// Optional limit on number of results.
    pub limit: Option<usize>,
}

impl LexQuery {
    pub fn new(lex: Lex) -> Self {
        Self { lex, limit: None }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        let lex = Lex::exact("hello");
        assert!(lex.matches("hello"));
        assert!(!lex.matches("hell"));
        assert!(!lex.matches("hello world"));
    }

    #[test]
    fn prefix_match() {
        let lex = Lex::prefix("2024/01/");
        assert!(lex.matches("2024/01/"));      // exact prefix also matches
        assert!(lex.matches("2024/01/15"));
        assert!(lex.matches("2024/01/30:14:00"));
        assert!(!lex.matches("2024/02/01"));
        assert!(!lex.matches("2023/01/15"));
    }

    #[test]
    fn range_match_inclusive() {
        let lex = Lex::range("alice", "fred");
        assert!(lex.matches("alice"));          // inclusive lower bound
        assert!(lex.matches("bob"));
        assert!(lex.matches("dave"));
        assert!(lex.matches("fred"));           // inclusive upper bound
        assert!(!lex.matches("george"));
        assert!(!lex.matches("aaa"));
    }

    #[test]
    fn gte_only() {
        let lex = Lex {
            gte: Some("m".into()),
            ..Default::default()
        };
        assert!(lex.matches("mark"));
        assert!(lex.matches("zebra"));
        assert!(!lex.matches("alice"));
    }

    #[test]
    fn lte_only() {
        let lex = Lex {
            lte: Some("m".into()),
            ..Default::default()
        };
        assert!(lex.matches("alice"));
        assert!(lex.matches("m"));
        assert!(!lex.matches("mark"));
    }

    #[test]
    fn reverse_flag() {
        let lex = Lex {
            lte: Some("zach".into()),
            reverse: true,
            ..Default::default()
        };
        assert!(lex.reverse);
        assert!(lex.matches("alice"));
    }

    #[test]
    fn empty_lex_matches_nothing() {
        let lex = Lex::default();
        assert!(!lex.matches("anything"));
    }

    #[test]
    fn exact_takes_priority_over_prefix() {
        // If exact is set, prefix/range are ignored
        let lex = Lex {
            exact: Some("hello".into()),
            prefix: Some("hell".into()),
            ..Default::default()
        };
        assert!(lex.matches("hello"));
        assert!(!lex.matches("hell"));      // prefix would match, but exact takes priority
        assert!(!lex.matches("hello world"));
    }
}
