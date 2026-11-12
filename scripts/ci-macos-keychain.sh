#!/usr/bin/env bash
# CI only: puts the self-signed release certificate (see macos-cert.sh) in a throwaway keychain codesign can
# use, and tells the Tauri build to sign with it. Tauri's own APPLE_CERTIFICATE import only accepts
# Apple-issued certificates, so this does the import itself.
# Needs MACOS_CERTIFICATE (base64 .p12), MACOS_CERTIFICATE_PASSWORD, RUNNER_TEMP and GITHUB_ENV.
set -euo pipefail

NAME="Quarterdeck Self-Signed"
KEYCHAIN=$RUNNER_TEMP/quarterdeck-signing.keychain-db
KEYCHAIN_PW=$(openssl rand -hex 16)
P12=$RUNNER_TEMP/quarterdeck-signing.p12

printf '%s' "$MACOS_CERTIFICATE" | base64 --decode > "$P12"
security create-keychain -p "$KEYCHAIN_PW" "$KEYCHAIN"
security set-keychain-settings -lut 21600 "$KEYCHAIN"
security unlock-keychain -p "$KEYCHAIN_PW" "$KEYCHAIN"
security import "$P12" -f pkcs12 -k "$KEYCHAIN" -P "$MACOS_CERTIFICATE_PASSWORD" -T /usr/bin/codesign >/dev/null
rm -f "$P12"
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$KEYCHAIN_PW" "$KEYCHAIN" >/dev/null
# shellcheck disable=SC2046 # the existing keychains are one path per word
security list-keychains -d user -s "$KEYCHAIN" $(security list-keychains -d user | tr -d '"')

echo "APPLE_SIGNING_IDENTITY=$NAME" >> "$GITHUB_ENV"
