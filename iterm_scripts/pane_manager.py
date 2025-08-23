#!/usr/bin/env python3
"""
Intelligent pane management for CrewChief agent spawning.

Layout Rules:
1. First spawn from a pane: Vertical split (parent left, child right)
2. Additional spawns from same parent: Horizontal split of the right pane
3. Spawns from sub-agents: Vertical split (parent left, new child right)
4. Handles manual pane operations gracefully
"""

import iterm2
import asyncio
import json
from typing import Optional, List, Dict, Tuple


class PaneManager:
    """Manages hierarchical pane layouts for agent spawning."""
    
    @staticmethod
    async def get_pane_info(session) -> Dict:
        """Get information about a pane including position and user variables."""
        info = {
            'session_id': session.session_id,
            'label': None,
            'parent_id': None,
            'children_ids': [],
            'agent_type': None,
            'spawn_count': 0
        }
        
        try:
            info['label'] = await session.async_get_variable("user.pane_label")
            info['parent_id'] = await session.async_get_variable("user.parent_pane_id")
            children_json = await session.async_get_variable("user.children_pane_ids")
            if children_json:
                info['children_ids'] = json.loads(children_json)
            info['agent_type'] = await session.async_get_variable("user.agent_type")
            spawn_count = await session.async_get_variable("user.spawn_count")
            if spawn_count:
                info['spawn_count'] = int(spawn_count)
        except:
            pass
        
        return info
    
    @staticmethod
    async def find_rightmost_child(app, parent_session) -> Optional[object]:
        """Find the rightmost child pane of a parent."""
        parent_info = await PaneManager.get_pane_info(parent_session)
        
        if not parent_info['children_ids']:
            return None
        
        # Find all child sessions
        child_sessions = []
        for window in app.terminal_windows:
            for tab in window.tabs:
                for session in tab.sessions:
                    if session.session_id in parent_info['children_ids']:
                        child_sessions.append(session)
        
        if not child_sessions:
            return None
        
        # For now, return the first child (we'll enhance this with position detection later)
        # In a more sophisticated version, we'd check actual pane positions
        return child_sessions[0]
    
    @staticmethod
    async def find_children_container(app, parent_session) -> Optional[object]:
        """
        Find the container (pane) where children of this parent should be added.
        If parent already has children, find the rightmost one to split.
        """
        parent_info = await PaneManager.get_pane_info(parent_session)
        
        # If parent has no children yet, return None (will do vertical split)
        if not parent_info['children_ids']:
            return None
        
        # Find the container for children (rightmost child or right pane)
        rightmost = await PaneManager.find_rightmost_child(app, parent_session)
        return rightmost
    
    @staticmethod
    async def update_parent_children(parent_session, new_child_id: str):
        """Update the parent's list of children."""
        try:
            children_json = await parent_session.async_get_variable("user.children_pane_ids")
            children = json.loads(children_json) if children_json else []
        except:
            children = []
        
        if new_child_id not in children:
            children.append(new_child_id)
        
        await parent_session.async_set_variable("user.children_pane_ids", json.dumps(children))
        
        # Increment spawn count
        try:
            spawn_count = await parent_session.async_get_variable("user.spawn_count")
            spawn_count = int(spawn_count) if spawn_count else 0
        except:
            spawn_count = 0
        
        await parent_session.async_set_variable("user.spawn_count", str(spawn_count + 1))
    
    @staticmethod
    async def determine_split_strategy(app, spawning_session) -> Tuple[object, bool]:
        """
        Determine where and how to split for a new agent.
        
        Returns: (session_to_split, is_vertical)
        - session_to_split: The session that should be split
        - is_vertical: True for vertical split, False for horizontal
        """
        spawning_info = await PaneManager.get_pane_info(spawning_session)
        
        # Check if spawning pane already has children
        if spawning_info['children_ids']:
            # Parent already has children - find the rightmost child and split it horizontally
            container = await PaneManager.find_children_container(app, spawning_session)
            if container:
                # Split the children container horizontally
                return (container, False)  # horizontal split
            else:
                # Fallback: split parent vertically
                return (spawning_session, True)  # vertical split
        else:
            # Parent has no children yet - split it vertically
            return (spawning_session, True)  # vertical split
    
    @staticmethod
    async def setup_new_session(new_session, parent_session, agent_name: str, agent_type: str):
        """Set up user variables for the new session."""
        parent_id = parent_session.session_id if parent_session else None
        
        # Set up the new session's metadata
        await new_session.async_set_variable("user.pane_label", agent_name)
        await new_session.async_set_variable("user.agent_type", agent_type)
        if parent_id:
            await new_session.async_set_variable("user.parent_pane_id", parent_id)
        await new_session.async_set_variable("user.children_pane_ids", json.dumps([]))
        await new_session.async_set_variable("user.spawn_count", "0")
        
        # Update parent's children list
        if parent_session:
            await PaneManager.update_parent_children(parent_session, new_session.session_id)
    
    @staticmethod
    async def get_pane_tree(app) -> Dict:
        """Build a tree structure of all panes and their relationships."""
        tree = {
            'roots': [],  # Panes with no parent
            'nodes': {}   # All panes indexed by session_id
        }
        
        for window in app.terminal_windows:
            for tab in window.tabs:
                for session in tab.sessions:
                    info = await PaneManager.get_pane_info(session)
                    tree['nodes'][info['session_id']] = info
                    
                    if not info['parent_id']:
                        tree['roots'].append(info['session_id'])
        
        return tree
    
    @staticmethod
    async def cleanup_orphaned_references(app):
        """Clean up references to closed panes."""
        tree = await PaneManager.get_pane_tree(app)
        
        # For each node, check if its children still exist
        for session_id, info in tree['nodes'].items():
            if info['children_ids']:
                valid_children = [
                    child_id for child_id in info['children_ids']
                    if child_id in tree['nodes']
                ]
                
                if valid_children != info['children_ids']:
                    # Update the children list
                    for window in app.terminal_windows:
                        for tab in window.tabs:
                            for session in tab.sessions:
                                if session.session_id == session_id:
                                    await session.async_set_variable(
                                        "user.children_pane_ids",
                                        json.dumps(valid_children)
                                    )
                                    break


async def main(connection):
    """Test/debug the pane manager."""
    app = await iterm2.async_get_app(connection)
    
    # Get the tree structure
    tree = await PaneManager.get_pane_tree(app)
    
    print("\n📊 Pane Tree Structure:")
    print("=" * 60)
    
    def print_node(node_id, indent=0):
        if node_id not in tree['nodes']:
            return
        
        node = tree['nodes'][node_id]
        prefix = "  " * indent + ("├─ " if indent > 0 else "")
        label = node['label'] or 'unlabeled'
        agent = f"[{node['agent_type']}]" if node['agent_type'] else ""
        children = f"({len(node['children_ids'])} children)" if node['children_ids'] else ""
        
        print(f"{prefix}{label} {agent} {children}")
        
        for child_id in node['children_ids']:
            print_node(child_id, indent + 1)
    
    if tree['roots']:
        for root_id in tree['roots']:
            print_node(root_id)
    else:
        print("No panes with tracking information found")
    
    print("=" * 60)
    
    # Clean up orphaned references
    await PaneManager.cleanup_orphaned_references(app)
    print("\n✅ Cleaned up orphaned pane references")


if __name__ == "__main__":
    iterm2.run_until_complete(main)