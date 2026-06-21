# Deployment Guide — Emberwake

Emberwake is a single-container Rust web app. This guide covers all deployment
modes, configuration, secrets, and verification.

## Quick Start (Docker)

```bash
docker run -p 5005:5005 \
  -v emberwake-data:/var/lib/emberwake \
  -e DATA_DIR=/var/lib/emberwake \
  --user 10001:10001 --read-only --tmpfs /tmp \
  ghcr.io/draco-lunaris/emberwake:latest
```

Open `http://localhost:5005` and complete the first-run admin setup.

## Docker Compose

```yaml
services:
  emberwake:
    image: ghcr.io/draco-lunaris/emberwake:latest
    container_name: emberwake
    user: "10001:10001"
    read_only: true
    tmpfs: [/tmp]
    volumes:
      - emberwake-data:/var/lib/emberwake
      # - /var/run/docker.sock:/var/run/docker.sock:ro  # only if Docker discovery is enabled
    ports: ["5005:5005"]
    environment:
      - DATA_DIR=/var/lib/emberwake
      - RUST_LOG=info
      # - SESSION_SECRET_FILE=/run/secrets/session_secret
      # - WEATHER_API_KEY_FILE=/run/secrets/weather_key
    # secrets: [session_secret, weather_key]
    restart: unless-stopped

volumes:
  emberwake-data: {}
# secrets:
#   session_secret: { file: ./secrets/session_secret }
#   weather_key: { file: ./secrets/weather_key }
```

## Configuration

All configuration is via environment variables. File-based secrets use the
`*_FILE` convention (file value wins; fail-loud if unreadable).

### Core

| Variable | Default | Description |
|----------|---------|-------------|
| `DATA_DIR` | `./data` | SQLite database + backup directory |
| `RUST_LOG` | `info` | Log level (trace, debug, info, warn, error) |
| `SESSION_SECRET` | generated | Session signing key (use `SESSION_SECRET_FILE` in production) |
| `SERVER_KEY` | derived | HMAC key for token hashing (use `SERVER_KEY_FILE` in production) |

### Optional Integrations

| Variable | Description |
|----------|-------------|
| `WEATHER_API_KEY` / `WEATHER_API_KEY_FILE` | WeatherAPI.com key for weather widget |
| `OIDC_ISSUER_URL` | OIDC provider issuer URL |
| `OIDC_CLIENT_ID` | OIDC client ID |
| `OIDC_CLIENT_SECRET` / `OIDC_CLIENT_SECRET_FILE` | OIDC client secret |
| `OIDC_REDIRECT_URL` | OIDC callback URL |

### Optional Built-in HTTPS (ACME)

Set these to enable built-in TLS via rustls + ACME (no reverse proxy needed):

| Variable | Description |
|----------|-------------|
| `ACME_DOMAIN` | Domain to obtain a certificate for |
| `ACME_EMAIL` | Email for Let's Encrypt registration |
| `ACME_CACHE_DIR` | Directory for ACME certificate cache |

Without these, terminate TLS at your reverse proxy.

## Reverse Proxy

Emberwake listens on `0.0.0.0:5005` (HTTP). Put a TLS-terminating proxy in
front:

### Caddy

```caddyfile
emberwake.example.com {
    reverse_proxy localhost:5005
}
```

### Nginx

```nginx
server {
    listen 443 ssl http2;
    server_name emberwake.example.com;

    # TLS config...

    location / {
        proxy_pass http://127.0.0.1:5005;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    # SSE support
    location /events {
        proxy_pass http://127.0.0.1:5005;
        proxy_set_header Connection '';
        proxy_http_version 1.1;
        proxy_buffering off;
        proxy_cache off;
        chunked_transfer_encoding on;
    }
}
```

## Secrets

**Never bake secrets into images or compose files.** Use the `*_FILE` convention:

```bash
# Generate a strong session secret
dd if=/dev/urandom bs=32 count=1 | base64 > secrets/session_secret

# Docker run
docker run -p 5005:5005 \
  -v emberwake-data:/var/lib/emberwake \
  -v ./secrets/session_secret:/run/secrets/session_secret:ro \
  -e DATA_DIR=/var/lib/emberwake \
  -e SESSION_SECRET_FILE=/run/secrets/session_secret \
  ghcr.io/draco-lunaris/emberwake:latest
```

## Image Verification

### Verify the signature

```bash
cosign verify \
  --certificate-identity-regexp "https://github.com/Draco-Lunaris/.*" \
  --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
  ghcr.io/draco-lunaris/emberwake:latest
```

### Review the SBOM

Download the CycloneDX SBOM from the GitHub Release page and review the
dependency list for known vulnerabilities.

```bash
# If cyclonedx-cli is installed
cyclonedx-cli analyze --input-file emberwake.cdx.json
```

## Health Checks

| Endpoint | Purpose |
|----------|---------|
| `/healthz` | Liveness — server is running |
| `/readyz` | Readiness — database connected, ready to serve |

The Dockerfile includes a `HEALTHCHECK` hitting `/readyz` every 30s.

## Resource Limits

Recommended minimums:

- CPU: 0.25 cores
- Memory: 64 MB (idle ~48 MB, budget per SC-003)
- Disk: 1 GB (SQLite DB grows with usage; backups stored in `DATA_DIR`)

## Docker Discovery (Optional)

To enable Docker container auto-discovery:

1. Mount the Docker socket read-only: `-v /var/run/docker.sock:/var/run/docker.sock:ro`
2. Enable in Settings → Integrations → Docker Discovery
3. Label containers with `emberwake.name`, `emberwake.url`, etc.

The integration is strictly read-only (list/inspect/watch only).

## Kubernetes Discovery (Optional)

To enable Kubernetes Ingress auto-discovery:

1. Ensure the container has a service account with Ingress read permissions
2. Enable in Settings → Integrations → Kubernetes Discovery
3. Annotate Ingress resources with `emberwake.name`, `emberwake.url`, etc.

The integration is strictly read-only (list/watch only).

## Backup

The SQLite database lives in `DATA_DIR`. To back up:

```bash
# Using the built-in export (admin → Settings → Export)
# Or directly copy the database file
cp /var/lib/emberwake/app.db /backup/emberwake-$(date +%Y%m%d).db
```

## Security Notes

- See [SECURITY.md](./SECURITY.md) for vulnerability reporting
- See [specs/001-greenfield/security.md](./specs/001-greenfield/security.md) for the full threat model
- See [SECURITY_VERIFICATION.md](./SECURITY_VERIFICATION.md) for the Phase 12 security verification
- Always use signed images; verify the cosign signature before deploying
- Run as non-root (UID 10001); use `--read-only` and `--tmpfs /tmp`
- Terminate TLS at a proxy or enable built-in ACME
