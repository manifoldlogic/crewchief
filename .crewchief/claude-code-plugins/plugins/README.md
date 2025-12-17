# CrewChief Plugins

Overview of available plugins in the crewchief marketplace.

## Installation

Install plugins individually:

```bash
/plugin install maproom@crewchief
/plugin install worktree@crewchief
```

## Available Plugins

### Maproom

**Version:** 0.1.0

Semantic code search using the crewchief-maproom CLI.

**Features:**
- Full-Text Search (FTS) for keyword-based search
- Vector search for semantic similarity
- Context expansion (callers, callees, tests)
- Multi-repository support

**Skill:** `maproom-search`

**[Read More](maproom/README.md)**

### Worktree

**Version:** 0.1.0

Git worktree management using the crewchief CLI.

**Features:**
- Parallel development environments
- Safe worktree lifecycle management
- Merge strategies (ff, squash, cherry-pick)
- Copy ignored files to worktrees

**Skill:** `worktree-management`

**[Read More](worktree/README.md)**
