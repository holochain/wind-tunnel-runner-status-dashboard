# wind-tunnel-runner-status-dashboard

A web app to view the connection status of Wind Tunnel Runner nodes.

The app polls the nomad api every minute for a list of nodes and caches the result to memory. There is no peristent storage.

## Building

```bash
cargo build --release
```

The binary will be at: ./target/release/nomad-clients-status

## Configuration

The application uses environment variables for configuration:

| Variable | Default | Description |
|----------|---------|-------------|
| `NOMAD_URL` | n/a | Nomad server URL |
| `NOMAD_ACCEPT_INVALID_CERT` | `false` | Should an invalid https certificate be accepted on requests to Nomad server api |
| `NOMAD_TOKEN` | n/a | Nomad server authentication token |
| `BIND_ADDR` | `0.0.0.0:3000` | Address and port to bind the HTTP server. |
| `UPDATE_SECONDS` | `60` | Interval to update the clients data, in seconds. |
| `RUST_LOG` | `info` | Log level (error, warn, info, debug, trace) |

## Running

### Run locally, connnected to wind tunnel nomad server

Replace `<nomad_token>` with a real token and run the following command:

```bash
NOMAD_URL=https://nomad-server-01.holochain.org:4646 NOMAD_ACCEPT_INVALID_CERT=true NOMAD_TOKEN=<nomad_token> cargo run --release
```

The server will be running at `127.0.0.1:3000`

### Run with docker

1. Build the image
```bash
docker build -t nomad-clients-status .
```

1. Replace `<nomad_token>` with a real token and run the image:
```bash
docker run -p 3000:3000 \
   -e NOMAD_URL=https://nomad-server-01.holochain.org:4646 \
   -e NOMAD_ACCEPT_INVALID_CERT=true \
   -e NOMAD_TOKEN=<nomad_token> \
   nomad-clients-status
```