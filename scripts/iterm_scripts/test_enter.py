#!/usr/bin/env python3
"""
Test different ways to send Enter key in iTerm2 with Claude CLI.
Creates a new pane to test command submission, simulating send_to_pane.py behavior.
"""

import iterm2
import asyncio
import sys


async def main(connection):
    """Test different Enter key methods with Claude CLI."""
    app = await iterm2.async_get_app(connection)
    current_session = app.current_terminal_window.current_tab.current_session
    
    if not current_session:
        print("❌ No active session found")
        return
    
    # Check if user provided CLI tool name
    cli_tool = "claude"  # default
    if len(sys.argv) > 1:
        cli_tool = sys.argv[1]
    
    print(f"Creating a test pane to test Enter key submission with '{cli_tool}'...")
    print("Watch the new pane to see which methods actually submit commands")
    print()
    
    # Split horizontally to create a test pane
    test_session = await current_session.async_split_pane(vertical=False)
    
    if not test_session:
        print("❌ Failed to create test pane")
        return
    
    # Wait for pane to be ready
    await asyncio.sleep(1)
    
    # Label the test pane
    await test_session.async_set_variable("user.pane_label", "test-enter")
    change = iterm2.LocalWriteOnlyProfile()
    change.set_badge_text(f"🧪 Testing {cli_tool}")
    await test_session.async_set_profile_properties(change)
    await test_session.async_set_name(f"[Test] {cli_tool} Enter Methods")
    
    # Start the CLI tool
    print(f"Starting {cli_tool} in test pane...")
    await test_session.async_send_text(cli_tool)
    await asyncio.sleep(0.1)
    await test_session.async_send_text("\n")
    
    # Wait for CLI to start
    print(f"Waiting for {cli_tool} to start...")
    await asyncio.sleep(3)
    
    print("Running Enter key tests...")
    print("-" * 50)
    
    # Test 1: Carriage return
    print("1. Testing \\r (carriage return)...")
    await test_session.async_send_text("Test 1: Please say 'Hello CR'")
    await asyncio.sleep(0.5)
    await test_session.async_send_text("\r")
    await asyncio.sleep(3)
    
    # Test 2: Line feed
    print("2. Testing \\n (line feed)...")
    await test_session.async_send_text("Test 2: Please say 'Hello LF'")
    await asyncio.sleep(0.5)
    await test_session.async_send_text("\n")
    await asyncio.sleep(3)
    
    # Test 3: Carriage return + line feed
    print("3. Testing \\r\\n (CRLF)...")
    await test_session.async_send_text("Test 3: Please say 'Hello CRLF'")
    await asyncio.sleep(0.5)
    await test_session.async_send_text("\r\n")
    await asyncio.sleep(3)
    
    # Test 4: ASCII code 13 (Enter key)
    print("4. Testing chr(13) (ASCII Enter)...")
    await test_session.async_send_text("Test 4: Please say 'Hello chr(13)'")
    await asyncio.sleep(0.5)
    await test_session.async_send_text(chr(13))
    await asyncio.sleep(3)
    
    # Test 5: ASCII code 10 (Line feed)
    print("5. Testing chr(10) (ASCII LF)...")
    await test_session.async_send_text("Test 5: Please say 'Hello chr(10)'")
    await asyncio.sleep(0.5)
    await test_session.async_send_text(chr(10))
    await asyncio.sleep(3)
    
    # Test 6: Double newline
    print("6. Testing \\n\\n (double newline)...")
    await test_session.async_send_text("Test 6: Please say 'Hello double newline'")
    await asyncio.sleep(0.5)
    await test_session.async_send_text("\n\n")
    await asyncio.sleep(3)
    
    # Test 7: Single newline with longer delay
    print("7. Testing \\n with longer delay...")
    await test_session.async_send_text("Test 7: Please say 'Hello newline with delay'")
    await asyncio.sleep(0.2)  # Longer delay to ensure text is received
    await test_session.async_send_text("\n")
    await asyncio.sleep(3)
    
    # Test 8: Text ending with newline plus another newline
    print("8. Testing text\\n + \\n...")
    await test_session.async_send_text("Test 8: Please say 'Hello text with newline'\n")
    await asyncio.sleep(0.2)
    await test_session.async_send_text("\n")
    await asyncio.sleep(3)
    
    # Test 9: Triple newline
    print("9. Testing \\n\\n\\n (triple newline)...")
    await test_session.async_send_text("Test 9: Please say 'Hello triple newline'")
    await asyncio.sleep(0.5)
    await test_session.async_send_text("\n\n\n")
    await asyncio.sleep(3)
    
    print("-" * 50)
    print(f"✅ Tests complete. Check the {cli_tool} pane to see which methods submitted the prompts.")
    print(f"💡 Look for which tests got responses from {cli_tool}.")
    print(f"💡 The test pane with {cli_tool} will remain open for inspection.")


iterm2.run_until_complete(main)