# Architecture: Maproom Search Tool Optimization

## Solution Overview

Apply genetic optimization learnings through a two-phase approach:
1. **Phase 1 (MVP)**: Adopt proven winner (variant-a-detailed) for immediate +1.9% gain
2. **Phase 2 (Future)**: Create enhanced variant with task-to-query mapping for >20% performance

Create permanent documentation to capture learnings and prevent knowledge loss.

## Component Architecture

### 1. Documentation Layer

**Location**: `docs/optimization/`

**Purpose**: Preserve genetic optimization insights for future tool description work

**Structure**:
```
docs/optimization/
├── README.md                           # Overview and quick reference
├── genetic-optimization-results.md     # Detailed findings and analysis
├── tool-description-patterns.md        # Winning patterns and anti-patterns
└── examples/
    ├── variant-a-detailed.md          # Best performer (19.6%)
    ├── variant-control.md             # Baseline (17.7%)
    └── variant-enhanced.md            # Future: with task-to-query mapping
```

**Content**:
- Quantitative results (scores, token counts, correlations)
- Qualitative insights (structural patterns, tone, example quality)
- Anti-patterns and failures
- Recommended enhancement for future testing

**Rationale**:
- Keep learnings accessible to human developers
- Enable future iterations without re-discovering patterns
- Provide templates for other MCP tool descriptions

### 2. Production Tool Description

**Location**: `packages/maproom-mcp/src/tools/search.ts`

**Current State**:
```typescript
export const searchTool = {
  name: 'search',
  description: `${CONTROL_VARIANT_TEXT}`, // 17.7% baseline
  // ...parameters
}
```

**Target State**:
```typescript
export const searchTool = {
  name: 'search',
  description: `${DETAILED_VARIANT_TEXT}`, // 19.6% winner
  // ...parameters (unchanged)
}
```

**Changes**:
- Replace `description` field with variant-a-detailed content
- No API changes (tool name, parameters, behavior unchanged)
- Maintain backward compatibility with existing agent integrations

**Deployment**:
- Update happens in TypeScript source
- Requires `pnpm build` to regenerate dist
- MCP server restart to load new description
- No database or schema changes

### 3. Variant Repository

**Location**: `packages/maproom-mcp/test/tool-description-optimization/variants/`

**Existing Variants** (keep as-is):
- `variant-control.json` (baseline)
- `variant-a-detailed.json` (winner - source for Phase 1)
- `variant-b-simple.json` (failed experiment)
- `variant-c-conversational.json`
- `variant-d-code-like.json`

**New Variant** (Phase 2):
- `variant-e-task-mapping.json` (enhanced with task-to-query section)

**Purpose**:
- Maintain genetic optimization experiment history
- Provide source material for documentation
- Enable future A/B testing

### 4. Test Infrastructure

**Location**: `packages/cli/src/search-optimization/`

**Existing**:
- Benchmark suite (worktree task + others)
- Competition runner (parallel agent execution)
- Genetic iterator (mutation engine)
- Scoring system

**No changes required** - infrastructure already supports:
- Loading variants from JSON
- Running benchmark comparisons
- Generating performance reports

**Usage** for validation:
```bash
# Run single-variant test
npx tsx src/search-optimization/run-single-variant.ts variant-a-detailed

# Compare control vs detailed
npx tsx src/search-optimization/run-comparison.ts variant-control variant-a-detailed
```

## Data Flow

### Phase 1: Adopt Winner

```
variant-a-detailed.json
    ↓ (copy description field)
packages/maproom-mcp/src/tools/search.ts
    ↓ (pnpm build)
dist/tools/search.js
    ↓ (MCP server restart)
Production MCP Server
    ↓ (Claude agents call tool)
Improved Agent Performance
```

### Phase 2: Document Learnings

```
Genetic Optimization Results
    ↓ (analysis and synthesis)
docs/optimization/*.md
    ↓ (commit to main branch)
Permanent Repository Knowledge
```

### Phase 3: Create Enhancement (Future)

```
variant-a-detailed.json
    ↓ (add task-to-query section)
variant-e-task-mapping.json
    ↓ (test in future genetic run)
Potential >20% Performance
```

## Technology Choices

### Documentation Format: Markdown

