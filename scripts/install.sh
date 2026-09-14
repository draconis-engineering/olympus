#!/usr/bin/env bash
#
# macOS / Linux setup wizard (installer + updater) for the Olympus cycling app.
#
# What it does:
#   1. Detects the OS + CPU architecture and maps them to a release artifact.
#   2. Resolves the release to install (latest from GitHub by default,
#      override with OLYMPUS_VERSION or $0's OLYMPUS_VERSION).
#   3. Downloads the artifact, verifies its SHA-256 when one is published.
#   4. Installs the binary and an `olympus` launcher onto your PATH.
#   5. Seeds the data home (data/workouts + a first-run profile.json).
#   6. Prints the right Bluetooth setup note for the detected OS.
#
# Re-run this script any time to update to the latest release.
#
# Every setting below can be overridden via the environment.
#
# Artifact convention (produced by scripts/release.* on the release host):
#   <release>/download/<tag>/olympus-<tag>-<os>-<arch>.tar.gz
#   <release>/download/<tag>/olympus-<tag>-<os>-<arch>.tar.gz.sha256
#   os:  linux | macos | windows
#   arch: x86_64 | arm64
#   Archive layout: olympus/ (the binary) + olympus/data/workouts/*.zwo

set -euo pipefail

# --- Configuration (all overridable via the environment) -------------------
BASE_URL="${OLYMPUS_BASE_URL:-https://github.com/draconis-engineering/olympus/releases}"
VERSION="${OLYMPUS_VERSION:-}"            # e.g. v1.0.0; empty -> latest via GitHub API
BIN_DIR="${OLYMPUS_BIN_DIR:-$HOME/.local/bin}"
DATA_HOME="${OLYMPUS_DATA_HOME:-$HOME/.local/share/olympus}"   # holds the data/ folder
REQUIRE_CHECKSUM="${OLYMPUS_REQUIRE_CHECKSUM:-0}"              # 1 = fail when no checksum

# --- Helpers ----------------------------------------------------------------
say() { printf '%b\n' "$*"; }
die() { say "error: $*" >&2; exit 1; }

# --- Detect OS + architecture ----------------------------------------------
os="$(uname -s)"
case "$os" in
  Linux)  os=linux ;;
  Darwin) os=macos ;;
  *) die "unsupported operating system: $os" ;;
esac

arch="$(uname -m)"
case "$arch" in
  x86_64|amd64)  arch=x86_64 ;;
  aarch64|arm64) arch=arm64 ;;
  *) die "unsupported CPU architecture: $arch" ;;
esac

# --- Resolve the release to install ----------------------------------------
if [ -z "$VERSION" ]; then
  api="https://api.github.com/repos/draconis-engineering/olympus/releases/latest"
  command -v curl >/dev/null 2>&1 || die "curl is required (or set OLYMPUS_VERSION)"
  VERSION="$(curl -fsSL "$api" | tr ',' '\n' | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)" || true
  [ -n "$VERSION" ] || die "could not resolve the latest release from $api (set OLYMPUS_VERSION to install a specific tag)"
  say "Resolved latest release: $VERSION"
else
  say "Installing release: $VERSION"
fi

artifact="olympus-${VERSION}-${os}-${arch}.tar.gz"
url="${BASE_URL}/download/${VERSION}/${artifact}"
sum_url="${url}.sha256"

# --- Download + verify ------------------------------------------------------
say "Fetching ${url}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -fSL -o "$tmp/$artifact" "$url" || die "download failed; check the release exists (got $url)"

hash_cmd="sha256sum"
command -v "$hash_cmd" >/dev/null 2>&1 || hash_cmd="shasum -a 256"

verify_checksum() {
  local expected actual
  if expected="$(curl -fsSL "$sum_url" 2>/dev/null | awk '{print $1; exit}')"; then
    actual="$($hash_cmd "$tmp/$artifact" | awk '{print $1}')"
    [ "$expected" = "$actual" ] || die "checksum mismatch (expected $expected, got $actual)"
    say "Checksum OK ($expected)"
  elif [ "$REQUIRE_CHECKSUM" = "1" ]; then
    die "no checksum published for ${artifact}; refusing (set OLYMPUS_REQUIRE_CHECKSUM=0 to allow)"
  else
    say "Warning: no checksum published for this artifact; skipping verification"
  fi
}
verify_checksum

# --- Unpack -----------------------------------------------------------------
say "Unpacking $artifact"
tar -xzf "$tmp/$artifact" -C "$tmp"

BIN_SRC="$(find "$tmp" \( -name 'olympus' -o -name 'olympus.exe' \) -type f -print | head -n1)"
[ -n "$BIN_SRC" ] || die "release artifact contains no olympus binary"

# --- Install binary + launcher ----------------------------------------------
mkdir -p "$BIN_DIR"
install -m 0755 "$BIN_SRC" "$BIN_DIR/olympus-core"

launcher="$BIN_DIR/olympus"
cat > "$launcher" <<EOF
#!/bin/sh
# Olympus launcher: runs the binary from the install-time data home so that
# the app's CWD-relative ./data resolves to \$DATA_HOME/data.
if [ -n "\${OLYMPUS_DATA_HOME:-}" ] && [ -d "\$OLYMPUS_DATA_HOME" ]; then
  cd "\$OLYMPUS_DATA_HOME" || exit 1
elif [ -d "\$HOME/.local/share/olympus" ]; then
  cd "\$HOME/.local/share/olympus" || exit 1
fi
exec "$BIN_DIR/olympus-core" "\$@"
EOF
chmod 0755 "$launcher"
say "Installed olympus (core) and olympus (launcher) -> $BIN_DIR"

# --- Seed the data home ------------------------------------------------------
WORKOUTS_DIR="$DATA_HOME/data/workouts"
mkdir -p "$WORKOUTS_DIR" "$DATA_HOME/data/.fit" "$DATA_HOME/data/user"
if [ -d "$tmp/olympus/data/workouts" ]; then
  cp -u "$tmp"/olympus/data/workouts/*.zwo "$WORKOUTS_DIR/" 2>/dev/null || true
  say "Seeded $WORKOUTS_DIR with bundled workouts"
fi

profile="$DATA_HOME/data/user/profile.json"
if [ ! -f "$profile" ]; then
  cat > "$profile" <<'EOF'
{
  "username": "Rider",
  "weight": 75.0,
  "height": 180.0,
  "ftp": 200,
  "max_hr": 180
}
EOF
  say "Wrote first-run profile: $profile"
fi

# --- Wrap-up -----------------------------------------------------------------
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) say "Note: $BIN_DIR is not on your PATH; add it (e.g. export PATH=\"$BIN_DIR:\$PATH\") or start a new shell" ;;
esac
say ""
say "Done. Run \`olympus\` from any terminal — ride data lives in $DATA_HOME/data."
say "To update later, just re-run this script (it always installs the latest release)."

case "$os" in
  linux)
    say ""
    say "Linux Bluetooth: start bluetoothd with the experimental flag it needs:"
    say "  sudo systemctl edit bluetooth   # add: [Service] ExecStart= and ExecStart=/usr/lib/bluetooth/bluetoothd -E"
    say "  sudo systemctl restart bluetooth"
    ;;
  macos)
    say ""
    say "macOS Bluetooth: grant Olympus Bluetooth access when the app first asks"
    say "  (System Settings  Privacy & Security  Bluetooth)."
    ;;
esac