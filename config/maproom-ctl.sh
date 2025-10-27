#!/bin/bash
# maproom-ctl.sh - Control script for Maproom LOCAL docker-compose stack

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

function print_header() {
    echo -e "${BLUE}================================${NC}"
    echo -e "${BLUE}  Maproom LOCAL Control Panel${NC}"
    echo -e "${BLUE}================================${NC}"
    echo
}

function print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

function print_error() {
    echo -e "${RED}✗ $1${NC}"
}

function print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

function print_info() {
    echo -e "${BLUE}ℹ $1${NC}"
}

function check_docker() {
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed or not in PATH"
        exit 1
    fi

    if ! docker compose version &> /dev/null; then
        print_error "Docker Compose v2 is required (use 'docker compose' not 'docker-compose')"
        exit 1
    fi
}

function start_services() {
    print_info "Starting Maproom LOCAL services..."
    docker compose up -d

    echo
    print_success "Services started!"
    print_info "First startup may take 3-6 minutes while Ollama downloads the model"
    print_info "Use '$0 logs' to monitor progress"
    print_info "Use '$0 status' to check health"
}

function stop_services() {
    print_info "Stopping Maproom LOCAL services..."
    docker compose down
    print_success "Services stopped"
}

function restart_services() {
    print_info "Restarting Maproom LOCAL services..."
    docker compose restart
    print_success "Services restarted"
}

function show_status() {
    print_info "Service Status:"
    echo
    docker compose ps
}

function show_logs() {
    local service="${1:-}"
    if [ -z "$service" ]; then
        print_info "Showing logs for all services (Ctrl+C to exit)..."
        docker compose logs -f
    else
        print_info "Showing logs for $service (Ctrl+C to exit)..."
        docker compose logs -f "$service"
    fi
}

function show_health() {
    print_header

    # Check if services are running
    local running=$(docker compose ps --format json 2>/dev/null | jq -r '.State' | grep -c "running" || echo "0")

    if [ "$running" -eq 0 ]; then
        print_warning "No services are running"
        echo
        print_info "Start services with: $0 start"
        exit 0
    fi

    echo "Health Status:"
    echo

    # Check each service
    for service in postgres ollama maproom; do
        local health=$(docker compose ps --format json | jq -r "select(.Service == \"$service\") | .Health")
        local state=$(docker compose ps --format json | jq -r "select(.Service == \"$service\") | .State")

        printf "%-10s: " "$service"

        if [ "$state" != "running" ]; then
            print_error "not running"
        elif [ "$health" == "healthy" ]; then
            print_success "healthy"
        elif [ "$health" == "starting" ]; then
            print_warning "starting..."
        else
            print_error "unhealthy"
        fi
    done

    echo

    # Show model status if Ollama is running
    if docker compose ps | grep -q "maproom-ollama.*Up"; then
        print_info "Ollama models:"
        docker compose exec -T ollama ollama list 2>/dev/null || print_warning "Ollama not ready yet"
    fi
}

function cleanup() {
    print_warning "This will stop services and delete all data volumes!"
    read -p "Are you sure? (yes/no): " confirm

    if [ "$confirm" == "yes" ]; then
        print_info "Stopping services and removing volumes..."
        docker compose down -v
        print_success "Cleanup complete"
    else
        print_info "Cleanup cancelled"
    fi
}

function show_help() {
    print_header

    cat << EOF
Usage: $0 <command> [options]

Commands:
    start       Start all services (docker compose up -d)
    stop        Stop all services (docker compose down)
    restart     Restart all services
    status      Show service status (docker compose ps)
    logs [svc]  Show logs (optionally for specific service)
    health      Show detailed health status
    cleanup     Stop services and delete all volumes (WARNING: deletes data)
    help        Show this help message

Examples:
    $0 start              # Start all services
    $0 logs               # Show all logs
    $0 logs ollama        # Show only Ollama logs
    $0 health             # Check health status
    $0 cleanup            # Reset everything

Services:
    postgres    PostgreSQL with pgvector
    ollama      Ollama LLM runtime
    maproom     Maproom MCP service

Ports:
    3000        Maproom MCP (configurable via MAPROOM_PORT)
    11434       Ollama API (configurable via OLLAMA_PORT)

Environment:
    Create a .env file to customize ports and paths.
    See README.md for details.

EOF
}

# Main script logic
check_docker

case "${1:-help}" in
    start)
        start_services
        ;;
    stop)
        stop_services
        ;;
    restart)
        restart_services
        ;;
    status)
        show_status
        ;;
    logs)
        show_logs "${2:-}"
        ;;
    health)
        show_health
        ;;
    cleanup)
        cleanup
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        print_error "Unknown command: $1"
        echo
        show_help
        exit 1
        ;;
esac
