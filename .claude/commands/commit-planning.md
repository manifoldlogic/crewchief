---
description: Commit planning documents to the repository for a specific project.
argument-hint: [PROJECT_SLUG]
---

# Project Context

Project: $ARGUMENTS
Project folder: `.agents/projects/$ARGUMENTS_*/`
Planning documents: `.agents/projects/$ARGUMENTS_*/planning/`
Tickets: `.agents/projects/$ARGUMENTS_*/tickets/`

# Task

Commit planning documents to the repository for the $ARGUMENTS project. If "$ARGUMENTS" is not provided, commit the most recent project that is not already committed.

Use a commit messages similar to the following, as appropriate:

- `docs(.agents): $ARGUMENTS planning documents`
- `docs(.agents): $ARGUMENTS planning documents and tickets`
- `docs(.agents): $ARGUMENTS tickets`
- `docs(.agents): $ARGUMENTS planning document updates`
- `docs(.agents): $ARGUMENTS ticket updates`
- etc.