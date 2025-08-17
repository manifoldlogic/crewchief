# Wireframes and UI Mockups

## Dashboard Layout

```
┌─────────────────────────────────────────────────────────────────┐
│ ▤ CrewChief    [🔍 Search...]            [@user] [🔔] [⚙️]     │
├─────────────┬───────────────────────────────────────────────────┤
│             │                                                    │
│ ◉ Dashboard │  ┌──────────────┐ ┌──────────────┐ ┌────────────┐│
│ ○ Search    │  │ Active       │ │ Indexed      │ │ Branches   ││
│ ○ Worktrees │  │ Agents: 3    │ │ Files: 1.2k  │ │ Active: 12 ││
│ ○ Agents    │  │ ▂▄▆█▅▃▁     │ │ ████████ 98% │ │ ●●●○○○○○○  ││
│ ○ Branches  │  └──────────────┘ └──────────────┘ └────────────┘│
│ ○ Settings  │                                                    │
│             │  Recent Activity                                   │
│ Quick Actions│  ┌──────────────────────────────────────────────┐│
│ [+ Worktree]│  │ 09:42 Agent-1 completed indexing task         ││
│ [+ Agent]   │  │ 09:38 Worktree 'feature-x' created           ││
│ [⟲ Index]   │  │ 09:35 Search: "authentication flow"          ││
│             │  │ 09:31 Agent-2 spawned for code review        ││
│             │  └──────────────────────────────────────────────┘│
│             │                                                    │
│             │  Active Agents                                     │
│             │  ┌─────────┐ ┌─────────┐ ┌─────────┐            │
│             │  │ Claude  │ │ Gemini  │ │ Mock    │            │
│             │  │ ● Working│ │ ○ Idle  │ │ ● Working│            │
│             │  │ Task #42 │ │ Ready   │ │ Testing │            │
│             │  └─────────┘ └─────────┘ └─────────┘            │
└─────────────┴───────────────────────────────────────────────────┘
│ ● Connected │ PostgreSQL: OK │ Tmux: Active │ CPU: 24% │ v1.0.0 │
└─────────────────────────────────────────────────────────────────┘
```

## Search Interface

```
┌─────────────────────────────────────────────────────────────────┐
│ ▤ CrewChief > Search                                [←] [@] [?] │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  [🔍 Enter semantic search query...                        ] [⚙]│
│                                                                  │
│  Filters: [All Worktrees ▼] [All Types ▼] [Any Time ▼] [+ More]│
│                                                                  │
├──────────────────────────┬──────────────────────────────────────┤
│ Results (42 matches)     │ Preview                               │
│                          │                                       │
│ ▶ auth/login.ts:45       │ 45  export async function login(     │
│   login() function       │ 46    username: string,              │
│   95% relevance          │ 47    password: string                │
│   feature-auth worktree  │ 48  ): Promise<AuthResult> {         │
│                          │ 49    const hashedPassword = await   │
│ ▷ auth/validate.ts:12    │ 50      bcrypt.hash(password, 10);   │
│   validateToken()        │ 51    // Check credentials            │
│   89% relevance          │ 52    const user = await db.users    │
│   main worktree          │ 53      .findOne({ username });       │
│                          │ 54    if (!user || !await bcrypt     │
│ ▷ middleware/auth.ts:78  │ 55      .compare(password, user.pwd)) │
│   requireAuth()          │ 56      throw new AuthError();        │
│   82% relevance          │ 57    }                               │
│   feature-api worktree   │                                       │
│                          │ [Open in Editor] [Copy] [Share]      │
└──────────────────────────┴──────────────────────────────────────┘
│ Search took 87ms │ Showing 1-20 of 42 │ [1] 2 3 → │            │
└─────────────────────────────────────────────────────────────────┘
```

## Worktree Management

