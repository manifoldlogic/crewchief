# Worktree Plugin

## Introduction

The Worktree plugin provides Git worktree management capabilities powered by the crewchief CLI. It enables Claude Code to create, manage, and merge parallel development branches safely without disrupting your main working directory. With the Worktree plugin, you can work on multiple features simultaneously in isolated environments, experiment safely, and maintain clean separation between different development tasks.

## Features

- **Parallel Development**: Work on multiple features or bug fixes simultaneously without branch switching
- **Isolated Environments**: Each worktree is a separate directory with its own checkout, preventing conflicts
- **Safe Merge**: Merge worktrees back to main with built-in safety checks and automatic cleanup
- **Automatic Cleanup**: Remove worktrees and their branches cleanly when work is complete
- **Branch Management**: Create, list, and manage git worktrees with simple commands
- **Status Tracking**: View all active worktrees and their current state

## Prerequisites

Before using the Worktree plugin, ensure you have:

1. **crewchief CLI installed**: The plugin requires the `crewchief` command-line tool to be available in your system PATH
2. **Git repository context**: You must be working within a git repository to use worktree functionality
3. **Clean working state**: Best practice is to have a clean working directory before creating worktrees

To verify your setup:
```bash
# Check CLI is installed
crewchief --version

# Verify you're in a git repository
git status

# List existing worktrees
crewchief worktree list
```

## Installation

Install the Worktree plugin using the Claude Code plugin command:

```
/plugin install worktree@crewchief
```

Once installed, the plugin will automatically be available for use in your Claude Code sessions.

## Usage Examples

### Feature Development Workflow
```
Create a new worktree for implementing user authentication
```
The plugin will create a new git worktree in an isolated directory where you can develop the authentication feature without affecting your main branch.

### Working on Multiple Features
```
I need to work on the login feature and the dashboard redesign simultaneously
```
Creates separate worktrees for each feature, allowing parallel development without branch switching.

### Experimenting Safely
```
Create a worktree to experiment with refactoring the database layer
```
Sets up an isolated environment where you can safely experiment without risk to your main codebase.

### Merging Completed Work
```
Merge the authentication worktree back to main
```
Safely merges the completed feature back to the main branch with automatic cleanup of the worktree.

### Cleaning Up
```
Remove the experimental-refactor worktree
```
Cleanly removes the worktree and optionally deletes the associated branch.

## Troubleshooting

### CLI Not Found
**Problem**: Plugin reports `crewchief: command not found`

**Solution**:
- Verify the CLI is installed: `which crewchief`
- Ensure it's in your PATH
- If using a development build, run `pnpm build` in the crewchief repository

### Not in Git Repository
**Problem**: Commands fail with "not a git repository" error

**Solution**:
- Navigate to a git repository directory
- Initialize a git repository if needed: `git init`
- Verify with `git status`

### Worktree Already Exists
**Problem**: Cannot create worktree because name or branch already exists

**Solution**:
- List existing worktrees: `crewchief worktree list`
- Choose a different name for the new worktree
- Remove the existing worktree if no longer needed: `crewchief worktree remove <name>`

### Cannot Remove Worktree
**Problem**: Worktree removal fails due to uncommitted changes

**Solution**:
- Navigate to the worktree directory
- Commit or stash your changes: `git commit` or `git stash`
- Alternatively, use force removal (warning: loses uncommitted changes)

### Merge Conflicts
**Problem**: Merging a worktree results in conflicts

**Solution**:
- The plugin will report conflicts and stop the merge
- Manually resolve conflicts in the worktree directory
- Complete the merge using standard git commands: `git add` and `git commit`
- Then retry the merge operation

### Branch Tracking Issues
**Problem**: Worktree branch isn't tracking correctly

**Solution**:
- Check branch status: `git branch -vv` in the worktree directory
- Set upstream if needed: `git branch -u origin/<branch>`
- Ensure you've pushed the branch to remote if collaboration is needed
