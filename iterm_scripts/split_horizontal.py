#!/usr/bin/env python3
"""
Split the current iTerm2 pane horizontally (top and bottom).
"""

import iterm2
import asyncio


async def main(connection):
    """Split the current pane horizontally."""
    app = await iterm2.async_get_app(connection)
    
    session = app.current_terminal_window.current_tab.current_session
    
    if not session:
        print("❌ No active session found")
        return
    
    # vertical=False means split horizontally (top and bottom)
    new_session = await session.async_split_pane(vertical=False)
    
    if new_session:
        print(f"✅ Split pane horizontally")
    else:
        print("❌ Failed to split pane")


iterm2.run_until_complete(main)