#!/usr/bin/env bash
# Signs an Android APK with Ingot's release key, taken from CI secrets:
#   ANDROID_KEYSTORE_BASE64    the keystore file, base64-encoded
#   ANDROID_KEYSTORE_PASSWORD  its password
#   ANDROID_KEY_ALIAS          the key's alias (default: ingot)
#   ANDROID_KEY_PASSWORD       the key's password (default: same as the keystore's)
#
# Every release must be signed with the same key: Android only installs an update over
# the installed app when the signatures match. A different key means uninstalling
# first, which deletes the user's servers and worlds.
#
# Usage: scripts/sign-android-apk.sh <unsigned.apk> <signed.apk> [--allow-throwaway-key]
#   --allow-throwaway-key  without the secrets, sign with a one-off key instead of
#                          failing (test builds only; they can't update a release)
set -euo pipefail

raw="$1"
out="$2"
mode="${3:-}"

sdk="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-/usr/local/lib/android/sdk}}"
# apksigner.bat on Windows, so the script also works for a local signing run
apksigner=$(find "$sdk/build-tools" -type f \( -name apksigner -o -name apksigner.bat \) | sort -V | tail -n 1)
if [ -z "$apksigner" ]; then
  echo "::error::apksigner not found under $sdk/build-tools"
  exit 1
fi

keystore="$(mktemp -d)/ingot.keystore"
trap 'rm -f "$keystore"' EXIT

if [ -n "${ANDROID_KEYSTORE_BASE64:-}" ]; then
  if [ -z "${ANDROID_KEYSTORE_PASSWORD:-}" ]; then
    echo "::error::ANDROID_KEYSTORE_PASSWORD is not set"
    exit 1
  fi
  printf '%s' "$ANDROID_KEYSTORE_BASE64" | base64 --decode > "$keystore"
  export ANDROID_KEY_PASSWORD="${ANDROID_KEY_PASSWORD:-$ANDROID_KEYSTORE_PASSWORD}"
  # Passwords go through env: so they never appear in the process list or logs
  "$apksigner" sign \
    --ks "$keystore" \
    --ks-key-alias "${ANDROID_KEY_ALIAS:-ingot}" \
    --ks-pass env:ANDROID_KEYSTORE_PASSWORD \
    --key-pass env:ANDROID_KEY_PASSWORD \
    --out "$out" "$raw"
elif [ "$mode" = "--allow-throwaway-key" ]; then
  echo "::warning::Signing secrets aren't set: this test APK uses a one-off key and can't be installed over a release build."
  THROWAWAY_PASSWORD="$(openssl rand -hex 24)"
  export THROWAWAY_PASSWORD
  keytool -genkeypair -keystore "$keystore" -alias throwaway -keyalg RSA -keysize 2048 -validity 30 \
    -storepass:env THROWAWAY_PASSWORD -keypass:env THROWAWAY_PASSWORD -dname "CN=Ingot test build" > /dev/null
  "$apksigner" sign --ks "$keystore" --ks-pass env:THROWAWAY_PASSWORD --out "$out" "$raw"
else
  echo "::error::Release signing secrets aren't set (ANDROID_KEYSTORE_BASE64, ANDROID_KEYSTORE_PASSWORD). See scripts/sign-android-apk.sh."
  exit 1
fi

"$apksigner" verify "$out"
# The certificate fingerprint is public; it shows every release uses the same key
"$apksigner" verify --print-certs "$out" | grep -i "SHA-256" || true
