#!/usr/bin/env python3
"""
Configuration for agent-specific Enter key methods.
Each agent CLI may require different key sequences to submit text.
"""

# Agent Enter key mappings
# chr(13) = ASCII Carriage Return (works for Claude)
# "\n" = Newline
# "\n\n" = Double newline
# "\r" = Carriage return string
# "\r\n" = CRLF

AGENT_ENTER_KEYS = {
    # Confirmed working
    "claude": chr(13),  # ASCII Enter key - confirmed working
    
    # Set all others to same as Claude initially
    # Update these as we discover what works for each
    "gemini": chr(13),
    "codex": chr(13),
    "cursor": chr(13),
    "aider": chr(13),
    "custom": chr(13),
    
    # Default for unknown agents
    "default": chr(13),
}

def get_enter_key(agent_type: str) -> str:
    """
    Get the appropriate Enter key sequence for an agent type.
    
    Args:
        agent_type: The agent type (e.g., 'claude', 'gemini', etc.)
    
    Returns:
        The Enter key sequence to use for submitting text to that agent
    """
    return AGENT_ENTER_KEYS.get(agent_type.lower(), AGENT_ENTER_KEYS["default"])