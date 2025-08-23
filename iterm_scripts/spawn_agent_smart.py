#!/usr/bin/env python3
"""
Spawn a new CLI agent with intelligent pane splitting.

Layout Strategy:
- First spawn from parent: Vertical split (parent left, child right)
- Additional spawns from parent: Horizontal split of right pane
- Spawns from sub-agents: Vertical split of their pane

Usage:
    python3 spawn_agent_smart.py claude            # Spawn Claude agent
    python3 spawn_agent_smart.py claude --name my-agent  # With specific name
    python3 spawn_agent_smart.py gemini            # Spawn Gemini agent
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
from pane_manager import PaneManager


def generate_agent_name(agent_type):
    """Generate a unique agent name."""
    timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    random_suffix = ''.join(random.choices(string.ascii_lowercase + string.digits, k=4))
    return f"{agent_type}-{timestamp}-{random_suffix}"


async def main(connection):
    """Spawn a new agent with intelligent pane management."""
    parser = argparse.ArgumentParser(description='Spawn a new CLI agent with smart splitting')
    parser.add_argument('agent', help='Agent type (claude, gemini, gpt, or custom command)')
    parser.add_argument('custom_command', nargs='?', help='Custom command if agent is "custom"')
    parser.add_argument('--name', help='Name for the agent worktree')
    parser.add_argument('--no-label', action='store_true', help="Don't label the new pane")
    parser.add_argument('--args', help='Additional arguments for the agent command')
    parser.add_argument('--project-dir', help='Project directory (defaults to current iTerm2 session directory)')
    parser.add_argument('--force-vertical', action='store_true', help='Force vertical split')
    parser.add_argument('--force-horizontal', action='store_true', help='Force horizontal split')
    
    args = parser.parse_args()
    
    app = await iterm2.async_get_app(connection)
    current_session = app.current_terminal_window.current_tab.current_session
    
    if not current_session:
        print("❌ No active session found")
        return
    
    # Clean up orphaned references first
    await PaneManager.cleanup_orphaned_references(app)
    
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
    initial_agent_name = agent_name
    
    print(f"🚀 Spawning {agent_type} agent: {agent_name}")
    
    # Step 1: Determine split strategy
    print("   1️⃣ Analyzing pane layout...")
    
    # Check if force flags are set
    if args.force_vertical:
        session_to_split = current_session
        is_vertical = True
        print("   📐 Forcing vertical split")
    elif args.force_horizontal:
        session_to_split = current_session
        is_vertical = False
        print("   📐 Forcing horizontal split")
    else:
        # Use intelligent splitting
        session_to_split, is_vertical = await PaneManager.determine_split_strategy(app, current_session)
        
        # Get info about sessions for logging
        current_info = await PaneManager.get_pane_info(current_session)
        split_info = await PaneManager.get_pane_info(session_to_split)
        
        if session_to_split == current_session:
            if is_vertical:
                if current_info['children_ids']:
                    print(f"   📐 Current pane '{current_info['label']}' has {len(current_info['children_ids'])} children")
                    print(f"   📐 But cannot find rightmost child, splitting current pane vertically")
                else:
                    print(f"   📐 First spawn from '{current_info['label']}' - vertical split")
            else:
                print(f"   📐 Splitting current pane horizontally")
        else:
            split_label = split_info['label'] or 'unlabeled'
            print(f"   📐 Parent has children - splitting rightmost child '{split_label}' horizontally")
    
    # Step 2: Split the appropriate pane
    split_type = "vertical" if is_vertical else "horizontal"
    print(f"   2️⃣ Performing {split_type} split...")
    
    new_session = await session_to_split.async_split_pane(vertical=is_vertical)
    
    if not new_session:
        print("❌ Failed to split pane")
        return
    
    # Small delay to ensure pane is ready
    await asyncio.sleep(0.5)
    
    # Step 3: Set up parent-child relationship
    print("   3️⃣ Setting up pane relationships...")
    await PaneManager.setup_new_session(new_session, current_session, agent_name, agent_type)
    
    # Step 4: Determine project directory
    if args.project_dir:
        # Use explicitly provided directory
        project_dir = os.path.expanduser(args.project_dir)
        print(f"   4️⃣ Using specified project directory: {project_dir}")
    else:
        # Copy the working directory from the current session
        print(f"   4️⃣ Copying working directory from current session...")
        
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
            print("   📝 Example: python3 spawn_agent_smart.py claude --project-dir /path/to/project")
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
        
        # Update the pane label with new name
        await new_session.async_set_variable("user.pane_label", agent_name)
    
    # Change new pane to project directory
    await new_session.async_send_text(f"cd {project_dir}")
    await new_session.async_send_text("\n")
    await asyncio.sleep(0.5)
    
    # Step 5: Create worktree
    print(f"   5️⃣ Creating worktree: {agent_name}")
    worktree_cmd = f"crewchief worktree create {agent_name} --no-cd"
    await new_session.async_send_text(worktree_cmd)
    await new_session.async_send_text("\n")
    
    # Wait for worktree creation to complete
    await asyncio.sleep(2)
    
    # Step 6: Change to worktree directory
    print(f"   6️⃣ Changing to worktree directory...")
    cd_cmd = f"cd {worktree_path}"
    await new_session.async_send_text(cd_cmd)
    await new_session.async_send_text("\n")
    
    # Verify we're in the right directory
    await asyncio.sleep(0.5)
    await new_session.async_send_text("pwd")
    await new_session.async_send_text("\n")
    
    # Small delay before launching agent
    await asyncio.sleep(0.5)
    
    # Step 7: Launch the agent
    print(f"   7️⃣ Launching {agent_type} agent...")
    if args.args:
        full_command = f"{agent_command} {args.args}"
    else:
        full_command = agent_command
    
    # Send command text first, then Enter key for submission
    enter_key = get_enter_key(agent_type)
    await new_session.async_send_text(full_command)
    await asyncio.sleep(0.1)
    await new_session.async_send_text(enter_key)
    
    # Step 8: Additional labeling and visual setup
    if not args.no_label:
        print(f"   8️⃣ Finalizing pane setup...")
        
        # Set additional metadata
        await new_session.async_set_variable("user.agent_command", agent_command)
        await new_session.async_set_variable("user.project_dir", project_dir)
        
        # Set badge
        change = iterm2.LocalWriteOnlyProfile()
        parent_info = await PaneManager.get_pane_info(current_session)
        parent_label = parent_info['label'] or 'primary'
        badge_text = f"🤖 {agent_name}\n📍 From: {parent_label}"
        change.set_badge_text(badge_text)
        await new_session.async_set_profile_properties(change)
        
        # Set session name
        await new_session.async_set_name(f"[{agent_type}] {agent_name}")
    
    # Step 9: Show pane tree
    print(f"\n✅ Agent spawned successfully!")
    print(f"   - Name: {agent_name}")
    print(f"   - Type: {agent_type}")
    print(f"   - Command: {full_command}")
    print(f"   - Worktree: {worktree_path}")
    print(f"   - Session ID: {new_session.session_id[:8]}...")
    
    # Display the updated pane tree
    tree = await PaneManager.get_pane_tree(app)
    print(f"\n📊 Current Pane Structure:")
    print("=" * 50)
    
    def print_node(node_id, indent=0):
        if node_id not in tree['nodes']:
            return
        
        node = tree['nodes'][node_id]
        prefix = "  " * indent + ("├─ " if indent > 0 else "")
        label = node['label'] or 'unlabeled'
        agent = f"[{node['agent_type']}]" if node['agent_type'] else ""
        children = f"({len(node['children_ids'])} children)" if node['children_ids'] else ""
        
        # Highlight the new session
        if node_id == new_session.session_id:
            print(f"{prefix}{label} {agent} {children} ← NEW")
        else:
            print(f"{prefix}{label} {agent} {children}")
        
        for child_id in node['children_ids']:
            print_node(child_id, indent + 1)
    
    # Find and print from root of current session's tree
    current_id = current_session.session_id
    root_id = current_id
    
    # Walk up to find root
    while root_id in tree['nodes'] and tree['nodes'][root_id]['parent_id']:
        root_id = tree['nodes'][root_id]['parent_id']
    
    print_node(root_id)
    print("=" * 50)
    
    print(f"\n💡 Tips:")
    print(f"   - Send commands: python3 send_to_pane.py --to {agent_name} --text 'your command'")
    print(f"   - View pane tree: python3 pane_manager.py")
    print(f"   - List all agents: python3 list_agents.py")


iterm2.run_until_complete(main)