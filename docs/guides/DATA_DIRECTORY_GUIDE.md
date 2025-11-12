# Locai Data Directory Configuration

## Overview

Locai stores all persistent data (RocksDB, SurrealDB embedded, indexes) in a configurable data directory. This guide explains how to configure it, especially for Docker deployments.

## Default Behavior

### Native Installation

**Default locations (platform-specific):**
- **Linux**: `~/.local/share/locai/data/`
- **macOS**: `~/Library/Application Support/org.locai.locai/data/`
- **Windows**: `%APPDATA%\locai\locai\data\`
- **Fallback**: `./data` (current directory)

### Docker Container

**Default**: `/data` (recommended for volume mounting)

## Data Structure

```
data_dir/
├── graph/           # RocksDB database files (SurrealDB embedded)
│   ├── 000001.log
│   ├── CURRENT
│   ├── LOCK
│   └── ...
└── vectors/         # Vector storage (if enabled)
    └── ...
```

## Configuration Methods

### 1. Environment Variable (Recommended for Docker)

The library uses `ConfigBuilder::with_data_dir()`, but there's no direct `LOCAI_DATA_DIR` environment variable in the server yet.

**Workaround**: Use a config file and mount it.

### 2. Config File (Best Method)

Create `config.json`:

```json
{
  "storage": {
    "data_dir": "/data",
    "graph": {
      "storage_type": "surrealdb",
      "path": "graph",
      "surrealdb": {
        "engine": "rocksdb",
        "connection": "/data/graph",
        "namespace": "locai",
        "database": "main"
      }
    }
  }
}
```

Or `config.yaml`:

```yaml
storage:
  data_dir: /data
  graph:
    storage_type: surrealdb
    path: graph
    surrealdb:
      engine: rocksdb
      connection: /data/graph
      namespace: locai
      database: main
```

### 3. Programmatic (CLI/Server Code)

The server uses `ConfigBuilder` in `main.rs`:

```rust
let config = ConfigBuilder::new()
    .with_data_dir("/path/to/data")
    .with_default_storage()
    .build()?;
```

## Docker Volume Configuration

### Option 1: Named Volume (Recommended)

**Best for**: Production, easy backups, Docker-managed storage

```bash
# Create named volume
docker volume create locai-data

# Run with named volume
docker run -d \
  --name locai-server \
  -p 3000:3000 \
  -v locai-data:/data \
  locai-server:latest
```

**docker-compose.yml:**
```yaml
services:
  locai-server:
    image: locai-server:latest
    volumes:
      - locai-data:/data  # Named volume
    environment:
      - LOCAI_DATA_DIR=/data  # Future support

volumes:
  locai-data:
    driver: local
```

### Option 2: Bind Mount (Host Directory)

**Best for**: Development, easy access to files, specific location

```bash
# Create host directory
mkdir -p ./locai-data

# Run with bind mount
docker run -d \
  --name locai-server \
  -p 3000:3000 \
  -v $(pwd)/locai-data:/data \
  locai-server:latest
```

**docker-compose.yml:**
```yaml
services:
  locai-server:
    image: locai-server:latest
    volumes:
      - ./locai-data:/data  # Bind mount to host
```

### Option 3: Custom Path with Config File

**Best for**: Multiple instances, custom directory structure

```bash
# Create config file
cat > config.json <<EOF
{
  "storage": {
    "data_dir": "/custom/path",
    "graph": {
      "surrealdb": {
        "engine": "rocksdb",
        "connection": "/custom/path/graph"
      }
    }
  }
}
EOF

# Run with custom config
docker run -d \
  --name locai-server \
  -p 3000:3000 \
  -v $(pwd)/config.json:/data/config.json:ro \
  -v $(pwd)/custom-data:/custom/path \
  locai-server:latest \
  --config /data/config.json
```

## Recommended Docker Setup

### Development

```yaml
version: '3.8'
services:
  locai-server:
    image: locai-server:latest
    ports:
      - "3000:3000"
    volumes:
      # Bind mount for easy access
      - ./locai-data:/data
    environment:
      - RUST_LOG=debug
    restart: unless-stopped
```

### Production

```yaml
version: '3.8'
services:
  locai-server:
    image: locai-server:0.1.0-alpha.1  # Pin version
    ports:
      - "3000:3000"
    volumes:
      # Named volume for Docker-managed storage
      - locai-data:/data
      # Read-only config
      - ./config.json:/data/config.json:ro
    environment:
      - RUST_LOG=info,locai=debug
      - LOCAI_ENABLE_AUTH=true
    restart: unless-stopped
    deploy:
      resources:
        limits:
          memory: 2G
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/api/health"]
      interval: 30s
      timeout: 5s
      retries: 3

volumes:
  locai-data:
    driver: local
    # Optional: Specify host mount point
    # driver_opts:
    #   type: none
    #   device: /mnt/storage/locai
    #   o: bind
