#!/usr/bin/env python3
"""Initialize the current pane as a primary orchestrator pane."""

import iterm2
import asyncio
import json


async def main(connection):
    """Set up current pane as primary orchestrator."""
    app = await iterm2.async_get_app(connection)
    current_session = app.current_terminal_window.current_tab.current_session
    
    if not current_session:
        print("❌ No active session found")
        return
    
    # Set up as primary orchestrator
    await current_session.async_set_variable("user.pane_label", "primary-orchestrator")
    await current_session.async_set_variable("user.agent_type", "orchestrator")
    await current_session.async_set_variable("user.parent_pane_id", None)
    await current_session.async_set_variable("user.children_pane_ids", json.dumps([]))
    await current_session.async_set_variable("user.spawn_count", "0")
    
    # Set session name
    await current_session.async_set_name("[Orchestrator] Primary")
    
    print("✅ Current pane initialized as primary orchestrator")
    print(f"   Session ID: {current_session.session_id[:8]}...")


iterm2.run_until_complete(main)