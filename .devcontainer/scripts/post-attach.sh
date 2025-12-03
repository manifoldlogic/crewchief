#!/bin/bash
set -e

# Fix for Cursor IDE: Ensure we're in the workspace directory
if [ "$TERM_PROGRAM" = "Cursor" ] || [ -n "$CURSOR_IDE" ] || [ "$REMOTE_CONTAINERS_IPC" = "cursor" ]; then
    echo "🖱️ Detected Cursor IDE - ensuring workspace directory"
    cd /workspace 2>/dev/null || true
    export WORKSPACE_DIR=/workspace
fi

# Always ensure we start in workspace for consistency
cd /workspace 2>/dev/null || true

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

clear

echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║${NC}          ${GREEN}Welcome to CrewChief Development Container${NC}          ${BLUE}║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Show current working directory (helpful for debugging Cursor issues)
echo -e "${YELLOW}Current Directory:${NC} $(pwd)"
echo ""

echo -e "${YELLOW}Quick Commands:${NC}"
echo "  ${GREEN}ccdev${NC}      - Run CrewChief CLI in dev mode"
echo "  ${GREEN}crewchief${NC}  - Run CrewChief CLI (globally installed)"
echo "  ${GREEN}claude${NC}     - Run Claude Code in dangerous mode"
echo "  ${GREEN}pnpm test${NC}  - Run tests"
echo ""

echo ""
echo -e "${YELLOW}Git Status:${NC}"
BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")
DIRTY=""
if ! git diff --quiet 2>/dev/null; then
    DIRTY=" (modified)"
fi
echo -e "  Branch: ${GREEN}${BRANCH}${NC}${DIRTY}"

# Check for uncommitted changes
if [ -n "$(git status --porcelain 2>/dev/null)" ]; then
    CHANGES=$(git status --porcelain | wc -l)
    echo -e "  Changes: ${YELLOW}${CHANGES} files${NC}"
fi

echo ""
echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
echo ""
