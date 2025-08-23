#!/usr/bin/env python3
"""
Bridge between TypeScript CLI and iTerm2 Python scripts.
Provides a JSON-RPC server for TypeScript to communicate with iTerm2.
"""

import iterm2
import asyncio
import json
import sys
import os
from typing import Any, Dict, Optional
from aiohttp import web
from iterm_agent_manager import AgentManager
import logging

# Set up logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


class ITermBridge:
    """JSON-RPC bridge for iTerm2 operations."""
    
    def __init__(self, port: int = 8765):
        self.port = port
        self.manager: Optional[AgentManager] = None
        self.app = web.Application()
        self.setup_routes()
        
    def setup_routes(self):
        """Set up HTTP routes for JSON-RPC."""
        self.app.router.add_post('/rpc', self.handle_rpc)
        self.app.router.add_get('/health', self.health_check)
        
    async def start(self):
        """Start the bridge server."""
        logger.info(f"Starting iTerm2 bridge on port {self.port}")
        
        # Initialize agent manager
        self.manager = AgentManager()
        await self.manager.connect()
        
        # Start monitoring in background
        asyncio.create_task(self.monitor_agents())
        
        # Start web server
        runner = web.AppRunner(self.app)
        await runner.setup()
        site = web.TCPSite(runner, 'localhost', self.port)
        await site.start()
        
        logger.info(f"Bridge running on http://localhost:{self.port}")
        
    async def stop(self):
        """Stop the bridge server."""
        if self.manager:
            await self.manager.disconnect()
            
    async def health_check(self, request):
        """Health check endpoint."""
        return web.json_response({
            "status": "healthy",
            "connected": self.manager is not None
        })
        
    async def handle_rpc(self, request):
        """Handle JSON-RPC requests."""
        try:
            data = await request.json()
            method = data.get('method')
            params = data.get('params', {})
            rpc_id = data.get('id')
            
            logger.info(f"RPC request: {method}")
            
            # Route to appropriate method
            result = await self.dispatch_method(method, params)
            
            # Return response
            response = {
                "jsonrpc": "2.0",
                "result": result,
                "id": rpc_id
            }
            
            return web.json_response(response)
            
        except Exception as e:
            logger.error(f"RPC error: {e}")
            error_response = {
                "jsonrpc": "2.0",
                "error": {
                    "code": -32603,
                    "message": str(e)
                },
                "id": data.get('id') if 'data' in locals() else None
            }
            return web.json_response(error_response, status=500)
            
    async def dispatch_method(self, method: str, params: Dict) -> Any:
        """Dispatch RPC method to appropriate handler."""
        if not self.manager:
            raise RuntimeError("Bridge not initialized")
            
        handlers = {
            # Session management
            "createSession": self.create_session,
            "closeSession": self.close_session,
            "listSessions": self.list_sessions,
            
            # Agent management
            "createAgent": self.create_agent,
            "stopAgent": self.stop_agent,
            "sendTask": self.send_task,
            "getAgentOutput": self.get_agent_output,
            "listAgents": self.list_agents,
            "getAgentStatus": self.get_agent_status,
            
            # Command execution
            "sendCommand": self.send_command,
            "sendText": self.send_text,
            "getContents": self.get_contents,
            
            # Layout management
            "createAgentGrid": self.create_agent_grid,
            "splitPane": self.split_pane,
            
            # Utilities
            "setBadge": self.set_badge,
            "focusSession": self.focus_session,
            "broadcast": self.broadcast
        }
        
        handler = handlers.get(method)
        if not handler:
            raise ValueError(f"Unknown method: {method}")
            
        return await handler(params)
        
    # Session management methods
    async def create_session(self, params: Dict) -> Dict:
        """Create a new session."""
        profile = params.get('profile')
        window_id = await self.manager.controller.create_window(profile)
        sessions = await self.manager.controller.list_sessions()
        
        # Find the new session
        for session in sessions:
            if session['window_id'] == window_id:
                return session
                
        return {"window_id": window_id}
        
    async def close_session(self, params: Dict) -> bool:
        """Close a session."""
        session_id = params['sessionId']
        await self.manager.controller.close_session(session_id)
        return True
        
    async def list_sessions(self, params: Dict) -> list:
        """List all sessions."""
        return await self.manager.controller.list_sessions()
        
    # Agent management methods
    async def create_agent(self, params: Dict) -> Dict:
        """Create a new agent."""
        agent_id = params['agentId']
        agent_type = params.get('agentType', 'worker')
        working_dir = params.get('workingDir')
        
        agent = await self.manager.create_agent(
            agent_id, agent_type, working_dir
        )
        
        return {
            "agentId": agent.agent_id,
            "sessionId": agent.session_info.session_id,
            "status": agent.status,
            "workingDir": agent.working_dir
        }
        
    async def stop_agent(self, params: Dict) -> bool:
        """Stop an agent."""
        agent_id = params['agentId']
        await self.manager.stop_agent(agent_id)
        return True
        
    async def send_task(self, params: Dict) -> bool:
        """Send a task to an agent."""
        agent_id = params['agentId']
        task = params['task']
        await self.manager.send_task(agent_id, task)
        return True
        
    async def get_agent_output(self, params: Dict) -> str:
        """Get agent output."""
        agent_id = params['agentId']
        lines = params.get('lines', 100)
        return await self.manager.get_agent_output(agent_id, lines)
        
    async def list_agents(self, params: Dict) -> list:
        """List all agents."""
        return await self.manager.list_agents()
        
    async def get_agent_status(self, params: Dict) -> Dict:
        """Get agent status."""
        agent_id = params['agentId']
        return await self.manager.get_agent_status(agent_id)
        
    # Command execution methods
    async def send_command(self, params: Dict) -> bool:
        """Send a command to a session."""
        session_id = params['sessionId']
        command = params['command']
        await self.manager.controller.send_command(session_id, command)
        return True
        
    async def send_text(self, params: Dict) -> bool:
        """Send text to a session."""
        session_id = params['sessionId']
        text = params['text']
        await self.manager.controller.send_text(session_id, text)
        return True
        
    async def get_contents(self, params: Dict) -> str:
        """Get session contents."""
        session_id = params['sessionId']
        lines = params.get('lines', 1000)
        return await self.manager.controller.get_contents(session_id, lines)
        
    # Layout management methods
    async def create_agent_grid(self, params: Dict) -> str:
        """Create an agent grid layout."""
        agent_ids = params['agentIds']
        rows = params.get('rows', 2)
        cols = params.get('cols', 2)
        
        window_id = await self.manager.create_agent_grid(
            agent_ids, rows, cols
        )
        return window_id
        
    async def split_pane(self, params: Dict) -> str:
        """Split a pane."""
        session_id = params['sessionId']
        vertical = params.get('vertical', True)
        before = params.get('before', False)
        
        new_session_id = await self.manager.controller.split_pane(
            session_id, vertical, before
        )
        return new_session_id
        
    # Utility methods
    async def set_badge(self, params: Dict) -> bool:
        """Set a badge on a session."""
        session_id = params['sessionId']
        badge = params['badge']
        await self.manager.controller.set_badge(session_id, badge)
        return True
        
    async def focus_session(self, params: Dict) -> bool:
        """Focus a session."""
        session_id = params['sessionId']
        await self.manager.controller.focus_session(session_id)
        return True
        
    async def broadcast(self, params: Dict) -> bool:
        """Broadcast command to agents."""
        agent_ids = params['agentIds']
        command = params['command']
        await self.manager.broadcast_to_agents(agent_ids, command)
        return True
        
    async def monitor_agents(self):
        """Monitor agents and send updates."""
        async def callback(agent_id: str, output: str):
            # In a real implementation, this would send updates
            # via WebSocket or Server-Sent Events
            logger.debug(f"Agent {agent_id} update: {len(output)} chars")
            
        await self.manager.monitor_agents(callback)


async def main():
    """Main entry point."""
    bridge = ITermBridge()
    
    try:
        await bridge.start()
        
        # Keep running
        while True:
            await asyncio.sleep(1)
            
    except KeyboardInterrupt:
        logger.info("Shutting down bridge...")
    finally:
        await bridge.stop()


def run_bridge():
    """Run the bridge server."""
    # This is called from iTerm2's Python runtime
    iterm2.run_forever(main)