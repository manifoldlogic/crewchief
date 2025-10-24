# Ticket: LANG_PARSE-3003: Go Conventions and Patterns

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
Implement Go-specific conventions and pattern recognition including exported/unexported symbol detection, receiver method extraction, embedded struct parsing, and interface satisfaction analysis.

## Background
Go has unique language conventions that affect code organization and API design. Understanding these patterns is critical for semantic code search:
- **Exported vs Unexported**: Capitalization determines visibility (PascalCase = public, camelCase = private)
- **Receiver Methods**: Methods are defined with receivers, creating method sets for types
- **Embedded Structs**: Go uses composition via embedding rather than inheritance
- **Interface Satisfaction**: Implicit interface implementation without explicit declarations

This ticket implements Phase 3, Week 5, Task 3 from the language parser planning document. These conventions must be properly indexed to enable accurate semantic search and code navigation for Go codebases.

## Acceptance Criteria
- [ ] Exported symbols (PascalCase) distinguished from unexported symbols (camelCase)
- [ ] Receiver methods correctly associated with their receiver types
- [ ] Embedded struct fields identified and parsed
- [ ] Interface satisfaction relationships tracked and indexed

## Technical Requirements
- Implement exported/unexported detection based on identifier capitalization rules
- Extract receiver types (pointer vs value receivers) and associate methods with types
- Parse embedded struct fields and track composition relationships
- Analyze interface definitions and track types that satisfy interfaces
- Store Go-specific metadata in the symbol index for query support
- Handle edge cases: receiver aliases, promoted methods from embedded types
- Support both named and anonymous interface satisfaction

## Implementation Notes

### Exported vs Unexported Detection
- Check first character of identifier: `char.is_uppercase()` = exported
- Apply to: functions, types, constants, variables, struct fields, methods
- Store visibility metadata in symbol attributes

### Receiver Method Handling
- Extract receiver type from method declaration: `func (r *Receiver) Method()`
- Identify pointer vs value receivers: `*Type` vs `Type`
- Associate method with receiver type in the index
- Track method sets for both value and pointer receivers

### Embedded Struct Parsing
- Detect embedded fields: struct fields without field names
- Example: `type MyStruct struct { EmbeddedType }`
- Track composition hierarchy for promoted method resolution
- Store embedding relationships in symbol metadata

### Interface Satisfaction Analysis
- Extract interface method signatures
- Match types with method sets that satisfy interface requirements
- Handle implicit satisfaction (no `implements` keyword)
- Support empty interface `interface{}` (any type)

### File Structure
- `crates/maproom/src/parser/go/conventions.rs`: Core convention detection logic
- Update `crates/maproom/src/parser/go/extractor.rs`: Integrate convention analysis
- `crates/maproom/tests/parser/go_conventions_test.rs`: Comprehensive test suite

## Dependencies
- LANG_PARSE-3002 (Go symbols) - Must be completed first to have basic symbol extraction

## Risk Assessment
- **Risk**: Receiver method association may be complex with type aliases
  - **Mitigation**: Use tree-sitter to resolve type aliases, add comprehensive test cases for aliases

- **Risk**: Interface satisfaction tracking may produce false positives with partial matches
  - **Mitigation**: Implement strict signature matching, validate all method parameters and return types

- **Risk**: Promoted methods from embedded structs require recursive resolution
  - **Mitigation**: Build composition hierarchy during parsing, implement depth-limited promotion resolution

- **Risk**: Performance impact from analyzing all type-interface relationships
  - **Mitigation**: Build index of interface requirements, only check types with matching method names

## Files/Packages Affected
- `crates/maproom/src/parser/go/conventions.rs` (new file)
- `crates/maproom/src/parser/go/extractor.rs` (modify)
- `crates/maproom/src/parser/go/mod.rs` (modify - add conventions module)
- `crates/maproom/tests/parser/go_conventions_test.rs` (new file)
- `crates/maproom/src/schema.rs` (potentially - if new metadata fields needed)
