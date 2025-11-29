---
description: Commit planning documents to the repository for a specific project.
argument-hint: [PROJECT_SLUG]
---

# Project Context

Project: $ARGUMENTS
Project folder: `.crewchief/projects/$ARGUMENTS_*/`
Planning documents: `.crewchief/projects/$ARGUMENTS_*/planning/`
Tickets: `.crewchief/projects/$ARGUMENTS_*/tickets/`

# Task

Commit planning documents to the repository for the $ARGUMENTS project. If "$ARGUMENTS" is not provided, commit the most recent project that is not already committed.

Use a commit messages similar to the following, as appropriate:

- `docs(.crewchief): $ARGUMENTS planning documents`
- `docs(.crewchief): $ARGUMENTS planning documents and tickets`
- `docs(.crewchief): $ARGUMENTS tickets`
- `docs(.crewchief): $ARGUMENTS planning document updates`
- `docs(.crewchief): $ARGUMENTS ticket updates`
- etc.