#!/bin/bash
# devcontainer-network-fix.sh
#
# Connects maproom-postgres to the devcontainer network if running in a devcontainer.
# This allows the CLI (running inside devcontainer) to access postgres via its hostname.
#
# Usage: ./devcontainer-network-fix.sh

set -e

DEVCONTAINER_NETWORK="crewchief_devcontainer_crewchief-network"
POSTGRES_CONTAINER="maproom-postgres"

# Check if we're in a devcontainer by looking for the network
if docker network ls | grep -q "$DEVCONTAINER_NETWORK"; then
  echo "📡 Devcontainer network detected: $DEVCONTAINER_NETWORK"

  # Check if postgres container exists
  if docker ps -a --format '{{.Names}}' | grep -q "^${POSTGRES_CONTAINER}$"; then
    echo "🗄️  Postgres container found: $POSTGRES_CONTAINER"

    # Check if already connected
    if docker inspect "$POSTGRES_CONTAINER" | grep -q "$DEVCONTAINER_NETWORK"; then
      echo "✓ Postgres already connected to devcontainer network"
    else
      echo "🔌 Connecting postgres to devcontainer network..."
      docker network connect "$DEVCONTAINER_NETWORK" "$POSTGRES_CONTAINER"
      echo "✓ Postgres connected to devcontainer network"
      echo ""
      echo "🎉 You can now access postgres via:"
      echo "   - Hostname: maproom-postgres:5432"
      echo "   - Connection string: postgresql://maproom:maproom@maproom-postgres:5432/maproom"
    fi
  else
    echo "⚠️  Postgres container not found. Start it first with:"
    echo "   docker-compose up -d postgres"
  fi
else
  echo "ℹ️  Not running in devcontainer (network $DEVCONTAINER_NETWORK not found)"
  echo "   Postgres should be accessible via: 127.0.0.1:5433"
fi
