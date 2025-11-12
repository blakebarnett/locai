#!/bin/bash
# Build Locai Server Docker image using HOST sccache cache
#
# This script mounts your host's sccache cache into the Docker build,
# allowing Docker to reuse compilation artifacts from host builds.
#
# If you've already built on the host, the Docker build will be nearly instant!

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCCACHE_DIR="${SCCACHE_DIR:-$HOME/.cache/sccache}"
IMAGE_NAME="${IMAGE_NAME:-locai-server}"
IMAGE_TAG="${IMAGE_TAG:-latest}"

echo -e "${GREEN}=== Locai Docker Build with HOST sccache ===${NC}"
echo ""

# Check if Docker BuildKit is available
if ! docker buildx version >/dev/null 2>&1; then
    echo -e "${RED}Error: Docker BuildKit (buildx) is not available${NC}"
    echo "Please install Docker with BuildKit support or enable it:"
    echo "  export DOCKER_BUILDKIT=1"
    exit 1
fi

# Check if sccache directory exists on host
if [ ! -d "$SCCACHE_DIR" ]; then
    echo -e "${YELLOW}Warning: Host sccache directory not found: $SCCACHE_DIR${NC}"
    echo "This is normal if you haven't built on the host yet."
    echo "The Docker build will still work, but won't be as fast."
    echo ""
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
else
    echo -e "${GREEN}Found host sccache cache at: $SCCACHE_DIR${NC}"
    
    # Show host cache stats if sccache is available
    if command -v sccache &> /dev/null; then
        echo ""
        echo -e "${BLUE}Current host sccache statistics:${NC}"
        sccache --show-stats | head -15
        echo ""
    fi
fi

echo -e "${GREEN}Building Docker image with HOST sccache...${NC}"
echo "  Image: $IMAGE_NAME:$IMAGE_TAG"
echo "  Host cache: $SCCACHE_DIR"
echo ""

# Create temporary Dockerfile that uses host cache
TEMP_DOCKERFILE=$(mktemp)
cat > "$TEMP_DOCKERFILE" << 'DOCKERFILE_CONTENT'
# Temporary Dockerfile with host sccache mount
FROM rustlang/rust:nightly-bookworm AS chef
RUN cargo install cargo-chef --locked && cargo install sccache --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS dependencies
RUN apt-get update && apt-get install -y \
    build-essential libclang-dev clang cmake pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
COPY .cargo/config.toml .cargo/config.toml
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/host-sccache
ENV SCCACHE_CACHE_SIZE="10G"
RUN --mount=type=bind,source=HOST_CACHE_PLACEHOLDER,target=/host-sccache \
    --mount=type=cache,target=/usr/local/cargo/registry \
    cargo chef cook --release --recipe-path recipe.json && \
    sccache --show-stats

FROM chef AS builder
RUN apt-get update && apt-get install -y \
    build-essential libclang-dev clang cmake pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
COPY --from=dependencies /app/target target
COPY --from=dependencies /usr/local/cargo /usr/local/cargo
COPY . .
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/host-sccache
ENV SCCACHE_CACHE_SIZE="10G"
RUN --mount=type=bind,source=HOST_CACHE_PLACEHOLDER,target=/host-sccache \
    --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release --bin locai-server && \
    echo "=== Final sccache statistics ===" && \
    sccache --show-stats
RUN ls -lh /app/target/release/locai-server && \
    file /app/target/release/locai-server && \
    /app/target/release/locai-server --help

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates libssl3 bash curl \
    && rm -rf /var/lib/apt/lists/* && apt-get clean
RUN groupadd -r locai && useradd -r -g locai -u 1000 locai
RUN mkdir -p /data && chown -R locai:locai /data
COPY --from=builder /app/target/release/locai-server /usr/local/bin/locai-server
RUN chmod +x /usr/local/bin/locai-server
USER locai
WORKDIR /data
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/api/health || exit 1
CMD ["locai-server"]
LABEL org.opencontainers.image.title="Locai Server" \
      org.opencontainers.image.description="Memory management service (HOST sccache mode)" \
      org.opencontainers.image.version="0.2.0"
DOCKERFILE_CONTENT

# Replace placeholder with actual cache directory
sed -i "s|HOST_CACHE_PLACEHOLDER|${SCCACHE_DIR}|g" "$TEMP_DOCKERFILE"

echo -e "${BLUE}Building with bind mount to host cache...${NC}"
echo ""

# Build with BuildKit
DOCKER_BUILDKIT=1 docker buildx build \
    --file "$TEMP_DOCKERFILE" \
    --tag "$IMAGE_NAME:$IMAGE_TAG" \
    --progress=plain \
    .

# Clean up
rm "$TEMP_DOCKERFILE"

echo ""
echo -e "${GREEN}=== Build Complete ===${NC}"

# Show updated host sccache stats if available
if command -v sccache &> /dev/null && [ -d "$SCCACHE_DIR" ]; then
    echo ""
    echo -e "${BLUE}Updated host sccache statistics:${NC}"
    sccache --show-stats | head -15
fi

echo ""
echo -e "${GREEN}✅ Docker image built successfully: $IMAGE_NAME:$IMAGE_TAG${NC}"
echo ""
echo -e "${BLUE}Cache mode: HOST sccache${NC}"
echo -e "${BLUE}Host cache used: $SCCACHE_DIR${NC}"
echo ""
echo "To run the server:"
echo "  docker run -p 3000:3000 $IMAGE_NAME:$IMAGE_TAG"






