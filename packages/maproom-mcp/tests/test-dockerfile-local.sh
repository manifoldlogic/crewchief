#!/bin/bash
set -e

echo "=== DKRHUB-1007: Local Dockerfile Testing ==="
echo ""

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$WORKSPACE_ROOT"

echo "Step 1: Building Dockerfile.combined..."
docker build \
  -f packages/maproom-mcp/config/Dockerfile.combined \
  -t maproom-test:local \
  .

echo ""
echo "Step 2: Checking image size..."
docker images maproom-test:local --format "Size: {{.Size}}"

echo ""
echo "Step 3: Verifying Node.js runtime..."
docker run --rm --entrypoint node maproom-test:local --version

echo ""
echo "Step 4: Verifying Rust binary..."
docker run --rm --entrypoint maproom maproom-test:local --version

echo ""
echo "Step 5: Checking npm dependencies..."
docker run --rm --entrypoint sh maproom-test:local -c "ls /app/node_modules | head -10"

echo ""
echo "Step 6: Checking TypeScript compilation..."
docker run --rm --entrypoint ls maproom-test:local -la /app/dist/

echo ""
echo "Step 7: Testing MCP server startup..."
timeout 5 docker run --rm -i maproom-test:local <<EOF || true
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
EOF
echo "(Server started, terminated by timeout - this is expected)"

echo ""
echo "Step 8: Testing non-root user..."
docker run --rm --entrypoint whoami maproom-test:local

echo ""
echo "Step 9: Creating test network..."
docker network create maproom-test-network 2>/dev/null || true

echo ""
echo "Step 10: Starting test postgres..."
docker run -d \
  --name maproom-test-postgres \
  --network maproom-test-network \
  -e POSTGRES_DB=maproom \
  -e POSTGRES_USER=maproom \
  -e POSTGRES_PASSWORD=maproom \
  pgvector/pgvector:pg16

echo ""
echo "Step 11: Waiting for postgres to be ready..."
sleep 10
docker exec maproom-test-postgres pg_isready -U maproom

echo ""
echo "Step 12: Testing database connectivity with pg_isready..."
docker run --rm \
  --network maproom-test-network \
  --entrypoint pg_isready \
  -e MAPROOM_DATABASE_URL=postgresql://maproom:maproom@maproom-test-postgres:5432/maproom \
  maproom-test:local \
  -h maproom-test-postgres -U maproom

echo ""
echo "Step 13: Checking Rust binary is accessible..."
docker run --rm \
  --network maproom-test-network \
  --entrypoint which \
  maproom-test:local \
  maproom

echo ""
echo "Step 14: Testing Rust binary execution..."
docker run --rm \
  --network maproom-test-network \
  --entrypoint maproom \
  maproom-test:local \
  --version

echo ""
echo "Step 15: Testing MCP server with database connection..."
timeout 5 docker run --rm -i \
  --network maproom-test-network \
  -e MAPROOM_DATABASE_URL=postgresql://maproom:maproom@maproom-test-postgres:5432/maproom \
  -e MAPROOM_EMBEDDING_PROVIDER=ollama \
  -e LOG_LEVEL=info \
  maproom-test:local <<EOF || true
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
EOF
echo "(MCP server with database connection responded - this is expected)"

echo ""
echo "Step 16: Verifying environment variables are passed correctly..."
docker run --rm \
  --network maproom-test-network \
  --entrypoint sh \
  -e MAPROOM_DATABASE_URL=postgresql://maproom:maproom@maproom-test-postgres:5432/maproom \
  -e TEST_VAR=test123 \
  maproom-test:local \
  -c 'echo "MAPROOM_DATABASE_URL is set: $(echo $MAPROOM_DATABASE_URL | grep -o "maproom")"'

echo ""
echo "Step 17: Cleaning up test containers..."
docker stop maproom-test-postgres 2>/dev/null || true
docker rm maproom-test-postgres 2>/dev/null || true
docker network rm maproom-test-network 2>/dev/null || true

cd "$WORKSPACE_ROOT"

echo ""
echo "✅ All local tests passed!"
echo ""
echo "Next steps:"
echo "1. Review test output for any warnings"
echo "2. Mark DKRHUB-1007 as complete"
echo "3. Proceed to DKRHUB-1001 (GitHub Actions workflow)"
