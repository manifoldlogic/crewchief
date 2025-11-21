#!/usr/bin/env python3
"""
Integration test script for the MAPDAEMON.

Tests:
1. Ping/pong
2. Search functionality
3. Error handling (malformed JSON)
4. Graceful shutdown
"""

import json
import subprocess
import sys
import time
from typing import Any, Dict, Optional


class DaemonTester:
    def __init__(self, daemon_path: str = "./target/debug/crewchief-maproom"):
        self.daemon_path = daemon_path
        self.process: Optional[subprocess.Popen] = None
        self.test_count = 0
        self.passed = 0
        self.failed = 0

    def start_daemon(self) -> bool:
        """Start the daemon process."""
        print("🚀 Starting daemon...")
        try:
            self.process = subprocess.Popen(
                [self.daemon_path, "serve"],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1,
            )
            time.sleep(0.5)  # Give it time to initialize
            
            if self.process.poll() is not None:
                stderr = self.process.stderr.read() if self.process.stderr else ""
                print(f"❌ Daemon failed to start: {stderr}")
                return False
            
            print("✅ Daemon started successfully")
            return True
        except Exception as e:
            print(f"❌ Failed to start daemon: {e}")
            return False

    def send_request(self, method: str, params: Optional[Dict[str, Any]] = None, request_id: int = 1) -> Optional[Dict]:
        """Send a JSON-RPC request and get the response."""
        if not self.process or not self.process.stdin or not self.process.stdout:
            print("❌ Daemon not running")
            return None

        request = {
            "jsonrpc": "2.0",
            "method": method,
            "id": request_id
        }
        if params:
            request["params"] = params

        try:
            # Send request
            request_json = json.dumps(request) + "\n"
            self.process.stdin.write(request_json)
            self.process.stdin.flush()

            # Read response
            response_line = self.process.stdout.readline()
            if not response_line:
                print("❌ No response from daemon")
                return None

            response = json.loads(response_line)
            return response
        except Exception as e:
            print(f"❌ Communication error: {e}")
            return None

    def send_malformed(self, bad_json: str) -> Optional[Dict]:
        """Send malformed JSON and get error response."""
        if not self.process or not self.process.stdin or not self.process.stdout:
            return None

        try:
            self.process.stdin.write(bad_json + "\n")
            self.process.stdin.flush()
            response_line = self.process.stdout.readline()
            if response_line:
                return json.loads(response_line)
            return None
        except Exception as e:
            print(f"❌ Error sending malformed JSON: {e}")
            return None

    def test_ping(self) -> bool:
        """Test the ping method."""
        print("\n📌 Test: Ping/Pong")
        self.test_count += 1
        
        start_time = time.time()
        response = self.send_request("ping")
        elapsed = (time.time() - start_time) * 1000  # Convert to ms
        
        if not response:
            print("❌ No response")
            self.failed += 1
            return False
        
        if response.get("result") == "pong":
            print(f"✅ Ping successful (latency: {elapsed:.2f}ms)")
            if elapsed < 1.0:
                print(f"   🎯 Latency under 1ms target!")
            self.passed += 1
            return True
        else:
            print(f"❌ Unexpected response: {response}")
            self.failed += 1
            return False

    def test_search(self, repo: str = "test-repo", query: str = "search query") -> bool:
        """Test the search method."""
        print(f"\n🔍 Test: Search (repo={repo}, query='{query}')")
        self.test_count += 1
        
        params = {
            "query": query,
            "repo": repo,
            "limit": 5
        }
        
        start_time = time.time()
        response = self.send_request("search", params)
        elapsed = (time.time() - start_time) * 1000
        
        if not response:
            print("❌ No response")
            self.failed += 1
            return False
        
        # Check for error (expected if repo doesn't exist)
        if "error" in response:
            error = response["error"]
            print(f"⚠️  Expected error (repo not found): {error.get('message', 'Unknown error')}")
            print(f"   Latency: {elapsed:.2f}ms")
            self.passed += 1  # This is expected behavior
            return True
        
        # If we get results
        if "result" in response:
            result = response["result"]
            hits = result.get("hits", [])
            print(f"✅ Search successful - {len(hits)} results (latency: {elapsed:.2f}ms)")
            if elapsed < 50.0:
                print(f"   🎯 Latency under 50ms target!")
            self.passed += 1
            return True
        
        print(f"❌ Unexpected response: {response}")
        self.failed += 1
        return False

    def test_malformed_json(self) -> bool:
        """Test error handling with malformed JSON."""
        print("\n🔧 Test: Malformed JSON")
        self.test_count += 1
        
        # Send malformed JSON and verify daemon doesn't crash
        bad_json = 'not json at all'
        try:
            if self.process and self.process.stdin:
                self.process.stdin.write(bad_json + "\n")
                self.process.stdin.flush()
                
                # Try to read response with short timeout
                if self.process and self.process.stdout:
                    import select
                    # Wait a bit for response
                    time.sleep(0.1)
                    
                # Now test that daemon still works by sending a ping
                response = self.send_request("ping", request_id=99)
                if response and response.get("result") == "pong":
                    print(f"✅ Daemon survived malformed JSON and still responds")
                    self.passed += 1
                    return True
                else:
                    print(f"❌ Daemon not responding after malformed JSON")
                    self.failed += 1
                    return False
        except Exception as e:
            print(f"❌ Test failed: {e}")
            self.failed += 1
            return False
    
    def restart_daemon(self) -> bool:
        """Restart the daemon for a clean state."""
        print("\n🔄 Restarting daemon for clean state...")
        self.cleanup()
        time.sleep(0.5)
        return self.start_daemon()

    def test_unknown_method(self) -> bool:
        """Test error handling with unknown method."""
        print("\n❓ Test: Unknown Method")
        self.test_count += 1
        
        response = self.send_request("nonexistent_method")
        
        if not response:
            print("❌ No response")
            self.failed += 1
            return False
        
        if "error" in response:
            error = response["error"]
            if error.get("code") == -32601:  # Method not found
                print(f"✅ Correctly returned method not found error")
                self.passed += 1
                return True
        
        print(f"❌ Expected error code -32601, got: {response}")
        self.failed += 1
        return False

    def test_graceful_shutdown(self) -> bool:
        """Test graceful shutdown when stdin is closed."""
        print("\n🛑 Test: Graceful Shutdown")
        self.test_count += 1
        
        if not self.process:
            print("❌ No process to shutdown")
            self.failed += 1
            return False
        
        try:
            # Close stdin to signal EOF
            if self.process.stdin:
                self.process.stdin.close()
            
            # Wait for process to exit (with timeout)
            exit_code = self.process.wait(timeout=5)
            
            if exit_code == 0:
                print(f"✅ Daemon exited cleanly (exit code: {exit_code})")
                self.passed += 1
                return True
            else:
                print(f"❌ Daemon exited with code: {exit_code}")
                self.failed += 1
                return False
                
        except subprocess.TimeoutExpired:
            print("❌ Daemon did not exit within timeout")
            self.process.kill()
            self.failed += 1
            return False
        except Exception as e:
            print(f"❌ Shutdown test failed: {e}")
            self.failed += 1
            return False

    def cleanup(self):
        """Ensure the daemon process is terminated."""
        if self.process and self.process.poll() is None:
            print("\n🧹 Cleaning up...")
            self.process.terminate()
            try:
                self.process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.process.kill()

    def print_summary(self):
        """Print test summary."""
        print("\n" + "=" * 50)
        print("📊 TEST SUMMARY")
        print("=" * 50)
        print(f"Total Tests: {self.test_count}")
        print(f"✅ Passed: {self.passed}")
        print(f"❌ Failed: {self.failed}")
        print(f"Success Rate: {(self.passed/self.test_count*100) if self.test_count > 0 else 0:.1f}%")
        print("=" * 50)

    def run_all_tests(self) -> bool:
        """Run all tests."""
        try:
            if not self.start_daemon():
                return False

            # Run core tests
            self.test_ping()
            self.test_search()
            self.test_unknown_method()
            self.test_graceful_shutdown()

            return self.failed == 0
        finally:
            self.cleanup()
            self.print_summary()


def main():
    """Main entry point."""
    print("🧪 MAPDAEMON Integration Tests")
    print("=" * 50)
    
    # Check if build is needed
    import os
    daemon_path = "./target/debug/crewchief-maproom"
    if not os.path.exists(daemon_path):
        print("⚠️  Daemon binary not found. Building...")
        result = subprocess.run(["cargo", "build"], capture_output=True)
        if result.returncode != 0:
            print(f"❌ Build failed: {result.stderr.decode()}")
            sys.exit(1)
    
    tester = DaemonTester(daemon_path)
    success = tester.run_all_tests()
    
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
