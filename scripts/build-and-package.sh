#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Find the project root (where crates/ directory exists)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Change to project root
cd "${PROJECT_ROOT}"

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

# Map architecture names
case "$ARCH" in
    x86_64)
        ARCH="x64"
        ;;
    aarch64|arm64)
        ARCH="arm64"
        ;;
esac

PLATFORM="${OS}-${ARCH}"
echo -e "${GREEN}Building for platform: ${PLATFORM}${NC}"
echo "Project root: ${PROJECT_ROOT}"

# Function to build and package a Rust binary
build_and_package() {
    local crate_name=$1
    local binary_name=$2
    local dest_dir=$3
    
    echo -e "${YELLOW}Building ${crate_name}...${NC}"
    
    # Build the Rust binary
    cargo build --release --manifest-path "crates/${crate_name}/Cargo.toml"
    
    if [ $? -eq 0 ]; then
        # Create platform-specific directory if it doesn't exist
        mkdir -p "${dest_dir}/${PLATFORM}"
        
        # Copy the binary to the destination
        cp "target/release/${binary_name}" "${dest_dir}/${PLATFORM}/"
        
        echo -e "${GREEN}✓ ${binary_name} built and copied to ${dest_dir}/${PLATFORM}/${NC}"
    else
        echo -e "${RED}✗ Failed to build ${crate_name}${NC}"
        exit 1
    fi
}

# Build maproom for CLI package
build_and_package "maproom" "maproom" "packages/cli/bin"

# Build maproom for MCP package (if it exists)
if [ -d "packages/maproom-mcp" ]; then
    build_and_package "maproom" "maproom" "packages/maproom-mcp/bin"
fi


# Create symlinks for the current platform in bin root (for backwards compatibility)
echo -e "${YELLOW}Creating platform symlinks...${NC}"

# Function to create a symlink
create_platform_link() {
    local binary_name=$1
    local package_dir=$2
    
    if [ -f "${package_dir}/bin/${PLATFORM}/${binary_name}" ]; then
        # Remove old non-platform binary if it exists
        if [ -f "${package_dir}/bin/${binary_name}" ] && [ ! -L "${package_dir}/bin/${binary_name}" ]; then
            rm "${package_dir}/bin/${binary_name}"
        fi
        
        # Create or update symlink
        ln -sf "${PLATFORM}/${binary_name}" "${package_dir}/bin/${binary_name}"
        echo -e "${GREEN}✓ Created symlink: ${package_dir}/bin/${binary_name} -> ${PLATFORM}/${binary_name}${NC}"
    fi
}

# Create symlinks for CLI package
create_platform_link "maproom" "packages/cli"

# Create symlinks for MCP package
if [ -d "packages/maproom-mcp" ]; then
    create_platform_link "maproom" "packages/maproom-mcp"
fi

echo -e "${GREEN}✨ Build and packaging complete!${NC}"
echo ""
echo "Binary locations:"
echo "  packages/cli/bin/${PLATFORM}/maproom"
if [ -d "packages/maproom-mcp" ]; then
    echo "  packages/maproom-mcp/bin/${PLATFORM}/maproom"
fi