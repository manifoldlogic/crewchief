#!/usr/bin/env python3
"""
Spawn a new CLI agent in a split pane with its own worktree.

This script:
1. Splits the current pane
2. Creates a worktree for the agent
3. Changes to that worktree directory
4. Launches the specified CLI agent

Usage:
    python3 spawn_agent.py claude              # Spawn Claude agent with auto-generated name
    python3 spawn_agent.py claude --name my-agent  # Spawn Claude with specific name
    python3 spawn_agent.py gemini              # Spawn Gemini agent
    python3 spawn_agent.py custom "my-cli-command"  # Custom CLI command
    python3 spawn_agent.py claude --vertical   # Split vertically (default: horizontal)
    python3 spawn_agent.py claude --project-dir /path/to/project  # Specify project directory
"""

import iterm2
import asyncio
import argparse
import sys
import os
from datetime import datetime
import random
import string
from agent_config import get_enter_key


def generate_agent_name(agent_type):
    """Generate a unique agent name."""
    timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    random_suffix = ''.join(random.choices(string.ascii_lowercase + string.digits, k=4))
    return f"{agent_type}-{timestamp}-{random_suffix}"


async def main(connection):
    """Spawn a new agent."""
    parser = argparse.ArgumentParser(description='Spawn a new CLI agent')
    parser.add_argument('agent', help='Agent type (claude, gemini, gpt, or custom command)')
    parser.add_argument('custom_command', nargs='?', help='Custom command if agent is "custom"')
    parser.add_argument('--name', help='Name for the agent worktree')
    parser.add_argument('--vertical', action='store_true', help='Split vertically instead of horizontally')
    parser.add_argument('--no-label', action='store_true', help="Don't label the new pane")
    parser.add_argument('--args', help='Additional arguments for the agent command')
    parser.add_argument('--project-dir', help='Project directory (defaults to current iTerm2 session directory)')
    
    args = parser.parse_args()
    
    app = await iterm2.async_get_app(connection)
    current_session = app.current_terminal_window.current_tab.current_session
    
    if not current_session:
        print("❌ No active session found")
        return
    
    # Determine the agent command
    agent_commands = {
        'claude': 'claude',
        'gemini': 'gemini',
        'codex': 'codex',
        'cursor': 'cursor',
        'aider': 'aider',
    }
    
    if args.agent.lower() in agent_commands:
        agent_command = agent_commands[args.agent.lower()]
        agent_type = args.agent.lower()
    elif args.agent.lower() == 'custom':
        if not args.custom_command:
            print("❌ Please provide a custom command")
            return
        agent_command = args.custom_command
        agent_type = 'custom'
    else:
        # Treat it as a direct command
        agent_command = args.agent
        agent_type = args.agent.split()[0] if args.agent else 'agent'
    
    # Generate or use provided name
    agent_name = args.name or generate_agent_name(agent_type)
    
    # We'll check if worktree exists after we have the project_dir
    initial_agent_name = agent_name
    
    print(f"🚀 Spawning {agent_type} agent: {agent_name}")
    
    # Step 1: Split the pane
    print("   1️⃣ Splitting pane...")
    new_session = await current_session.async_split_pane(vertical=args.vertical)
    
    if not new_session:
        print("❌ Failed to split pane")
        return
    
    # Small delay to ensure pane is ready
    await asyncio.sleep(0.5)
    
    # Step 2: Determine project directory
    if args.project_dir:
        # Use explicitly provided directory
        project_dir = os.path.expanduser(args.project_dir)
        print(f"   2️⃣ Using specified project directory: {project_dir}")
    else:
        # Copy the working directory from the current session
        print(f"   2️⃣ Copying working directory from current session...")
        
        # First, set a marker variable in current session
        marker = f"PWD_{random.randint(1000, 9999)}"
        await current_session.async_send_text(f"echo 'MARKER:{marker}:$PWD'\n")
        await asyncio.sleep(0.5)
        
        # Capture the output to find the directory
        output = await current_session.async_get_screen_contents()
        
        # Find the marker in output
        project_dir = None
        if output and output.text:
            for line in output.text.split('\n'):
                if f'MARKER:{marker}:' in line:
                    project_dir = line.split(f'MARKER:{marker}:')[1].strip()
                    break
        
        if not project_dir:
            print("   ⚠️  Could not detect project directory. Please specify with --project-dir")
            print("   📝 Example: python3 spawn_agent.py claude --project-dir /path/to/project")
            return
        
        print(f"   📁 Detected project directory: {project_dir}")
    
    # Check if worktree already exists and append timestamp if needed
    worktree_path = os.path.join(project_dir, ".crewchief", "worktrees", agent_name)
    if os.path.exists(worktree_path):
        # Append timestamp if worktree exists
        timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        agent_name = f"{initial_agent_name}_{timestamp}"
        worktree_path = os.path.join(project_dir, ".crewchief", "worktrees", agent_name)
        print(f"   ⚠️  Worktree {initial_agent_name} exists, using {agent_name}")
    
    # Change new pane to project directory
    await new_session.async_send_text(f"cd {project_dir}")
    await new_session.async_send_text("\n")
    await asyncio.sleep(0.5)
    
    # Step 3: Create worktree
    print(f"   3️⃣ Creating worktree: {agent_name}")
    worktree_cmd = f"crewchief worktree create {agent_name} --no-cd"
    await new_session.async_send_text(worktree_cmd)
    await new_session.async_send_text("\n")
    
    # Wait for worktree creation to complete
    await asyncio.sleep(2)
    
    # Step 4: Change to worktree directory using absolute path
    print(f"   4️⃣ Changing to worktree directory...")
    # Build absolute path to worktree
    worktree_path = os.path.join(project_dir, ".crewchief", "worktrees", agent_name)
    cd_cmd = f"cd {worktree_path}"
    await new_session.async_send_text(cd_cmd)
    await new_session.async_send_text("\n")
    
    # Verify we're in the right directory
    await asyncio.sleep(0.5)
    await new_session.async_send_text("pwd")
    await new_session.async_send_text("\n")
    
    # Small delay before launching agent
    await asyncio.sleep(0.5)
    
    # Step 5: Launch the agent
    print(f"   5️⃣ Launching {agent_type} agent...")
    if args.args:
        full_command = f"{agent_command} {args.args}"
    else:
        full_command = agent_command
    
    # Send command text first, then Enter key for submission
    # Use agent-specific Enter key (chr(13) for Claude, etc.)
    enter_key = get_enter_key(agent_type)
    await new_session.async_send_text(full_command)
    await asyncio.sleep(0.1)  # Small delay to ensure text is received
    await new_session.async_send_text(enter_key)
    
    # Step 6: Label the pane (unless disabled)
    if not args.no_label:
        print(f"   6️⃣ Labeling pane as: {agent_name}")
        
        # Set user variable for querying
        await new_session.async_set_variable("user.pane_label", agent_name)
        await new_session.async_set_variable("user.agent_type", agent_type)
        await new_session.async_set_variable("user.agent_command", agent_command)
        await new_session.async_set_variable("user.project_dir", project_dir)
        
        # Set badge
        change = iterm2.LocalWriteOnlyProfile()
        change.set_badge_text(f"🤖 {agent_name}")
        await new_session.async_set_profile_properties(change)
        
        # Set session name
        await new_session.async_set_name(f"[{agent_type}] {agent_name}")
    
    print(f"\n✅ Agent spawned successfully!")
    print(f"   - Name: {agent_name}")
    print(f"   - Type: {agent_type}")
    print(f"   - Command: {full_command}")
    print(f"   - Worktree: {worktree_path}")
    print(f"   - Session ID: {new_session.session_id[:8]}...")
    print(f"\n💡 Tips:")
    print(f"   - Send commands: python3 send_to_pane.py --to {agent_name} --text 'your command'")
    print(f"   - List all agents: python3 list_agents.py")
    print(f"   - View worktree: crewchief worktree list")


iterm2.run_until_complete(main)