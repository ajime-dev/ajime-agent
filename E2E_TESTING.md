# End-to-End Testing Guide: Cloud Artifact Connectivity and Deployment

This guide documents the complete E2E flow for installing the Ajime agent from cloud artifacts (GitHub releases), establishing connectivity via WebSocket relay, and deploying containers.

## Prerequisites

1. **Backend Server Running**
   - Web server running at `PUBLIC_BACKEND_URL` (e.g., `https://api.ajime.com` or `http://localhost:8000`)
   - Database migrations applied (including `013_agent_deployments.sql`)
   - Environment variables configured:
     - `PUBLIC_BACKEND_URL`: Public URL for agent installation
     - `DEVICE_JWT_SECRET`: Secret for JWT token generation
     - Supabase credentials

2. **GitHub Release Published**
   - Repository: `ajime-dev/ajime-agent` (must be public or have access tokens)
   - Release tag: `v0.1.0` (or latest)
   - Binaries built for:
     - `ajigent-linux-x86_64` (musl, static)
     - `ajigent-linux-arm64` (musl, static)
     - `ajigent-linux-armv7` (musl, static)

3. **Target Device**
   - Linux system (Raspberry Pi, Jetson, or x86_64)
   - Internet connectivity
   - `curl`, `sudo` available
   - Docker installed (for container deployments)

## E2E Test Flow

### Phase 1: Agent Installation from Cloud Artifact

#### 1.1 Generate Device Credentials (Web UI)

1. Navigate to Device Management page
2. Click "Install Agent" or "Add Device"
3. Click "Generate Credentials"
4. Backend creates:
   - `device_id`: Unique string (e.g., `agent-abc123`)
   - `device_secret`: Random 32-byte base64 string
   - `secret_hash`: SHA256 hash stored in `devices.metadata.secret_hash`
   - Device row in database with `status="offline"`, `relay_status="disconnected"`

**Expected Result:**
- UI displays installation command:
  ```bash
  curl -sSL https://api.ajime.com/api/v1/install/sh?device_id=agent-abc123&device_secret=<secret> | sudo bash
  ```

#### 1.2 Run Installation Script on Target Device

```bash
# SSH into target device
ssh user@device-ip

# Run installation command (copy from UI)
curl -sSL https://api.ajime.com/api/v1/install/sh?device_id=agent-abc123&device_secret=<secret> | sudo bash
```

**What Happens:**
1. Script detects architecture (`uname -m`)
2. Downloads binary from GitHub releases:
   - URL: `https://github.com/ajime-dev/ajime-agent/releases/download/v0.1.0/ajigent-linux-<arch>`
   - Via backend redirect: `GET /api/v1/install/binaries/<platform>/latest/ajigent`
3. Creates `/etc/ajime/device.json`:
   ```json
   {
     "id": "agent-abc123",
     "name": "Edge Device",
     "owner_id": "unknown",
     "token": "<device_secret>",
     "device_type": "Generic",
     "capabilities": [],
     "metadata": {},
     "activated_at": <timestamp>,
     "last_sync_at": null
   }
   ```
4. Creates `/etc/ajime/settings.json`:
   ```json
   {
     "backend": {
       "base_url": "https://api.ajime.com/api/v1"
     },
     "log_level": "info",
     "enable_poller": true,
     "enable_deployer": true,
     "enable_relay_worker": true
   }
   ```
5. Installs binary to `/usr/local/bin/ajigent`
6. Creates systemd service `/etc/systemd/system/ajigent.service`
7. Starts service: `sudo systemctl start ajigent`

**Expected Result:**
- Binary downloaded successfully
- Service running: `sudo systemctl status ajigent`
- Logs show startup: `sudo journalctl -u ajigent -f`

**Verification Commands:**
```bash
# Check binary
/usr/local/bin/ajigent --version

# Check service
sudo systemctl status ajigent

# Check logs
sudo journalctl -u ajigent -n 50 --no-pager
```

### Phase 2: WebSocket Relay Connection

#### 2.1 Agent Connects to Relay

**Agent Side (automatic on startup):**
1. Loads `device.json` and `settings.json`
2. `TokenManager::load_token()` attempts to decode `token` as JWT
3. JWT decode fails → fallback to `DeviceToken::from_secret(device.id, token)`
4. Creates `HttpClient::with_device_id(backend_url, device.id)`
5. Relay worker builds WebSocket URL:
   - Input: `https://api.ajime.com/api/v1`
   - Output: `wss://api.ajime.com/api/v1/agent-relay/ws`
6. Connects with headers:
   - `X-Device-ID: agent-abc123`
   - `X-Device-Secret: <device_secret>`

**Backend Side:**
1. WebSocket endpoint `/api/v1/agent-relay/ws` receives connection
2. Validates headers:
   - Loads device by `X-Device-ID`
   - Computes `SHA256(X-Device-Secret)` and compares to `metadata.secret_hash`
3. On success:
   - Updates device: `relay_status="connected"`, `status="online"`
   - Broadcasts device update via WebSocket to UI
   - Accepts WebSocket connection

