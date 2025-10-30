#!/bin/bash
set -e

echo "==================================="
echo "ARM64 Platform Validation Test"
echo "==================================="
echo ""

# Architecture
echo "1. Architecture Verification:"
echo "   - System: $(uname -m)"
echo "   - Docker: $(docker info | grep Architecture | awk '{print $2}')"
echo ""

# PostgreSQL
echo "2. PostgreSQL + pgvector Test:"
POSTGRES_VERSION=$(docker exec maproom-postgres psql -U maproom -d maproom -t -c "SELECT version();" | head -1 | xargs)
echo "   - Version: ${POSTGRES_VERSION}"

PGVECTOR_VERSION=$(docker exec maproom-postgres psql -U maproom -d maproom -t -c "CREATE EXTENSION IF NOT EXISTS vector; SELECT extversion FROM pg_extension WHERE extname = 'vector';" | tail -1 | xargs)
echo "   - pgvector: ${PGVECTOR_VERSION}"

# Test vector operations
echo "   - Testing vector operations..."
docker exec maproom-postgres psql -U maproom -d maproom -c "
CREATE TABLE IF NOT EXISTS test_vectors (
    id SERIAL PRIMARY KEY,
    embedding vector(768)
);
INSERT INTO test_vectors (embedding) VALUES ('[1,2,3]'::vector);
SELECT COUNT(*) as vector_insert_test FROM test_vectors;
DROP TABLE test_vectors;
" > /dev/null 2>&1 && echo "     ✓ Vector insert/query successful" || echo "     ✗ Vector operations failed"

echo ""

# Ollama
echo "3. Ollama Model Test:"
OLLAMA_MODEL=$(docker exec maproom-ollama ollama list | grep nomic-embed-text | awk '{print $1, $3}')
echo "   - Model: ${OLLAMA_MODEL}"
echo "   - Status: $(docker inspect maproom-ollama --format='{{.State.Health.Status}}')"
echo ""

# Maproom binary
echo "4. Maproom Rust Binary Test:"
echo "   - Image size: $(docker images config-maproom-mcp --format '{{.Size}}')"
echo "   - Binary size: $(docker run --rm --entrypoint /bin/sh config-maproom-mcp -c 'ls -lh /usr/local/bin/crewchief-maproom' | awk '{print $5}')"
echo "   - Version: $(docker run --rm config-maproom-mcp --version)"
echo "   - Available commands:"
docker run --rm config-maproom-mcp --help | grep "Commands:" -A 20 | grep "^  " | head -8
echo ""

# Performance metrics
echo "5. Performance Metrics:"
echo "   - CPU cores: $(nproc)"
echo "   - Total memory: $(free -h | grep Mem | awk '{print $2}')"
echo "   - Postgres memory: $(docker stats maproom-postgres --no-stream --format '{{.MemUsage}}')"
echo "   - Ollama memory: $(docker stats maproom-ollama --no-stream --format '{{.MemUsage}}')"
echo ""

# Service health
echo "6. Service Health:"
echo "   - Postgres: $(docker inspect maproom-postgres --format='{{.State.Health.Status}}')"
echo "   - Ollama: $(docker inspect maproom-ollama --format='{{.State.Health.Status}}')"
echo ""

echo "==================================="
echo "ARM64 Validation: COMPLETE"
echo "==================================="
