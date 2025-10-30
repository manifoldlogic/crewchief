#!/usr/bin/env python3
"""Debug version of send_to_pane to understand the issue."""

import iterm2
import asyncio
import sys
from agent_config import get_enter_key

async def main(connection):
    """Debug sending to a pane."""
    if len(sys.argv) < 3:
        print("Usage: python3 debug_send.py <pane_label> <text>")
        return
    
    pane_label = sys.argv[1]
    text = sys.argv[2]
    
    app = await iterm2.async_get_app(connection)
    
    # Find pane by label
    for window in app.terminal_windows:
        for tab in window.tabs:
            for session in tab.sessions:
                try:
                    label = await session.async_get_variable("user.pane_label")
                    if label and label.lower() == pane_label.lower():
                        print(f"✅ Found pane: {label}")
                        
                        # Check for agent_type variable
                        agent_type = None
                        try:
                            agent_type = await session.async_get_variable("user.agent_type")
                            print(f"✅ Agent type from variable: {agent_type}")
                        except:
                            print(f"⚠️  No agent_type variable set")
                        
                        # If not set, try to parse from label
                        if not agent_type and "__" in label:
                            parts = label.split("__")
                            if len(parts) == 2:
                                possible_agent = parts[1]
                                print(f"🔍 Parsed from label: {possible_agent}")
                                agent_type = possible_agent
                        
                        if agent_type:
                            enter_key = get_enter_key(agent_type)
                            print(f"✅ Using agent-specific enter key for '{agent_type}'")
                            print(f"   Enter key repr: {repr(enter_key)}")
                            
                            # Send text and enter separately
                            await session.async_send_text(text)
                            await asyncio.sleep(0.05)
                            await session.async_send_text(enter_key)
                            print(f"✅ Sent text + agent enter key")
                        else:
                            print(f"⚠️  Using default newline (no agent type detected)")
                            await session.async_send_text(text)
                            await asyncio.sleep(0.05)
                            await session.async_send_text('\n')
                            print(f"✅ Sent text + newline")
                        
                        return
                except Exception as e:
                    print(f"Error checking session: {e}")
                    pass
    
    print(f"❌ Pane '{pane_label}' not found")

iterm2.run_until_complete(main)