**Expected Result:**
- Agent logs: `Connected to relay: wss://api.ajime.com/api/v1/agent-relay/ws`
- Backend logs: `[AGENT_CONNECTED] Agent connected: agent-abc123`
- UI shows device status: `SSH: disconnected`, `Cloud: connected`

**Verification:**
```bash
# Agent logs
sudo journalctl -u ajigent -f | grep -i relay

# Backend logs (if accessible)
# Look for: "Agent connected: agent-abc123"

# Database query
SELECT device_id, status, relay_status, ssh_status, last_seen 
FROM devices 
WHERE device_id = 'agent-abc123';
```

### Phase 3: HTTP API Authentication

#### 3.1 Agent Polls for Deployments

**Agent Side (deployer worker, every 30s):**
1. Gets token from `TokenManager` (device_secret)
2. Calls `GET /api/v1/agent/devices/{device_id}/deployments`
3. Headers:
   - `Authorization: Bearer <device_secret>`
   - `X-Device-ID: agent-abc123`

**Backend Side:**
1. `get_device_from_token(authorization, x_device_id)` called
2. Tries JWT decode → fails
3. Sees `X-Device-ID` present → tries device_secret auth:
   - Loads device by `x_device_id`
   - Computes `SHA256(Bearer token)` and compares to `metadata.secret_hash`
4. On success: returns device info
5. Resolves `device_id` (string) to `devices.id` (UUID)
6. Queries `deployments` table: `.eq("device_id", uuid).eq("status", "pending")`
7. Returns pending deployments

**Expected Result:**
- Agent logs: `Polling for deployments...` (no errors)
- Backend logs: HTTP 200 for deployment poll
- No 401/403 errors

**Verification:**
```bash
# Agent logs
sudo journalctl -u ajigent -f | grep -i deployment

# Manual API test (from device)
curl -H "Authorization: Bearer <device_secret>" \
     -H "X-Device-ID: agent-abc123" \
     https://api.ajime.com/api/v1/agent/devices/agent-abc123/deployments
```

### Phase 4: Container Deployment

#### 4.1 Create Deployment (Web UI)

1. Navigate to device details page
2. Click "Deploy Container"
3. Fill form:
   - Type: `docker`
   - Image: `nginx:latest`
   - Config: `{"ports": ["80:80"]}`
4. Submit

**Backend Side:**
1. `POST /api/v1/deployments` receives request
2. Resolves `device_id` (string) to `devices.id` (UUID)
3. Inserts into `deployments` table:
   ```json
   {
     "device_id": "<uuid>",
     "deployment_type": "docker",
     "config": {"image": "nginx:latest", "ports": ["80:80"]},
     "status": "pending",
     "created_by": "<user_id>"
   }
   ```
4. Triggers real-time notification via relay (if connected)

**Expected Result:**
- Deployment created with status `pending`
- Deployment ID returned to UI

#### 4.2 Agent Receives and Executes Deployment

**Agent Side:**
1. Deployer worker polls or receives relay notification
2. Fetches pending deployments via HTTP API
3. For each deployment:
   - Parses config
   - Executes `docker pull nginx:latest`
   - Executes `docker run -d -p 80:80 nginx:latest`
   - Reports status updates:
     - `PATCH /api/v1/deployments/{id}/status` with `status="in_progress"`
     - `POST /api/v1/deployments/{id}/logs` with log entries
     - `PATCH /api/v1/deployments/{id}/status` with `status="success"` or `"failed"`

**Backend Side:**
1. Status update endpoint authenticates device (same as deployment poll)
2. Verifies deployment belongs to device:
   - Loads deployment
   - Joins with `devices` table
   - Checks `devices.device_id` matches authenticated device
3. Updates deployment status and timestamps
4. Logs inserted into `deployment_logs` table

**Expected Result:**
- Agent logs: `Deploying docker: nginx:latest`, `Deployment <id> completed`
- Container running: `docker ps` shows nginx
- UI shows deployment status: `success`
- Deployment logs visible in UI

**Verification:**
```bash
# Agent logs
sudo journalctl -u ajigent -f | grep -i deploy

# Check container
docker ps
curl http://localhost:80  # Should return nginx welcome page

# Database query
SELECT id, status, started_at, completed_at, error_message 
FROM deployments 
WHERE device_id = (SELECT id FROM devices WHERE device_id = 'agent-abc123')
ORDER BY created_at DESC LIMIT 5;

SELECT * FROM deployment_logs 
WHERE deployment_id = '<deployment_id>' 
ORDER BY timestamp;
```

## Security Validation

### Authentication Flow

1. **Device Secret Storage:**
   - Never stored in plaintext in database
   - Only `SHA256(secret)` stored in `devices.metadata.secret_hash`
   - Secret provided to agent via install script, stored in `/etc/ajime/device.json`

2. **HTTP API Auth:**
   - Agent sends: `Authorization: Bearer <device_secret>` + `X-Device-ID: <device_id>`
   - Backend validates: `SHA256(Bearer token) == metadata.secret_hash`
   - No JWT required for initial connection

3. **WebSocket Relay Auth:**
   - Agent sends: `X-Device-ID` + `X-Device-Secret` headers
   - Backend validates: `SHA256(X-Device-Secret) == metadata.secret_hash`

