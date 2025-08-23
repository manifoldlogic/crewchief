#!/usr/bin/env python3
"""Test agent type detection for a pane."""

import iterm2
import asyncio
import sys

async def main(connection):
    """Check agent type for a labeled pane."""
    if len(sys.argv) < 2:
        print("Usage: python3 test_agent_detection.py <pane_label>")
        return
    
    pane_label = sys.argv[1]
    app = await iterm2.async_get_app(connection)
    
    # Find pane by label
    for window in app.terminal_windows:
        for tab in window.tabs:
            for session in tab.sessions:
                try:
                    label = await session.async_get_variable("user.pane_label")
                    if label and label.lower() == pane_label.lower():
                        print(f"Found pane: {label}")
                        
                        # Check for agent_type variable
                        try:
                            agent_type = await session.async_get_variable("user.agent_type")
                            print(f"  agent_type: {agent_type}")
                        except:
                            print(f"  agent_type: NOT SET")
                        
                        # Check for other user variables
                        try:
                            all_vars = await session.async_get_variable("user")
                            print(f"  all user vars: {all_vars}")
                        except Exception as e:
                            print(f"  couldn't get all vars: {e}")
                        
                        return
                except:
                    pass
    
    print(f"Pane '{pane_label}' not found")

iterm2.run_until_complete(main)