#!/usr/bin/env python3
"""
Test script to find the correct way to set badges in iTerm2.
"""

import iterm2
import asyncio


async def main(connection):
    """Test badge setting methods."""
    app = await iterm2.async_get_app(connection)
    session = app.current_terminal_window.current_tab.current_session
    
    if not session:
        print("❌ No active session found")
        return
    
    print(f"Session ID: {session.session_id}")
    
    # Try different methods to set badge
    try:
        # Method 1: Try setting badge format through profile
        profile = await session.async_get_profile()
        print(f"Current profile: {profile}")
        
        # Method 2: Try async_set_profile_property
        await session.async_set_profile_property("Badge Text", "TEST")
        print("✅ Set via profile property 'Badge Text'")
    except Exception as e:
        print(f"❌ Badge Text failed: {e}")
    
    try:
        # Method 3: Try with different property name
        await session.async_set_profile_property("BadgeText", "TEST")
        print("✅ Set via profile property 'BadgeText'")
    except Exception as e:
        print(f"❌ BadgeText failed: {e}")
        
    try:
        # Method 4: Try badge_format
        await session.async_set_profile_property("Badge Format", "\\(session.name)")
        print("✅ Set Badge Format")
    except Exception as e:
        print(f"❌ Badge Format failed: {e}")
    
    # Try to inject a variable for badge
    try:
        await session.async_set_variable("badge", "TEST BADGE")
        await session.async_set_profile_property("Badge Format", "\\(user.badge)")
        print("✅ Set badge via variable")
    except Exception as e:
        print(f"❌ Variable badge failed: {e}")
    
    print("\nAvailable methods on session:")
    for attr in dir(session):
        if 'badge' in attr.lower():
            print(f"  - {attr}")


iterm2.run_until_complete(main)