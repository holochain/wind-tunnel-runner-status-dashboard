# wind-tunnel-runner-status-dashboard

A web app to view the connection status of Wind Tunnel Runner nodes.

## Features

- **Automatic Polling**: Queries Nomad server every 60 seconds for client list
- **In-Memory Cache**: Fast lookups with thread-safe caching using RwLock
- **REST API**: Simple JSON API to query client status by hostname
- **Web Interface**: Clean, modern HTML/JavaScript frontend
- **CORS Enabled**: Ready for cross-origin requests


## Building

```bash
# Build in release mode for optimal performance
cargo build --release

# The binary will be at: ./target/release/nomad-clients-status
```

## Configuration

The application uses environment variables for configuration:

| Variable | Default | Description |
|----------|---------|-------------|
| `NOMAD_ADDR` | `http://localhost:4646` | Nomad server URL |
| `BIND_ADDR` | `0.0.0.0:8080` | Address and port to bind the HTTP server |
| `RUST_LOG` | `info` | Log level (error, warn, info, debug, trace) |

## Running

### Basic usage

```bash
# Run with default settings (Nomad at localhost:4646, API at 0.0.0.0:8080)
cargo run --release
```

### With custom Nomad server

```bash
# Point to a remote Nomad server
NOMAD_ADDR=http://nomad.example.com:4646 cargo run --release
```

### With custom bind address

```bash
# Bind to specific address
BIND_ADDR=127.0.0.1:3000 cargo run --release
```

### With debug logging

```bash
# Enable debug logging
RUST_LOG=debug cargo run --release
```

### Full example

```bash
NOMAD_ADDR=http://nomad.example.com:4646 \
BIND_ADDR=0.0.0.0:8080 \
RUST_LOG=info \
./target/release/nomad-clients-status
```

## API Endpoints

### 1. Get Client Status

Returns the status of a specific Nomad client by hostname.

**Endpoint**: `GET /api/client/{hostname}`

**Example**:
```bash
curl http://localhost:8080/api/client/node-001
```

**Response** (client found):
```json
{
  "hostname": "node-001",
  "status": "ready",
  "status_description": "Node is ready",
  "datacenter": "dc1",
  "node_class": "compute",
  "version": "1.7.2",
  "id": "abc123def456...",
  "found": true
}
```

**Response** (client not found):
```json
{
  "hostname": "node-999",
  "status": null,
  "status_description": null,
  "datacenter": null,
  "node_class": null,
  "version": null,
  "id": null,
  "found": false
}
```

### 2. List All Clients

Returns an array of all known client hostnames.

**Endpoint**: `GET /api/clients`

**Example**:
```bash
curl http://localhost:8080/api/clients
```

**Response**:
```json
[
  "node-001",
  "node-002",
  "node-003"
]
```

### 3. Web Interface

Access the web interface at the root URL.

**Endpoint**: `GET /`

**URL**: http://localhost:8080/

## Web Interface Features

The web interface provides:

- **Search by Hostname**: Enter a client hostname to view its status
- **List All Clients**: View all registered clients (clickable)
- **Real-time Data**: Shows cached data from the last Nomad API poll
- **Status Badges**: Color-coded status indicators (ready, down, initializing)
- **Detailed Information**: Shows datacenter, node class, version, and node ID

## Architecture

### Components

1. **Nomad API Client** (src/main.rs:48-67)
   - Fetches node list from Nomad's `/v1/nodes` endpoint
   - Handles HTTP errors and timeouts
   - Uses reqwest for HTTP requests

2. **In-Memory Cache** (src/main.rs:42-44)
   - Thread-safe HashMap protected by RwLock
   - Maps hostname → NomadNode
   - Updated every 60 seconds

3. **Background Refresh Task** (src/main.rs:89-94)
   - Tokio task that runs indefinitely
   - Calls update_cache() every 60 seconds
   - Logs errors but continues running

4. **REST API** (src/main.rs:97-136)
   - Built with actix-web
   - CORS enabled for cross-origin requests
   - JSON responses with serde

