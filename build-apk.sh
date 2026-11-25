#!/usr/bin/env bash
# TubeScript derleme betiği
#
# Kullanım:
#   ./build-apk.sh           # imzalı release APK (arm64-v8a)
#   ./build-apk.sh debug     # debug APK
#
# Ortam değişkenleri:
#   JAVA_HOME, ANDROID_HOME, NDK_HOME  (yoksa aşağıdaki varsayılanlar kullanılır)
#   TUBESCRIPT_KEY_PASS                (imza parolası, varsayılan: tubescript)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"

# Taşınabilir varsayılanlar: yalnızca ortam değişkeni boşsa ve yerel kurulum varsa kullan.
if [ -z "${JAVA_HOME:-}" ] && [ -x "$HOME/.local/opt/jdk17/bin/java" ]; then
  export JAVA_HOME="$HOME/.local/opt/jdk17"
fi
if [ -z "${ANDROID_HOME:-}" ]; then
  for d in "$HOME/Android/Sdk" "$HOME/android-sdk" "/opt/android-sdk"; do
    [ -d "$d" ] && export ANDROID_HOME="$d" && break
  done
fi
export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$ANDROID_HOME}"
if [ -z "${NDK_HOME:-}" ] && [ -n "${ANDROID_HOME:-}" ]; then
  export NDK_HOME="$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort -V | tail -n 1)"
fi
[ -d "$HOME/.local/opt/node/bin" ] && export PATH="$HOME/.local/opt/node/bin:$PATH"

: "${ANDROID_HOME:?ANDROID_HOME bulunamadı (Android SDK yolunu ayarla)}"

KEY_PASS="${TUBESCRIPT_KEY_PASS:-tubescript}"
KEYSTORE="$ROOT/keystore/tubescript.jks"
KEY_ALIAS="tubescript"

cd "$ROOT"

if [ ! -d src-tauri/gen/android ]; then
  npx tauri android init --ci
fi

# Yerel imza anahtarı yoksa üret (repoda tutulmaz).
if [ ! -f "$KEYSTORE" ]; then
  mkdir -p "$(dirname "$KEYSTORE")"
  "$JAVA_HOME/bin/keytool" -genkeypair -v \
    -keystore "$KEYSTORE" -alias "$KEY_ALIAS" \
    -keyalg RSA -keysize 2048 -validity 10000 \
    -storepass "$KEY_PASS" -keypass "$KEY_PASS" \
    -dname "CN=TubeScript, OU=App, O=TubeScript, L=Internet, C=TR"
fi

KSPROP="src-tauri/gen/android/keystore.properties"
cat > "$KSPROP" <<EOF
storeFile=$KEYSTORE
storePassword=$KEY_PASS
keyAlias=$KEY_ALIAS
keyPassword=$KEY_PASS
EOF

MODE="${1:-release}"
if [ "$MODE" = "debug" ]; then
  npx tauri android build --apk --debug --target aarch64
else
  npx tauri android build --apk --target aarch64
fi

find src-tauri/gen/android/app/build/outputs/apk -name '*.apk' -print

# ---- imzalama: Gradle çıktısı imzasızsa apksigner ile imzala ----
RAW="$(find src-tauri/gen/android/app/build/outputs/apk -name '*release-unsigned.apk' | head -n 1)"
if [ -n "$RAW" ]; then
  BT="$(ls -d "$ANDROID_HOME"/build-tools/* | sort -V | tail -n 1)"
  OUT="$ROOT/../TubeScript.apk"
  "$BT/zipalign" -p -f 4 "$RAW" "$ROOT/../TubeScript-aligned.apk"
  "$BT/apksigner" sign \
    --ks "$KEYSTORE" \
    --ks-pass "pass:$KEY_PASS" \
    --key-pass "pass:$KEY_PASS" \
    --ks-key-alias "$KEY_ALIAS" \
    --out "$OUT" "$ROOT/../TubeScript-aligned.apk"
  rm -f "$ROOT/../TubeScript-aligned.apk"
  "$BT/apksigner" verify --print-certs "$OUT" | head -n 3
  ls -lh "$OUT"
fi
