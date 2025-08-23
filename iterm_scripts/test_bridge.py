#!/usr/bin/env python3
"""
Test script for the iTerm2 bridge functionality.
Run this to verify the bridge is working correctly.
"""

import asyncio
import json
import sys
from typing import Dict, Any
import aiohttp


class BridgeTester:
    """Test client for the iTerm2 bridge."""
    
    def __init__(self, host: str = "localhost", port: int = 8765):
        self.host = host
        self.port = port
        self.base_url = f"http://{host}:{port}"
        self.rpc_id = 1
        
    async def call_rpc(self, method: str, params: Dict[str, Any] = None) -> Any:
        """Make an RPC call to the bridge."""
        request = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params or {},
            "id": self.rpc_id
        }
        self.rpc_id += 1
        
        async with aiohttp.ClientSession() as session:
            async with session.post(f"{self.base_url}/rpc", json=request) as response:
                data = await response.json()
                
                if "error" in data:
                    print(f"❌ Error in {method}: {data['error']['message']}")
                    return None
                    
                return data.get("result")
                
    async def check_health(self) -> bool:
        """Check if the bridge is healthy."""
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(f"{self.base_url}/health") as response:
                    if response.status == 200:
                        data = await response.json()
                        return data.get("status") == "healthy"
        except:
            return False
        return False
        
    async def run_tests(self):
        """Run all tests."""
        print("🧪 iTerm2 Bridge Test Suite")
        print("=" * 50)
        
        # Check health
        print("\n1. Checking bridge health...")
        if not await self.check_health():
            print("❌ Bridge is not healthy. Is it running?")
            print("   Run: python3 iterm_bridge.py")
            return
        print("✅ Bridge is healthy")
        
        # List sessions
        print("\n2. Listing sessions...")
        sessions = await self.call_rpc("listSessions")
        if sessions is not None:
            print(f"✅ Found {len(sessions)} sessions")
            for session in sessions[:3]:  # Show first 3
                print(f"   - Session: {session.get('sessionId', 'unknown')}")
        
        # Create a test agent
        print("\n3. Creating test agent...")
        agent_result = await self.call_rpc("createAgent", {
            "agentId": "test-agent-001",
            "agentType": "test",
            "workingDir": "/tmp"
        })
        
        if agent_result:
            print(f"✅ Created agent: {agent_result.get('agentId')}")
            print(f"   Session ID: {agent_result.get('sessionId')}")
            print(f"   Status: {agent_result.get('status')}")
            
            agent_id = agent_result.get('agentId')
            session_id = agent_result.get('sessionId')
            
            # Send a command to the agent
            print("\n4. Sending command to agent...")
            await self.call_rpc("sendCommand", {
                "sessionId": session_id,
                "command": "echo 'Hello from test agent!'"
            })
            print("✅ Command sent")
            
            # Get agent output
            print("\n5. Getting agent output...")
            await asyncio.sleep(1)  # Wait for command to execute
            output = await self.call_rpc("getAgentOutput", {
                "agentId": agent_id,
                "lines": 20
            })
            if output:
                print("✅ Got output:")
                print("   " + "\n   ".join(output.split("\n")[:5]))  # Show first 5 lines
            
            # Set a badge
            print("\n6. Setting badge...")
            await self.call_rpc("setBadge", {
                "sessionId": session_id,
                "badge": "TEST ✓"
            })
            print("✅ Badge set")
            
            # List agents
            print("\n7. Listing agents...")
            agents = await self.call_rpc("listAgents")
            if agents:
                print(f"✅ Found {len(agents)} agents")
                for agent in agents:
                    print(f"   - {agent.get('agentId')}: {agent.get('status')}")
            
            # Clean up - stop the test agent
            print("\n8. Stopping test agent...")
            await self.call_rpc("stopAgent", {"agentId": agent_id})
            print("✅ Agent stopped")
            
        print("\n" + "=" * 50)
        print("✅ All tests completed!")
        
    async def interactive_test(self):
        """Run an interactive test session."""
        print("\n🎮 Interactive Test Mode")
        print("=" * 50)
        print("Commands:")
        print("  create <agent_id>  - Create a new agent")
        print("  send <agent_id> <cmd> - Send command to agent")
        print("  output <agent_id>  - Get agent output")
        print("  list               - List all agents")
        print("  stop <agent_id>    - Stop an agent")
        print("  quit               - Exit")
        print("=" * 50)
        
        while True:
            try:
                cmd = input("\n> ").strip().split(None, 2)
                
                if not cmd:
                    continue
                    
                if cmd[0] == "quit":
                    break
                    
                elif cmd[0] == "create" and len(cmd) >= 2:
                    result = await self.call_rpc("createAgent", {
                        "agentId": cmd[1],
                        "agentType": "interactive"
                    })
                    if result:
                        print(f"Created agent {cmd[1]}")
                        
                elif cmd[0] == "send" and len(cmd) >= 3:
                    agents = await self.call_rpc("listAgents")
                    agent = next((a for a in agents if a["agentId"] == cmd[1]), None)
                    if agent:
                        await self.call_rpc("sendCommand", {
                            "sessionId": agent["sessionId"],
                            "command": cmd[2]
                        })
                        print(f"Sent command to {cmd[1]}")
                    else:
                        print(f"Agent {cmd[1]} not found")
                        
                elif cmd[0] == "output" and len(cmd) >= 2:
                    output = await self.call_rpc("getAgentOutput", {
                        "agentId": cmd[1],
                        "lines": 50
                    })
                    if output:
                        print(output)
                        
                elif cmd[0] == "list":
                    agents = await self.call_rpc("listAgents")
                    if agents:
                        for agent in agents:
                            print(f"- {agent['agentId']}: {agent['status']}")
                    else:
                        print("No agents found")
                        
                elif cmd[0] == "stop" and len(cmd) >= 2:
                    await self.call_rpc("stopAgent", {"agentId": cmd[1]})
                    print(f"Stopped agent {cmd[1]}")
                    
                else:
                    print("Invalid command")
                    
            except KeyboardInterrupt:
                break
            except Exception as e:
                print(f"Error: {e}")
                
        print("\nGoodbye!")


async def main():
    """Main entry point."""
    tester = BridgeTester()
    
    if len(sys.argv) > 1 and sys.argv[1] == "--interactive":
        await tester.interactive_test()
    else:
        await tester.run_tests()


if __name__ == "__main__":
    asyncio.run(main())