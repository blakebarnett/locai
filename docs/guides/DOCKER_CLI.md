# Locai CLI Docker Guide

The Locai CLI is available as a Docker image for running commands in containerized environments.

## Why a Separate CLI Image?

**Use Cases:**
- 🔧 Run one-off memory operations
- 🤖 CI/CD automation and pipelines
- 🚀 Kubernetes Jobs and CronJobs
- 📦 Portable tooling without local installation
- 🔗 Interact with Locai server from other containers

## Quick Start

### Build the CLI Image

```bash
# Build CLI image
docker build -f Dockerfile.cli -t locai-cli:latest .

# Or use the Makefile
make docker-build-cli
```

### Run CLI Commands

```bash
# Show help
docker run --rm locai-cli:latest --help

# Create a memory
docker run --rm -v locai-data:/data locai-cli:latest \
  memory add "Docker CLI is working!" --priority high

# List memories
docker run --rm -v locai-data:/data locai-cli:latest \
  memory list

# Search memories
docker run --rm -v locai-data:/data locai-cli:latest \
  memory search "docker" --limit 10
```

## Using with Docker Compose

### Interactive Mode

```bash
# Start server
docker-compose up -d locai-server

# Run CLI commands against shared data
docker-compose run --rm locai-cli memory list
docker-compose run --rm locai-cli memory add "Hello from CLI"
docker-compose run --rm locai-cli entity list
```

### Shell Access

```bash
# Open interactive shell with CLI available
docker-compose run --rm locai-cli bash

# Inside the container:
locai-cli --help
locai-cli memory list
locai-cli memory search "query"
```

## Common Patterns

### 1. One-Off Commands

```bash
# Create a memory
docker run --rm -v $(pwd)/data:/data locai-cli:latest \
  memory add "Important note" \
  --priority high \
  --tags important,note

# Export memories to JSON
docker run --rm -v $(pwd)/data:/data -v $(pwd):/output locai-cli:latest \
  memory list --output json > /output/memories.json
```

### 2. CI/CD Integration

```yaml
# GitHub Actions example
jobs:
  backup-memories:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Pull CLI image
        run: docker pull ghcr.io/blakebarnett/locai-cli:latest
      
      - name: Export memories
        run: |
          docker run --rm \
            -v ${{ secrets.DATA_PATH }}:/data \
            -v ./backup:/output \
            ghcr.io/blakebarnett/locai-cli:latest \
            memory list --output json > backup/memories-$(date +%Y%m%d).json
      
      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: memory-backup
          path: backup/
```

### 3. Kubernetes CronJob

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: locai-memory-cleanup
spec:
  schedule: "0 2 * * *"  # Daily at 2 AM
  jobTemplate:
    spec:
      template:
        spec:
          containers:
          - name: locai-cli
            image: ghcr.io/blakebarnett/locai-cli:latest
            command:
            - locai-cli
            - memory
            - cleanup
            - --older-than
            - 30d
            volumeMounts:
            - name: data
              mountPath: /data
          volumes:
          - name: data
            persistentVolumeClaim:
              claimName: locai-data
          restartPolicy: OnFailure
```

### 4. Batch Operations

```bash
# Import memories from file
cat memories.txt | while read line; do
  docker run --rm -v locai-data:/data locai-cli:latest \
    memory add "$line"
done

# Or use a script
docker run --rm -v locai-data:/data -v ./scripts:/scripts locai-cli:latest \
  bash /scripts/import-memories.sh
```

## Advanced Usage

### Data Volume Management

```bash
# Use specific host directory
docker run --rm -v /path/to/data:/data locai-cli:latest memory list

# Share data with server
docker run --rm \
  --volumes-from locai-server \
  locai-cli:latest memory list

# Create backup
docker run --rm \
  -v locai-data:/data \
  -v $(pwd)/backups:/backup \
  alpine tar czf /backup/locai-backup.tar.gz /data
```

### Environment Variables

```bash
# Set data directory
docker run --rm \
  -e LOCAI_DATA_DIR=/custom/path \
  -v $(pwd)/data:/custom/path \
  locai-cli:latest memory list

# Enable debug logging
docker run --rm \
  -e RUST_LOG=debug \
  -v locai-data:/data \
  locai-cli:latest memory search "query"
