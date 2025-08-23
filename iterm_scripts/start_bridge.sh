#!/bin/bash
# Start the iTerm2 bridge server

# Check if we're running inside iTerm2 by checking the TERM_PROGRAM variable
if [[ "$TERM_PROGRAM" != "iTerm.app" ]]; then
    echo "❌ This script must be run from within iTerm2."
    echo "   Current terminal: ${TERM_PROGRAM:-unknown}"
    exit 1
fi

# Check Python version
if ! command -v python3 &> /dev/null; then
    echo "❌ Python 3 is not installed"
    exit 1
fi

# Get script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Check if requirements are installed
if ! python3 -c "import iterm2" 2>/dev/null; then
    echo "📦 Installing Python requirements..."
    pip3 install -r "$SCRIPT_DIR/requirements.txt"
fi

# Start the bridge
echo "🚀 Starting iTerm2 bridge server..."
echo "   Port: 8765"
echo "   Press Ctrl+C to stop"
echo ""

cd "$SCRIPT_DIR"
python3 iterm_bridge.py