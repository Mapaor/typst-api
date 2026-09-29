#!/usr/bin/env bash
set -e

REPO="Mapaor/typst-api"
INSTALL_DIR=""
VERSION=""
WITH_SERVICE=false

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
    --with-service)
      WITH_SERVICE=true
      shift
      ;;
    -h|--help)
      echo "Usage: $0 [options]"
      echo "Options:"
      echo "  --version <ver>      Install a specific version (e.g., v0.2.4)"
      echo "  --install-dir <dir>  Custom installation directory"
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
ASSET_NAME=$(echo "$RELEASE_DATA" | grep -Eo "\"name\": \"[^\"]*${TARGET}\.(tar\.xz|tar\.gz)\"" | head -n 1 | cut -d'"' -f4)

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

# --- Download & Verify ---
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

echo "Downloading $ASSET_NAME..."
curl -L -f -o "$TMP_DIR/$ASSET_NAME" "$DOWNLOAD_URL"
echo "Downloading checksum..."
if curl -L -s -f -o "$TMP_DIR/checksum.sha256" "$CHECKSUM_URL"; then
  echo "Verifying checksum..."
  cd "$TMP_DIR"
  EXPECTED_HASH=$(cat checksum.sha256 | awk '{print $1}')
  
  if command -v sha256sum >/dev/null 2>&1; then
    ACTUAL_HASH=$(sha256sum "$ASSET_NAME" | awk '{print $1}')
  elif command -v shasum >/dev/null 2>&1; then
    ACTUAL_HASH=$(shasum -a 256 "$ASSET_NAME" | awk '{print $1}')
  else
    echo "Warning: Neither sha256sum nor shasum found. Skipping verification."
  fi
  
  if [[ -n "$ACTUAL_HASH" && "$EXPECTED_HASH" != "$ACTUAL_HASH" && -n "$EXPECTED_HASH" ]]; then
    echo "Error: Checksum verification failed!"
    echo "Expected: $EXPECTED_HASH"
    echo "Actual:   $ACTUAL_HASH"
    exit 1
  elif [[ -n "$ACTUAL_HASH" ]]; then
    echo "Checksum verified."
  fi
  cd - > /dev/null
else
  echo "Warning: Checksum file not found. Skipping verification."
fi

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
    cp "$EXTRACT_DIR/.env.example" "$INSTALL_DIR/.env"
  else
    echo "Creating basic .env file..."
    cat > "$INSTALL_DIR/.env" <<EOF
MAX_CONCURRENT_COMPILATIONS=10
# AUTH_TOKEN=my-secret-token
# ADMIN_TOKEN=my-admin-secret-token
EOF
  fi
else
  echo ".env file already exists. Skipping generation."
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
WorkingDirectory=/var/lib/typst-api
EnvironmentFile=-/etc/typst-api/typst-api.env
ExecStart=$INSTALL_DIR/typst-api
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

  systemctl daemon-reload
  echo "Service installed at $SERVICE_FILE"
  echo "IMPORTANT (MANUAL STEP): To start it run: \`sudo systemctl start typst-api\`"
  echo "IMPORTANT (MANUAL STEP): To enable it starting automatically on boot run: \`sudo systemctl enable typst-api\`"
fi

# --- Health Check ---
echo "Running health check..."
if "$INSTALL_DIR/typst-api" --help >/dev/null 2>&1 || "$INSTALL_DIR/typst-api" --version >/dev/null 2>&1 || [ -x "$INSTALL_DIR/typst-api" ]; then
  echo "Installation completed successfully! 🎉"
  if [[ "$WITH_SERVICE" != true ]]; then
    echo "You can run the server using:"
    echo "  cd \"$INSTALL_DIR\" && ./typst-api"
  fi
else
  echo "Warning: Executable check failed. It might require additional libraries or a different architecture."
fi
