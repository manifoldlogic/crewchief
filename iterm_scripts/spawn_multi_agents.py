#!/usr/bin/env python3
"""
Spawn multiple agents simultaneously with intelligent pane splitting.

Usage:
    python3 spawn_multi_agents.py claude,gemini implement-auth
    python3 spawn_multi_agents.py claude,gemini,codex code-review
    python3 spawn_multi_agents.py "claude, gemini" "fix-bug"
"""

import iterm2
import asyncio
import argparse
import sys
import os
from datetime import datetime
import random
import string
import time
from agent_config import get_enter_key
from pane_manager import PaneManager


def parse_agent_spec(spec: str) -> list:
    """
    Parse agent specification into list of agent types.
    Supports comma-separated format.
    Examples: "claude,gemini" or "claude, gemini"
    """
    
    # Split and clean up
    agents = [a.strip() for a in spec.split(',') if a.strip()]
    
    # Remove duplicates while preserving order
    seen = set()
    unique_agents = []
    for agent in agents:
        if agent.lower() not in seen:
            seen.add(agent.lower())
            unique_agents.append(agent.lower())
    
    return unique_agents


def generate_agent_name(task: str, agent_type: str) -> str:
    """Generate agent name in format: task__agent_type."""
    if task:
        # Sanitize task name
        safe_task = task.replace(' ', '-').replace('_', '-').lower()
        return f"{safe_task}__{agent_type}"
    else:
        # Generate a timestamp-based name
        timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        return f"{agent_type}-{timestamp}"


async def spawn_single_agent(
    app,
    parent_session,
    agent_type: str,
    agent_name: str,
    project_dir: str,
    args: str = None,
    agent_index: int = 0,
    total_agents: int = 1
):
    """Spawn a single agent with smart splitting."""
    
    print(f"\n🤖 [{agent_index + 1}/{total_agents}] Spawning {agent_type} as '{agent_name}'")
    
    # Clean up orphaned references
    await PaneManager.cleanup_orphaned_references(app)
    
    # Step 1: Determine split strategy
    print(f"   📐 Determining split strategy...")
    session_to_split, is_vertical = await PaneManager.determine_split_strategy(app, parent_session)
    
    split_type = "vertical" if is_vertical else "horizontal"
    print(f"   📐 Performing {split_type} split...")
    
    # Step 2: Split the pane
    new_session = await session_to_split.async_split_pane(vertical=is_vertical)
    
    if not new_session:
        print(f"   ❌ Failed to split pane for {agent_type}")
        return None
    
    await asyncio.sleep(0.5)
    
    # Step 3: Set up parent-child relationship
    await PaneManager.setup_new_session(new_session, parent_session, agent_name, agent_type)
    
    # Step 4: Check if worktree exists
    worktree_path = os.path.join(project_dir, ".crewchief", "worktrees", agent_name)
    if os.path.exists(worktree_path):
        # Append timestamp if exists
        timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
        agent_name = f"{agent_name}_{timestamp}"
        worktree_path = os.path.join(project_dir, ".crewchief", "worktrees", agent_name)
        print(f"   ⚠️  Worktree exists, using {agent_name}")
        await new_session.async_set_variable("user.pane_label", agent_name)
    
    # Step 5: Setup working directory
    await new_session.async_send_text(f"cd {project_dir}\n")
    await asyncio.sleep(0.5)
    
    # Step 6: Create worktree
    print(f"   🌳 Creating worktree: {agent_name}")
    worktree_cmd = f"crewchief worktree create {agent_name} --no-cd"
    await new_session.async_send_text(f"{worktree_cmd}\n")
    await asyncio.sleep(2)
    
    # Step 7: Change to worktree
    await new_session.async_send_text(f"cd {worktree_path}\n")
    await asyncio.sleep(0.5)
    
    # Step 8: Launch the agent
    print(f"   🚀 Launching {agent_type}...")
    
    # Determine agent command
    agent_commands = {
        'claude': 'claude',
        'gemini': 'gemini',
        'codex': 'codex',
        'cursor': 'cursor',
        'aider': 'aider',
    }
    
    agent_command = agent_commands.get(agent_type, agent_type)
    
    if args:
        full_command = f"{agent_command} {args}"
    else:
        full_command = agent_command
    
    # Send command with appropriate enter key
    enter_key = get_enter_key(agent_type)
    await new_session.async_send_text(full_command)
    await asyncio.sleep(0.1)
    await new_session.async_send_text(enter_key)
    
    # Step 9: Visual setup
    print(f"   🎨 Setting up visual indicators...")
    await new_session.async_set_variable("user.agent_command", agent_command)
    await new_session.async_set_variable("user.project_dir", project_dir)
    
    # Set badge with agent icon
    agent_icons = {
        'claude': '🧠',
        'gemini': '✨',
        'codex': '🤖',
        'cursor': '🖱️',
        'aider': '🛠️',
    }
    icon = agent_icons.get(agent_type, '🤖')
    
    change = iterm2.LocalWriteOnlyProfile()
    badge_text = f"{icon} {agent_name}\n[{agent_index + 1}/{total_agents}]"
    change.set_badge_text(badge_text)
    await new_session.async_set_profile_properties(change)
    
    # Set session name
    await new_session.async_set_name(f"[{agent_type}] {agent_name}")
    
    print(f"   ✅ {agent_type} spawned successfully!")
    
    return new_session


