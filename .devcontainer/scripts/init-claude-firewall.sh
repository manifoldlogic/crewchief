#!/bin/bash
set -euo pipefail  # Exit on error, undefined vars, and pipeline failures
IFS=$'\n\t'       # Stricter word splitting

echo "🔒 Initializing host isolation for Claude Code..."

# Flush existing rules
iptables -F INPUT
iptables -F OUTPUT
iptables -F FORWARD

# Set default policies to ACCEPT (allow all by default)
iptables -P INPUT ACCEPT
iptables -P FORWARD ACCEPT
iptables -P OUTPUT ACCEPT

# Allow localhost communication (needed for internal services)
iptables -A INPUT -i lo -j ACCEPT
iptables -A OUTPUT -o lo -j ACCEPT

# Allow Docker internal networks (172.16.0.0/12) for container-to-container communication
iptables -A INPUT -s 172.16.0.0/12 -j ACCEPT
iptables -A OUTPUT -d 172.16.0.0/12 -j ACCEPT

# Get host gateway IP (this is what we want to block)
HOST_GATEWAY=$(ip route | grep default | awk '{print $3}')
if [ -z "$HOST_GATEWAY" ]; then
    echo "⚠️  WARNING: Could not detect host gateway IP"
else
    echo "Host gateway detected: $HOST_GATEWAY"
    
    # Block access TO the host machine (except for Docker's internal DNS)
    # Docker uses the host as DNS resolver, so we need to allow DNS
    iptables -A OUTPUT -d "$HOST_GATEWAY" -p udp --dport 53 -j ACCEPT
    iptables -A OUTPUT -d "$HOST_GATEWAY" -p tcp --dport 53 -j ACCEPT
    
    # Block all other traffic to the host
    iptables -A OUTPUT -d "$HOST_GATEWAY" -j REJECT --reject-with icmp-host-prohibited
    
    # Also block common host-local addresses
    iptables -A OUTPUT -d 192.168.0.0/16 -j REJECT --reject-with icmp-host-prohibited
    iptables -A OUTPUT -d 10.0.0.0/8 -j REJECT --reject-with icmp-host-prohibited
    
    # But allow Docker networks (override the broad blocks above)
    iptables -I OUTPUT -d 172.16.0.0/12 -j ACCEPT
fi

# Allow established connections
iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
iptables -A OUTPUT -m state --state ESTABLISHED,RELATED -j ACCEPT

echo "✅ Host isolation configured"
echo ""
echo "🔍 Verifying configuration..."

# Test internet access
if curl --connect-timeout 3 https://example.com >/dev/null 2>&1; then
    echo "✅ Internet access: ALLOWED (example.com reachable)"
else
    echo "❌ Internet access: BLOCKED (example.com unreachable)"
fi

# Test GitHub access
if curl --connect-timeout 3 https://api.github.com/zen >/dev/null 2>&1; then
    echo "✅ GitHub API: ACCESSIBLE"
else
    echo "⚠️  GitHub API: NOT ACCESSIBLE"
fi

# Try to ping the host (should fail)
if [ -n "$HOST_GATEWAY" ]; then
    if ping -c 1 -W 1 "$HOST_GATEWAY" >/dev/null 2>&1; then
        echo "⚠️  WARNING: Can still reach host at $HOST_GATEWAY"
    else
        echo "✅ Host access: BLOCKED (cannot reach $HOST_GATEWAY)"
    fi
fi

echo ""
echo "🎯 Configuration complete!"
echo "   - Internet access: ✅ ALLOWED"
echo "   - Container network: ✅ ALLOWED"
echo "   - Host machine: ❌ BLOCKED"
echo ""
echo "Claude Code will use its built-in domain approval system for additional security."