```

## Permission Management

### Issue: Permission Denied

The Docker container runs as user `locai` (UID 1000), so the volume needs proper permissions.

**Fix for bind mounts:**

```bash
# Option 1: Change ownership to UID 1000
sudo chown -R 1000:1000 ./locai-data

# Option 2: Run container as your user
docker run -d \
  --user $(id -u):$(id -g) \
  -v $(pwd)/locai-data:/data \
  locai-server:latest

# Option 3: Create directory with correct permissions
mkdir -m 755 locai-data
docker run --rm -v $(pwd)/locai-data:/data alpine chown 1000:1000 /data
```

## Volume Operations

### Backup Data

```bash
# Named volume backup
docker run --rm \
  -v locai-data:/data \
  -v $(pwd)/backups:/backup \
  alpine tar czf /backup/locai-backup-$(date +%Y%m%d).tar.gz /data

# Bind mount backup
tar czf backup.tar.gz ./locai-data/
```

### Restore Data

```bash
# Named volume restore
docker run --rm \
  -v locai-data:/data \
  -v $(pwd)/backups:/backup \
  alpine tar xzf /backup/locai-backup-20241024.tar.gz -C /

# Bind mount restore
tar xzf backup.tar.gz
```

### Inspect Volume

```bash
# List volumes
docker volume ls | grep locai

# Inspect volume
docker volume inspect locai-data

# View contents
docker run --rm -v locai-data:/data alpine ls -la /data
docker run --rm -v locai-data:/data alpine du -sh /data/*
```

### Migrate Data

```bash
# Copy from one volume to another
docker volume create locai-data-new
docker run --rm \
  -v locai-data:/source \
  -v locai-data-new:/dest \
  alpine sh -c "cp -av /source/* /dest/"
```

## Troubleshooting

### Problem: Database Lock Error

```
Error: Database lock failed
```

**Cause**: Another process is using the database.

**Solution**:
```bash
# Stop all containers using the volume
docker ps -a | grep locai
docker stop locai-server

# Check for lock files
docker run --rm -v locai-data:/data alpine ls -la /data/graph/LOCK

# Remove lock file if stuck
docker run --rm -v locai-data:/data alpine rm /data/graph/LOCK

# Restart
docker start locai-server
```

### Problem: Volume Not Persisting

```bash
# Verify volume is mounted
docker inspect locai-server | jq '.[0].Mounts'

# Check volume exists
docker volume ls | grep locai

# Verify data is written
docker exec locai-server ls -la /data
```

### Problem: Out of Disk Space

```bash
# Check volume size
docker system df -v | grep locai-data

# Check container disk usage
docker exec locai-server du -sh /data/*

# Cleanup old data (if needed)
docker exec locai-server find /data -type f -mtime +30 -delete
```

## Best Practices

1. **Use named volumes in production**
   - Docker-managed
   - Easier backups
   - Better isolation

2. **Use bind mounts in development**
   - Easy file access
   - Direct editing
   - Faster iteration

3. **Always specify volume**
   - Never rely on container storage
   - Data will be lost on container removal

4. **Set proper permissions**
   - Container runs as UID 1000
   - Ensure directory is writable

5. **Regular backups**
   - Automate with cron
   - Test restore process
   - Keep multiple versions

6. **Monitor disk usage**
   - RocksDB can grow large
   - Set up alerts
   - Plan for growth

7. **Use read-only config**
   - Mount config as `:ro`
   - Prevents accidental changes
   - Better security

## Advanced: Multiple Instances

Run multiple Locai instances with separate data:

```yaml
version: '3.8'
services:
  locai-dev:
    image: locai-server:latest
    ports:
      - "3000:3000"
    volumes:
      - locai-dev-data:/data

  locai-staging:
    image: locai-server:latest
    ports:
      - "3001:3000"
    volumes:
      - locai-staging-data:/data

  locai-prod:
    image: locai-server:0.1.0-alpha.1
    ports:
      - "3002:3000"
    volumes:
      - locai-prod-data:/data

volumes:
  locai-dev-data:
  locai-staging-data:
  locai-prod-data:
```

## Future Enhancements

Planned improvements:

- [ ] `LOCAI_DATA_DIR` environment variable support
- [ ] Data directory validation on startup
- [ ] Automatic backup integration
- [ ] Cloud storage backends (S3, GCS)
- [ ] Data encryption at rest
- [ ] Multi-region replication

## Related Documentation

- [Docker Guide](./DOCKER.md)
- [Configuration Reference](./CONFIGURATION.md)
- [Backup & Recovery](./BACKUP.md)
- [Performance Tuning](./PERFORMANCE.md)

## Summary

**Recommended approach for Docker:**

```yaml
# docker-compose.yml
services:
  locai-server:
    image: locai-server:latest
    volumes:
      - locai-data:/data  # Named volume = best practice
    # Data will be stored in /data/graph/ (RocksDB)

volumes:
  locai-data:
```

**Key Points:**
- ✅ Default container path: `/data`
- ✅ RocksDB files: `/data/graph/`
- ✅ Always mount a volume (named or bind)
- ✅ Container runs as UID 1000
- ✅ Use config file for custom paths


