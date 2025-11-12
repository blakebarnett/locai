# Docker Quick Start

Get Locai running in Docker in under 2 minutes! 🚀

## Prerequisites

- Docker 20.10+ or Docker Desktop
- docker-compose 1.29+ (usually included with Docker Desktop)

## Option 1: Docker Compose (Easiest)

```bash
# Clone the repository
git clone https://github.com/blakebarnett/locai.git
cd locai

# Start the server
docker-compose up -d

# View logs
docker-compose logs -f
```

**That's it!** The server is now running at:
- API: http://localhost:3000
- Swagger UI: http://localhost:3000/docs
- OpenAPI Spec: http://localhost:3000/api-docs/openapi.json

### Quick Test

```bash
# Check health
curl http://localhost:3000/api/health

# Create a memory
curl -X POST http://localhost:3000/api/memories \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Docker makes deployment easy!",
    "memory_type": "fact",
    "priority": "Normal"
  }'

# List memories
curl http://localhost:3000/api/memories
```

## Option 2: Using Makefile (Recommended for Development)

```bash
# Build the Docker image
make docker-build

# Run the container
make docker-run

# View logs
make docker-logs

# Stop the container
make docker-stop

# See all available commands
make help
```

## Option 3: Plain Docker

```bash
# Build
docker build -t locai-server:latest .

# Run
docker run -d \
  --name locai-server \
  -p 3000:3000 \
  -v locai-data:/data \
  locai-server:latest

# Check logs
docker logs -f locai-server
```

## Configuration

### Environment Variables

Create a `.env` file from the example:

```bash
cp .env.example .env
```

Edit `.env` to customize:
- Server port
- Logging levels
- Authentication settings
- Database configuration

Then restart:

```bash
docker-compose up -d --force-recreate
```

### Persistent Data

By default, data is stored in a Docker volume named `locai-data`.

To use a specific host directory:

```yaml
# In docker-compose.yml
volumes:
  - ./data:/data  # Use local ./data directory
```

## Common Tasks

### View Logs

```bash
# Follow logs
docker-compose logs -f

# Last 100 lines
docker-compose logs --tail 100

# Specific service logs
docker-compose logs locai-server
```

### Update to Latest Version

```bash
# Pull latest code
git pull origin main

# Rebuild and restart
docker-compose up -d --build
```

### Backup Data

```bash
# Backup the data volume
docker run --rm \
  -v locai-data:/data \
  -v $(pwd):/backup \
  alpine tar czf /backup/locai-backup-$(date +%Y%m%d).tar.gz /data
```

### Restore Data

```bash
# Restore from backup
docker run --rm \
  -v locai-data:/data \
  -v $(pwd):/backup \
  alpine tar xzf /backup/locai-backup-20240101.tar.gz -C /
```

### Shell Access

```bash
# Access running container
docker exec -it locai-server bash

# Or with docker-compose
docker-compose exec locai-server bash
```

### Debugging

```bash
# Enable debug logging
docker-compose stop
docker-compose run -e RUST_LOG=debug -e RUST_BACKTRACE=1 locai-server

# Run with interactive terminal
docker run -it --rm locai-server:latest bash
```

## Production Deployment

For production, consider:

1. **Use a specific version tag**:
   ```yaml
   image: locai-server:0.1.0-alpha.1  # Not :latest
   ```

2. **Enable authentication**:
   ```yaml
   environment:
     - LOCAI_ENABLE_AUTH=true
     - LOCAI_JWT_SECRET=your-secure-secret-here
   ```

3. **Set resource limits**:
   ```yaml
   deploy:
     resources:
       limits:
         cpus: '2'
         memory: 2G
   ```

4. **Use a reverse proxy** (nginx, traefik):
   ```yaml
   labels:
     - "traefik.enable=true"
     - "traefik.http.routers.locai.rule=Host(`locai.example.com`)"
   ```

5. **Regular backups**:
   ```bash
   # Add to crontab
   0 2 * * * cd /path/to/locai && docker-compose exec -T locai-server tar czf - /data > backup-$(date +\%Y\%m\%d).tar.gz
   ```

## Troubleshooting

### Container won't start

```bash
# Check logs for errors
docker logs locai-server

# Verify port isn't in use
lsof -i :3000
```

### Permission errors

```bash
# Fix data volume permissions
docker run --rm -v locai-data:/data alpine chown -R 1000:1000 /data
```

### Out of disk space

```bash
# Clean up unused Docker resources
docker system prune -a

# Check volume sizes
docker system df -v
```

### Slow performance

```bash
# Check resource usage
docker stats locai-server

# Increase memory limit in docker-compose.yml
```

## Next Steps

- 📖 Read the full [Docker Documentation](./DOCKER.md)
- 🔧 Explore the [API Documentation](http://localhost:3000/docs)
- 🚀 Check out the [Examples](../examples/)
- 💬 Join our [Community Discord](https://discord.gg/locai)

## Support

- 🐛 [Report Issues](https://github.com/blakebarnett/locai/issues)
- 💡 [Feature Requests](https://github.com/blakebarnett/locai/discussions)
- 📚 [Full Documentation](https://github.com/blakebarnett/locai/tree/main/docs)


