#!/usr/bin/env python3
"""
Kill an agent session and optionally clean up its worktree.

Usage:
    python3 kill_agent.py <agent-name>              # Kill agent by name
    python3 kill_agent.py <agent-name> --cleanup    # Also remove worktree
    python3 kill_agent.py --all                     # Kill all agents
"""

import iterm2
import asyncio
import argparse
import sys


async def find_agent_session(app, agent_name):
    """Find an agent session by name."""
    for window in app.terminal_windows:
        for tab in window.tabs:
            for session in tab.sessions:
                try:
                    label = await session.async_get_variable("user.pane_label")
                    agent_type = await session.async_get_variable("user.agent_type")
                    
                    # Check if this is an agent and matches the name
                    if agent_type and label and label.lower() == agent_name.lower():
                        return session, label
                except:
                    pass
    return None, None


async def main(connection):
    """Kill an agent."""
    parser = argparse.ArgumentParser(description='Kill an agent session')
    parser.add_argument('agent_name', nargs='?', help='Agent name to kill')
    parser.add_argument('--all', action='store_true', help='Kill all agents')
    parser.add_argument('--cleanup', action='store_true', help='Also remove the worktree')
    
    args = parser.parse_args()
    
    if not args.agent_name and not args.all:
        print("❌ Please specify an agent name or use --all")
        return
    
    app = await iterm2.async_get_app(connection)
    
    if args.all:
        # Find all agent sessions
        agent_sessions = []
        for window in app.terminal_windows:
            for tab in window.tabs:
                for session in tab.sessions:
                    try:
                        agent_type = await session.async_get_variable("user.agent_type")
                        if agent_type:
                            label = await session.async_get_variable("user.pane_label")
                            agent_sessions.append((session, label or "unnamed"))
                    except:
                        pass
        
        if not agent_sessions:
            print("📭 No agents found to kill")
            return
        
        print(f"🔪 Killing {len(agent_sessions)} agents...")
        
        for session, name in agent_sessions:
            # Send Ctrl+C to interrupt, then exit command
            await session.async_send_text("\x03")  # Ctrl+C
            await asyncio.sleep(0.2)
            await session.async_send_text("exit\n")
            print(f"   ✅ Killed: {name}")
        
        print(f"\n✅ All agents killed")
        
        if args.cleanup:
            print("\n🧹 To clean up worktrees, run:")
            print("   crewchief worktree clean --all")
    
    else:
        # Find specific agent
        session, actual_name = await find_agent_session(app, args.agent_name)
        
        if not session:
            print(f"❌ Agent '{args.agent_name}' not found")
            print("\nAvailable agents:")
            
            # List available agents
            for window in app.terminal_windows:
                for tab in window.tabs:
                    for s in tab.sessions:
                        try:
                            agent_type = await s.async_get_variable("user.agent_type")
                            if agent_type:
                                label = await s.async_get_variable("user.pane_label")
                                print(f"  - {label}")
                        except:
                            pass
            return
        
        print(f"🔪 Killing agent: {actual_name}")
        
        # Send Ctrl+C to interrupt, then exit command
        await session.async_send_text("\x03")  # Ctrl+C
        await asyncio.sleep(0.2)
        await session.async_send_text("exit\n")
        
        print(f"✅ Agent killed: {actual_name}")
        
        if args.cleanup:
            print(f"\n🧹 To clean up the worktree, run:")
            print(f"   crewchief worktree clean")
            print(f"   # or specifically:")
            print(f"   rm -rf .crewchief/worktrees/{actual_name}")


iterm2.run_until_complete(main)