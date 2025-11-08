# Ticket: BRWATCH-4001: Create user documentation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (documentation ticket)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Create comprehensive user documentation for the automatic branch switch detection feature, including usage guide, troubleshooting, and performance tuning.

## Background
This ticket implements Step 4.1 from the implementation plan (plan.md - Phase 4). Good documentation is essential for user adoption - developers need to understand how to use `maproom watch`, what to expect, and how to troubleshoot issues.

**Planning Reference**: `/workspace/.agents/projects/BRWATCH_branch-switch-detection/planning/plan.md` - Step 4.1

## Acceptance Criteria
- [ ] Feature documentation created at `docs/features/automatic-indexing.md`
- [ ] Usage guide with command examples
- [ ] Troubleshooting section with common issues and solutions
- [ ] Performance characteristics documented
- [ ] Prerequisites and setup instructions
- [ ] Examples for different workflows
- [ ] Links to related documentation (BRANCHX, incremental updates)
- [ ] Clear, well-formatted markdown

## Technical Requirements
- Create new file: `/workspace/docs/features/automatic-indexing.md`
- Follow existing documentation style and format
- Include code examples with proper syntax highlighting
- Add screenshots or terminal output examples (as text)
- Link to relevant planning documents for technical details
- Keep language user-friendly (not overly technical)

## Implementation Notes

### Documentation Structure

```markdown
# Automatic Branch Switch Detection

## Overview
Brief introduction to the feature and its benefits.

## Prerequisites
- BRANCHX implementation complete
- PostgreSQL database configured
- DATABASE_URL environment variable set

## Quick Start
```bash
# Start watching your repository
maproom watch --repo /path/to/myproject

# The watcher automatically indexes branches as you switch
git checkout feature-auth  # Auto-indexed in <1 minute
```

## Usage

### Basic Usage
Command syntax and common options.

### Workflow Examples
- Solo developer workflow
- Team collaboration with multiple branches
- Feature branch development

## How It Works
High-level explanation of file watching, change detection, and incremental updates.

## Performance
- Detection latency: <1 second
- Update time: <1 minute (typical)
- Resource usage: <5% CPU, <20MB RAM while idle

## Troubleshooting

### Common Issues
1. "DATABASE_URL not set"
   - Solution: Export DATABASE_URL environment variable

2. "Not a git repository"
   - Solution: Ensure repository has .git directory

3. "Permission denied reading .git/HEAD"
   - Solution: Check file permissions

### Logs and Debugging
How to enable verbose mode and interpret logs.

## Configuration
Any configurable options (if added in future).

## Related Documentation
- [BRANCHX: Branch-Aware Indexing](../architecture/BRANCHX.md)
- [Incremental Updates](../architecture/incremental-updates.md)

## Technical Details
Link to architecture.md for implementation details.
```

### Example Content

**Quick Start Section**:
```markdown
## Quick Start

Start the branch watcher in your repository:

```bash
cd /path/to/your/project
maproom watch --repo .
```

Output:
```
[INFO] Starting branch watcher for /path/to/your/project
[INFO] Connected to database
[INFO] Watching for branch switches...
[INFO] Indexing current branch: main
[INFO] Index updated in 0.1s (0 files changed)
[INFO] Waiting for changes...
```

Now when you switch branches, indexing happens automatically:

```bash
# In another terminal
git checkout feature-auth
```

Watcher output:
```
[INFO] Branch switch detected: feature-auth
[INFO] Index updated in 45.2s:
[INFO]   Files processed: 150
[INFO]   Chunks processed: 7,500
[INFO]   Cache hit rate: 84.2%
[INFO]   Embeddings generated: 1,185
[INFO]   Estimated cost: $0.0237
[INFO] Waiting for changes...
```

**Troubleshooting Section**:
```markdown
## Troubleshooting

### Watcher Not Detecting Branch Switches

**Symptoms**: Branch switches don't trigger indexing

**Causes**:
1. Watcher not running
2. .git/HEAD not being watched (worktree configuration)
3. File watcher permissions

**Solutions**:
```bash
# Verify watcher is running
ps aux | grep "maproom watch"

# Check .git/HEAD exists
ls -la .git/HEAD

# Enable verbose logging
maproom watch --repo . --verbose
```

### High CPU Usage

**Symptoms**: maproom watch consuming >10% CPU

**Expected**: <5% CPU while idle

**Causes**:
1. Rapid branch switching (indexing in progress)
2. Large repository (many files to process)

**Solutions**:
- Wait for indexing to complete
- Check logs for errors causing retry loops
- Monitor with `top` or `htop`

### Database Connection Errors

**Symptoms**: "Failed to connect to database"

**Solution**:
```bash
# Verify DATABASE_URL is set
echo $DATABASE_URL

# Test database connection
psql $DATABASE_URL -c "SELECT 1"

# Set DATABASE_URL if missing
export DATABASE_URL="postgresql://user:pass@localhost/maproom"
```
```

## Dependencies
- BRWATCH implementation complete (Phases 1-3)
- docs/ directory structure exists

## Risk Assessment
- **Risk**: Documentation becomes outdated as code evolves
  - **Mitigation**: Update docs with code changes, add to PR checklist
- **Risk**: Examples don't match actual behavior
  - **Mitigation**: Test all examples manually before committing

## Files/Packages Affected
- `/workspace/docs/features/automatic-indexing.md` (new file)
