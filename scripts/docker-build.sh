#!/bin/bash
set -euo pipefail

# CrewChief Web UI Docker Build Script
# Builds Docker images for both development and production

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
BUILD_TYPE="production"
PLATFORM="linux/amd64"
NO_CACHE=false
PUSH=false
TAG_LATEST=false

# Function to print usage
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Build Docker images for CrewChief Web UI"
    echo ""
    echo "Options:"
    echo "  -t, --type TYPE        Build type: development, production (default: production)"
    echo "  -p, --platform PLATFORM  Target platform (default: linux/amd64)"
    echo "  --no-cache            Don't use Docker cache"
    echo "  --push                Push images to registry"
    echo "  --tag-latest          Tag as latest"
    echo "  -h, --help            Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                           # Build production image"
    echo "  $0 -t development            # Build development image"
    echo "  $0 --no-cache --push         # Build and push without cache"
    echo "  $0 -t production --tag-latest # Build production and tag as latest"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -t|--type)
            BUILD_TYPE="$2"
            shift 2
            ;;
        -p|--platform)
            PLATFORM="$2"
            shift 2
            ;;
        --no-cache)
            NO_CACHE=true
            shift
            ;;
        --push)
            PUSH=true
            shift
            ;;
        --tag-latest)
            TAG_LATEST=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo -e "${RED}Error: Unknown option $1${NC}"
            usage
            exit 1
            ;;
    esac
done

# Validate build type
if [[ ! "$BUILD_TYPE" =~ ^(development|production)$ ]]; then
    echo -e "${RED}Error: Invalid build type '$BUILD_TYPE'. Must be 'development' or 'production'${NC}"
    exit 1
fi

# Set image name and tag
IMAGE_NAME="crewchief/web-ui"
VERSION=$(grep '"version"' "$PROJECT_ROOT/packages/web-ui/package.json" | sed 's/.*"version": "\(.*\)".*/\1/')
IMAGE_TAG="${IMAGE_NAME}:${VERSION}-${BUILD_TYPE}"

if [[ "$TAG_LATEST" == "true" ]]; then
    LATEST_TAG="${IMAGE_NAME}:latest"
fi

echo -e "${BLUE}🚀 Building CrewChief Web UI Docker Image${NC}"
echo -e "${BLUE}======================================${NC}"
echo -e "Build Type: ${YELLOW}${BUILD_TYPE}${NC}"
echo -e "Platform: ${YELLOW}${PLATFORM}${NC}"
echo -e "Image Tag: ${YELLOW}${IMAGE_TAG}${NC}"
if [[ "$TAG_LATEST" == "true" ]]; then
    echo -e "Latest Tag: ${YELLOW}${LATEST_TAG}${NC}"
fi
echo -e "No Cache: ${YELLOW}${NO_CACHE}${NC}"
echo -e "Push: ${YELLOW}${PUSH}${NC}"
echo ""

# Change to project root
cd "$PROJECT_ROOT"

# Determine Dockerfile and target
if [[ "$BUILD_TYPE" == "development" ]]; then
    DOCKERFILE="packages/web-ui/Dockerfile.dev"
    TARGET="development"
else
    DOCKERFILE="packages/web-ui/Dockerfile"
    TARGET="production"
fi

# Build arguments
BUILD_ARGS=(
    "--file" "$DOCKERFILE"
    "--target" "$TARGET"
    "--platform" "$PLATFORM"
    "--tag" "$IMAGE_TAG"
)

if [[ "$TAG_LATEST" == "true" ]]; then
    BUILD_ARGS+=("--tag" "$LATEST_TAG")
fi

if [[ "$NO_CACHE" == "true" ]]; then
    BUILD_ARGS+=("--no-cache")
fi

# Build the image
echo -e "${BLUE}📦 Building Docker image...${NC}"
if docker build "${BUILD_ARGS[@]}" .; then
    echo -e "${GREEN}✅ Docker image built successfully!${NC}"
else
    echo -e "${RED}❌ Docker build failed!${NC}"
    exit 1
fi

# Show image info
echo -e "${BLUE}📊 Image Information:${NC}"
docker images "$IMAGE_NAME" --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}"

# Push if requested
if [[ "$PUSH" == "true" ]]; then
    echo -e "${BLUE}📤 Pushing Docker image...${NC}"
    if docker push "$IMAGE_TAG"; then
        echo -e "${GREEN}✅ Image pushed successfully!${NC}"
        
        if [[ "$TAG_LATEST" == "true" ]]; then
            if docker push "$LATEST_TAG"; then
                echo -e "${GREEN}✅ Latest tag pushed successfully!${NC}"
            else
                echo -e "${RED}❌ Failed to push latest tag!${NC}"
                exit 1
            fi
        fi
    else
        echo -e "${RED}❌ Failed to push image!${NC}"
        exit 1
    fi
fi

echo -e "${GREEN}🎉 Build completed successfully!${NC}"

# Security scan (if available)
if command -v docker-scout &> /dev/null; then
    echo -e "${BLUE}🔍 Running security scan...${NC}"
    docker scout quickview "$IMAGE_TAG" || echo -e "${YELLOW}⚠️  Security scan failed or not available${NC}"
fi