#!/usr/bin/env python3
"""
Agent-specific management for iTerm2 integration.
Handles agent lifecycle, command execution, and monitoring.
"""

import iterm2
import asyncio
import json
import os
from typing import Optional, Dict, List, Any
from dataclasses import dataclass, asdict
from datetime import datetime
from iterm_controller import ITermController, SessionInfo


@dataclass
class AgentSession:
    """Represents an agent's terminal session."""
    agent_id: str
    session_info: SessionInfo
    status: str  # 'running', 'idle', 'stopped'
    created_at: str
    working_dir: str
    log_file: Optional[str] = None
    output_buffer: List[str] = None
    
    def __post_init__(self):
        if self.output_buffer is None:
            self.output_buffer = []


class AgentManager:
    """Manages agent sessions in iTerm2."""
    
    def __init__(self, base_dir: str = ".crewchief/worktrees"):
        self.controller = ITermController("CrewChief-Agents")
        self.base_dir = base_dir
        self.agents: Dict[str, AgentSession] = {}
        self.monitoring = False
        
    async def connect(self):
        """Connect to iTerm2."""
        await self.controller.connect()
        
    async def disconnect(self):
        """Disconnect from iTerm2."""
        self.monitoring = False
        await self.controller.disconnect()
        
    async def create_agent(self, agent_id: str, agent_type: str = "worker",
                          working_dir: Optional[str] = None) -> AgentSession:
        """Create a new agent session."""
        # Determine working directory
        if not working_dir:
            working_dir = os.path.join(self.base_dir, agent_id)
            
        # Create agent workspace
        workspace = await self.controller.create_agent_workspace(
            agent_id, working_dir
        )
        
        # Create session info
        session_info = SessionInfo(**workspace)
        
        # Set agent-specific variables
        await self.controller.set_variable(
            session_info.session_id, 
            "agent_id", 
            agent_id
        )
        await self.controller.set_variable(
            session_info.session_id,
            "agent_type",
            agent_type
        )
        
        # Set visual indicators
        badge = f"{agent_type.upper()}: {agent_id[:8]}"
        await self.controller.set_badge(session_info.session_id, badge)
        
        # Create agent session
        agent_session = AgentSession(
            agent_id=agent_id,
            session_info=session_info,
            status="idle",
            created_at=datetime.now().isoformat(),
            working_dir=working_dir
        )
        
        self.agents[agent_id] = agent_session
        
        # Initialize agent environment
        await self._initialize_agent_env(agent_session)
        
        return agent_session
        
    async def _initialize_agent_env(self, agent: AgentSession):
        """Initialize the agent's environment."""
        session_id = agent.session_info.session_id
        
        # Clear screen
        await self.controller.send_text(session_id, "\x0c")  # Ctrl+L
        
        # Set up environment
        commands = [
            f"cd {agent.working_dir}",
            "export AGENT_ID=" + agent.agent_id,
            "export CREWCHIEF_MODE=agent",
            "clear",
            f"echo '=== Agent {agent.agent_id} initialized ==='"
        ]
        
        for cmd in commands:
            await self.controller.send_command(session_id, cmd)
            await asyncio.sleep(0.1)
            
    async def send_task(self, agent_id: str, task: Dict[str, Any]):
        """Send a task to an agent."""
        if agent_id not in self.agents:
            raise ValueError(f"Agent {agent_id} not found")
            
        agent = self.agents[agent_id]
        session_id = agent.session_info.session_id
        
        # Update status
        agent.status = "running"
        await self.controller.set_badge(session_id, f"BUSY: {agent_id[:8]}")
        
        # Format and send task
        task_json = json.dumps(task)
        command = f"crewchief agent execute --task '{task_json}'"
        
        await self.controller.send_command(session_id, command)
        
    async def get_agent_output(self, agent_id: str, 
                              lines: int = 100) -> str:
        """Get recent output from an agent."""
        if agent_id not in self.agents:
            raise ValueError(f"Agent {agent_id} not found")
            
        agent = self.agents[agent_id]
        session_id = agent.session_info.session_id
        
        return await self.controller.get_contents(session_id, lines)
        
    async def stop_agent(self, agent_id: str):
        """Stop an agent and close its session."""
        if agent_id not in self.agents:
            raise ValueError(f"Agent {agent_id} not found")
            
        agent = self.agents[agent_id]
        session_id = agent.session_info.session_id
        
        # Send interrupt signal
        await self.controller.send_text(session_id, "\x03")  # Ctrl+C
        
        # Wait a bit
        await asyncio.sleep(0.5)
        
        # Close session
        await self.controller.close_session(session_id)
        
        # Update status
        agent.status = "stopped"
        del self.agents[agent_id]
        
    async def broadcast_to_agents(self, agent_ids: List[str], 
                                 command: str):
        """Broadcast a command to multiple agents."""
        tasks = []
        for agent_id in agent_ids:
            if agent_id in self.agents:
                agent = self.agents[agent_id]
                session_id = agent.session_info.session_id
                tasks.append(
                    self.controller.send_command(session_id, command)
                )
                
        await asyncio.gather(*tasks)
        
    async def monitor_agents(self, callback=None):
        """Monitor all agents for output changes."""
        self.monitoring = True
        
        while self.monitoring:
            for agent_id, agent in self.agents.items():
                if agent.status == "running":
                    try:
                        output = await self.get_agent_output(agent_id, 50)
                        
                        # Check for completion markers
                        if "=== Task completed ===" in output:
                            agent.status = "idle"
                            badge = f"IDLE: {agent_id[:8]}"
                            await self.controller.set_badge(
                                agent.session_info.session_id, 
                                badge
                            )
                            
                        # Callback with updates
                        if callback:
                            await callback(agent_id, output)
                            
                    except Exception as e:
                        print(f"Error monitoring {agent_id}: {e}")
                        
            await asyncio.sleep(1)
            
    async def create_agent_grid(self, agent_ids: List[str], 
                               rows: int = 2, cols: int = 2) -> str:
        """Create a grid layout of agent sessions."""
        if len(agent_ids) > rows * cols:
            raise ValueError(f"Too many agents for {rows}x{cols} grid")
            
        # Create a new window for the grid
        window_id = await self.controller.create_window()
        
        # Get the first tab of the new window
        app = self.controller.app
        window = None
        for w in app.terminal_windows:
            if w.window_id == window_id:
                window = w
                break
                
        if not window or not window.tabs:
            raise RuntimeError("Failed to create agent grid")
            
        tab = window.tabs[0]
        base_session = tab.sessions[0]
        
        # Create grid of panes
        sessions = [base_session]
        
        # Split horizontally for rows
        for row in range(1, rows):
            new_session = await sessions[0].async_split_pane(
                vertical=False
            )
            sessions.append(new_session)
            
        # Split each row vertically for columns
        final_sessions = []
        for row_session in sessions:
            row_sessions = [row_session]
            for col in range(1, cols):
                new_session = await row_session.async_split_pane(
                    vertical=True
                )
                row_sessions.append(new_session)
            final_sessions.extend(row_sessions)
            
        # Assign agents to panes
        for i, agent_id in enumerate(agent_ids):
            if i < len(final_sessions):
                session = final_sessions[i]
                
                # Create agent in this session
                working_dir = os.path.join(self.base_dir, agent_id)
                
                # Set up the session for the agent
                await self.controller.send_command(
                    session.session_id,
                    f"cd {working_dir}"
                )
                await self.controller.set_badge(
                    session.session_id,
                    f"Agent: {agent_id[:8]}"
                )
                
                # Store agent info
                session_info = SessionInfo(
                    session_id=session.session_id,
                    tab_id=tab.tab_id,
                    window_id=window_id,
                    name=agent_id
                )
                
                agent_session = AgentSession(
                    agent_id=agent_id,
                    session_info=session_info,
                    status="idle",
                    created_at=datetime.now().isoformat(),
                    working_dir=working_dir
                )
                
                self.agents[agent_id] = agent_session
                
        return window_id
        
    async def get_agent_status(self, agent_id: str) -> Dict:
        """Get the status of a specific agent."""
        if agent_id not in self.agents:
            raise ValueError(f"Agent {agent_id} not found")
            
        agent = self.agents[agent_id]
        return {
            "agent_id": agent.agent_id,
            "status": agent.status,
            "created_at": agent.created_at,
            "working_dir": agent.working_dir,
            "session_id": agent.session_info.session_id
        }
        
    async def list_agents(self) -> List[Dict]:
        """List all active agents."""
        return [
            await self.get_agent_status(agent_id)
            for agent_id in self.agents.keys()
        ]


async def main():
    """Test the agent manager."""
    manager = AgentManager()
    
    try:
        await manager.connect()
        
        # Create a few test agents
        agent1 = await manager.create_agent("agent-001", "worker")
        print(f"Created agent: {agent1.agent_id}")
        
        agent2 = await manager.create_agent("agent-002", "reviewer")
        print(f"Created agent: {agent2.agent_id}")
        
        # Send a task
        task = {
            "type": "code_review",
            "files": ["main.py", "utils.py"]
        }
        await manager.send_task("agent-001", task)
        
        # Wait a bit
        await asyncio.sleep(2)
        
        # Get output
        output = await manager.get_agent_output("agent-001")
        print(f"Agent output:\n{output}")
        
        # List agents
        agents = await manager.list_agents()
        print(f"Active agents: {json.dumps(agents, indent=2)}")
        
    finally:
        await manager.disconnect()


if __name__ == "__main__":
    asyncio.run(main())