#!/usr/bin/env python3
"""
Demo script to showcase intelligent pane spawning.

This demonstrates the hierarchical pane layout:
1. Primary pane spawns first agent → vertical split
2. Primary spawns second agent → horizontal split of right pane
3. Sub-agent spawns its own agent → vertical split
"""

import iterm2
import asyncio
import time


async def main(connection):
    """Demo the smart spawning capabilities."""
    app = await iterm2.async_get_app(connection)
    
    print("🎬 Smart Pane Spawning Demo")
    print("=" * 60)
    print()
    print("This demo will show how intelligent pane splitting works:")
    print("1. First spawn from primary → vertical split (left/right)")
    print("2. Second spawn from primary → horizontal split of right pane")
    print("3. Spawn from sub-agent → vertical split of its pane")
    print()
    print("Press Enter to continue...")
    input()
    
    # Step 1: Initialize current pane as primary
    print("\n📍 Step 1: Initializing current pane as primary orchestrator...")
    current_session = app.current_terminal_window.current_tab.current_session
    
    if not current_session:
        print("❌ No active session found")
        return
    
    # Run init_primary.py
    await current_session.async_send_text("python3 iterm_scripts/init_primary.py\n")
    await asyncio.sleep(2)
    
    print("   ✅ Primary orchestrator initialized")
    print("\nPress Enter to spawn first agent...")
    input()
    
    # Step 2: Spawn first agent (should create vertical split)
    print("\n📍 Step 2: Spawning first agent from primary...")
    print("   Expected: Vertical split (primary left, agent-1 right)")
    
    await current_session.async_send_text("python3 iterm_scripts/spawn_agent_smart.py claude --name agent-1\n")
    await asyncio.sleep(5)
    
    print("   ✅ First agent spawned")
    print("\nPress Enter to spawn second agent from primary...")
    input()
    
    # Step 3: Return to primary pane and spawn second agent
    print("\n📍 Step 3: Spawning second agent from primary...")
    print("   Expected: Horizontal split of right pane (agent-1 top, agent-2 bottom)")
    
    # We need to activate the primary pane first
    # In a real scenario, the user would click on the primary pane
    print("   ⚠️  Please click on the PRIMARY (leftmost) pane, then press Enter...")
    input()
    
    # Get the current session again (should be primary)
    current_session = app.current_terminal_window.current_tab.current_session
    await current_session.async_send_text("python3 iterm_scripts/spawn_agent_smart.py gemini --name agent-2\n")
    await asyncio.sleep(5)
    
    print("   ✅ Second agent spawned")
    print("\nPress Enter to spawn sub-agent from agent-1...")
    input()
    
    # Step 4: Spawn from a sub-agent
    print("\n📍 Step 4: Spawning sub-agent from agent-1...")
    print("   Expected: Vertical split of agent-1's pane")
    print("   ⚠️  Please click on AGENT-1 pane (top-right), then press Enter...")
    input()
    
    # Get the current session (should be agent-1)
    current_session = app.current_terminal_window.current_tab.current_session
    await current_session.async_send_text("python3 iterm_scripts/spawn_agent_smart.py gpt --name sub-agent-1\n")
    await asyncio.sleep(5)
    
    print("   ✅ Sub-agent spawned")
    
    # Step 5: Show final pane tree
    print("\n📍 Step 5: Final pane structure")
    print("   Running pane_manager.py to show the tree...")
    
    await current_session.async_send_text("python3 iterm_scripts/pane_manager.py\n")
    await asyncio.sleep(2)
    
    print("\n🎉 Demo Complete!")
    print("\nExpected layout:")
    print("┌─────────────┬─────────────┬─────────────┐")
    print("│             │   agent-1   │ sub-agent-1 │")
    print("│   primary   ├─────────────┴─────────────┤")
    print("│ orchestrator│          agent-2          │")
    print("└─────────────┴───────────────────────────┘")
    print()
    print("Tips:")
    print("- Use 'python3 iterm_scripts/pane_manager.py' to view pane tree")
    print("- Each pane tracks its parent and children")
    print("- Smart splitting adapts to manual pane operations")


iterm2.run_until_complete(main)