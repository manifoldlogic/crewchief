#!/bin/bash
set -euo pipefail

# CrewChief Web UI Docker Run Script
# Manages Docker Compose deployments for development and production

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Default values
COMMAND="up"
ENVIRONMENT="development"
DETACH=false
BUILD=false
PULL=false
RECREATE=false
REMOVE_ORPHANS=true
PROFILE=""

# Function to print usage
usage() {
    echo "Usage: $0 [COMMAND] [OPTIONS]"
    echo ""
    echo "Manage CrewChief Web UI Docker deployment"
    echo ""
    echo "Commands:"
    echo "  up          Start services (default)"
    echo "  down        Stop and remove services"
    echo "  restart     Restart services"
    echo "  logs        Show service logs"
    echo "  ps          Show running services"
    echo "  exec        Execute command in running container"
    echo "  shell       Open shell in web-ui container"
    echo "  build       Build images"
    echo "  pull        Pull latest images"
    echo "  clean       Clean up volumes and networks"
    echo ""
    echo "Options:"
    echo "  -e, --env ENV         Environment: development, production (default: development)"
    echo "  -d, --detach          Run in background"
    echo "  -b, --build           Build images before starting"
    echo "  -p, --pull            Pull latest images before starting"
    echo "  -r, --recreate        Recreate containers"
    echo "  --no-remove-orphans   Don't remove orphaned containers"
    echo "  --profile PROFILE     Use specific compose profile (e.g., dev-tools)"
    echo "  -h, --help            Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                                    # Start development environment"
    echo "  $0 up -e production -d                # Start production in background"
    echo "  $0 up --profile dev-tools             # Start with development tools"
    echo "  $0 logs web-ui                        # Show web-ui logs"
    echo "  $0 exec web-ui pnpm test              # Run tests in container"
    echo "  $0 shell                              # Open shell in web-ui container"
    echo "  $0 down                               # Stop all services"
    echo "  $0 clean                              # Clean up everything"
}

# Function to get compose files based on environment
get_compose_files() {
    local env="$1"
    local files=("-f" "docker-compose.yml")
    
    if [[ "$env" == "development" ]]; then
        files+=("-f" "docker-compose.dev.yml")
    fi
    
    echo "${files[@]}"
}

# Function to get compose project name
get_project_name() {
    local env="$1"
    if [[ "$env" == "development" ]]; then
        echo "crewchief-dev"
    else
        echo "crewchief"
    fi
}

# Function to check for .env file
check_env_file() {
    if [[ ! -f "$PROJECT_ROOT/.env" ]]; then
        echo -e "${YELLOW}⚠️  No .env file found. Creating from .env.example...${NC}"
        if [[ -f "$PROJECT_ROOT/.env.example" ]]; then
            cp "$PROJECT_ROOT/.env.example" "$PROJECT_ROOT/.env"
            echo -e "${YELLOW}📝 Please edit .env file with your configuration before running again.${NC}"
            exit 1
        else
            echo -e "${RED}❌ No .env.example file found!${NC}"
            exit 1
        fi
    fi
}

# Function to run docker-compose with proper arguments
run_compose() {
    local cmd="$1"
    shift
    
    local compose_files
    IFS=' ' read -ra compose_files <<< "$(get_compose_files "$ENVIRONMENT")"
    
    local project_name
    project_name=$(get_project_name "$ENVIRONMENT")
    
    local args=(
        "--project-name" "$project_name"
        "${compose_files[@]}"
    )
    
    if [[ -n "$PROFILE" ]]; then
        args+=("--profile" "$PROFILE")
    fi
    
    args+=("$cmd")
    
    # Add additional arguments passed to function
    args+=("$@")
    
    echo -e "${BLUE}🐳 Running: docker compose ${args[*]}${NC}"
    docker compose "${args[@]}"
}

# Parse command line arguments
if [[ $# -gt 0 ]] && [[ ! "$1" =~ ^- ]]; then
    COMMAND="$1"
    shift
fi

while [[ $# -gt 0 ]]; do
    case $1 in
        -e|--env)
            ENVIRONMENT="$2"
            shift 2
            ;;
        -d|--detach)
            DETACH=true
            shift
            ;;
        -b|--build)
            BUILD=true
            shift
            ;;
        -p|--pull)
            PULL=true
            shift
            ;;
        -r|--recreate)
            RECREATE=true
            shift
            ;;
        --no-remove-orphans)
            REMOVE_ORPHANS=false
            shift
            ;;
        --profile)
            PROFILE="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            # Assume remaining args are for the compose command
            break
            ;;
    esac
