# RSTFIX: Rust Build Cleanup

## Status: Planning Complete

## Problem

The `crewchief-maproom` Rust crate produces 67 compiler warnings and has 1 failing test, reducing code quality and developer productivity.

## Solution

Systematic removal of unused imports, dead code, and fix for the failing config validation test. No functional changes - purely cleanup.

## Scope

- 67 Rust warnings across ~20 files
- 1 failing test in config/hot_reload.rs
- C vendor warnings (sqlite-vec) are out of scope

## Agents

- **rust-indexer-engineer**: Main implementation agent
- **unit-test-runner**: Verification

## Planning Documents

- [Analysis](planning/analysis.md) - Problem breakdown and affected files
- [Architecture](planning/architecture.md) - Cleanup approach and decision tree
- [Quality Strategy](planning/quality-strategy.md) - Verification commands and acceptance criteria
- [Security Review](planning/security-review.md) - No security concerns
- [Plan](planning/plan.md) - 5 phases, 10 tickets

## Tickets

See `tickets/RSTFIX_TICKET_INDEX.md` for ticket list.
