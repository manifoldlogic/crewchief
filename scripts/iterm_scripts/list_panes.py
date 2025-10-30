#!/usr/bin/env python3
"""
List all iTerm2 panes with their labels, IDs, and locations.

Usage:
    python3 list_panes.py           # List all panes
    python3 list_panes.py --json    # Output as JSON
"""

import iterm2
import asyncio
import json
import argparse


async def main(connection):
    """List all panes with their information."""
    parser = argparse.ArgumentParser(description='List all iTerm2 panes')
    parser.add_argument('--json', action='store_true', help='Output as JSON')
    args = parser.parse_args()
    
    app = await iterm2.async_get_app(connection)
    
    panes = []
    index = 1
    
    # Iterate through all windows, tabs, and sessions
    for window_index, window in enumerate(app.terminal_windows):
        for tab_index, tab in enumerate(window.tabs):
            for session in tab.sessions:
                # Get the label if set
                try:
                    label = await session.async_get_variable("user.pane_label")
                except:
                    label = None
                
                # Check if this is the current session
                is_current = (
                    app.current_terminal_window == window and
                    app.current_terminal_window.current_tab == tab and
                    tab.current_session == session
                )
                
                pane_info = {
                    "index": index,
                    "label": label or "",
                    "session_id": session.session_id,
                    "window": window_index + 1,
                    "tab": tab_index + 1,
                    "current": is_current
                }
                panes.append(pane_info)
                index += 1
    
    if args.json:
        # JSON output
        print(json.dumps(panes, indent=2))
    else:
        # Human-readable output
        print("\n📋 Available Panes:")
        print("=" * 60)
        
        for pane in panes:
            current_marker = " 👉" if pane["current"] else ""
            label_str = f"[{pane['label']}]" if pane['label'] else "[unlabeled]"
            
            print(f"{pane['index']:2}. {label_str:15} "
                  f"Window:{pane['window']} Tab:{pane['tab']} "
                  f"ID:{pane['session_id'][:8]}...{current_marker}")
        
        print("=" * 60)
        print(f"Total: {len(panes)} panes")
        print("\nUse 'label_pane.py' to label panes for easier identification")


iterm2.run_until_complete(main)