done

# Validate environment
if [[ ! "$ENVIRONMENT" =~ ^(development|production)$ ]]; then
    echo -e "${RED}Error: Invalid environment '$ENVIRONMENT'. Must be 'development' or 'production'${NC}"
    exit 1
fi

# Change to project root
cd "$PROJECT_ROOT"

# Check for .env file
check_env_file

echo -e "${BLUE}🚀 CrewChief Web UI Docker Management${NC}"
echo -e "${BLUE}====================================${NC}"
echo -e "Command: ${YELLOW}${COMMAND}${NC}"
echo -e "Environment: ${YELLOW}${ENVIRONMENT}${NC}"
if [[ -n "$PROFILE" ]]; then
    echo -e "Profile: ${YELLOW}${PROFILE}${NC}"
fi
echo ""

# Handle different commands
case $COMMAND in
    up)
        up_args=()
        
        if [[ "$DETACH" == "true" ]]; then
            up_args+=("--detach")
        fi
        
        if [[ "$BUILD" == "true" ]]; then
            up_args+=("--build")
        fi
        
        if [[ "$PULL" == "true" ]]; then
            up_args+=("--pull" "always")
        fi
        
        if [[ "$RECREATE" == "true" ]]; then
            up_args+=("--force-recreate")
        fi
        
        if [[ "$REMOVE_ORPHANS" == "true" ]]; then
            up_args+=("--remove-orphans")
        fi
        
        # Add any additional arguments
        up_args+=("$@")
        
        echo -e "${GREEN}🚀 Starting services...${NC}"
        run_compose up "${up_args[@]}"
        
        if [[ "$DETACH" == "false" ]]; then
            echo -e "${GREEN}✅ Services started successfully!${NC}"
        else
            echo -e "${GREEN}✅ Services started in background!${NC}"
            echo -e "${BLUE}📊 Use '$0 ps' to check status${NC}"
            echo -e "${BLUE}📋 Use '$0 logs' to view logs${NC}"
        fi
        ;;
        
    down)
        echo -e "${YELLOW}🛑 Stopping services...${NC}"
        down_args=("$@")
        
        if [[ "$REMOVE_ORPHANS" == "true" ]]; then
            down_args+=("--remove-orphans")
        fi
        
        run_compose down "${down_args[@]}"
        echo -e "${GREEN}✅ Services stopped successfully!${NC}"
        ;;
        
    restart)
        echo -e "${YELLOW}🔄 Restarting services...${NC}"
        run_compose restart "$@"
        echo -e "${GREEN}✅ Services restarted successfully!${NC}"
        ;;
        
    logs)
        run_compose logs --follow "$@"
        ;;
        
    ps)
        run_compose ps "$@"
        ;;
        
    exec)
        if [[ $# -eq 0 ]]; then
            echo -e "${RED}Error: No service specified for exec command${NC}"
            exit 1
        fi
        run_compose exec "$@"
        ;;
        
    shell)
        service="${1:-web-ui}"
        shell_cmd="${2:-sh}"
        echo -e "${BLUE}🐚 Opening shell in $service container...${NC}"
        run_compose exec "$service" "$shell_cmd"
        ;;
        
    build)
        echo -e "${BLUE}🔨 Building images...${NC}"
        run_compose build "$@"
        echo -e "${GREEN}✅ Images built successfully!${NC}"
        ;;
        
    pull)
        echo -e "${BLUE}📥 Pulling images...${NC}"
        run_compose pull "$@"
        echo -e "${GREEN}✅ Images pulled successfully!${NC}"
        ;;
        
    clean)
        echo -e "${YELLOW}🧹 Cleaning up Docker resources...${NC}"
        
        # Stop and remove containers
        run_compose down --remove-orphans --volumes
        
        # Remove unused networks
        docker network prune -f
        
        # Remove unused volumes (ask for confirmation)
        echo -e "${YELLOW}⚠️  This will remove unused Docker volumes. Continue? (y/N)${NC}"
        read -r response
        if [[ "$response" =~ ^[Yy]$ ]]; then
            docker volume prune -f
        fi
        
        echo -e "${GREEN}✅ Cleanup completed!${NC}"
        ;;
        
    *)
        echo -e "${RED}Error: Unknown command '$COMMAND'${NC}"
        usage
        exit 1
        ;;
esac