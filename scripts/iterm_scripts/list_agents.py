#!/usr/bin/env python3
"""
List all spawned agents with their details.

Usage:
    python3 list_agents.py              # List all agents
    python3 list_agents.py --json       # Output as JSON
    python3 list_agents.py --type claude  # Filter by agent type
"""

import iterm2
import asyncio
import json
import argparse


async def main(connection):
    """List all agents."""
    parser = argparse.ArgumentParser(description='List all spawned agents')
    parser.add_argument('--json', action='store_true', help='Output as JSON')
    parser.add_argument('--type', help='Filter by agent type')
    
    args = parser.parse_args()
    
    app = await iterm2.async_get_app(connection)
    
    agents = []
    
    # Find all sessions with agent metadata
    for window_index, window in enumerate(app.terminal_windows):
        for tab_index, tab in enumerate(window.tabs):
            for session in tab.sessions:
                try:
                    # Check if this is an agent session
                    agent_type = await session.async_get_variable("user.agent_type")
                    if not agent_type:
                        continue
                    
                    # Get agent details
                    label = await session.async_get_variable("user.pane_label")
                    command = await session.async_get_variable("user.agent_command")
                    
                    # Apply type filter if specified
                    if args.type and agent_type != args.type:
                        continue
                    
                    # Check if this is the current session
                    is_current = (
                        app.current_terminal_window == window and
                        app.current_terminal_window.current_tab == tab and
                        tab.current_session == session
                    )
                    
                    agent_info = {
                        "name": label or "unnamed",
                        "type": agent_type,
                        "command": command or "unknown",
                        "session_id": session.session_id,
                        "window": window_index + 1,
                        "tab": tab_index + 1,
                        "current": is_current
                    }
                    agents.append(agent_info)
                    
                except:
                    # Session doesn't have agent metadata, skip it
                    pass
    
    if args.json:
        # JSON output
        print(json.dumps(agents, indent=2))
    else:
        # Human-readable output
        if not agents:
            print("📭 No agents found")
            print("\nSpawn an agent with:")
            print("  python3 spawn_agent.py claude")
            return
        
        print(f"\n🤖 Active Agents ({len(agents)} total):")
        print("=" * 70)
        
        for i, agent in enumerate(agents, 1):
            current_marker = " 👉" if agent["current"] else ""
            print(f"{i:2}. {agent['name']:20} Type: {agent['type']:10} "
                  f"Window:{agent['window']} Tab:{agent['tab']}{current_marker}")
            print(f"    Command: {agent['command']}")
            print(f"    Session: {agent['session_id'][:12]}...")
            if i < len(agents):
                print()
        
        print("=" * 70)
        print("\n💡 Commands:")
        print("  Send to agent:  python3 send_to_pane.py --to <agent-name> --text 'command'")
        print("  Kill agent:     python3 kill_agent.py <agent-name>")
        print("  Spawn new:      python3 spawn_agent.py <type>")


iterm2.run_until_complete(main)