```
┌─────────────────────────────────────────────────────────────────┐
│ ▤ CrewChief > Worktrees                           [+ New] [⟲]   │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│ Active Worktrees (5)                      [List ▼] [Clean Stale]│
│                                                                  │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │ ● main                                          [Actions ▼]│ │
│ │   ~/crewchief                                               │ │
│ │   ✓ Clean  ↑0 ↓0  Last commit: 2h ago                     │ │
│ ├────────────────────────────────────────────────────────────┤ │
│ │ ● feature-web-ui                                [Actions ▼]│ │
│ │   ~/crewchief/.crewchief/worktrees/feature-web-ui          │ │
│ │   ⚠ 3 files modified  ↑2 ↓0  Last commit: 10m ago         │ │
│ │   👤 Agent-1 (active)  👤 Agent-2 (idle)                   │ │
│ ├────────────────────────────────────────────────────────────┤ │
│ │ ○ bugfix-auth                                   [Actions ▼]│ │
│ │   ~/crewchief/.crewchief/worktrees/bugfix-auth             │ │
│ │   ✓ Clean  ↑1 ↓3  Last commit: 1d ago                     │ │
│ └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│ Stale Worktrees (2)                               [Clean All]   │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │ ⚪ old-feature-123  (30 days old)                  [Delete]│ │
│ │ ⚪ experiment-456   (45 days old)                  [Delete]│ │
│ └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Agent Control Panel

```
┌─────────────────────────────────────────────────────────────────┐
│ ▤ CrewChief > Agents                         [+ Spawn] [⚔ Battle]│
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│ ┌─────────────┬─────────────┬─────────────┬─────────────┐      │
│ │ Claude-1    │ Gemini-1    │ Mock-1      │ + New Agent │      │
│ │ ● Working   │ ○ Idle      │ ● Working   │             │      │
│ ├─────────────┼─────────────┼─────────────┼─────────────┤      │
│ │ CPU: 45%    │ CPU: 2%     │ CPU: 12%    │             │      │
│ │ MEM: 512MB  │ MEM: 128MB  │ MEM: 64MB   │   [+]       │      │
│ │             │             │             │             │      │
│ │ Task:       │ Awaiting    │ Running     │  Add Agent  │      │
│ │ Code Review │ Task        │ Tests       │             │      │
│ │             │             │             │             │      │
│ │ [Message]   │ [Message]   │ [Message]   │             │      │
│ │ [Logs]      │ [Logs]      │ [Logs]      │             │      │
│ │ [Close]     │ [Close]     │ [Close]     │             │      │
│ └─────────────┴─────────────┴─────────────┴─────────────┘      │
│                                                                  │
│ Message Bus                                    [Clear] [Export] │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │ 09:45:23 Claude-1 → Orchestrator: Task completed          │ │
│ │ 09:45:20 Orchestrator → Claude-1: Acknowledged            │ │
│ │ 09:45:15 Claude-1 → Orchestrator: Found 3 issues         │ │
│ │ 09:44:50 Mock-1 → Orchestrator: Tests passing            │ │
│ │ 09:44:45 Orchestrator → All: Status check                │ │
│ └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Branch Visualizer