async def main(connection):
    """Spawn multiple agents simultaneously."""
    parser = argparse.ArgumentParser(
        description='Spawn multiple CLI agents with smart splitting',
        epilog='Examples:\n'
               '  python3 spawn_multi_agents.py claude,gemini implement-auth\n'
               '  python3 spawn_multi_agents.py "claude,gemini,codex" code-review\n'
               '  python3 spawn_multi_agents.py claude,gemini --args "--model gpt-4"'
    )
    parser.add_argument('agents', help='Agent types (comma-separated: claude,gemini)')
    parser.add_argument('task', nargs='?', help='Task name for all agents')
    parser.add_argument('--args', help='Additional arguments for all agents')
    parser.add_argument('--project-dir', help='Project directory (defaults to current)')
    parser.add_argument('--delay', type=float, default=1.0, help='Delay between spawning agents (seconds)')
    parser.add_argument('--parallel', action='store_true', help='Spawn agents in parallel (experimental)')
    
    args = parser.parse_args()
    
    # Parse agent specification
    agent_types = parse_agent_spec(args.agents)
    
    if not agent_types:
        print("❌ No valid agent types specified")
        print("   Example: python3 spawn_multi_agents.py claude,gemini implement-auth")
        return
    
    app = await iterm2.async_get_app(connection)
    current_session = app.current_terminal_window.current_tab.current_session
    
    if not current_session:
        print("❌ No active session found")
        return
    
    # Determine project directory
    if args.project_dir:
        project_dir = os.path.expanduser(args.project_dir)
    else:
        # Try to detect from current session
        marker = f"PWD_{random.randint(1000, 9999)}"
        await current_session.async_send_text(f"echo 'MARKER:{marker}:$PWD'\n")
        await asyncio.sleep(0.5)
        
        output = await current_session.async_get_screen_contents()
        project_dir = None
        
        if output and output.text:
            for line in output.text.split('\n'):
                if f'MARKER:{marker}:' in line:
                    project_dir = line.split(f'MARKER:{marker}:')[1].strip()
                    break
        
        if not project_dir:
            print("❌ Could not detect project directory")
            print("   Use --project-dir /path/to/project")
            return
    
    # Display spawn plan
    print(f"\n🎯 Multi-Agent Spawn Plan")
    print("=" * 60)
    print(f"📁 Project: {project_dir}")
    print(f"📝 Task: {args.task or '(auto-generated)'}")
    print(f"🤖 Agents to spawn: {len(agent_types)}")
    for i, agent_type in enumerate(agent_types, 1):
        agent_name = generate_agent_name(args.task, agent_type)
        print(f"   {i}. {agent_type:10} → {agent_name}")
    print("=" * 60)
    
    # Initialize parent session if needed
    parent_info = await PaneManager.get_pane_info(current_session)
    if not parent_info['label']:
        print("\n📍 Initializing current pane as orchestrator...")
        await current_session.async_set_variable("user.pane_label", "multi-spawn-orchestrator")
        await current_session.async_set_variable("user.agent_type", "orchestrator")
        await current_session.async_set_variable("user.children_pane_ids", "[]")
        await current_session.async_set_variable("user.spawn_count", "0")
        
        change = iterm2.LocalWriteOnlyProfile()
        change.set_badge_text(f"🎯 Orchestrator\nSpawning {len(agent_types)} agents")
        await current_session.async_set_profile_properties(change)
    
    # Spawn agents
    spawned_sessions = []
    start_time = time.time()
    
    if args.parallel:
        print("\n⚡ Spawning agents in parallel (experimental)...")
        # Parallel spawning (experimental - may have race conditions)
        tasks = []
        for i, agent_type in enumerate(agent_types):
            agent_name = generate_agent_name(args.task, agent_type)
            task = spawn_single_agent(
                app, current_session, agent_type, agent_name,
                project_dir, args.args, i, len(agent_types)
            )
            tasks.append(task)
            if i < len(agent_types) - 1:
                await asyncio.sleep(0.5)  # Small delay to avoid conflicts
        
        results = await asyncio.gather(*tasks, return_exceptions=True)
        for result in results:
            if isinstance(result, Exception):
                print(f"   ⚠️  Error: {result}")
            elif result:
                spawned_sessions.append(result)
    else:
        print(f"\n🔄 Spawning agents sequentially (delay: {args.delay}s)...")
        # Sequential spawning (safer, recommended)
        for i, agent_type in enumerate(agent_types):
            agent_name = generate_agent_name(args.task, agent_type)
            
            session = await spawn_single_agent(
                app, current_session, agent_type, agent_name,
                project_dir, args.args, i, len(agent_types)
            )
            
            if session:
                spawned_sessions.append(session)
            
            # Delay between spawns (except for last one)
            if i < len(agent_types) - 1 and args.delay > 0:
                print(f"\n⏳ Waiting {args.delay}s before next spawn...")
                await asyncio.sleep(args.delay)
    
    elapsed = time.time() - start_time
    
    # Summary
    print(f"\n{'=' * 60}")
    print(f"✅ Multi-Agent Spawn Complete!")
    print(f"   Time: {elapsed:.1f}s")
    print(f"   Spawned: {len(spawned_sessions)}/{len(agent_types)} agents")
    
    if args.task:
        print(f"   Task: {args.task}")
    
    print(f"\n📊 Agent Summary:")
    for i, agent_type in enumerate(agent_types):
        agent_name = generate_agent_name(args.task, agent_type)
        status = "✅" if i < len(spawned_sessions) else "❌"
        print(f"   {status} {agent_type:10} → {agent_name}")
    
    # Show pane tree
    print(f"\n📊 Pane Structure:")
    tree = await PaneManager.get_pane_tree(app)
    
    def print_node(node_id, indent=0):
        if node_id not in tree['nodes']:
            return
        
        node = tree['nodes'][node_id]
        prefix = "  " * indent + ("├─ " if indent > 0 else "")
        label = node['label'] or 'unlabeled'
        agent = f"[{node['agent_type']}]" if node['agent_type'] else ""
        
        # Highlight spawned sessions
        is_new = any(s.session_id == node_id for s in spawned_sessions)
        marker = " ← NEW" if is_new else ""
        
        print(f"{prefix}{label} {agent}{marker}")
        
        for child_id in node['children_ids']:
            print_node(child_id, indent + 1)
    
    # Find root
    current_id = current_session.session_id
    root_id = current_id
    while root_id in tree['nodes'] and tree['nodes'][root_id]['parent_id']:
        root_id = tree['nodes'][root_id]['parent_id']
    
    print_node(root_id)
    
    print(f"\n💡 Tips:")
    print(f"   - Send messages: python3 send_to_pane.py --to <agent-name> --text 'message'")
    print(f"   - List agents: crewchief agent list")
    print(f"   - View tree: python3 pane_manager.py")


iterm2.run_until_complete(main)