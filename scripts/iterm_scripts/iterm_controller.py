#!/usr/bin/env python3
"""
Core iTerm2 controller for CrewChief.
Provides low-level iTerm2 API operations for session management.
"""

import iterm2
import asyncio
import json
import sys
from typing import Optional, Dict, List, Tuple
from dataclasses import dataclass, asdict


@dataclass
class SessionInfo:
    """Information about an iTerm2 session."""
    session_id: str
    tab_id: str
    window_id: str
    name: Optional[str] = None
    profile: Optional[str] = None


class ITermController:
    """Core controller for iTerm2 operations."""
    
    def __init__(self, app_name: str = "CrewChief"):
        self.app_name = app_name
        self.connection: Optional[iterm2.Connection] = None
        self.app: Optional[iterm2.App] = None
        self.sessions: Dict[str, SessionInfo] = {}
        
    async def connect(self):
        """Establish connection to iTerm2."""
        self.connection = await iterm2.Connection.async_create()
        self.app = await iterm2.async_get_app(self.connection)
        
    async def disconnect(self):
        """Close connection to iTerm2."""
        if self.connection:
            await self.connection.async_close()
            
    async def create_window(self, profile: Optional[str] = None) -> str:
        """Create a new iTerm2 window."""
        if not self.app:
            raise RuntimeError("Not connected to iTerm2")
            
        window = await iterm2.Window.async_create(
            self.connection,
            profile=profile or "Default"
        )
        return window.window_id
        
    async def create_tab(self, window_id: Optional[str] = None, 
                        profile: Optional[str] = None) -> str:
        """Create a new tab in specified window or current window."""
        if not self.app:
            raise RuntimeError("Not connected to iTerm2")
            
        window = None
        if window_id:
            for w in self.app.terminal_windows:
                if w.window_id == window_id:
                    window = w
                    break
        else:
            window = self.app.current_terminal_window
            
        if not window:
            raise ValueError(f"Window {window_id} not found")
            
        tab = await window.async_create_tab(profile=profile)
        return tab.tab_id
        
    async def split_pane(self, session_id: str, vertical: bool = True,
                        before: bool = False) -> str:
        """Split a pane to create a new session."""
        session = await self._get_session(session_id)
        if not session:
            raise ValueError(f"Session {session_id} not found")
            
        new_session = await session.async_split_pane(
            vertical=vertical,
            before=before
        )
        return new_session.session_id
        
    async def send_text(self, session_id: str, text: str):
        """Send text to a specific session."""
        session = await self._get_session(session_id)
        if not session:
            raise ValueError(f"Session {session_id} not found")
            
        await session.async_send_text(text)
        
    async def send_command(self, session_id: str, command: str):
        """Send a command (with newline) to a specific session."""
        await self.send_text(session_id, f"{command}\n")
        
    async def get_contents(self, session_id: str, 
                          history_lines: int = 1000) -> str:
        """Get the contents of a session's screen and scrollback."""
        session = await self._get_session(session_id)
        if not session:
            raise ValueError(f"Session {session_id} not found")
            
        # Get screen contents
        screen_contents = await session.async_get_screen_contents()
        
        # Get scrollback history
        history = await session.async_get_line_info()
        
        lines = []
        start = max(0, len(history) - history_lines)
        for line_info in history[start:]:
            lines.append(line_info.string)
            
        return "\n".join(lines)
        
    async def set_variable(self, session_id: str, name: str, value: str):
        """Set a user-defined variable in a session."""
        session = await self._get_session(session_id)
        if not session:
            raise ValueError(f"Session {session_id} not found")
            
        await session.async_set_variable(name, value)
        
    async def get_variable(self, session_id: str, name: str) -> Optional[str]:
        """Get a user-defined variable from a session."""
        session = await self._get_session(session_id)
        if not session:
            return None
            
        try:
            return await session.async_get_variable(name)
        except:
            return None
            
    async def set_badge(self, session_id: str, badge: str):
        """Set a badge on a session."""
        session = await self._get_session(session_id)
        if not session:
            raise ValueError(f"Session {session_id} not found")
            
        await session.async_set_badge_text(badge)
        
    async def close_session(self, session_id: str):
        """Close a specific session."""
        session = await self._get_session(session_id)
        if not session:
            raise ValueError(f"Session {session_id} not found")
            
        await session.async_close(force=True)
        
    async def _get_session(self, session_id: str) -> Optional[iterm2.Session]:
        """Get a session by ID."""
        if not self.app:
            return None
            
        for window in self.app.terminal_windows:
            for tab in window.tabs:
                for session in tab.sessions:
                    if session.session_id == session_id:
                        return session
        return None
        
    async def list_sessions(self) -> List[Dict]:
        """List all sessions with their info."""
        if not self.app:
            return []
            
        sessions = []
        for window in self.app.terminal_windows:
            for tab in window.tabs:
                for session in tab.sessions:
                    info = {
                        "session_id": session.session_id,
                        "tab_id": tab.tab_id,
                        "window_id": window.window_id,
                        "name": session.name
                    }
                    sessions.append(info)
        return sessions
        
    async def create_agent_workspace(self, agent_id: str, 
                                    working_dir: Optional[str] = None) -> Dict:
        """Create a dedicated workspace for an agent."""
        # Create a new tab for the agent
        tab_id = await self.create_tab()
        
        # Get the tab's default session
        tab = None
        for window in self.app.terminal_windows:
            for t in window.tabs:
                if t.tab_id == tab_id:
                    tab = t
                    break
            if tab:
                break
                
        if not tab or not tab.sessions:
            raise RuntimeError("Failed to create agent workspace")
            
        main_session = tab.sessions[0]
        main_session_id = main_session.session_id
        
        # Set working directory if provided
        if working_dir:
            await self.send_command(main_session_id, f"cd {working_dir}")
            
        # Set badge to identify agent
        await self.set_badge(main_session_id, f"Agent: {agent_id}")
        
        # Store session info
        info = SessionInfo(
            session_id=main_session_id,
            tab_id=tab_id,
            window_id=tab.window.window_id,
            name=agent_id
        )
        self.sessions[agent_id] = info
        
        return asdict(info)
        
    async def focus_session(self, session_id: str):
        """Focus on a specific session."""
        session = await self._get_session(session_id)
        if not session:
            raise ValueError(f"Session {session_id} not found")
            
        await session.async_activate()


async def main():
    """Main entry point for testing."""
    controller = ITermController()
    
    try:
        await controller.connect()
        
        # Example: Create a new window
        window_id = await controller.create_window()
        print(f"Created window: {window_id}")
        
        # List all sessions
        sessions = await controller.list_sessions()
        print(f"Sessions: {json.dumps(sessions, indent=2)}")
        
    finally:
        await controller.disconnect()


if __name__ == "__main__":
    asyncio.run(main())