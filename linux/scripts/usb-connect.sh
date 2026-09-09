#!/bin/bash
# Run by android-stream-desk-usb-connect.service (triggered by the udev rule
# in ../udev/99-android-stream-desk.rules) whenever a paired phone is plugged
# in over USB. Re-establishes the adb port-forward the Companion's WebSocket
# server is reached through.
#
# Runs as root (systemd system service) but that's fine: `adb` is a plain
# TCP client to the adb server on 127.0.0.1:5037 — it doesn't matter which
# uid started that server or which uid calls `adb` now, same as it doesn't
# matter for any other local `adb` invocation on this machine.
set -euo pipefail

ADB="${ADB_BIN:-/usr/bin/adb}"
WS_PORT="${ANDROID_STREAM_DESK_PORT:-8089}"

# The device was just enumerated but adb may not have finished its own
# handshake yet (especially first plug after boot) — give it a moment
# rather than failing outright.
"$ADB" wait-for-usb-device 2>&1

"$ADB" reverse "tcp:${WS_PORT}" "tcp:${WS_PORT}"
logger -t android-stream-desk-usb-connect "adb reverse tcp:${WS_PORT} established"
