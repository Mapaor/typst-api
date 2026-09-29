#!/usr/bin/env bash
set -e

REPO="Mapaor/typst-api"
INSTALL_DIR=""
VERSION=""
WITH_SERVICE=false
REQUESTED_PORT=""

# --- Parse Arguments ---
while [[ $# -gt 0 ]]; do
  case $1 in
    --version)
      VERSION="$2"
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="$2"
      shift 2
      ;;
    --port)
      REQUESTED_PORT="$2"
      shift 2
      ;;
    --with-service)
      WITH_SERVICE=true
      shift
      ;;
    -h|--help)
      echo "Usage: $0 [options]"
      echo "Options:"
      echo "  --version <ver>      Install a specific version (e.g., v0.2.4)"
      echo "  --install-dir <dir>  Custom installation directory"
      echo "  --port <port>        Server port (otherwise prompt; default: 8080)"
      echo "  --with-service       Set up systemd service (Linux only)"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

# --- Environment Detection ---
OS=$(uname -s)
ARCH=$(uname -m)

if [[ "$OS" == "Linux" ]]; then
  OS_TARGET="unknown-linux-gnu"
elif [[ "$OS" == "Darwin" ]]; then
  OS_TARGET="apple-darwin"
else
  echo "Unsupported OS: $OS"
  exit 1
fi

if [[ "$ARCH" == "x86_64" || "$ARCH" == "amd64" ]]; then
  ARCH_TARGET="x86_64"
elif [[ "$ARCH" == "aarch64" || "$ARCH" == "arm64" ]]; then
  ARCH_TARGET="aarch64"
else
  echo "Unsupported Architecture: $ARCH"
  exit 1
fi

TARGET="${ARCH_TARGET}-${OS_TARGET}"

# --- Service Verification ---
if [[ "$WITH_SERVICE" == true ]]; then
  if [[ "$OS" != "Linux" ]]; then
    echo "Error: --with-service is only supported on Linux."
    exit 1
  fi
  if ! command -v systemctl >/dev/null 2>&1 && [ ! -d "/run/systemd/system" ]; then
    echo "Error: systemd is not detected on this system. Cannot install service."
    exit 1
  fi
fi