**Choice**: Markdown files in `docs/`

**Alternatives considered**:
- JSON (too structured, not human-readable)
- Wiki/external docs (risk of going stale)
- Code comments (scattered, not comprehensive)

**Rationale**:
- Human-readable and searchable
- Supports tables, code blocks, examples
- Version controlled with code
- Renders nicely on GitHub

### Tool Description Storage: TypeScript String Literal

**Choice**: Inline template literal in `search.ts`

**Alternatives considered**:
- External JSON file (extra indirection)
- Database configuration (overkill for static content)
- Environment variable (hard to version/review)

**Rationale**:
- Co-located with tool definition
- Type-checked and linted
- Easy to review in PRs
- No runtime file I/O

### Variant Format: JSON

**Choice**: Keep existing JSON structure

```json
{
  "id": "variant-a-detailed",
  "name": "Detailed (Comprehensive)",
  "description": "...",
  "tokens": 450,
  "generation": 0,
  "parent_ids": [],
  "created_at": "2025-11-06T00:00:00.000Z",
  "notes": "..."
}
```

**Rationale**:
- Already used by genetic optimizer
- Structured for automated testing
- Preserves metadata (generation, parents, token count)

## Design Decisions

### Decision 1: Two-Phase Approach

**Question**: Should we immediately deploy the enhancement (task-to-query mapping) or adopt the proven winner first?

**Decision**: Adopt proven winner (Phase 1), create enhancement for future testing (Phase 2)

**Rationale**:
- **Risk mitigation**: Enhancement is untested, could regress performance
- **Quick wins**: Proven winner gives immediate +1.9% with zero risk
- **Validation**: Can test enhancement in next genetic run before production
- **Incremental improvement**: Avoid big-bang changes

**Trade-off**: Delays potential >20% performance, but ensures we don't regress from current 17.7%

### Decision 2: Documentation in Repo vs External

**Question**: Where should learnings be documented?

**Decision**: In-repo `docs/optimization/` directory

**Rationale**:
- **Version controlled**: Changes tracked alongside code
- **Discoverable**: Searchable with grep, visible in IDE
- **Owned by team**: No external service dependencies
- **Review process**: Subject to PR review like code

**Trade-off**: Less discoverable than wiki, but more likely to stay current

### Decision 3: Full Replacement vs Gradual Migration

**Question**: Should we gradually introduce new description or fully replace?

**Decision**: Full replacement of description field

**Rationale**:
- **Simple**: Single string replacement, no conditional logic
- **Testable**: Clear before/after comparison
- **Reversible**: Git revert if issues arise
- **Clean**: No hybrid descriptions or feature flags

**Trade-off**: All-or-nothing change, but risk is low (only description text)

### Decision 4: Validate Before or After Deployment?

**Question**: Should we run validation tests before merging or monitor after deployment?

**Decision**: Validate before merging (run comparison test)

**Rationale**:
- **Safety**: Catch regressions before production
- **Confidence**: Ensure experimental results replicate
- **Documentation**: Test results become part of PR evidence
- **Reversibility**: No production impact if test fails

**Trade-off**: Adds ~30 minutes to deployment timeline, but worth the confidence

## Performance Considerations

### Description Token Count

**Current**: 350 tokens (control variant)
**Target**: 450 tokens (detailed variant)
**Overhead**: +100 tokens per tool call

**Impact Analysis**:
- Claude Sonnet 4 context: 200K tokens
- Typical agent conversation: 20-50K tokens used
- Tool description overhead: 0.05% of context budget
- **Conclusion**: Negligible impact, focus on effectiveness over efficiency

### MCP Server Memory

**Current**: ~50MB base + descriptions loaded
**After**: +100 tokens = ~400 bytes additional memory
**Impact**: Negligible (< 0.001% increase)

### Agent Execution Time

**Current**: ~10-30 seconds per search task
**After**: May improve (better guidance → fewer retries)
**Monitoring**: Track agent turn count and search call frequency

## Constraints & Limitations

### Technical Constraints

1. **MCP Protocol Compatibility**: Tool description must be valid MCP tool schema
2. **Token Limits**: Keep under 600 tokens for efficient model processing
3. **Backward Compatibility**: Existing agents must work without modification
4. **TypeScript Types**: Description field type remains `string`

