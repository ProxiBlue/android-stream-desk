# Linux: USB mode (no Wi-Fi needed)

This project is written LAN-first — the phone finds the Companion over
your local Wi-Fi network by IP address. That's fine at home, but it means
the phone and PC both need a network, and adds a small amount of latency
and one more thing that can go wrong (wrong Wi-Fi, firewall, router
isolation between devices, etc).

`adb reverse` gives you an alternative that needs no Wi-Fi at all: the
phone is plugged in by USB cable, and `adb reverse` forwards a TCP port
from the phone's own `localhost` straight to the PC's `localhost` over
that cable. The Companion and the Android app don't need to change at
all — a WebSocket connection to `127.0.0.1:8089` behaves identically
whether that's a real loopback interface or one bridged over USB by adb.
This directory automates the one manual step that approach needs: telling
`adb` to (re-)establish that forward every time the phone is plugged in.

## What's here

```
linux/
├── udev/99-android-stream-desk.rules          # triggers on USB plug-in
├── systemd/android-stream-desk-usb-connect.service   # runs `adb reverse`
└── scripts/usb-connect.sh                      # the actual adb command
```

## One-time setup

**0. Lock the Companion to loopback (recommended).** By default the Companion listens on `0.0.0.0`, so it stays reachable from the LAN even while you use USB. `adb reverse` only ever talks to the PC's `127.0.0.1`, so in USB mode you can close the network side entirely: in the Companion open **Settings → Network → Listen Scope** and switch it to **USB Only**, then let it relaunch. Or edit `server.json` directly and set `"loopbackOnly": true` — see [CONFIGURATION.md](CONFIGURATION.md) for the file's location. Default is `false` (LAN mode), so nothing changes unless you opt in.

**1. Install the Companion** as a normal desktop app that starts with your
session (a systemd `--user` unit, since it's a GUI app with a tray icon —
same idea as any other app you want running when you log in):

```bash
mkdir -p ~/Applications
mv ~/Downloads/Android.Stream.Desk_*.AppImage ~/Applications/android-stream-desk.AppImage
chmod +x ~/Applications/android-stream-desk.AppImage

mkdir -p ~/.config/systemd/user
cp linux/systemd/android-stream-desk-companion.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now android-stream-desk-companion.service
```

**2. Install the USB automation** (needs root, one-time):

```bash
sudo cp linux/udev/99-android-stream-desk.rules /etc/udev/rules.d/
sudo cp linux/systemd/android-stream-desk-usb-connect.service /etc/systemd/system/
sudo cp linux/scripts/usb-connect.sh /usr/local/bin/
sudo chmod +x /usr/local/bin/usb-connect.sh

sudo systemctl daemon-reload
sudo udevadm control --reload-rules
```

That's it — nothing needs to be *enabled*; the udev rule starts the
`usb-connect` service on demand each time the phone is plugged in
(`systemctl status android-stream-desk-usb-connect.service` afterward to
confirm it ran, or `journalctl -t android-stream-desk-usb-connect` for its
one log line).

## Phone-side, once per phone

1. Settings → About phone → tap "Build number" 7× → Developer options unlocked.
2. Developer options → enable **USB debugging**.
3. Plug the phone in via USB.
4. A prompt appears on the phone: **"Allow USB debugging?"** — tick
   **"Always allow from this computer"** and tap **Allow**. This only
   needs doing once per PC; the authorization is tied to this machine's
   adb key, not the USB port, and survives reboots/replugs.
5. Open the Android Stream Desk app, and set its connection address to
   `127.0.0.1` (not a LAN IP) and port `8089`. With `loopbackOnly` on, the
   Companion's connect QR code already encodes `127.0.0.1`, so scanning it
   works too.

## Every time after that

Just plug the phone in. The udev rule + `usb-connect.sh` re-run
`adb reverse` automatically; the app's own auto-reconnect (already built
into it, retries every ~3s) picks the connection back up within a couple
of seconds.

If you move the phone to a **different USB port**, or unplug/replug for
any reason: the same thing happens automatically — the udev rule matches
on the phone's USB vendor ID, not which port it's in.

## Troubleshooting

**Phone shows "unauthorized" in `adb devices`.** The RSA-fingerprint
prompt is waiting on the phone screen and hasn't been tapped yet, or the
adb server was restarted after the phone was already connected (kill and
restart it to force a fresh handshake: `adb kill-server && adb start-server`,
then check the phone screen again for the prompt).

**Nothing happens on plug-in.** Confirm the udev rule actually matches
your phone's vendor ID:
```bash
lsusb | grep -i <your phone brand>
```
`99-android-stream-desk.rules` ships with Samsung's vendor ID (`04e8`)
only. Add a line for your phone's vendor ID if it's a different brand —
see [android-udev-rules](https://github.com/M0Rf30/android-udev-rules) for
a maintained list across manufacturers.

**Port 8089 already in use.** Some other service on your machine is
already bound to it. Change the Companion's WebSocket port in its
Settings → Network, and update `ANDROID_STREAM_DESK_PORT` in
`/etc/systemd/system/android-stream-desk-usb-connect.service` to match,
then `sudo systemctl daemon-reload`.

**Wayland: shortcuts/media keys not simulating correctly.** This is an
`enigo` (the Companion's key-simulation library) limitation, not related
to USB mode — it works best under X11. If you're on Wayland and hit this,
switching your login session to X11 is the current workaround; see the
main README's Linux install notes.

## Uninstall

```bash
systemctl --user disable --now android-stream-desk-companion.service
rm ~/.config/systemd/user/android-stream-desk-companion.service

sudo rm /etc/udev/rules.d/99-android-stream-desk.rules
sudo rm /etc/systemd/system/android-stream-desk-usb-connect.service
sudo rm /usr/local/bin/usb-connect.sh
sudo systemctl daemon-reload
sudo udevadm control --reload-rules
```
