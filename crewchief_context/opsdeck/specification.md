
# **CrewChief — Ops Deck Spec & Repo Layout (Rust)**

## **Naming rules (authoritative)**

- **Feature name:** Ops Deck (UI grid) and Roster View (list).
- **Crate name:** `opsdeck` (Rust).
- **Binary name:** `crewchief-opsdeck`.
- **Socket/log files:** `opsdeck.sock`, `opsdeck.log`, `opsdeck.events.jsonl`.
- **CLI flags/modes:** `--mode opsdeck`, `--mode roster`.
- **Docs:** “Ops Deck” title-case in prose; opsdeck for code/paths.

---

## **Repository layout (single repo, clean boundaries)**

```
crewchief/
├─ packages/
│  ├─ cli/                         # Node/TS crewchief CLI
│  └─ schemas/                     # JSON Schema & TS types for heartbeats
├─ crates/
│  └─ opsdeck/                     # Rust: Aggregator + Dashboard Painter (Ops Deck)
│     ├─ Cargo.toml
│     └─ src/
│        ├─ agg/                   # aggregator modules
│        ├─ paint/                 # painter/renderers
│        ├─ ingest/                # UDS, file-tail adapters
│        ├─ model/                 # data model & derived metrics
│        ├─ ui/                    # input handling (filters/sorts)
│        └─ main.rs
├─ crewchief_context/
│  └─ opsdeck/
│     ├─ heartbeat.md              # JSONL contract (single source of truth)
│     └─ specification.md          # this spec
├─ scripts/
│  ├─ build-opsdeck.sh             # cross-compile helpers
│  └─ release.sh
├─ fixtures/
│  ├─ sample-heartbeats.jsonl      # for replay demos
│  └─ burst-100-agents.jsonl
├─ .github/workflows/
│  ├─ opsdeck-release.yml          # matrix build for macOS/Linux
│  └─ cli-release.yml
├─ README.md
└─ LICENSE
```

> If you previously had crates/deck/, rename it to crates/opsdeck/ and update Cargo workspace members accordingly.

---

## **Binaries & subcommands**

**Binary:** `crewchief-opsdeck`

**Subcommands:**

- `agg` — aggregator (ingest/normalize/state)
- `paint` — painter (render Ops Deck/Roster)
- `ui` — convenience: spawns both in one process

### **Examples**

```
crewchief-opsdeck ui --mode opsdeck --fps 2
crewchief-opsdeck ui --mode roster --fps 2
crewchief-opsdeck agg --uds "$RUN_DIR/crewchief/opsdeck.sock"
crewchief-opsdeck paint --mode opsdeck --fields id,state,q,tok_s,lat,err,age
```

---

## **CLI flags (canonical)**

**Shared**

- `--uds PATH` (default: $RUN_DIR/crewchief/opsdeck.sock)
- `--hb-dir DIR` (fallback file-tail directory)
- `--fps N` (1–5 recommended, hard cap 10)
- `--fields CSV` (e.g., id,state,q,tok_s,lat,err,age)
- `--sort KEYS` (e.g., health,-tok_s)
- `--filter EXPR` (e.g., tags~tournament)
- `--width --height` (override detected TTY)
- `--no-color`, `--debug`

**Aggregator-only**

- `--stall DURATION` (default 6s)
- `--retire-after DURATION` (default 10m)
- `--snapshot FILE` (writes merged state JSON)
- `--events FILE` (write replay log opsdeck.events.jsonl)

**Painter-only**

- `--mode opsdeck|roster`
- `--tty /dev/ttys012` (optional direct TTY)
- interactive keys: `/` search, `s` sort, `f` filter, `ENTER` details

---

## **Runtime paths**

- UDS socket: `$RUN_DIR/crewchief/opsdeck.sock`
- Aggregator log: `$RUN_DIR/crewchief/opsdeck.log`
- Snapshot JSON: `$RUN_DIR/crewchief/snapshot.json`
- Event log (optional): `$RUN_DIR/crewchief/opsdeck.events.jsonl`
- Agent heartbeats (fallback): `$RUN_DIR/crewchief/agents/*/heartbeat.jsonl`

---

## **Heartbeat contract (unchanged, reaffirmed)**

- **Format:** JSONL; one object per line; ~0.5–2 Hz per agent.
- **Required:** `agent_id`, `ts`, `state`
- **Enums:** `state ∈ {IDLE, RUNNING, WAITING, ERROR, DONE}`

Minimal example:

```
{"agent_id":"a-42","ts":"2025-08-10T20:31:22.815Z","state":"RUNNING"}
```

Full example (friendly with Ops Deck):

