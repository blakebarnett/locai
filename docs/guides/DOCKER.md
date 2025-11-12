# Docker Deployment Guide

This guide covers building and deploying Locai using Docker.

## Quick Start

### Using Docker Compose (Recommended)

```bash
# Build and start the service
docker-compose up -d

# View logs
docker-compose logs -f

# Stop the service
docker-compose down

# Stop and remove data
docker-compose down -v
```

The server will be available at `http://localhost:3000`.

### Using Docker Directly

```bash
# Build the image
docker build -t locai-server:latest .

# Run the container
docker run -d \
  --name locai-server \
  -p 3000:3000 \
  -v locai-data:/data \
  -e RUST_LOG=info \
  locai-server:latest

# View logs
docker logs -f locai-server

# Stop the container
docker stop locai-server
docker rm locai-server
```

## Image Architecture

### Multi-Stage Build

The Dockerfile uses a 5-stage build process for optimal size and caching:

1. **Chef Stage**: Sets up cargo-chef for dependency caching
2. **Planner Stage**: Analyzes dependencies
3. **Dependencies Stage**: Builds and caches dependencies (cached layer)
4. **Builder Stage**: Compiles the application
5. **Runtime Stage**: Final minimal image (~150MB)

### Why Debian and Not Alpine?

**Locai uses `debian:bookworm-slim` for the runtime** because:

- ✅ **Native Dependencies**: RocksDB and SurrealDB require glibc
- ✅ **Build Reliability**: No musl compatibility issues
- ✅ **Full Featured**: Includes bash, curl, and essential tools
- ✅ **Production Ready**: Well-tested for Rust applications
- ⚠️ **Size Trade-off**: ~150MB final image vs Alpine's potential ~50MB (but with reliability issues)

The binaries are **dynamically linked** against glibc but are fully optimized with:
- Link-time optimization (LTO)
- Symbol stripping
- Aggressive optimization level

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `LOCAI_HOST` | `0.0.0.0` | Server bind address |
| `LOCAI_PORT` | `3000` | Server port |
| `LOCAI_DATA_DIR` | `/data` | Data directory path |
| `LOCAI_STORAGE_TYPE` | `embedded` | Storage backend type |
| `LOCAI_ENABLE_AUTH` | `false` | Enable authentication |
| `RUST_LOG` | `info` | Logging level |
| `RUST_BACKTRACE` | `0` | Enable backtraces (1 or full) |

### Volumes

Mount `/data` to persist:
- Database files
- Configuration
- Logs
- Embeddings cache

```bash
docker run -v /host/path:/data locai-server:latest
```

### Configuration File

You can mount a configuration file:

```bash
docker run -v ./config.toml:/data/config.toml:ro locai-server:latest
```

## Build Optimizations

### Using BuildKit and sccache

Enable BuildKit for faster builds with better caching. The Dockerfile includes sccache support for dramatically faster builds:

```bash
# One-time
export DOCKER_BUILDKIT=1

# Or per-command
DOCKER_BUILDKIT=1 docker build -t locai-server:latest .
```

**Performance**: With sccache, subsequent builds achieve 95%+ cache hit rates, reducing build time from 10-15 minutes to 2-4 minutes for unchanged code.

### Build Arguments

**Note:** Locai uses Rust Edition 2024, which requires nightly Rust. The Dockerfile uses `rustlang/rust:nightly-bookworm` by default.

The Dockerfile supports standard Rust build args if you need a specific nightly version:

```bash
docker build \
  --build-arg RUST_VERSION=nightly-2024-01-01 \
  -t locai-server:latest \
  .
```

### Cache Layers

The multi-stage build caches dependencies separately from application code:

- **Dependency changes**: Rebuilds from Stage 3 (~2-5 minutes)
- **Code changes only**: Rebuilds from Stage 4 (~1-2 minutes)
- **No changes**: Uses cached image (~5 seconds)

## Production Deployment

### Security Considerations

1. **Non-root User**: The image runs as user `locai` (UID 1000)
2. **Read-only Filesystem**: Consider using `--read-only` with a writable `/data`:
   ```bash
   docker run --read-only --tmpfs /tmp -v data:/data locai-server
   ```
3. **Drop Capabilities**: Limit container capabilities:
   ```bash
   docker run --cap-drop=ALL --cap-add=NET_BIND_SERVICE locai-server
   ```

### Resource Limits

