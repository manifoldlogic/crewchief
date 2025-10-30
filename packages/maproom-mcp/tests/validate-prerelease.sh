#!/bin/bash
set -e

echo "=== DKRHUB-2904: Pre-Release Image Validation ==="
echo ""

PRERELEASE_TAG="${PRERELEASE_TAG:-1.1.10-rc1}"
IMAGE="crewchief/maproom-mcp:$PRERELEASE_TAG"

echo "Validating: $IMAGE"
echo ""

# ========================================
# Multi-Platform Manifest Check
# ========================================
echo "Step 1: Checking multi-platform manifest..."
docker manifest inspect "$IMAGE" | grep -E "architecture|os" || {
  echo "❌ Manifest not found or incomplete"
  exit 1
}

# ========================================
# AMD64 Validation
# ========================================
echo ""
echo "Step 2: Pulling AMD64 image..."
docker pull --platform linux/amd64 "$IMAGE"

echo ""
echo "Step 3: Validating AMD64 image..."
echo "- Checking Node.js..."
docker run --rm --platform linux/amd64 "$IMAGE" node --version

echo "- Checking Rust binary..."
docker run --rm --platform linux/amd64 "$IMAGE" crewchief-maproom --version

echo "- Checking npm dependencies..."
docker run --rm --platform linux/amd64 "$IMAGE" ls /app/node_modules | wc -l

echo "- Checking image size..."
docker images "$IMAGE" --format "{{.Size}}"

# ========================================
# ARM64 Validation
# ========================================
echo ""
echo "Step 4: Pulling ARM64 image..."
docker pull --platform linux/arm64 "$IMAGE"

echo ""
echo "Step 5: Validating ARM64 image..."
echo "- Checking Node.js..."
docker run --rm --platform linux/arm64 "$IMAGE" node --version

echo "- Checking Rust binary..."
docker run --rm --platform linux/arm64 "$IMAGE" crewchief-maproom --version

echo "- Checking npm dependencies..."
docker run --rm --platform linux/arm64 "$IMAGE" ls /app/node_modules | wc -l

# ========================================
# End-to-End Test
# ========================================
echo ""
echo "Step 6: End-to-end validation with docker-compose..."

# Backup current docker-compose.yml
cd "$(dirname "${BASH_SOURCE[0]}")/../config"
cp docker-compose.yml docker-compose.yml.backup

# Update to use pre-release tag
export MAPROOM_VERSION="$PRERELEASE_TAG"

echo "- Starting services with MAPROOM_VERSION=$MAPROOM_VERSION..."
docker-compose up -d

echo "- Waiting for services to be healthy..."
sleep 45

echo "- Checking service status..."
docker-compose ps

echo "- Testing MCP server..."
timeout 5 docker exec -i maproom-mcp node /app/dist/index.js <<EOF || true
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}
EOF

echo ""
echo "- Checking logs for errors..."
docker logs maproom-mcp 2>&1 | tail -20

echo ""
echo "- Cleaning up..."
docker-compose down
mv docker-compose.yml.backup docker-compose.yml

cd -

# ========================================
# Summary
# ========================================
echo ""
echo "✅ Pre-release validation complete!"
echo ""
echo "Images validated:"
echo "  - $IMAGE (AMD64)"
echo "  - $IMAGE (ARM64)"
echo ""
echo "Next steps:"
echo "1. Review GitHub Security tab for Trivy scan results"
echo "2. If all clear, proceed to DKRHUB-3001 (version bump)"
echo "3. If issues found, fix and re-publish pre-release"
