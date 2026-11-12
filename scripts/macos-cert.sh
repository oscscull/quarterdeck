#!/usr/bin/env bash
# Creates (or reuses) the self-signed certificate macOS releases are signed with, and sets the two CI secrets
# that carry it. A stable signature lets macOS remember permissions (Documents, Calendars) across updates;
# ad-hoc signed builds look like a new app on every release. Run by the repo owner; safe to rerun.
# The .p12's password is kept only encrypted, in `.env.release`; its key stays in ~/.tauri, beside the .p12.
# Losing the .p12 means one more round of permission prompts after the next release, nothing worse.
set -euo pipefail

REPO=oscscull/quarterdeck
NAME="Quarterdeck Self-Signed"
DIR=$HOME/.tauri
P12=$DIR/quarterdeck-macos.p12
ENV_KEYS=$DIR/quarterdeck-release.env.keys
ROOT=$(cd "$(dirname "$0")/.." && pwd)
ENV_FILE=$ROOT/.env.release

mkdir -p "$DIR" && chmod 700 "$DIR"

if [ -e "$P12" ]; then
  echo "Using the existing certificate at $P12."
  PW=$(dotenvx get MACOS_CERTIFICATE_PASSWORD -f "$ENV_FILE" -fk "$ENV_KEYS")
else
  WORK=$(mktemp -d)
  trap 'rm -rf "$WORK"' EXIT
  PW=$(openssl rand -hex 24)
  openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -subj "/CN=$NAME" \
    -keyout "$WORK/key.pem" -out "$WORK/cert.pem" \
    -addext "basicConstraints=critical,CA:false" \
    -addext "keyUsage=critical,digitalSignature" \
    -addext "extendedKeyUsage=critical,codeSigning" 2>/dev/null
  # macOS `security import` only reads the older p12 encryption.
  openssl pkcs12 -export -legacy -inkey "$WORK/key.pem" -in "$WORK/cert.pem" -name "$NAME" \
    -passout "pass:$PW" -out "$P12"
  dotenvx set MACOS_CERTIFICATE_PASSWORD "$PW" \
    --no-native --no-armor -f "$ENV_FILE" -fk "$ENV_KEYS" >/dev/null
fi
chmod 600 "$P12" "$ENV_KEYS"

base64 -i "$P12" | gh secret set MACOS_CERTIFICATE --repo "$REPO"
printf '%s' "$PW" | gh secret set MACOS_CERTIFICATE_PASSWORD --repo "$REPO"
unset PW
echo "Done: secrets set, password saved encrypted in .env.release."
