#!/bin/bash
# Collect hardware specifications for benchmark documentation

set -e

echo "# Hardware Specifications - $(date)"
echo ""

echo "## System"
uname -a
echo ""

echo "## CPU"
if command -v lscpu &> /dev/null; then
    lscpu | grep -E "Model name|CPU\(s\)|Thread|MHz|Cache"
elif command -v sysctl &> /dev/null; then
    echo "Model: $(sysctl -n machdep.cpu.brand_string)"
    echo "CPUs: $(sysctl -n hw.ncpu)"
    echo "Physical CPUs: $(sysctl -n hw.physicalcpu)"
fi
echo ""

echo "## Memory"
if command -v free &> /dev/null; then
    free -h
elif command -v sysctl &> /dev/null; then
    echo "Total: $(sysctl -n hw.memsize | awk '{print $1/1024/1024/1024 " GB"}')"
fi
echo ""

echo "## Docker"
if command -v docker &> /dev/null; then
    echo "Version: $(docker version --format '{{.Server.Version}}' 2>/dev/null || echo 'N/A')"
    docker info --format 'CPUs: {{.NCPU}}, Memory: {{.MemTotal}}' 2>/dev/null || echo "Docker not running"
fi
echo ""

echo "## Ollama"
if command -v ollama &> /dev/null; then
    ollama --version 2>/dev/null || echo "Ollama CLI not available"
fi

# Check Ollama API
if curl -s http://localhost:11434/api/tags &> /dev/null; then
    echo "Models:"
    curl -s http://localhost:11434/api/tags | jq -r '.models[]? | "  - \(.name) (\(.size/1024/1024/1024 | round)GB)"' 2>/dev/null || echo "  (jq not available for JSON parsing)"
else
    echo "Ollama service not running at localhost:11434"
fi
echo ""

echo "## GPU (if available)"
if command -v nvidia-smi &> /dev/null; then
    nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv,noheader
elif command -v rocm-smi &> /dev/null; then
    rocm-smi --showproductname
else
    echo "No GPU detected (or nvidia-smi/rocm-smi not available)"
fi
echo ""

echo "## Storage"
df -h / | tail -n 1
echo ""

echo "## Rust/Cargo Version"
if command -v rustc &> /dev/null; then
    echo "rustc: $(rustc --version)"
    echo "cargo: $(cargo --version)"
fi
echo ""

echo "---"
echo "Specifications collected at $(date -u)"
