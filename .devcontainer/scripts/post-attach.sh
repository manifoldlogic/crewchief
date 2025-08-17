#!/bin/bash
set -e

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

# Show service status
echo -e "${YELLOW}Service Status:${NC}"
if pg_isready -h postgres -p 5432 -U postgres &>/dev/null; then
    echo -e "  PostgreSQL: ${GREEN}✓ Running${NC}"
else
    echo -e "  PostgreSQL: ❌ Not running"
fi

if redis-cli -h redis ping &>/dev/null; then
    echo -e "  Redis:      ${GREEN}✓ Running${NC}"
else
    echo -e "  Redis:      ❌ Not running"
fi

if [ -f "/usr/local/bin/crewchief-maproom" ]; then
    echo -e "  Maproom:    ${GREEN}✓ Installed${NC}"
else
    echo -e "  Maproom:    ⚠️  Not installed"
fi

echo ""
echo -e "${YELLOW}Quick Commands:${NC}"
echo "  ${GREEN}webui${NC}      - Start Web UI development server"
echo "  ${GREEN}ccdev${NC}      - Run CrewChief CLI in dev mode"
echo "  ${GREEN}maproom${NC}    - Run Maproom commands"
echo "  ${GREEN}pnpm test${NC}  - Run tests"
echo ""

echo -e "${YELLOW}tmux Sessions:${NC}"
if tmux has-session -t crewchief 2>/dev/null; then
    echo -e "  ${GREEN}crewchief${NC} session available"
    echo "  Run: ${GREEN}tmux attach -t crewchief${NC} to attach"
else
    echo "  No tmux sessions running"
    echo "  Run: ${GREEN}tn crewchief${NC} to create one"
fi

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

# If no tmux session exists, offer to create one
if ! tmux has-session -t crewchief 2>/dev/null; then
    echo -e "${YELLOW}Tip:${NC} Start a tmux session for better workflow:"
    echo "  ${GREEN}tn crewchief${NC}"
    echo ""
fi