### Operational Constraints

1. **No Breaking Changes**: Tool API (name, parameters) must stay identical
2. **Deployment Window**: Can deploy anytime (no downtime required)
3. **Rollback Plan**: Git revert if issues detected
4. **Testing Time**: Budget 30-60 minutes for validation before merge

### Business Constraints

1. **MVP Focus**: Ship proven winner, don't over-engineer
2. **Documentation Debt**: Must document learnings, not just code changes
3. **Future-Proofing**: Create enhancement variant for next iteration
4. **Knowledge Transfer**: Ensure team understands patterns

## Migration Strategy

### Phase 1: Documentation (Immediate)

1. Create `docs/optimization/` structure
2. Write comprehensive findings documentation
3. Extract variant examples
4. Document winning patterns and anti-patterns
5. Commit to main branch

### Phase 2: Production Update (After documentation)

1. Copy variant-a-detailed description to `search.ts`
2. Run validation comparison test
3. Review results (must show ≥19.0% performance)
4. Create PR with test evidence
5. Merge and deploy

### Phase 3: Enhancement Creation (Future)

1. Clone variant-a-detailed → variant-e-task-mapping
2. Add task-to-query mapping section
3. Store in variants directory
4. Include in next genetic optimization run
5. If successful, repeat Phase 2 for enhancement

## Rollback Plan

If production deployment shows issues:

**Immediate** (< 5 minutes):
```bash
git revert <commit-sha>
git push origin main
# Rebuild and restart MCP server
```

**Validation** (< 15 minutes):
```bash
# Run quick comparison test
npx tsx src/search-optimization/run-comparison.ts variant-control variant-a-detailed
```

**Analysis** (< 1 hour):
- Review agent conversation logs
- Compare search query quality
- Identify specific failure modes
- Document findings for future attempt

## Monitoring & Validation

### Pre-Deployment Validation

**Test**: Run benchmark comparison
```bash
npx tsx src/search-optimization/run-comparison.ts \
  variant-control \
  variant-a-detailed \
  --tasks=impl-worktree-001 \
  --iterations=5
```

**Success Criteria**:
- variant-a-detailed scores ≥19.0% (allow 0.6% margin for variance)
- No tool call errors or timeouts
- Query quality matches expected patterns

### Post-Deployment Monitoring

**Week 1**: Active monitoring
- Sample agent conversations using maproom search
- Review search queries formulated by agents
- Check for error rates or quality degradation
- Compare to baseline metrics

**Week 2-4**: Passive monitoring
- Aggregate search success rates
- Track query pattern distribution
- Monitor for unexpected tool usage patterns

**Metrics to Track**:
- Search calls per agent session
- Query length distribution (expect 2-3 word queries)
- Result count distribution
- Agent retry frequency

## Success Criteria

### Phase 1 (Documentation)

- [ ] Documentation published to `docs/optimization/`
- [ ] All genetic optimization insights captured
- [ ] Winning patterns clearly documented
- [ ] Examples include control vs detailed comparison
- [ ] Anti-patterns documented with explanations

### Phase 2 (Production Update)

- [ ] Pre-deployment test shows ≥19.0% performance
- [ ] PR includes test evidence and results
- [ ] Code review approved
- [ ] Deployment successful
- [ ] Post-deployment spot check confirms functionality

### Phase 3 (Enhancement Creation)

- [ ] variant-e-task-mapping created
- [ ] Task-to-query section properly formatted
- [ ] Stored in variants directory
- [ ] Ready for next genetic optimization run

## Future Considerations

### Potential Enhancements (Beyond Scope)

1. **Multi-language support**: Optimize descriptions for different coding contexts
2. **Dynamic descriptions**: Adjust based on agent capabilities or task complexity
3. **A/B testing framework**: Systematically test variations in production
4. **Feedback loop**: Collect agent success/failure data to guide iterations

### Long-term Vision

- **Tool description library**: Reusable patterns for all MCP tools
- **Automated optimization**: Continuous genetic improvement pipeline
- **Benchmarking suite**: Comprehensive task coverage beyond single worktree task
- **Agent-specific tuning**: Different descriptions for different agent types

These remain future opportunities, not current scope.