4. **Deployment Auth:**
   - Status/logs endpoints require device auth
   - Verify deployment belongs to authenticated device
   - Prevent cross-device access

### TLS/Encryption

- Production: Use `https://` and `wss://` (TLS)
- Development: `http://` and `ws://` acceptable for localhost/LAN
- Agent validates certificates (default reqwest behavior)

## Troubleshooting

### Agent Won't Connect to Relay

**Symptoms:** Agent logs show connection errors, device status remains offline

**Debug Steps:**
1. Check relay URL construction:
   ```bash
   # Agent should connect to: wss://api.ajime.com/api/v1/agent-relay/ws
   sudo journalctl -u ajigent | grep "Connecting to relay"
   ```
2. Verify backend URL in settings:
   ```bash
   cat /etc/ajime/settings.json | grep base_url
   # Should be: "base_url": "https://api.ajime.com/api/v1"
   ```
3. Test WebSocket manually:
   ```bash
   # Install wscat: npm install -g wscat
   wscat -c "wss://api.ajime.com/api/v1/agent-relay/ws" \
     -H "X-Device-ID: agent-abc123" \
     -H "X-Device-Secret: <secret>"
   ```
4. Check backend logs for authentication errors

### HTTP API Returns 401

**Symptoms:** Agent logs show 401 Unauthorized for deployment polls

**Debug Steps:**
1. Verify device_secret matches database:
   ```bash
   # On device
   cat /etc/ajime/device.json | grep token
   
   # Compute hash
   echo -n "<device_secret>" | sha256sum
   
   # Compare to database
   SELECT metadata->'secret_hash' FROM devices WHERE device_id = 'agent-abc123';
   ```
2. Check X-Device-ID header is sent:
   ```bash
   # Agent should log device_id on startup
   sudo journalctl -u ajigent | grep "device_id"
   ```
3. Test API manually:
   ```bash
   curl -v \
     -H "Authorization: Bearer <device_secret>" \
     -H "X-Device-ID: agent-abc123" \
     https://api.ajime.com/api/v1/agent/devices/agent-abc123/deployments
   ```

### Deployment Stuck in Pending

**Symptoms:** Deployment created but never executes

**Debug Steps:**
1. Check agent is polling:
   ```bash
   sudo journalctl -u ajigent -f | grep -i "polling\|deployment"
   ```
2. Verify deployer worker is enabled:
   ```bash
   cat /etc/ajime/settings.json | grep enable_deployer
   # Should be: "enable_deployer": true
   ```
3. Check deployment device_id matches:
   ```sql
   -- In database
   SELECT d.id, d.device_id, dev.device_id as device_string
   FROM deployments d
   JOIN devices dev ON d.device_id = dev.id
   WHERE d.status = 'pending';
   ```
4. Restart agent:
   ```bash
   sudo systemctl restart ajigent
   sudo journalctl -u ajigent -f
   ```

### Container Fails to Deploy

**Symptoms:** Deployment status changes to `failed`

**Debug Steps:**
1. Check deployment logs:
   ```sql
   SELECT * FROM deployment_logs 
   WHERE deployment_id = '<id>' 
   ORDER BY timestamp;
   ```
2. Check agent logs:
   ```bash
   sudo journalctl -u ajigent -n 100 | grep -A 10 "deployment"
   ```
3. Verify Docker is installed and running:
   ```bash
   docker --version
   sudo systemctl status docker
   ```
4. Check deployment config is valid:
   ```sql
   SELECT config FROM deployments WHERE id = '<id>';
   ```

## Success Criteria

✅ **Installation:**
- Binary downloaded from GitHub releases
- Service running: `systemctl status ajigent` shows `active (running)`
- No errors in logs: `journalctl -u ajigent -n 50`

✅ **Relay Connection:**
- Agent logs: `Connected to relay: wss://...`
- Backend logs: `Agent connected: <device_id>`
- Database: `relay_status='connected'`, `status='online'`
- UI: Device shows "Cloud: connected"

✅ **HTTP API:**
- Agent polls deployments every 30s
- No 401/403 errors in logs
- Manual curl test returns 200 OK

✅ **Deployment:**
- Deployment created via UI
- Agent receives and executes within 30s
- Container running: `docker ps` shows container
- Deployment status: `success`
- Logs visible in UI

✅ **Security:**
- Device secret never in plaintext in database
- Only SHA256 hash stored
- TLS used in production (wss://, https://)
- Deployment auth prevents cross-device access

## Performance Expectations

- **Installation:** < 2 minutes (depends on network speed)
- **Relay Connection:** < 5 seconds after agent startup
- **Deployment Poll:** Every 30 seconds
- **Deployment Execution:** < 2 minutes for small images (nginx)
- **Status Updates:** Real-time via WebSocket, < 1 second latency

## Next Steps

After successful E2E test:
1. Test with multiple devices simultaneously
2. Test reconnection after network interruption
3. Test deployment of various types (git, docker-compose)
4. Load test: 100+ devices, 1000+ deployments
5. Security audit: penetration testing, secret rotation
6. Monitoring: Prometheus metrics, alerting