```

### Custom Entry Point

```bash
# Run arbitrary commands
docker run --rm -it --entrypoint bash locai-cli:latest

# Inside container:
locai-cli --version
locai-cli memory --help
```

## Aliases and Helper Scripts

### Bash Aliases

Add to your `~/.bashrc` or `~/.zshrc`:

```bash
# Simple alias
alias locai-cli='docker run --rm -v locai-data:/data locai-cli:latest'

# Function with current directory data
locai() {
  docker run --rm -v $(pwd)/locai-data:/data locai-cli:latest "$@"
}

# Usage:
locai-cli memory list
locai memory create "Hello"
```

### Wrapper Script

Create `locai-docker.sh`:

```bash
#!/bin/bash
# Wrapper script for Locai CLI in Docker

DATA_DIR="${LOCAI_DATA_DIR:-$(pwd)/locai-data}"
IMAGE="${LOCAI_CLI_IMAGE:-locai-cli:latest}"

# Create data directory if it doesn't exist
mkdir -p "$DATA_DIR"

# Run CLI with mounted data directory
docker run --rm \
  -v "$DATA_DIR:/data" \
  -e RUST_LOG="${RUST_LOG:-info}" \
  "$IMAGE" "$@"
```

Make it executable and use:
```bash
chmod +x locai-docker.sh
./locai-docker.sh memory list
./locai-docker.sh --help
```

## Image Comparison

| Feature | Server Image | CLI Image |
|---------|-------------|-----------|
| Size | ~150MB | ~150MB |
| Purpose | Long-running service | One-off commands |
| Entry Point | `locai-server` | `locai-cli` |
| Ports | 3000 exposed | None |
| Dependencies | Full (embedded DB) | Full (embedded DB) |
| Use Case | API server | Command-line tools |

**Note:** Both images currently have similar size because the CLI uses embedded database support. A future "slim" CLI that only talks to the API server could be much smaller.

## Troubleshooting

### Permission Issues

```bash
# Fix data directory permissions
docker run --rm -v locai-data:/data alpine chown -R 1000:1000 /data

# Run as specific user
docker run --rm --user 1000:1000 -v locai-data:/data locai-cli:latest memory list
```

### Volume Not Found

```bash
# Create volume explicitly
docker volume create locai-data

# Inspect volume
docker volume inspect locai-data

# List volumes
docker volume ls | grep locai
```

### CLI Not Found

```bash
# Verify binary location
docker run --rm --entrypoint ls locai-cli:latest -la /usr/local/bin

# Test binary directly
docker run --rm --entrypoint /usr/local/bin/locai-cli locai-cli:latest --version
```

## Best Practices

1. **Volume Mounting**
   - Always mount `/data` for persistence
   - Use named volumes for production
   - Use bind mounts for development

2. **Resource Limits**
   ```bash
   docker run --rm \
     --memory="512m" \
     --cpus="1.0" \
     -v locai-data:/data \
     locai-cli:latest memory list
   ```

3. **Security**
   - CLI runs as non-root user (UID 1000)
   - Use read-only mounts when possible
   - Don't expose unnecessary volumes

4. **Logging**
   - Set `RUST_LOG` for appropriate verbosity
   - Redirect output to files for records
   - Use `--output json` or `--machine` for machine-readable output

## Future Enhancements

Planned improvements for the CLI image:

- 🎯 **Slim variant** - API-only client (~30MB)
- 🔌 **Remote mode** - Connect to Locai server over HTTP
- 📊 **Additional formats** - CSV, YAML output support
- 🔄 **Batch operations** - Built-in import/export tools
- 🐚 **Interactive mode** - REPL-style interface

## Examples Repository

See the [`examples/docker-cli/`](../../examples/docker-cli/) directory for:
- Sample scripts
- CI/CD templates
- Kubernetes manifests
- Common workflows

## Related Documentation

- [Main Docker Guide](./DOCKER.md)
- [Docker Quick Start](./DOCKER_QUICK_START.md)
- [CLI User Guide](./CLI.md)
- [API Documentation](./API.md)

## Support

Questions or issues with the Docker CLI?
- 🐛 [Report Issues](https://github.com/blakebarnett/locai/issues)
- 💬 [Discussions](https://github.com/blakebarnett/locai/discussions)
- 📚 [Documentation](https://github.com/blakebarnett/locai/tree/main/docs)


