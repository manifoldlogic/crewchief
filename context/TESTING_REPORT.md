# CrewChief Testing Report

## Features to Test

### 1. Agent Spawning in Tmux
**Status**: Implementation exists but needs testing
**Test Command**: `crewchief agent spawn mock-agent "test task"`
**Questions**:
- Does the mock-agent spawn successfully in a tmux pane?
- Does the agent receive and log messages properly?
- Does the message bus work between agents?

### 2. Competition Mode
**Status**: Commands exist but need verification
**Test Commands**:
```bash
crewchief competition start "test competition" mock-agent-1 mock-agent-2
crewchief competition assign <competition-id>
crewchief competition evaluate <competition-id>
```
**Questions**:
- Do competitions create multiple worktrees properly?
- Does evaluation work with actual scoring?
- Does the auto-merge threshold work?

### 3. Main Entry Point
**Status**: Unclear if `crewchief` without arguments starts session
**Test Command**: `crewchief`
**Expected**: Should start tmux session and launch default agents
**Questions**:
- Does it auto-run setup wizard on first use?
- Does it launch default agents from config?
- Does it attach to existing session if one exists?

### 4. Task Assignment
**Status**: Command exists, implementation unclear
**Test Command**: `crewchief task assign mock-agent "implement feature X"`
**Questions**:
- Does task assignment create proper worktrees?
- Are tasks tracked in the run manager?

### 5. Evaluation and Auto-Merge
**Status**: Commands exist, scoring implementation unclear
**Test Commands**:
```bash
crewchief eval run <run-id>
crewchief merge auto <run-id>
```
**Questions**:
- What evaluation metrics are actually implemented?
- Does auto-merge check tests/linting?
- What is the actual threshold for auto-merge?

### 6. Cross-Agent Input Injection
**Status**: Not found in implementation
**Expected Command**: `crewchief agent inject <from> <to> "<keys>"`
**Current State**: Likely NOT IMPLEMENTED

### 7. Realm & Semantic Retrieval
**Status**: Not found in implementation
**Expected Commands**: `crewchief realm build`, `crewchief realm query`
**Current State**: Likely NOT IMPLEMENTED

### 8. Benchmarking & Tournaments
**Status**: Not found in implementation
**Expected Command**: `crewchief eval benchmark <scenario>`
**Current State**: Likely NOT IMPLEMENTED

## Working Features (Verified)

1. **Worktree Management** ✅
   - `crewchief worktree create`
   - `crewchief worktree list`
   - `crewchief worktree clean`
   - `crewchief worktree cd`

2. **Maproom Integration** ✅
   - `crewchief maproom:db`
   - `crewchief maproom:scan`
   - `crewchief maproom:search`
   - `crewchief maproom:upsert`
   - `crewchief maproom:watch`

3. **Basic Agent Commands** ✅
   - `crewchief agent spawn`
   - `crewchief agent message`
   - `crewchief agent close`

4. **Run Management** ✅
   - `crewchief runs list`
   - `crewchief runs events`
   - `crewchief runs logs`

5. **Setup & Init** ✅
   - `crewchief init`
   - `crewchief setup`

## Recommended Tests

Please run the following commands and report the results:

1. `crewchief --help` - Check available commands
2. `crewchief` - See if it starts a tmux session
3. `crewchief agent spawn mock-agent "test"` - Test agent spawning
4. `crewchief competition start "test" mock-agent` - Test competition
5. `crewchief doctor` - Check system dependencies

Report back on which features work, which fail, and any error messages.