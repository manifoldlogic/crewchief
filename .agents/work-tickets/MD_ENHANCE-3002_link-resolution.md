# Ticket: MD_ENHANCE-3002: Link Resolution

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- parser-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Find all markdown links (internal anchors, relative file paths, external URLs), resolve relative paths to absolute file references, detect cross-references between documents, and create a link database to track document relationships. This enables graph-based navigation and broken link detection.

## Background
Documentation links create a knowledge graph showing how documents relate. Internal links (#anchors) help users navigate within docs, relative links (./other.md) connect related docs, and external links reference additional resources. By resolving and tracking these, we can build a documentation graph and detect broken links.

Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_PLAN.md` lines 63-67

## Acceptance Criteria
- [ ] All markdown links extracted from AST
- [ ] Link types classified: anchor (#section), relative (./path.md), external (https://)
- [ ] Relative file paths resolved to absolute paths or chunk IDs
- [ ] Link text and target captured
- [ ] Links stored in doc_links database table
- [ ] Broken links detected and logged
- [ ] Cross-references between docs tracked

## Technical Requirements
- Create `LinkResolver` struct with base_path for resolution
- Implement `resolve_links(tree, source)` method to find all link nodes
- Extract URL/href from link node
- Extract display text from link node
- Classify links into: External, Anchor, File, or Broken
- For file links, resolve relative paths against base_path
- Check file existence for relative links
- Create Link records with source_chunk_id and target_chunk_id
- Insert links into maproom.doc_links table

Architecture Reference: `/workspace/crewchief_context/maproom/MD_ENHANCE/MD_ENHANCE_ARCHITECTURE.md` lines 186-237

## Implementation Notes

### Link Resolver Structure
```rust
pub struct LinkResolver {
    base_path: PathBuf,
}

impl LinkResolver {
    fn resolve_target(&self, url: &str) -> Option<Target> {
        if url.starts_with("http") {
            Some(Target::External(url.to_string()))
        } else if url.starts_with("#") {
            Some(Target::Anchor(url[1..].to_string()))
        } else {
            // Resolve relative path
            let path = self.base_path.join(url);
            if path.exists() {
                Some(Target::File(path))
            } else {
                None // Broken link
            }
        }
    }
}
```

### Link Types
- **External**: `https://example.com` or `http://`
- **Anchor**: `#section-name` (internal document anchor)
- **Relative**: `../other.md` or `./sibling.md`
- **Absolute**: `/docs/guide.md` (from repo root)

### Link Database Schema
```sql
CREATE TABLE maproom.doc_links (
  source_chunk_id BIGINT REFERENCES maproom.chunks(id),
  target_chunk_id BIGINT REFERENCES maproom.chunks(id),
  link_type TEXT,
  anchor TEXT,
  PRIMARY KEY (source_chunk_id, target_chunk_id)
);
```

### Anchor Resolution
- Find heading with matching text
- Convert heading text to slug (lowercase, hyphens)
- Match against anchor in link

### Edge Cases
- Links without text: `[](url)`
- Image links: `![alt](image.png)`
- Reference-style links: `[text][ref]`
- Broken links to non-existent files
- Malformed URLs

Reference Architecture: lines 201-236 for link resolution logic

## Dependencies
- MD_ENHANCE-2001 (Parent Tracking) - Understanding document structure
- MD_ENHANCE-2002 (Section Boundaries) - Anchors map to sections

## Risk Assessment
- **Risk**: Relative path resolution incorrect for deeply nested docs
  - **Mitigation**: Test with various directory structures, validate against file system, use canonical paths

- **Risk**: Anchor matching fails due to heading text transformations
  - **Mitigation**: Implement standard slug algorithm (lowercase, replace spaces with hyphens), test GitHub anchor format

- **Risk**: Link database grows too large
  - **Mitigation**: Monitor table size, index appropriately, consider pruning external links

## Files/Packages Affected
- `crates/maproom/src/parser/links.rs` - New file for LinkResolver
- `crates/maproom/src/parser/markdown.rs` - Integrate link resolution
- `crates/maproom/migrations/` - New migration for doc_links table
- `crates/maproom/src/db/schema.rs` - Add doc_links schema
- `crates/maproom/tests/links_test.rs` - Test link resolution and classification
- `crates/maproom/tests/fixtures/linked_docs/` - Test files with various link types
