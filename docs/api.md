# Ajime Agent API Documentation

## Overview

The Ajime Agent exposes two types of APIs:

1. **Local HTTP API** - For local device control and monitoring
2. **Backend API** - For communication with the Ajime web server

## Local HTTP API

The local API runs on `http://localhost:8080` by default.

### Health Check

```http
GET /health
```

**Response:**
```json
{
  "status": "healthy",
  "service": "ajime-agent",
  "version": "0.1.0"
}
```

### Version Info

```http
GET /version
```

**Response:**
```json
{
  "version": "0.1.0",
  "git_hash": "abc1234",
  "build_time": "2025-02-07 10:00:00 UTC"
}
```

### Device Info

```http
GET /device
```

**Response:**
```json
{
  "id": "device-abc123",
  "name": "my-raspberry-pi",
  "device_type": "raspberry_pi",
  "status": "online",
  "owner_id": "user-xyz789"
}
```

### Trigger Sync

```http
POST /device/sync
```

**Response:**
```json
{
  "success": true,
  "message": "Sync completed successfully"
}
```

### List Deployed Workflows

```http
GET /workflows/deployed
```

**Response:**
```json
{
  "workflows": [
    {
      "id": "wf-123",
      "name": "Camera Capture",
      "status": "deployed"
    }
  ],
  "total": 1
}
```

### System Metrics

```http
GET /telemetry/metrics
```

**Response:**
```json
{
  "cpu_usage": 25.5,
  "memory_used": 512000000,
  "memory_total": 4096000000,
  "memory_percent": 12.5,
  "disk_used": 10000000000,
  "disk_total": 32000000000,
  "disk_percent": 31.25,
  "uptime_secs": 86400,
  "hostname": "my-raspberry-pi"
}
```

## Backend API (Agent Client)

These endpoints are called by the agent to communicate with the Ajime web server.

### Device Activation

```http
POST /api/v1/agent/devices/activate
Content-Type: application/json

{
  "activation_token": "base64-encoded-token",
  "device_name": "my-device",
  "device_type": "raspberry_pi"
}
```

**Response:**
```json
{
  "device_id": "device-abc123",
  "owner_id": "user-xyz789",
  "token": "jwt-device-token",
  "device_name": "my-device"
}
```

### Token Refresh

```http
POST /api/v1/agent/devices/{device_id}/token/refresh
Authorization: Bearer <current-token>
```

**Response:**
```json
{
  "token": "new-jwt-token",
  "expires_at": "2025-05-07T00:00:00Z"
}
```

### Update Device Status

```http
PUT /api/v1/agent/devices/{device_id}/status
Authorization: Bearer <device-token>
Content-Type: application/json

{
  "status": "online",
  "agent_version": "0.1.0",
  "last_sync_at": 1707307200,
  "metrics": {
    "cpu_usage": 25.5,
    "memory_percent": 12.5
  }
}
```

### Sync Device

```http
POST /api/v1/agent/devices/{device_id}/sync
Authorization: Bearer <device-token>
Content-Type: application/json

{
  "agent_version": "0.1.0",
  "local_workflow_digests": ["hash1", "hash2"]
}
```

**Response:**
```json
{
  "device_id": "device-abc123",
  "workflows_to_update": ["wf-456"],
  "workflows_to_remove": ["wf-old"],
  "settings_updated": false
}
```

### Get Device Workflows

```http
GET /api/v1/agent/devices/{device_id}/workflows
Authorization: Bearer <device-token>
```

**Response:**
```json
{
  "workflows": [
    {
      "id": "wf-123",
      "name": "Camera Capture",
      "status": "active",
      "logic_hash": "sha256-hash",
      "updated_at": "2025-02-07T00:00:00Z"
    }
  ],
  "total": 1
}
```

### Sync Workflows

```http
POST /api/v1/agent/devices/{device_id}/workflows/sync
Authorization: Bearer <device-token>
Content-Type: application/json

[
  {
    "workflow_id": "wf-123",
    "digest": "local-hash"
  }
]
```

**Response:**
```json
{
  "workflows": [
    {
      "id": "wf-123",
      "name": "Camera Capture",
      "graph_data": { ... }
    }
  ],
  "digests": [
    {
      "workflow_id": "wf-123",
      "digest": "new-hash",
      "updated_at": "2025-02-07T00:00:00Z"
    }
  ]
}
```

### Report Workflow Status

```http
POST /api/v1/agent/devices/{device_id}/workflows/{workflow_id}/status
Authorization: Bearer <device-token>
Content-Type: application/json

{
  "status": "running",
  "started_at": "2025-02-07T10:00:00Z",
  "node_statuses": [
    {
      "node_id": "node-1",
      "status": "completed",
      "outputs": { "frame": "base64..." }
    }
  ]
}
```

### Report Telemetry

```http
POST /api/v1/agent/devices/{device_id}/telemetry
Authorization: Bearer <device-token>
Content-Type: application/json

{
  "cpu_usage": 25.5,
  "memory_used": 512000000,
  "memory_total": 4096000000,
  "uptime_secs": 86400
}
```

## MQTT Topics

The agent subscribes to and publishes on the following MQTT topics:

### Subscribe (Commands from Backend)

- `ajime/device/{device_id}/command` - Device commands (sync, restart, etc.)
- `ajime/workflow/{workflow_id}/control` - Workflow control (start, stop, pause)

### Publish (Status to Backend)

- `ajime/device/{device_id}/status` - Device status updates
- `ajime/device/{device_id}/telemetry` - Telemetry data
- `ajime/workflow/{workflow_id}/status` - Workflow execution status

## Error Responses

All endpoints return errors in the following format:

```json
{
  "error": "error_code",
  "message": "Human readable error message",
  "details": { ... }
}
```

Common HTTP status codes:
- `400` - Bad Request (invalid input)
- `401` - Unauthorized (invalid or expired token)
- `403` - Forbidden (not authorized for this resource)
- `404` - Not Found
- `500` - Internal Server Error