# --- Determine Version and Asset URL ---
if [[ -z "$VERSION" ]]; then
  echo "Fetching latest version information..."
  LATEST_RELEASE=$(curl -s "https://api.github.com/repos/$REPO/releases/latest")
  VERSION=$(echo "$LATEST_RELEASE" | grep -m1 '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
  if [[ -z "$VERSION" ]]; then
    echo "Failed to fetch latest version."
    exit 1
  fi
fi

# Determine the prefix which is sometimes used by cargo-dist older versions vs newer
# Try to fetch release metadata for the specific version to get the exact asset name
RELEASE_DATA=$(curl -s "https://api.github.com/repos/$REPO/releases/tags/$VERSION")
ASSET_NAME=$(echo "$RELEASE_DATA" \
  | grep '"name":' \
  | grep "${TARGET}" \
  | grep -E '\.(tar\.xz|tar\.gz)"' \
  | grep -v '\.sha256"' \
  | head -n 1 \
  | sed -E 's/.*"name": "([^"]+)".*/\1/')

if [[ -z "$ASSET_NAME" ]]; then
    echo "Error: Could not find an asset for target $TARGET in release $VERSION."
    exit 1
fi

echo "Installing typst-api version: $VERSION"
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/$ASSET_NAME"
CHECKSUM_URL="https://github.com/$REPO/releases/download/$VERSION/${ASSET_NAME}.sha256"

# --- Directory Setup ---
if [[ -z "$INSTALL_DIR" ]]; then
  if [[ "$WITH_SERVICE" == true || "$EUID" -eq 0 ]]; then
    INSTALL_DIR="/opt/typst-api"
  else
    INSTALL_DIR="$HOME/.local/typst-api"
  fi
fi

echo "Installation directory: $INSTALL_DIR"
if [[ ! -w "$(dirname "$INSTALL_DIR")" && ! -w "$INSTALL_DIR" ]]; then
  echo "Error: You do not have write permission for $INSTALL_DIR"
  echo "Run with sudo or choose a different directory with --install-dir"
  exit 1
fi

mkdir -p "$INSTALL_DIR"
mkdir -p "$INSTALL_DIR/fonts"

port_in_use() {
  if command -v ss >/dev/null 2>&1; then
    ss -ltnH "sport = :$1" | grep -q .
  elif command -v lsof >/dev/null 2>&1; then
    lsof -nP -iTCP:"$1" -sTCP:LISTEN -t 2>/dev/null | grep -q .
  else
    return 1
  fi
}

PORT=8080
if [[ -f "$INSTALL_DIR/.env" ]]; then
  EXISTING_PORT=$(sed -nE 's/^PORT=([0-9]+).*/\1/p' "$INSTALL_DIR/.env" | head -n 1)
  if [[ -n "$EXISTING_PORT" ]]; then
    PORT="$EXISTING_PORT"
  fi
fi
while true; do
  if [[ -n "$REQUESTED_PORT" ]]; then
    PORT_INPUT="$REQUESTED_PORT"
  else
    if [[ ! -r /dev/tty || ! -w /dev/tty ]]; then
      echo "Error: An interactive terminal is required to select the server port." >&2
      echo "Run the installer from a terminal, or provide --port in a non-interactive environment." >&2
      exit 1
    fi
    printf "Enter the server port [%s]: " "$PORT" > /dev/tty
    IFS= read -r PORT_INPUT < /dev/tty || {
      echo >&2
      echo "Error: Could not read the server port from the terminal." >&2
      exit 1
    }
  fi
  PORT_INPUT=${PORT_INPUT:-$PORT}
  if [[ ! "$PORT_INPUT" =~ ^[0-9]+$ ]] || (( PORT_INPUT < 1 || PORT_INPUT > 65535 )); then
    if [[ -n "$REQUESTED_PORT" ]]; then
      echo "Error: Port must be a number between 1 and 65535." >&2
      exit 1
    fi
    echo "Please enter a valid port between 1 and 65535."
  elif port_in_use "$PORT_INPUT" && { [[ "$PORT_INPUT" != "$EXISTING_PORT" ]] || ! systemctl is-active --quiet typst-api 2>/dev/null; }; then
    if [[ -n "$REQUESTED_PORT" ]]; then
      echo "Error: Port $PORT_INPUT is already in use. Please choose another port." >&2
      exit 1
    fi
    echo "Port $PORT_INPUT is already in use. Please choose another port."
  else
    PORT="$PORT_INPUT"
    break
  fi
done

# --- Download & Verify ---
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading $ASSET_NAME..."
curl -L -f -o "$TMP_DIR/$ASSET_NAME" "$DOWNLOAD_URL"
echo "Downloading checksum..."
curl -L -s -f -o "$TMP_DIR/checksum.sha256" "$CHECKSUM_URL"
echo "Verifying checksum..."
cd "$TMP_DIR"
EXPECTED_HASH=$(awk '{print $1}' checksum.sha256)

if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL_HASH=$(sha256sum "$ASSET_NAME" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
  ACTUAL_HASH=$(shasum -a 256 "$ASSET_NAME" | awk '{print $1}')
else
  echo "Error: Neither sha256sum nor shasum is available for checksum verification."
  exit 1
fi

if [[ -z "$EXPECTED_HASH" || "$EXPECTED_HASH" != "$ACTUAL_HASH" ]]; then
  echo "Error: Checksum verification failed!"
  echo "Expected: $EXPECTED_HASH"
  echo "Actual:   $ACTUAL_HASH"
  exit 1
fi
echo "Checksum verified."
cd - > /dev/null

# --- Extraction ---
echo "Extracting archive..."
tar -xf "$TMP_DIR/$ASSET_NAME" -C "$TMP_DIR"

# cargo-dist extracts into a directory named after the asset (without its archive extension)
EXTRACT_DIR="$TMP_DIR/${ASSET_NAME%.tar.xz}"
EXTRACT_DIR="${EXTRACT_DIR%.tar.gz}"
if [[ ! -d "$EXTRACT_DIR" ]]; then
  # Fallback if the extracted dir structure is different
  EXTRACT_DIR="$TMP_DIR"
fi

cp -f "$EXTRACT_DIR/typst-api" "$INSTALL_DIR/"
if [[ -d "$EXTRACT_DIR/assets/fonts" ]]; then
  cp -rf "$EXTRACT_DIR/assets/fonts/"* "$INSTALL_DIR/fonts/" 2>/dev/null || true
fi

# --- Configuration Setup ---
if [[ ! -f "$INSTALL_DIR/.env" ]]; then
  if [[ -f "$EXTRACT_DIR/.env.example" ]]; then
    echo "Generating .env file from .env.example..."
    sed -E "s/^PORT=.*/PORT=$PORT/" "$EXTRACT_DIR/.env.example" > "$INSTALL_DIR/.env"
  else
    echo "Creating basic .env file..."
    cat > "$INSTALL_DIR/.env" <<EOF
PORT=$PORT
MAX_CONCURRENT_COMPILATIONS=10
# AUTH_TOKEN=my-secret-token
# ADMIN_TOKEN=my-admin-secret-token
EOF
  fi
else
  echo ".env file already exists. Skipping generation."
  if grep -q '^PORT=' "$INSTALL_DIR/.env"; then
    sed -i -E "s/^PORT=.*/PORT=$PORT/" "$INSTALL_DIR/.env"
  else
    printf '\nPORT=%s\n' "$PORT" >> "$INSTALL_DIR/.env"
  fi
fi

# --- Service Installation ---
if [[ "$WITH_SERVICE" == true ]]; then
  echo "Setting up systemd service..."
  
  if ! id -u typst-api >/dev/null 2>&1; then
    echo "Creating typst-api user..."
    useradd -r -s /usr/sbin/nologin typst-api
  fi
  
  # Ensure ownership
  chown -R typst-api:typst-api "$INSTALL_DIR"
  
  # Create service file configuration paths
  mkdir -p /etc/typst-api
  mkdir -p /var/lib/typst-api
  chown typst-api:typst-api /var/lib/typst-api
  
  # Move or symlink .env to /etc/typst-api/typst-api.env for systemd
  if [[ -f "$INSTALL_DIR/.env" && ! -f "/etc/typst-api/typst-api.env" ]]; then
    cp "$INSTALL_DIR/.env" "/etc/typst-api/typst-api.env"
  fi
  if [[ -f "/etc/typst-api/typst-api.env" ]]; then
    if grep -q '^PORT=' /etc/typst-api/typst-api.env; then
      sed -i -E "s/^PORT=.*/PORT=$PORT/" /etc/typst-api/typst-api.env
    else
      printf '\nPORT=%s\n' "$PORT" >> /etc/typst-api/typst-api.env
    fi
  fi
  
  SERVICE_FILE="/etc/systemd/system/typst-api.service"
  # Re-write service with custom installation directory (if set)
  cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=Typst API
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=typst-api
Group=typst-api
WorkingDirectory=$INSTALL_DIR
EnvironmentFile=-/etc/typst-api/typst-api.env
ExecStart=$INSTALL_DIR/typst-api
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

  systemctl daemon-reload
  echo "Service installed at $SERVICE_FILE"
fi

# --- Installation Check ---
echo "Checking installed executable..."
if [ -x "$INSTALL_DIR/typst-api" ]; then
  echo "Installation completed successfully! 🎉"
  CONFIGURED_PORT=$(sed -nE 's/^PORT=([0-9]+).*/\1/p' "$INSTALL_DIR/.env" | head -n 1)
  HEALTH_PORT=${CONFIGURED_PORT:-8080}
  if [[ "$WITH_SERVICE" == true ]]; then
    SERVICE_ENV_FILE="/etc/typst-api/typst-api.env"
    if [[ -f "$SERVICE_ENV_FILE" ]]; then
      CONFIGURED_PORT=$(sed -nE 's/^PORT=([0-9]+).*/\1/p' "$SERVICE_ENV_FILE" | head -n 1)
      if [[ -n "$CONFIGURED_PORT" ]]; then
        HEALTH_PORT="$CONFIGURED_PORT"
      fi
    fi

    echo "Starting and enabling typst-api service..."
    systemctl reset-failed typst-api 2>/dev/null || true
    systemctl enable typst-api
    systemctl restart typst-api
    HEALTH_URL="http://127.0.0.1:${HEALTH_PORT}/health"
    echo "Waiting for $HEALTH_URL..."
    SERVICE_HEALTHY=false
    for _ in {1..15}; do
      if curl -fsS --max-time 2 "$HEALTH_URL" >/dev/null 2>&1; then
        SERVICE_HEALTHY=true
        break
      fi
      sleep 1
    done

    if [[ "$SERVICE_HEALTHY" == true ]]; then
      echo "Health check passed: $HEALTH_URL"
      systemctl --no-pager --full status typst-api
    else
      echo "Error: typst-api did not become healthy at $HEALTH_URL"
      systemctl --no-pager --full status typst-api || true
      exit 1
    fi
  else
    echo "You can run the server using:"
    echo "  cd \"$INSTALL_DIR\" && ./typst-api"
    echo "Then check its health with:"
    echo "  curl http://127.0.0.1:$HEALTH_PORT/health"
  fi
else
  echo "Warning: Executable check failed. It might require additional libraries or a different architecture."
fi