```bash
docker run \
  --memory="2g" \
  --cpus="2.0" \
  --pids-limit 100 \
  locai-server:latest
```

Or in docker-compose.yml (already configured in the provided file).

### Health Checks

The image includes a health check endpoint:

```bash
# Kubernetes/k8s
livenessProbe:
  httpGet:
    path: /api/health
    port: 3000
  initialDelaySeconds: 10
  periodSeconds: 30

readinessProbe:
  httpGet:
    path: /api/health
    port: 3000
  initialDelaySeconds: 5
  periodSeconds: 10
```

## Development Workflow

### Faster Development Builds

For development, you can mount source code and build inside the container:

```yaml
# docker-compose.dev.yml
services:
  locai-dev:
    image: rust:1.83-bookworm
    working_dir: /app
    volumes:
      - .:/app
      - cargo-cache:/usr/local/cargo/registry
    command: cargo run --bin locai-server
    ports:
      - "3000:3000"
    environment:
      - RUST_LOG=debug
```

### Hot Reload (cargo-watch)

```bash
# Install cargo-watch in the container
docker-compose run locai-dev cargo install cargo-watch

# Run with auto-reload
docker-compose run -p 3000:3000 locai-dev \
  cargo watch -x 'run --bin locai-server'
```

## Registry and Distribution

### Tagging Strategy

```bash
# Tag with version
docker tag locai-server:latest locai-server:0.1.0-alpha.1

# Tag with git commit
docker tag locai-server:latest locai-server:$(git rev-parse --short HEAD)

# Tag with date
docker tag locai-server:latest locai-server:$(date +%Y%m%d)
```

### Push to Registry

```bash
# Docker Hub
docker tag locai-server:latest username/locai-server:latest
docker push username/locai-server:latest

# GitHub Container Registry
docker tag locai-server:latest ghcr.io/username/locai-server:latest
docker push ghcr.io/username/locai-server:latest

# Private registry
docker tag locai-server:latest registry.example.com/locai-server:latest
docker push registry.example.com/locai-server:latest
```

## Troubleshooting

### Check Container Logs

```bash
docker logs locai-server
docker logs -f --tail 100 locai-server  # Follow last 100 lines
```

### Interactive Shell

```bash
# Access running container
docker exec -it locai-server bash

# Start with shell (debugging)
docker run -it --entrypoint bash locai-server:latest
```

### Inspect Image

```bash
# Image layers
docker history locai-server:latest

# Image size breakdown
docker image inspect locai-server:latest | jq '.[0].Size'

# Files in image
docker run --rm locai-server:latest ls -la /usr/local/bin
```

### Common Issues

**Issue**: Container exits immediately
```bash
# Check logs for errors
docker logs locai-server

# Run with interactive terminal
docker run -it locai-server:latest
```

**Issue**: Permission denied errors
```bash
# Check volume permissions
docker run --rm -v locai-data:/data alpine ls -la /data

# Fix permissions
docker run --rm -v locai-data:/data alpine chown -R 1000:1000 /data
```

**Issue**: Out of memory
```bash
# Check container resource usage
docker stats locai-server

# Increase memory limit
docker update --memory="4g" locai-server
```

## Size Optimization

Current image size breakdown:
- Base image (debian:bookworm-slim): ~80MB
- Runtime dependencies: ~20MB
- Compiled binary: ~30-50MB
- **Total**: ~150MB

To reduce further:
1. Consider `gcr.io/distroless/cc-debian12` (~20MB) but no shell/debug tools
2. Use `strip` more aggressively (already done in release profile)
3. Remove unused dependencies from Cargo.toml

## Advanced Usage

### Multi-Architecture Builds

Build for multiple platforms:

```bash
# Enable buildx
docker buildx create --use

# Build for multiple architectures
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t locai-server:latest \
  --push \
  .
```

### Build Cache Optimization

Use external cache for CI/CD:

```bash
# Export cache
docker buildx build \
  --cache-to type=local,dest=.docker-cache \
  -t locai-server:latest \
  .

# Import cache
docker buildx build \
  --cache-from type=local,src=.docker-cache \
  -t locai-server:latest \
  .
```

## References

- [Dockerfile Best Practices](https://docs.docker.com/develop/develop-images/dockerfile_best-practices/)
- [Rust Docker Guide](https://docs.docker.com/language/rust/)
- [Docker BuildKit](https://docs.docker.com/build/buildkit/)