```
┌─────────────────────────────────────────────────────────────────┐
│ ▤ CrewChief > Branches                      [Fetch] [Pull] [Push]│
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   main ────●────●────●────●────●──┐                            │
│             ↓                       │                            │
│   feature-web-ui ──●────●────●     │                            │
│                     ↓               ↓                            │
│   feature-auth ────●────●────●─────●──→ PR #123                │
│                                     ↑                            │
│   bugfix-login ────●────●──────────┘                           │
│                                                                  │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │ Selected: feature-auth                                      │ │
│ │ ↑ 2 ahead  ↓ 1 behind main                                 │ │
│ │ Last commit: "Fix authentication flow" by @user (2h ago)   │ │
│ │                                                             │ │
│ │ [Create PR] [Merge] [Rebase] [Cherry-pick] [Delete]       │ │
│ └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│ Pull Requests (3)                                 [Create New]  │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │ #125 ✓ Add web UI [feature-web-ui → main]  ● 3 approvals  │ │
│ │ #124 ⚠ Fix auth bug [bugfix-auth → main]   ○ Changes req  │ │
│ │ #123 ● Update API [feature-api → main]     ● In review    │ │
│ └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Settings Editor

```
┌─────────────────────────────────────────────────────────────────┐
│ ▤ CrewChief > Settings                    [Save] [Reset] [Export]│
├───────────────┬──────────────────────────────────────────────────┤
│ General       │  crewchief.config.js                             │
│ › Repository  │  ┌──────────────────────────────────────────────┐│
│ › Orchestrator│  │ module.exports = {                          1││
│ › Agents      │  │   repository: {                              2││
│ › Tmux        │  │     mainBranch: 'main',                      3││
│ ▼ Worktree    │  │     worktreeBasePath: '.crewchief/worktrees'4││
│   Copy Files  │  │   },                                         5││
│   Overwrite   │  │   worktree: {                                6││
│ › Evaluation  │  │     copyIgnoredFiles: [                      7││
│               │  │       '.env',                                8││
│ Environment   │  │       '.env.local',                          9││
│ Database      │  │       '.cursorrules'                        10││
│ Security      │  │     ],                                      11││
│ About         │  │     overwriteStrategy: 'skip'               12││
│               │  │   }                                          13││
│               │  │ }                                            14││
│               │  └──────────────────────────────────────────────┘│
│               │  ⓘ Changes will take effect immediately          │
│               │  ✓ Configuration valid                           │
└───────────────┴──────────────────────────────────────────────────┘
```

## Modal: Create Worktree

```
┌─────────────────────────────────────────────────────────────────┐
│ Create New Worktree                                        [✕] │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Worktree Name *                                               │
│  [feature-awesome-thing                                    ]   │
│                                                                  │
│  Base Branch *                                                 │
│  [main                                                    ▼]   │
│                                                                  │
│  ☑ Create new branch from base                                 │
│  ☑ Copy .env files from main worktree                         │
│  ☐ Spawn agent in this worktree                               │
│                                                                  │
│  Advanced Options                                         [▼]   │
│                                                                  │
│                                      [Cancel] [Create Worktree] │
└─────────────────────────────────────────────────────────────────┘
```

## Log Viewer

```
┌─────────────────────────────────────────────────────────────────┐
│ Run #1234 - Logs                    [Download] [Share] [✕]     │
├─────────────────────────────────────────────────────────────────┤
│ [Filter...] Level: [All ▼] [Follow] [Wrap] [Clear]             │
├─────────────────────────────────────────────────────────────────┤
│ 2024-01-15 09:45:23.456 INFO  Starting task execution          │
│ 2024-01-15 09:45:23.789 DEBUG Initializing agent Claude-1      │
│ 2024-01-15 09:45:24.123 INFO  Worktree: feature-web-ui         │
│ 2024-01-15 09:45:24.456 INFO  Running: npm install             │
│ 2024-01-15 09:45:28.789 DEBUG npm output: added 234 packages   │
│ 2024-01-15 09:45:29.123 INFO  Running: npm run build           │
│ 2024-01-15 09:45:35.456 INFO  Build successful                 │
│ 2024-01-15 09:45:35.789 WARN  2 TypeScript warnings found      │
│ 2024-01-15 09:45:36.123 INFO  Running tests...                 │
│ 2024-01-15 09:45:42.456 ERROR Test failed: auth.test.ts        │
│ 2024-01-15 09:45:42.789 ERROR   Expected 200, got 401          │
│ 2024-01-15 09:45:43.123 INFO  Attempting to fix...             │
│ 2024-01-15 09:45:45.456 INFO  Fix applied, retrying tests      │
│ 2024-01-15 09:45:51.789 INFO  All tests passing ✓              │
│ 2024-01-15 09:45:52.123 INFO  Task completed successfully      │
│▓                                                                │
└─────────────────────────────────────────────────────────────────┘
```

## Mobile Responsive View

```
┌─────────────────┐
│ ☰ CrewChief  🔍 │
├─────────────────┤
│ Active Agents: 3│
│ ████████░░ 80%  │
│                 │
│ Recent Activity │
│ ┌─────────────┐ │
│ │09:42 Index  │ │
│ │09:38 Create │ │
│ │09:35 Search │ │
│ └─────────────┘ │
│                 │
│ Quick Actions   │
│ [+ Worktree]    │
│ [+ Agent]       │
│ [⟲ Index]       │
│                 │
├─────────────────┤
│ [🏠][🔍][🌳][🤖]│
└─────────────────┘
```

## Component States

### Loading States
```
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│ ░░░░░░░░░░░  │  │ ⟳ Loading... │  │ ▓▓▓▓▓▓░░░░░ │
│ ░░░░░░░░░░░  │  │              │  │    60%       │
│ ░░░░░░░░░░░  │  │              │  │              │
└──────────────┘  └──────────────┘  └──────────────┘
  Skeleton          Spinner           Progress Bar
```

### Empty States
```
┌────────────────────────┐
│                        │
│      📭                │
│   No results found     │
│                        │
│ Try adjusting filters  │
│   or [Create New]      │
│                        │
└────────────────────────┘
```

### Error States
```
┌────────────────────────┐
│ ⚠️ Connection Error    │
│                        │
│ Unable to connect to   │
│ database.              │
│                        │
│ [Retry] [Details]      │
└────────────────────────┘
```