5. **Web Frontend** (static/index.html)
   - Pure HTML/CSS/JavaScript (no frameworks)
   - Responsive design
   - Client-side API calls with fetch()

### Data Flow

```
Nomad Server → [HTTP Poll] → Nomad API Client
                                     ↓
                              Update Cache (RwLock)
                                     ↓
Web Client → [HTTP GET] → API Endpoint → Read Cache → JSON Response
```

## Cache Behavior

- **Initial Population**: Cache is populated on startup before accepting requests
- **Refresh Interval**: 60 seconds (fixed)
- **Cache Update**: Complete replacement (clear + insert all)
- **Read Performance**: O(1) HashMap lookup with read lock
- **Thread Safety**: Multiple concurrent reads, exclusive writes

## Logging

The application logs to stdout/stderr using env_logger. Log levels:

- `ERROR`: Critical errors
- `WARN`: Warnings (not used currently)
- `INFO`: Startup, cache updates, API requests
- `DEBUG`: Detailed execution flow
- `TRACE`: Very verbose output

Example log output:
```
[2024-01-06T12:00:00Z INFO  nomad_clients_status] Starting Nomad Clients Status API
[2024-01-06T12:00:00Z INFO  nomad_clients_status] Nomad server: http://localhost:4646
[2024-01-06T12:00:00Z INFO  nomad_clients_status] Performing initial cache population...
[2024-01-06T12:00:00Z INFO  nomad_clients_status] Fetching clients from Nomad: http://localhost:4646/v1/nodes
[2024-01-06T12:00:01Z INFO  nomad_clients_status] Successfully fetched 5 clients from Nomad
[2024-01-06T12:00:01Z INFO  nomad_clients_status] Cache updated with 5 clients
[2024-01-06T12:00:01Z INFO  nomad_clients_status] Starting HTTP server on 0.0.0.0:8080
```

## Error Handling

- **Nomad Unreachable**: Logged as error, cache remains unchanged
- **Invalid JSON**: Logged as error, cache remains unchanged
- **Timeout**: 10-second timeout on Nomad API requests
- **Client Not Found**: Returns JSON with `found: false`

## Production Deployment

### Systemd Service

Create `/etc/systemd/system/nomad-clients-status.service`:

```ini
[Unit]
Description=Nomad Clients Status API
After=network.target

[Service]
Type=simple
User=nomad-api
Environment="NOMAD_ADDR=http://localhost:4646"
Environment="BIND_ADDR=0.0.0.0:8080"
Environment="RUST_LOG=info"
ExecStart=/usr/local/bin/nomad-clients-status
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable nomad-clients-status
sudo systemctl start nomad-clients-status
```

### Docker

Create a `Dockerfile`:

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/nomad-clients-status /usr/local/bin/
ENV NOMAD_ADDR=http://localhost:4646
ENV BIND_ADDR=0.0.0.0:8080
EXPOSE 8080
CMD ["nomad-clients-status"]
```

Build and run:
```bash
docker build -t nomad-clients-status .
docker run -p 8080:8080 -e NOMAD_ADDR=http://host.docker.internal:4646 nomad-clients-status
```

## Development

### Running in development mode

```bash
# Faster compile times, includes debug symbols
cargo run
```

### Running tests

```bash
cargo test
```

### Code formatting

```bash
cargo fmt
```

### Linting

```bash
cargo clippy
```

## License

This project is provided as-is for use with HashiCorp Nomad.

## Troubleshooting

### "linker `cc` not found"

Install a C compiler:
```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# Fedora/RHEL
sudo dnf install gcc
```

### "Failed to fetch clients from Nomad"

Check:
1. Nomad server is running: `curl http://localhost:4646/v1/status/leader`
2. NOMAD_ADDR is correct
3. Firewall allows access to Nomad port

### "Address already in use"

Another process is using port 8080. Either:
1. Stop the other process
2. Use a different port: `BIND_ADDR=0.0.0.0:8081 cargo run`

### Empty client list

Check:
1. Nomad server has registered clients: `nomad node status`
2. API is actually querying Nomad (check logs)
3. Cache refresh is working (check logs every 60s)
