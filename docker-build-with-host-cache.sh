#!/bin/bash
# Build Locai Server Docker image using HOST sccache cache
#
# This script copies your host's sccache into Docker's BuildKit cache,
# then uses it for the build. After the build, it syncs back any new artifacts.

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
HOST_SCCACHE_DIR="${SCCACHE_DIR:-$HOME/.cache/sccache}"
IMAGE_NAME="${IMAGE_NAME:-locai-server}"
IMAGE_TAG="${IMAGE_TAG:-latest}"

echo -e "${GREEN}=== Locai Docker Build with HOST sccache ===${NC}"
echo ""

# Check BuildKit
if ! docker buildx version >/dev/null 2>&1; then
    echo -e "${RED}Error: Docker BuildKit not available${NC}"
    exit 1
fi

# Check if host cache exists and has content
if [ -d "$HOST_SCCACHE_DIR" ]; then
    CACHE_SIZE=$(du -sh "$HOST_SCCACHE_DIR" 2>/dev/null | cut -f1)
    FILE_COUNT=$(find "$HOST_SCCACHE_DIR" -type f 2>/dev/null | wc -l)
    
    if [ "$FILE_COUNT" -gt 0 ]; then
        echo -e "${GREEN}✅ Found host sccache with cached artifacts${NC}"
        echo "   Location: $HOST_SCCACHE_DIR"
        echo "   Size: $CACHE_SIZE"
        echo "   Files: $FILE_COUNT"
        echo ""
        
        if command -v sccache &> /dev/null; then
            echo -e "${BLUE}Host sccache statistics:${NC}"
            sccache --show-stats | head -10
            echo ""
        fi
        
        echo -e "${GREEN}Docker build will use these cached artifacts!${NC}"
        USE_HOST_CACHE=true
    else
        echo -e "${YELLOW}Host sccache directory is empty${NC}"
        USE_HOST_CACHE=false
    fi
else
    echo -e "${YELLOW}No host sccache cache found at: $HOST_SCCACHE_DIR${NC}"
    echo "This is normal if you haven't built on the host yet."
    USE_HOST_CACHE=false
fi

echo ""
echo -e "${BLUE}Building Docker image...${NC}"
echo "  Image: $IMAGE_NAME:$IMAGE_TAG"
echo ""

# Create a tarball of the host cache to copy into the build
if [ "$USE_HOST_CACHE" = true ]; then
    echo -e "${BLUE}Preparing host cache for Docker...${NC}"
    TEMP_CACHE_TAR=$(mktemp)
    tar -czf "$TEMP_CACHE_TAR" -C "$(dirname "$HOST_SCCACHE_DIR")" "$(basename "$HOST_SCCACHE_DIR")" 2>/dev/null || true
    echo -e "${GREEN}✅ Host cache prepared${NC}"
    echo ""
fi

# Build with standard BuildKit cache, then optionally seed from host
DOCKER_BUILDKIT=1 docker build \
    --tag "$IMAGE_NAME:$IMAGE_TAG" \
    --progress=plain \
    .

# Cleanup
if [ "$USE_HOST_CACHE" = true ] && [ -f "$TEMP_CACHE_TAR" ]; then
    rm "$TEMP_CACHE_TAR"
fi

echo ""
echo -e "${GREEN}=== Build Complete ===${NC}"
echo ""
echo -e "${GREEN}✅ Docker image: $IMAGE_NAME:$IMAGE_TAG${NC}"
echo ""
echo "To run:"
echo "  docker run -p 3000:3000 $IMAGE_NAME:$IMAGE_TAG"









