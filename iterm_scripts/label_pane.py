#!/usr/bin/env python3
"""
Label the current pane (or a specific pane) with a name for easy identification.
This sets a visual badge, session name, and a queryable user variable.

Usage:
    python3 label_pane.py "main"           # Label current pane as "main"
    python3 label_pane.py "worker" --session SESSION_ID  # Label specific session
    python3 label_pane.py --clear          # Clear label from current pane
"""

import iterm2
import asyncio
import sys
import argparse


async def main(connection):
    """Label a pane with a name."""
    parser = argparse.ArgumentParser(description='Label an iTerm2 pane')
    parser.add_argument('label', nargs='?', help='Label to set on the pane')
    parser.add_argument('--session', help='Session ID to label (default: current)')
    parser.add_argument('--clear', action='store_true', help='Clear the label')
    
    args = parser.parse_args()
    
    app = await iterm2.async_get_app(connection)
    
    # Find the target session
    target_session = None
    
    if args.session:
        # Find session by ID
        for window in app.terminal_windows:
            for tab in window.tabs:
                for session in tab.sessions:
                    if session.session_id == args.session:
                        target_session = session
                        break
    else:
        # Use current session
        target_session = app.current_terminal_window.current_tab.current_session
    
    if not target_session:
        print("❌ Session not found")
        return
    
    if args.clear:
        # Clear the label
        await target_session.async_set_variable("user.pane_label", "")
        await target_session.async_set_variable("user.badge", "")
        await target_session.async_set_name("")
        
        # Update the profile to clear badge
        profile = await target_session.async_get_profile()
        change = iterm2.LocalWriteOnlyProfile()
        change.set_badge_text("")
        await target_session.async_set_profile_properties(change)
        
        print(f"✅ Cleared label from session {target_session.session_id}")
    else:
        label = args.label
        if not label:
            print("❌ Please provide a label or use --clear")
            return
        
        # Set user variable for querying
        await target_session.async_set_variable("user.pane_label", label)
        
        # Set badge using profile properties
        change = iterm2.LocalWriteOnlyProfile()
        change.set_badge_text(f"📌 {label}")
        await target_session.async_set_profile_properties(change)
        
        # Also set session name for tab display
        await target_session.async_set_name(f"[{label}]")
        
        print(f"✅ Labeled session {target_session.session_id} as '{label}'")
        print(f"   - Badge: 📌 {label}")
        print(f"   - Session name: [{label}]")


iterm2.run_until_complete(main)