# Ajime Agent

Lightweight Rust agent for edge devices that enables remote workflow orchestration, fleet management, and telemetry collection.

## Features

**Core Capabilities**
- Workflow execution engine with dependency resolution
- Real-time telemetry streaming via MQTT
- Automated sync with cloud backend
- Docker container orchestration
- Git repository management for CI/CD
- Hardware integration (GPIO, I2C, cameras)

**Security**
- JWT-based device authentication
- TLS encryption for all communications
- Secure token storage and auto-refresh
- Hardware-bound device identity

## Supported Platforms

- Raspberry Pi (32-bit and 64-bit ARM)
- NVIDIA Jetson (Nano, Xavier, Orin)
- x86_64 Linux servers
- Any ARM64 or x86_64 Linux system

## Quick Start

### Installation

```bash
curl -sSL https://install.ajime.io | sudo bash -s -- --token=<your-token>
```

Or download the binary for your platform and install manually:

```bash
sudo mv ajigent /usr/local/bin/ajigent
sudo chmod +x /usr/local/bin/ajigent
sudo ajigent --install --token=<your-token>
sudo systemctl enable --now ajigent
```

### Verify Installation

```bash
systemctl status ajigent
ajigent --version
curl http://localhost:8080/health
```

## Configuration

Agent configuration is stored at `/etc/ajime/settings.json`:

```json
{
  "log_level": "info",
  "backend": {
    "base_url": "https://api.ajime.io/agent/v1"
  },
  "mqtt_broker": {
    "host": "mqtt.ajime.io",
    "port": 8883,
    "tls": true
  },
  "polling_interval_secs": 30,
  "hardware": {
    "enable_camera": false,
    "enable_gpio": false
  }
}
```

## Local API

The agent exposes an HTTP API on `localhost:8080`:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/version` | GET | Agent version |
| `/device` | GET | Device information |
| `/device/sync` | POST | Trigger immediate sync |
| `/workflows/deployed` | GET | List deployed workflows |
| `/telemetry/metrics` | GET | System metrics |

## Management

```bash
# View logs
journalctl -u ajigent -f

# Restart agent
systemctl restart ajigent

# Force sync
curl -X POST http://localhost:8080/device/sync

# Check device info
curl http://localhost:8080/device
```

## Building from Source

```bash
cd agent
cargo build --release
```

The binary will be at `target/release/ajigent`.

## Architecture

**Components:**
- HTTP Server (Axum) for local API
- MQTT Client (rumqttc) for real-time messaging
- Workflow Executor with FSM-based state management
- Token Manager with automatic refresh
- Telemetry collector using sysinfo
- Docker integration via Docker Engine API

**Dependencies:**
- Tokio async runtime
- Axum web framework
- rumqttc MQTT client
- reqwest HTTP client
- serde for serialization
- tracing for structured logging

## Development

Run tests:
```bash
cargo test
```

Run locally:
```bash
cargo run -- --config config/dev.yaml
```

Build for all platforms:
```bash
cd build
./build.sh
```

## Troubleshooting

**Agent won't start:**
Check logs with `journalctl -u ajigent -n 50` and verify `/etc/ajime/device.json` exists.

**Connection issues:**
Verify network connectivity to `api.ajime.io` and ensure firewall allows outbound HTTPS (443) and MQTTS (8883).

**Permission errors:**
Ensure agent runs as root or has appropriate permissions for `/etc/ajime` directory.

## Support

- Documentation: https://docs.ajime.io/agent
- Issues: https://github.com/ajime/ajigent/issues
- Email: support@ajime.io