```
{
  "agent_id": "a-42",
  "agent_type": "claude-code",
  "session_id": "sess-2025-08-10-1430-utc",
  "ts": "2025-08-10T20:31:22.815Z",
  "state": "RUNNING",
  "phase": "retrieval",
  "last_action": "embed:kb://design/plan.md",
  "queue_len": 3,
  "tokens_s": 27.5,
  "lat_ms": 1800,
  "errors": 0,
  "score": 0.71,
  "branch": "opsdeck/proto-a",
  "pane_id": ":%3.1",
  "cpu_pct": 3.1,
  "mem_mb": 212,
  "custom": { "task": "PR-1283", "tags": ["tournament","sr"] }
}
```

---

## **Aggregator behavior (no surprises)**

- **Ingest:** `UDS` (preferred), `file-tail` (fallback).
- **Coalesce:** keep newest per-agent update per paint tick.
- **State:** `HashMap<AgentId, AgentState>` + ring buffer (N=50) per agent.
- **Derived:** `agent_health` (`OK`/`DEGRADED`/`STALLED`/`ERROR`), throughput, error rate, latency p50/p95.
- **Stall:** mark `STALLED` if no heartbeat > `--stall` and not `DONE`/`ERROR`.
- **Retention:** retire after `--retire-after` unless `--keep-all`.
- **Persistence:** snapshot JSON (overwrite), optional events log.

---

## **Painter behavior (Ops Deck + Roster)**

- **Targets:** `stdout` (default) or `--tty`.
- **Layouts:**
  - **Ops Deck:** fixed-width grid, optimized for dozens of agents in 120×30.
  - **Roster:** sortable list for detail-first workflows.
- **Fields:** configurable; sensible defaults: `id`,`state`,`q`,`tok_s`,`lat`,`err`,`age`,`branch`.
- **Colors:** minimal ANSI; `ERROR` red, `STALLED` yellow, `DONE` green, `WAITING` dim.
- **Diff rendering:** cache last frame; repaint only on change; one buffer write per frame; hide cursor while painting.
- **Rate:** default 2 Hz; generally 1–4 Hz.

---

## **Tmux integration**

- Typical: open a pane and run `crewchief-opsdeck ui --mode opsdeck`.
- Or have the `crewchief` CLI spawn a hidden window per agent and one visible Ops Deck pane.
- **Do not** tail per-agent logs inside panes; agents emit heartbeats to UDS and the aggregator reads them centrally.

---

## **Performance budgets (target)**

- **Ingest:** ≤ 100 agents × 2 Hz × 500 B ≈ 100 KB/s; handle 10× bursts.
- **CPU:** Painter ≤ 3–5% at 2 Hz on a normal dev laptop.
- **Mem:** ≤ 64 MB including histories and buffers.

---

## **Security & permissions**

- UDS perms 0600 by default (same user).
- No TCP unless explicitly `--tcp`.
- Sanitize strings before painting (strip control chars).

---

## **CI/CD & release**

**Workflows**

- `.github/workflows/opsdeck-release.yml`:
  - Build `crewchief-opsdeck` for: `macOS` `arm64`/`x86_64`; Linux `x86_64`/`aarch64` (consider musl).
  - Upload artifacts to GitHub Releases.
- `.github/workflows/cli-release.yml` unchanged.

**Install UX**

- CLI checks for crewchief-opsdeck on PATH when user runs crewchief opsdeck. If missing:
  - Offer to download the latest release (prompt) or print brew/curl instructions.
- Optional Homebrew tap later: brew install crewchief-opsdeck (future).

**Versioning**

- Tag repo `vX.Y.Z`.
- Crate uses semver; CLI declares min-compatible opsdeck version (e.g., >=0.3.0); warn if older.

---

## **Environment variables**

- `CREWCHIEF_RUN_DIR` (overrides `$RUN_DIR`)
- `CREWCHIEF_OPSDECK_FPS`
- `CREWCHIEF_OPSDECK_FIELDS`
- `NO_COLOR` respected
- `LC_ALL` for Unicode width handling

---

## **Extensibility**

- custom fields preserved and renderable via `--fields` custom.foo.
- Input adapters implement Ingestor trait (e.g., future `gRPC`/`MQTT`).
- Renderers implement Renderer trait (future layouts if needed).

---

## **Testing strategy**

- **Unit:** validation, stall logic, health calculation, diff renderer.
- **Property/fuzz:** randomized sequences; assert no panics, bounded memory.
- **Perf:** 200 agents × 5 Hz × 10 min under CPU/mem caps.
- **TTY:** capture/replay frames (script/ttyrec) for stable output.

---

## **README copy snippets (to keep things consistent)**

- **Feature blurb:**
        _Ops Deck — Monitor dozens of agents at a glance in a tmux pane. Sort, filter, and jump into any session in real time—without crossing your fingers._
- **Commands:**
  - crewchief opsdeck → runs the **Ops Deck** dashboard (requires crewchief-opsdeck).
  - crewchief opsdeck --mode roster → list-first monitoring.
- **Install hint:**
        _“Ops Deck requires the companion binary_ _`crewchief-opsdeck`. Install via GitHub Releases or your package manager.”_
