# crewchief docs — interactive documentation hub

The repository's documentation site. **Gunmetal** is the first product tab:
an interactive catalog where every capability of the `gunmetal` crate is
demonstrated as the archetypal web pattern it exists to enable — and every
demo doubles as an end-to-end test running real wasm clients (and, for the
flagship, real GUN.js) against the real `gunmetal-relay` binary.

Spec and plan: `_SPECS/crewchief/spec/gunmetal-web-catalog.md`.

## Stack

SvelteKit 2 (Svelte 5, runes) · Tailwind CSS v4 · shadcn-svelte ·
mode-watcher (dark mode) · mdsvex · adapter-static (full prerender) ·
Bun (package manager, runtime, and the production static server) ·
Playwright (e2e).

This package is **bun-managed** (`bun.lock`) and intentionally outside the
repo's pnpm workspace.

## Prerequisites

- **Bun** ≥ 1.3
- **Rust** with the `wasm32-unknown-unknown` target
- **wasm-bindgen-cli matching the workspace lockfile** — currently
  `0.2.108`. The build script enforces the version match and fails with
  install instructions if it drifts:
  `cargo install wasm-bindgen-cli --version 0.2.108`
- The `gun` submodule checked out (`git submodule update --init`) for the
  gunjs-interop demo (the build warns and the demo degrades without it)

## Scripts

| Script | What it does |
|--------|--------------|
| `bun run dev` | Vite dev server (run `bun run wasm` once first) |
| `bun run wasm` | Build the gunmetal wasm bundle into `src/lib/wasm/` (size-budgeted at 3 MB; ~460 KB actual) and vendor `gun.js` into `static/vendor/` |
| `bun run build` | Production build into `build/` (full prerender; the build itself is the dead-link check) |
| `bun run serve` | `Bun.serve` static server for `build/` (port 4173, `--port`/`PORT` to change) |
| `bun run relay:build` | Build the `gunmetal-relay` binary the demos sync through |
| `bun run test:e2e` | The whole pipeline: wasm → site build → relay build → Playwright (site on 4173, relay on 8766 with fresh radata) |

To browse the demos locally: `bun run wasm && bun run build && bun run serve`
plus a relay in another terminal:
`cargo run -p gunmetal --features relay --bin gunmetal-relay -- --port 8765`.
Every demo shows a degraded state with run instructions when the relay is
unreachable.

## Architecture

- `src/lib/catalog.ts` — **the manifest, single source of truth**:
  capabilities, demos, learn chapters, reference modules. The sidebar,
  grids, cross-link triangle (learn↔demo↔reference), prerender entries,
  and the link-integrity test all derive from it. Add a demo here first.
- `src/lib/demos/<slug>/` — client components booted by the bare
  `/gunmetal/demos/[slug]/client` pages inside demo-page iframes. Each
  iframe is an isolated JS realm = a genuine separate session.
  `registry.ts` maps slugs to components; `implemented.ts` gates the live
  stage; `content.ts` holds per-demo why/snippets/gotchas.
- `src/lib/gun/client.ts` — wasm boot + the **readiness contract**:
  `body[data-ready="true"]` and `window.__gmReady` resolve only after
  wasm init + relay handshake (peerPid) + subscriptions. Every Playwright
  spec waits on it; never interact with a frame before it.
- `src/lib/learn/` — chapter content components keyed by slug.
- `src/lib/reference.ts` — per-item API contracts for reference pages.
- `server.ts` — the production Bun server (exact → `.html` →
  `index.html`, query-stripped, traversal-safe, immutable caching for
  `/_app/immutable`).
- `e2e/` — Playwright specs. **Workers are pinned to 1** deliberately:
  pages boot multiple wasm instances and the SEA demos run
  deliberately-slow PBKDF2; parallel workers starve each other's
  readiness timeouts on shared-CPU runners. The full serial suite runs in
  about a minute.

## Deployment posture (v1)

Local + CI only. The site is fully static, but every demo needs a live
relay; public hosting would need a persistent WSS relay and a static host
— nothing in the design precludes it (relay URLs are injectable via
`?relay=`), it's just out of scope for v1.
