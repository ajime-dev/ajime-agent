# Ajime Agent Installation Guide

## Overview

The Ajime Agent is a lightweight Rust binary that runs on edge devices (Raspberry Pi, Jetson Nano, etc.) and syncs with the Ajime web server for workflow orchestration.

## Quick Installation

### Using the Installation Script

```bash
curl -sSL https://install.ajime.io | sudo bash -s -- --token=<your-activation-token>
```

### Manual Installation

1. Download the appropriate binary for your platform:
   - `ajigent-linux-aarch64` for Raspberry Pi 64-bit, Jetson Nano
   - `ajigent-linux-armv7` for Raspberry Pi 32-bit
   - `ajigent-linux-x86_64` for x86_64 Linux

2. Install the binary:
   ```bash
   sudo mv ajigent-linux-* /usr/local/bin/ajigent
   sudo chmod +x /usr/local/bin/ajigent
   ```

3. Create configuration directories:
   ```bash
   sudo mkdir -p /etc/ajime/{cache/workflows,cache/configs,deployments,logs,tokens}
   sudo chmod 700 /etc/ajime /etc/ajime/tokens
   ```

4. Activate the agent:
   ```bash
   sudo ajigent --install --token=<your-activation-token>
   ```

5. Install and start the systemd service:
   ```bash
   sudo systemctl enable ajigent
   sudo systemctl start ajigent
   ```

## Getting an Activation Token

1. Log in to the Ajime web dashboard
2. Navigate to Devices > Add Device
3. Select "Agent Device" as the connection type
4. Click "Generate Activation Token"
5. Copy the token and use it in the installation command

## Configuration

The agent configuration is stored in `/etc/ajime/settings.json`:

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
  "is_persistent": true,
  "enable_socket_server": true,
  "enable_mqtt_worker": true,
  "enable_poller": true,
  "polling_interval_secs": 30,
  "hardware": {
    "enable_camera": false,
    "enable_gpio": false,
    "camera_device": "/dev/video0"
  }
}
```

## Useful Commands

```bash
# Check agent status
systemctl status ajigent

# View logs
journalctl -u ajigent -f

# Restart agent
systemctl restart ajigent

# Stop agent
systemctl stop ajigent

# Check agent version
ajigent --version

# Manual sync
curl http://localhost:8080/device/sync -X POST
```

## Local API

The agent exposes a local HTTP API on port 8080:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/version` | GET | Agent version |
| `/device` | GET | Device info |
| `/device/sync` | POST | Trigger sync |
| `/workflows/deployed` | GET | List deployed workflows |
| `/telemetry/metrics` | GET | System metrics |

## Troubleshooting

### Agent won't start

1. Check logs: `journalctl -u ajigent -n 50`
2. Verify device file exists: `ls -la /etc/ajime/device.json`
3. Check settings file: `cat /etc/ajime/settings.json`

### Connection issues

1. Verify network connectivity: `ping api.ajime.io`
2. Check firewall rules for outbound HTTPS (443) and MQTT (8883)
3. Verify token hasn't expired

### Permission issues

1. Ensure agent runs as root or has appropriate permissions
2. Check directory permissions: `ls -la /etc/ajime`

## Uninstallation

```bash
sudo systemctl stop ajigent
sudo systemctl disable ajigent
sudo rm /lib/systemd/system/ajigent.service
sudo rm /usr/local/bin/ajigent
sudo rm -rf /etc/ajime  # Optional: remove configuration
sudo systemctl daemon-reload
```

## Support

For issues and questions:
- Documentation: https://docs.ajime.io/agent
- GitHub Issues: https://github.com/ajime/ajigent/issues
- Email: support@ajime.io
