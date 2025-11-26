# /docs - Permanent Codebase Documentation

Long-term documentation for the CrewChief project, read by **both agents and humans**.

## Purpose

This directory contains finalized, authoritative documentation about the codebase. Content here should be stable, accurate, and useful for anyone working with or understanding the project.

## What Belongs Here

- Architecture documentation (`architecture/`)
- How-to guides (`guides/`)
- API references
- Technical specifications
- Configuration documentation (`configuration/`)
- Performance analysis and optimization guides (`optimization/`, `performance/`)
- Feature documentation (`features/`)
- Testing strategies (`testing/`)

## What Does NOT Belong Here

- Work-in-progress research (goes in `.agents/knowledge/`)
- Project planning documents (go in `.agents/projects/`)
- Active tickets or execution tracking (go in `.agents/`)
- Temporary reports or analysis (go in `.agents/reports/`)

## Key Rule

> Agents document active work in `.agents/`, finalized knowledge goes in `docs/`.

When work completes in `.agents/`, synthesize useful knowledge here for long-term reference.

## Maintenance

- Keep documentation accurate and up-to-date with code changes
- Remove obsolete documentation when features are removed
- Prefer updating existing docs over creating new files
