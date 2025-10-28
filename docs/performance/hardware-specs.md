# Performance Benchmark Hardware Specifications

This document records the hardware specifications used for performance benchmarks to ensure reproducibility and provide context for results.

## Current Test Environment

**Last Updated**: 2025-10-28

### System Information

```bash
# Run these commands to populate:
uname -a
```

**OS**: TBD
**Kernel**: TBD
**Architecture**: TBD

### CPU

```bash
# Linux
lscpu | grep -E "Model name|CPU\(s\)|Thread|MHz|Cache"

# macOS
sysctl -n machdep.cpu.brand_string
sysctl -n hw.ncpu
```

**Model**: TBD
**Cores**: TBD
**Threads**: TBD
**Base Clock**: TBD
**Cache**: TBD

### Memory

```bash
# Linux
free -h

# macOS
sysctl hw.memsize
```

**Total RAM**: TBD
**Available**: TBD
**Type**: TBD

### GPU (Optional)

```bash
# NVIDIA
nvidia-smi

# AMD
rocm-smi

# macOS
system_profiler SPDisplaysDataType
```

**GPU Model**: TBD (or "N/A - CPU only benchmarks")
**VRAM**: TBD
**Driver Version**: TBD
**CUDA/ROCm Version**: TBD

### Storage

```bash
df -h /
lsblk -o NAME,SIZE,TYPE,MOUNTPOINT
```

**Type**: TBD (SSD/NVMe/HDD)
**Capacity**: TBD
**Mount Point**: TBD

### Docker Environment

```bash
docker version
docker info | grep -E "CPUs|Total Memory"
```

**Docker Version**: TBD
**Docker CPUs**: TBD
**Docker Memory Limit**: TBD

### Ollama Configuration

```bash
ollama --version
curl http://localhost:11434/api/tags | jq '.models[] | select(.name | contains("nomic-embed-text"))'
```

**Ollama Version**: TBD
**Model**: nomic-embed-text
**Model Size**: TBD
**Model Quantization**: TBD

### Network (for OpenAI comparison)

```bash
# Test network latency to OpenAI
ping -c 5 api.openai.com
curl -o /dev/null -s -w "Time: %{time_total}s\n" https://api.openai.com
```

**Network Type**: TBD (Local/Cloud/VPN)
**OpenAI Latency**: TBD ms (average ping)
**OpenAI HTTP Latency**: TBD ms (baseline for comparison exclusion)

## Reference Hardware Profiles

### Profile: Developer Laptop (Typical)

- **CPU**: Apple M1 Pro / Intel i7-11800H
- **RAM**: 16-32 GB
- **GPU**: Integrated / MX450
- **Expected Throughput**: 300-600 chunks/min (CPU)

### Profile: Workstation (High-End)

- **CPU**: AMD Ryzen 9 5950X / Intel i9-12900K
- **RAM**: 64 GB
- **GPU**: NVIDIA RTX 3080 / AMD RX 6800 XT
- **Expected Throughput**: 800-1500 chunks/min (CPU), 2500-4000 chunks/min (GPU)

### Profile: Server (Production)

- **CPU**: AMD EPYC / Intel Xeon
- **RAM**: 128+ GB
- **GPU**: NVIDIA A100 / A6000 (optional)
- **Expected Throughput**: 1000+ chunks/min (CPU), 4000+ chunks/min (GPU)

## Benchmark Conditions

### Environment Variables

```bash
# Record environment during benchmarks
env | grep -E "OLLAMA|OPENAI|RUST|CARGO" | sort
```

**Key Variables**: TBD

### Resource Constraints

```bash
# Check if any resource limits are set
ulimit -a
```

**Open Files**: TBD
**Max Processes**: TBD
**Memory Limit**: TBD

### Background Load

> **Note**: Benchmarks should be run with minimal background processes for accuracy.

**CPU Usage Before Benchmark**: TBD% (via `top` or `htop`)
**Memory Usage Before Benchmark**: TBD MB

### Temperature & Throttling

```bash
# Linux
sensors

# macOS
sudo powermetrics --samplers smc -i 1 -n 1
```

**CPU Temp**: TBD°C
**Throttling**: TBD (Yes/No)

## Reproducibility Checklist

- [ ] All hardware specs recorded
- [ ] Docker version documented
- [ ] Ollama version and model documented
- [ ] Background processes minimized
- [ ] No thermal throttling detected
- [ ] Consistent power settings (no battery saver mode)
- [ ] Network latency measured (for OpenAI comparison)
- [ ] Environment variables documented

## Scripts for Data Collection

### Automated Hardware Info

Create `/workspace/scripts/record-hardware-specs.sh`:

```bash
#!/bin/bash
# Collect hardware specifications for benchmark documentation

echo "# Hardware Specifications - $(date)"
echo ""

echo "## System"
uname -a
echo ""

echo "## CPU"
if command -v lscpu &> /dev/null; then
    lscpu | grep -E "Model name|CPU\(s\)|Thread|MHz|Cache"
elif command -v sysctl &> /dev/null; then
    sysctl -n machdep.cpu.brand_string
    sysctl -n hw.ncpu
fi
echo ""

echo "## Memory"
if command -v free &> /dev/null; then
    free -h
elif command -v sysctl &> /dev/null; then
    sysctl hw.memsize
fi
echo ""

echo "## Docker"
docker version --format '{{.Server.Version}}'
docker info --format 'CPUs: {{.NCPU}}, Memory: {{.MemTotal}}'
echo ""

echo "## Ollama"
if command -v ollama &> /dev/null; then
    ollama --version
fi
curl -s http://localhost:11434/api/tags 2>/dev/null | jq -r '.models[] | select(.name | contains("nomic-embed-text")) | .name'
echo ""

echo "## GPU (if available)"
if command -v nvidia-smi &> /dev/null; then
    nvidia-smi --query-gpu=name,memory.total --format=csv,noheader
fi
```

### Usage

```bash
chmod +x scripts/record-hardware-specs.sh
./scripts/record-hardware-specs.sh > docs/performance/hardware-specs-$(date +%Y%m%d).txt
```

---

## Comparison Across Hardware

When running benchmarks on different hardware, record results in this table:

| Hardware Profile | CPU | RAM | GPU | Throughput (chunks/min) | Single Latency (ms) | Notes |
|------------------|-----|-----|-----|-------------------------|---------------------|-------|
| TBD              | TBD | TBD | TBD | TBD                     | TBD                 | TBD   |

---

**Last Updated**: 2025-10-28
**Next Review**: After running first benchmark suite
