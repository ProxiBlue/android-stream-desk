# Companion configuration (`server.json`)

The Companion keeps its listener settings in one small JSON file, `server.json`, in the per-user Tauri app-config directory. The Dashboard's **Settings → Network** panel reads and writes this same file, so there is no separate "UI config" versus "file config" — editing the file by hand and toggling the UI are two views of the same thing.

There is no config file inside the repository that the app reads. `docs/server.example.json` is a documented copy of the defaults you can paste from; the app never loads it.

## Location

The identifier in `src-tauri/tauri.conf.json` is `com.ania.android.stream.desk`, so Tauri's `app_config_dir()` resolves to:

| OS | Path |
|---|---|
| Linux | `~/.config/com.ania.android.stream.desk/server.json` |
| macOS | `~/Library/Application Support/com.ania.android.stream.desk/server.json` |
| Windows | `%APPDATA%\com.ania.android.stream.desk\server.json` |

The file is created with defaults on first launch if it does not exist.

## Keys

| Key | Type | Default | Meaning |
|---|---|---|---|
| `wsPort` | number | `8089` | TCP port of the WebSocket server the Android app connects to. |
| `webEnabled` | boolean | `false` | Also serve the browser-based client over HTTP. |
| `webPort` | number | `8090` | TCP port of that HTTP server. Ignored when `webEnabled` is `false`. |
| `loopbackOnly` | boolean | `false` | Bind both listeners to `127.0.0.1` instead of `0.0.0.0`. See below. |

Keys are camelCase on disk (serde `rename_all = "camelCase"`). Unknown keys are ignored. A missing `loopbackOnly` is treated as `false`, so files written by older builds keep working unchanged.

### `loopbackOnly`

Default `false`: the Companion listens on all interfaces, which is what Wi-Fi / LAN use needs.

Set to `true` for **USB mode** (`adb reverse`, see `LINUX_USB_MODE.md`). `adb reverse` delivers the phone's connection to the PC's own loopback interface, so the Companion only needs to listen on `127.0.0.1`. With this on, no other machine on the network can open the WebSocket or HTTP port at all. The Dashboard then shows `127.0.0.1` as the server address and the connect QR code encodes that address, which is exactly what the phone must dial in USB mode.

The toggle for this is **Settings → Network → Listen Scope** (`LAN` / `USB Only`).

## Validation

Saving (via UI or on load from disk) enforces:

- `wsPort` and `webPort` must be in `1024..=65535`.
- `wsPort` and `webPort` must differ when `webEnabled` is `true`.

If the file is unreadable, not valid JSON, or fails validation, the Companion starts with all defaults (and does **not** overwrite your file). Fix the file and relaunch.

## Applying changes

Listeners bind once at startup. After editing `server.json` by hand, restart the Companion. Saving from the Dashboard writes the file atomically (`server.json.tmp` then rename) and relaunches the app for you.

## Example: USB-only Companion

```json
{
  "wsPort": 8089,
  "webEnabled": false,
  "webPort": 8090,
  "loopbackOnly": true
}
```
