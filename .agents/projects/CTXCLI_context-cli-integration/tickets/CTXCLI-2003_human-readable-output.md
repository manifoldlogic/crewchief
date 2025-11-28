# Ticket: CTXCLI-2003: Add Human-Readable Output Format

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement pretty-printed human-readable output for the CLI context command when `--json` flag is not set.

## Background
This is the final ticket of Phase 2 (CLI Context Command). While `--json` output is essential for machine consumption, human-readable output makes the CLI more user-friendly for debugging and exploration. This is considered polish but valuable for developer experience.

Reference: [planning/plan.md](../planning/plan.md) - CTXCLI-2003 Example Output

## Acceptance Criteria
- [ ] `format_context_bundle()` function created
- [ ] Output without `--json` is readable and well-formatted
- [ ] Primary chunk displayed with file path, line range, and symbol name
- [ ] Related items grouped by role (CALLER, CALLEE, TEST, HOOK, etc.)
- [ ] Token counts displayed for each item and total
- [ ] Truncation status clearly indicated
- [ ] Budget usage summary shown (used/total)

## Technical Requirements
- Create standalone `format_context_bundle()` function (or module)
- Use Unicode box-drawing characters or ASCII art for visual separation
- Show role with emoji or clear prefix (PRIMARY, CALLER, TEST, etc.)
- Display `reason` field for related items
- Handle edge cases: empty bundle, single item, truncated bundle

## Implementation Notes

### Example Output Format
```
📦 Context Bundle for chunk #12345
   Budget: 6000 tokens | Used: 2450 tokens | Truncated: No

📄 PRIMARY: src/auth.ts:10-30 (authenticate)
   ─────────────────────────────────────────
   async function authenticate(user: User) {
     const token = await generateToken(user);
     return { token, user };
   }
   ─────────────────────────────────────────
   Tokens: 150

🔗 CALLER: src/login.ts:40-60 (login)
   Reason: Calls authenticate function
   Tokens: 120

🧪 TEST: src/__tests__/auth.test.ts:5-25 (authenticate tests)
   Reason: Test file for primary function
   Tokens: 200
```

### Role Emoji Mapping
```rust
fn role_emoji(role: &str) -> &str {
    match role {
        "primary" => "📄",
        "caller" => "🔗",
        "callee" => "📤",
        "test" => "🧪",
        "doc" => "📚",
        "config" => "⚙️",
        "hook" => "🪝",
        "jsx_parent" => "⬆️",
        "jsx_child" => "⬇️",
        _ => "📎",
    }
}
```

### Function Signature
```rust
fn format_context_bundle(bundle: &ContextBundle, chunk_id: i64, budget: usize) -> String {
    let mut output = String::new();

    // Header
    writeln!(&mut output, "📦 Context Bundle for chunk #{}", chunk_id);
    writeln!(&mut output, "   Budget: {} tokens | Used: {} tokens | Truncated: {}",
        budget, bundle.total_tokens, if bundle.truncated { "Yes" } else { "No" });
    writeln!(&mut output);

    // Items grouped by role
    for item in &bundle.items {
        // ... format each item
    }

    output
}
```

## Dependencies
- CTXCLI-2002 (CLI handler must exist to call format function)

## Risk Assessment
- **Risk**: Terminal encoding issues with Unicode/emoji
  - **Mitigation**: Use ASCII fallback if emoji causes issues, or detect terminal capabilities
- **Risk**: Very long content lines
  - **Mitigation**: Truncate preview content to reasonable width (80-120 chars)

## Files/Packages Affected
- `crates/maproom/src/main.rs` (modify - add format_context_bundle function and use it)
