#!/usr/bin/env python3
"""
Send text/commands to another iTerm2 pane.

Usage:
    python3 send_to_pane.py --to main --text "echo hello"   # Send to pane labeled "main"
    python3 send_to_pane.py --to 2 --text "ls -la"          # Send to pane index 2
    python3 send_to_pane.py --id SESSION_ID --text "pwd"    # Send to specific session ID
    python3 send_to_pane.py --agent                        # Agent mode (uses chr(13) for Enter)
    python3 send_to_pane.py                                 # Interactive mode
    echo "command" | python3 send_to_pane.py --to worker    # Pipe input
"""

import iterm2
import asyncio
import sys
import argparse
from agent_config import get_enter_key


async def find_session_by_identifier(app, identifier):
    """Find a session by label, index, or ID."""
    sessions_list = []
    index = 1
    
    # Build list of all sessions
    for window in app.terminal_windows:
        for tab in window.tabs:
            for session in tab.sessions:
                try:
                    label = await session.async_get_variable("user.pane_label")
                except:
                    label = None
                
                sessions_list.append({
                    "index": index,
                    "label": label or "",
                    "session": session,
                    "session_id": session.session_id
                })
                index += 1
    
    # Try to match the identifier
    # 1. Try as label
    for item in sessions_list:
        if item["label"] and item["label"].lower() == identifier.lower():
            return item["session"]
    
    # 2. Try as index
    try:
        idx = int(identifier)
        for item in sessions_list:
            if item["index"] == idx:
                return item["session"]
    except ValueError:
        pass
    
    # 3. Try as session ID (partial match)
    for item in sessions_list:
        if item["session_id"].startswith(identifier):
            return item["session"]
    
    return None


async def interactive_select(app):
    """Show interactive pane selector."""
    print("\n📋 Select target pane:")
    print("=" * 60)
    
    sessions_list = []
    index = 1
    
    for window_idx, window in enumerate(app.terminal_windows):
        for tab_idx, tab in enumerate(window.tabs):
            for session in tab.sessions:
                try:
                    label = await session.async_get_variable("user.pane_label")
                except:
                    label = None
                
                is_current = (
                    app.current_terminal_window == window and
                    app.current_terminal_window.current_tab == tab and
                    tab.current_session == session
                )
                
                label_str = f"[{label}]" if label else "[unlabeled]"
                current_str = " (current)" if is_current else ""
                
                print(f"{index:2}. {label_str:15} Window:{window_idx+1} Tab:{tab_idx+1}{current_str}")
                
                sessions_list.append(session)
                index += 1
    
    print("=" * 60)
    
    try:
        choice = input("Enter number (or 'q' to quit): ").strip()
        if choice.lower() == 'q':
            return None
        
        idx = int(choice) - 1
        if 0 <= idx < len(sessions_list):
            return sessions_list[idx]
        else:
            print("❌ Invalid selection")
            return None
    except (ValueError, KeyboardInterrupt):
        return None


async def main(connection):
    """Send text to a pane."""
    parser = argparse.ArgumentParser(description='Send text to an iTerm2 pane')
    parser.add_argument('--to', help='Target pane (label, index, or partial ID)')
    parser.add_argument('--id', help='Exact session ID')
    parser.add_argument('--text', help='Text to send')
    parser.add_argument('--no-newline', action='store_true', help="Don't add newline after text")
    parser.add_argument('--agent', help='Agent mode - specify agent type for proper Enter key (e.g., claude, gemini)')
    
    args = parser.parse_args()
    
    app = await iterm2.async_get_app(connection)
    
    # Get text to send
    text_to_send = args.text
    
    if not text_to_send:
        # Check for piped input
        if not sys.stdin.isatty():
            text_to_send = sys.stdin.read()
        else:
            text_to_send = input("Enter text to send: ")
    
    if not text_to_send:
        print("❌ No text to send")
        return
    
    # Find target session
    target_session = None
    
    if args.id:
        # Find by exact session ID
        for window in app.terminal_windows:
            for tab in window.tabs:
                for session in tab.sessions:
                    if session.session_id == args.id:
                        target_session = session
                        break
    elif args.to:
        # Find by label, index, or partial ID
        target_session = await find_session_by_identifier(app, args.to)
    else:
        # Interactive mode
        target_session = await interactive_select(app)
    
    if not target_session:
        print("❌ Target pane not found")
        return
    
    # Try to auto-detect agent type from pane's user variables
    agent_type = args.agent
    if not agent_type:
        try:
            agent_type = await target_session.async_get_variable("user.agent_type")
            if agent_type:
                print(f"🤖 Auto-detected agent type: {agent_type}")
        except:
            agent_type = None
    
    # Send the text and Enter key separately for proper submission
    if args.no_newline:
        # Just send the text without newline
        await target_session.async_send_text(text_to_send)
    elif agent_type:
        # Agent mode: use agent-specific Enter key
        enter_key = get_enter_key(agent_type)
        await target_session.async_send_text(text_to_send)
        await asyncio.sleep(0.1)  # Increased delay to ensure text is received
        await target_session.async_send_text(enter_key)
        print(f"🔑 Used {agent_type} Enter key: {repr(enter_key)}")
    elif text_to_send.endswith('\n'):
        # Text already has newline, send as is
        await target_session.async_send_text(text_to_send)
    else:
        # Regular command: use standard newline
        await target_session.async_send_text(text_to_send)
        await asyncio.sleep(0.1)  # Increased delay to ensure text is received
        await target_session.async_send_text('\n')
    
    # Get label for confirmation message
    try:
        label = await target_session.async_get_variable("user.pane_label")
        target_desc = f"'{label}'" if label else f"session {target_session.session_id[:8]}..."
    except:
        target_desc = f"session {target_session.session_id[:8]}..."
    
    print(f"✅ Sent to {target_desc}")


iterm2.run_until_complete(main)