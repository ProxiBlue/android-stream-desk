//! USB plug-and-play bridge for loopback-only (USB) mode.
//!
//! The Android client in USB mode dials its *own* `127.0.0.1:<wsPort>`, and
//! `adb reverse tcp:<wsPort> tcp:<wsPort>` carries that over the cable to the
//! Companion's loopback listener. This module keeps that reverse forward
//! alive without any OS-level automation (udev, systemd, Task Scheduler):
//!
//! 1. locate an `adb` binary (config override, `PATH`, common SDK paths),
//! 2. `adb start-server`,
//! 3. stream `adb track-devices` — the adb server pushes a fresh device list
//!    on every USB attach/detach/authorise event,
//! 4. for every USB device that reaches state `device` **and is on the
//!    user's allowlist** (`usbAllowedDevices`), run `adb reverse`, install the
//!    client APK if the phone lacks it and `usbAutoInstall` is on (see
//!    [`locate_apk`]), and bring the app to the foreground. Phones not on the
//!    list are only reported, so the Dashboard can offer to allow them.
//!
//! The phone side already auto-reconnects every few seconds, so once the
//! forward is (re)established the grid comes back on its own. If the adb
//! server dies the stream ends and the loop starts over.
//!
//! Status is kept in a global and emitted as the `usb-bridge-status` Tauri
//! event so the Dashboard can show "linked / waiting / adb not found".

use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::Notify;

pub const USB_BRIDGE_STATUS_EVENT: &str = "usb-bridge-status";

/// Android application id of the client (matches `identifier` in
/// tauri.conf.json and the package in gen/android).
pub const APP_PACKAGE: &str = "com.ania.android.stream.desk";
const APP_LAUNCH_COMPONENT: &str = "com.ania.android.stream.desk/.MainActivity";
/// File name looked for next to the Companion executable (and in the app
/// config dir) when `apkPath` is not set.
pub const DEFAULT_APK_NAME: &str = "android-stream-desk.apk";

const ADB_MISSING_RETRY: Duration = Duration::from_secs(10);
const ADB_RESTART_DELAY: Duration = Duration::from_secs(2);
const REVERSE_RETRY_DELAY: Duration = Duration::from_millis(1500);

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UsbDevice {
    pub serial: String,
    /// Raw adb transport state: `device`, `unauthorized`, `offline`, ...
    pub state: String,
    pub reversed: bool,
    /// On the user's allowlist. Unlisted phones are left alone.
    pub allowed: bool,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UsbBridgeStatus {
    pub enabled: bool,
    pub adb_path: Option<String>,
    /// `disabled` | `adb-missing` | `waiting` | `pending` | `installing` | `linked` | `unauthorized` | `error`
    pub state: String,
    pub devices: Vec<UsbDevice>,
    pub message: Option<String>,
}

/// What the user allowed the bridge to do. Changed live from the Dashboard
/// via [`set_policy`]; the bridge re-evaluates connected phones on change.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UsbPolicy {
    pub allowed_devices: Vec<String>,
    pub auto_install: bool,
}

impl UsbPolicy {
    pub fn from_config(config: &crate::ServerConfig) -> Self {
        Self {
            allowed_devices: normalize_serials(config.usb_allowed_devices.clone()),
            auto_install: config.usb_auto_install,
        }
    }

    pub fn allows(&self, serial: &str) -> bool {
        self.allowed_devices.iter().any(|s| s == serial)
    }
}

/// Trim, drop empties and duplicates, keep the user's order.
pub fn normalize_serials(serials: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for serial in serials {
        let serial = serial.trim().to_string();
        if !serial.is_empty() && !out.contains(&serial) {
            out.push(serial);
        }
    }
    out
}

lazy_static::lazy_static! {
    static ref USB_BRIDGE_STATUS: Mutex<UsbBridgeStatus> = Mutex::new(UsbBridgeStatus {
        state: "disabled".to_string(),
        ..UsbBridgeStatus::default()
    });
    static ref USB_POLICY: Mutex<UsbPolicy> = Mutex::new(UsbPolicy::default());
    static ref USB_POLICY_CHANGED: Notify = Notify::new();
}

pub fn set_policy(policy: UsbPolicy) {
    if let Ok(mut guard) = USB_POLICY.lock() {
        *guard = policy;
    }
    USB_POLICY_CHANGED.notify_one();
}

fn current_policy() -> UsbPolicy {
    USB_POLICY.lock().map(|p| p.clone()).unwrap_or_default()
}

pub fn current_usb_bridge_status() -> UsbBridgeStatus {
    USB_BRIDGE_STATUS
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default()
}

fn publish(app: &tauri::AppHandle, status: UsbBridgeStatus) {
    if let Ok(mut guard) = USB_BRIDGE_STATUS.lock() {
        *guard = status.clone();
    }
    let _ = app.emit(USB_BRIDGE_STATUS_EVENT, status);
}

fn status(
    adb: Option<&Path>,
    state: &str,
    devices: Vec<UsbDevice>,
    message: Option<String>,
) -> UsbBridgeStatus {
    UsbBridgeStatus {
        enabled: true,
        adb_path: adb.map(|p| p.display().to_string()),
        state: state.to_string(),
        devices,
        message,
    }
}

// ---------------------------------------------------------------------------
// Locating adb
// ---------------------------------------------------------------------------

fn adb_binary_name() -> &'static str {
    if cfg!(windows) {
        "adb.exe"
    } else {
        "adb"
    }
}

fn candidate_sdk_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(v) = std::env::var(var) {
            if !v.is_empty() {
                roots.push(PathBuf::from(v));
            }
        }
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        roots.push(home.join("Android/Sdk"));
        roots.push(home.join("Library/Android/sdk"));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        roots.push(local.join("Android").join("Sdk"));
    }
    roots.push(PathBuf::from("/usr/lib/android-sdk"));
    roots.push(PathBuf::from("/opt/android-sdk"));
    roots
}

/// Resolve the `adb` executable. Order: explicit config path, `PATH`, then
/// well-known SDK `platform-tools` directories.
pub fn locate_adb(configured: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = configured.map(str::trim).filter(|p| !p.is_empty()) {
        let path = PathBuf::from(p);
        return path.is_file().then_some(path);
    }

    let name = adb_binary_name();
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    for root in candidate_sdk_roots() {
        let candidate = root.join("platform-tools").join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    for fallback in ["/usr/local/bin/adb", "/usr/bin/adb", "/opt/homebrew/bin/adb"] {
        let candidate = PathBuf::from(fallback);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

/// Resolve the client APK to auto-install on phones that lack the app.
/// Order: explicit config path, `<exe dir>/android-stream-desk.apk`,
/// `<app config dir>/android-stream-desk.apk`. `None` = auto-install off.
pub fn locate_apk(configured: Option<&str>, app: &tauri::AppHandle) -> Option<PathBuf> {
    if let Some(p) = configured.map(str::trim).filter(|p| !p.is_empty()) {
        let path = PathBuf::from(p);
        return path.is_file().then_some(path);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(DEFAULT_APK_NAME);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    if let Ok(dir) = app.path().app_config_dir() {
        let candidate = dir.join(DEFAULT_APK_NAME);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn adb_command(adb: &Path) -> Command {
    #[allow(unused_mut)] // only mutated on Windows (creation_flags)
    let mut cmd = Command::new(adb);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

// ---------------------------------------------------------------------------
// `adb track-devices` stream parsing
// ---------------------------------------------------------------------------

/// `adb track-devices` dumps the raw `host:track-devices` protocol stream:
/// each message is a 4-hex-digit byte length followed by that many bytes of
/// `serial\tstate\n` lines. Pull one complete frame off the front of `buf`.
pub fn take_frame(buf: &mut Vec<u8>) -> Option<String> {
    if buf.len() < 4 {
        return None;
    }
    let len = match std::str::from_utf8(&buf[..4])
        .ok()
        .and_then(|h| usize::from_str_radix(h, 16).ok())
    {
        Some(len) => len,
        None => {
            // Not a length prefix — stream is out of sync; drop and resync
            // on the next message rather than looping forever.
            buf.clear();
            return None;
        }
    };
    if buf.len() < 4 + len {
        return None;
    }
    let frame = String::from_utf8_lossy(&buf[4..4 + len]).into_owned();
    buf.drain(..4 + len);
    Some(frame)
}

/// Parse a device-list frame into `(serial, state)` pairs.
pub fn parse_device_list(frame: &str) -> Vec<(String, String)> {
    frame
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            let serial = parts.next()?.trim();
            let state = parts.next()?.trim();
            if serial.is_empty() || state.is_empty() {
                return None;
            }
            Some((serial.to_string(), state.to_string()))
        })
        .collect()
}

/// USB mode means cable only: skip TCP transports (`host:port` serials from
/// `adb connect` / Wi-Fi debugging) and emulators.
pub fn is_usb_serial(serial: &str) -> bool {
    !serial.contains(':') && !serial.starts_with("emulator-")
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

async fn reverse_port(adb: &Path, serial: &str, port: u16) -> Result<(), String> {
    let spec = format!("tcp:{}", port);
    let out = adb_command(adb)
        .args(["-s", serial, "reverse", spec.as_str(), spec.as_str()])
        .output()
        .await
        .map_err(|e| format!("failed to run adb reverse: {}", e))?;
    if out.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

async fn adb_run(adb: &Path, args: &[&str]) -> Result<String, String> {
    let out = adb_command(adb)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("failed to run adb {}: {}", args.join(" "), e))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

async fn app_installed(adb: &Path, serial: &str) -> Result<bool, String> {
    let out = adb_run(adb, &["-s", serial, "shell", "pm", "list", "packages", APP_PACKAGE]).await?;
    Ok(out
        .lines()
        .any(|l| l.trim() == format!("package:{}", APP_PACKAGE)))
}

async fn install_apk(adb: &Path, serial: &str, apk: &Path) -> Result<(), String> {
    let apk_str = apk.display().to_string();
    adb_run(adb, &["-s", serial, "install", "-r", &apk_str]).await.map(|_| ())
}

async fn remove_reverse(adb: &Path, serial: &str, port: u16) -> Result<(), String> {
    let spec = format!("tcp:{}", port);
    adb_run(adb, &["-s", serial, "reverse", "--remove", spec.as_str()])
        .await
        .map(|_| ())
}

async fn launch_app(adb: &Path, serial: &str) -> Result<(), String> {
    // Phone may be dark on the desk; wake it so the launch is visible. The
    // activity itself also sets turnScreenOn/showWhenLocked, this is belt
    // and braces for devices that ignore those on a warm resume.
    let _ = adb_run(adb, &["-s", serial, "shell", "input", "keyevent", "KEYCODE_WAKEUP"]).await;
    adb_run(
        adb,
        &["-s", serial, "shell", "am", "start", "-n", APP_LAUNCH_COMPONENT],
    )
    .await
    .map(|_| ())
}

/// Make sure the client is on the phone and in the foreground. Returns a
/// message for the status line when something needs the user's attention.
async fn ensure_app(
    adb: &Path,
    serial: &str,
    apk: Option<&Path>,
    auto_install: bool,
    devices_snapshot: &[UsbDevice],
    app: &tauri::AppHandle,
) -> Option<String> {
    let installed = match app_installed(adb, serial).await {
        Ok(v) => v,
        Err(e) => return Some(format!("could not query packages on {}: {}", serial, e)),
    };

    if !installed {
        if !auto_install {
            return Some(format!(
                "{} is not installed on {}. Install it, or turn on Auto-install in Settings → Network.",
                APP_PACKAGE, serial
            ));
        }
        let Some(apk) = apk else {
            return Some(format!(
                "{} is not installed on {} and no APK was found to install (set apkPath in server.json or place {} next to the Companion).",
                APP_PACKAGE, serial, DEFAULT_APK_NAME
            ));
        };
        publish(
            app,
            status(
                Some(adb),
                "installing",
                devices_snapshot.to_vec(),
                Some(format!("Installing {} on {}…", apk.display(), serial)),
            ),
        );
        println!("USB bridge: installing {} on {}", apk.display(), serial);
        if let Err(e) = install_apk(adb, serial, apk).await {
            eprintln!("USB bridge: install failed on {}: {}", serial, e);
            return Some(format!("APK install failed on {}: {}", serial, e));
        }
        println!("USB bridge: installed {} on {}", APP_PACKAGE, serial);
    }

    match launch_app(adb, serial).await {
        Ok(()) => {
            println!("USB bridge: launched {} on {}", APP_PACKAGE, serial);
            None
        }
        Err(e) => Some(format!("could not launch app on {}: {}", serial, e)),
    }
}

async fn handle_device_list(
    adb: &Path,
    apk: Option<&Path>,
    ws_port: u16,
    frame: &str,
    reversed: &mut HashSet<String>,
    app: &tauri::AppHandle,
) {
    let policy = current_policy();
    let listed: Vec<(String, String)> = parse_device_list(frame)
        .into_iter()
        .filter(|(serial, _)| is_usb_serial(serial))
        .collect();

    // A phone the user just removed from the allowlist: drop its forward so
    // it stops reaching the Companion right away, not only after a replug.
    let revoked: Vec<String> = reversed
        .iter()
        .filter(|serial| !policy.allows(serial))
        .cloned()
        .collect();
    for serial in &revoked {
        if listed.iter().any(|(s, state)| s == serial && state == "device") {
            match remove_reverse(adb, serial, ws_port).await {
                Ok(()) => println!("USB bridge: removed adb reverse for {}", serial),
                Err(e) => eprintln!("USB bridge: adb reverse --remove failed for {}: {}", serial, e),
            }
        }
    }

    // Forget devices that went away, dropped out of `device` state or were
    // un-allowed, so a replug / re-allow re-runs `adb reverse` (the forward
    // dies with the transport).
    reversed.retain(|serial| {
        policy.allows(serial)
            && listed
                .iter()
                .any(|(s, state)| s == serial && state == "device")
    });

    let snapshot = |reversed: &HashSet<String>| -> Vec<UsbDevice> {
        listed
            .iter()
            .map(|(serial, state)| UsbDevice {
                serial: serial.clone(),
                state: state.clone(),
                reversed: reversed.contains(serial),
                allowed: policy.allows(serial),
            })
            .collect()
    };

    let mut message: Option<String> = None;
    for (serial, state) in &listed {
        if state != "device" || reversed.contains(serial) || !policy.allows(serial) {
            continue;
        }
        let mut result = reverse_port(adb, serial, ws_port).await;
        if result.is_err() {
            // Device may still be settling right after authorisation.
            tokio::time::sleep(REVERSE_RETRY_DELAY).await;
            result = reverse_port(adb, serial, ws_port).await;
        }
        match result {
            Ok(()) => {
                println!("USB bridge: adb reverse tcp:{} established for {}", ws_port, serial);
                reversed.insert(serial.clone());
                // Forward is up: get the app onto the phone and onto the screen.
                if let Some(m) =
                    ensure_app(adb, serial, apk, policy.auto_install, &snapshot(reversed), app).await
                {
                    eprintln!("USB bridge: {}", m);
                    message = Some(m);
                }
            }
            Err(e) => {
                eprintln!("USB bridge: adb reverse failed for {}: {}", serial, e);
                message = Some(format!("adb reverse failed for {}: {}", serial, e));
            }
        }
    }

    let devices = snapshot(reversed);
    let state = summarize_state(&devices, message.is_some());
    match state {
        "unauthorized" => {
            message.get_or_insert_with(|| {
                "Phone is asking to allow USB debugging — accept the prompt on the phone.".to_string()
            });
        }
        "pending" => {
            message.get_or_insert_with(|| {
                "A phone is connected but not allowed yet. Allow it below to use it as a deck.".to_string()
            });
        }
        _ => {}
    }

    publish(app, status(Some(adb), state, devices, message));
}

/// Overall bridge state for the status line. `pending` = a phone is ready on
/// USB but not on the allowlist, so the bridge is leaving it alone.
pub fn summarize_state(devices: &[UsbDevice], has_error: bool) -> &'static str {
    if devices.iter().any(|d| d.reversed) {
        "linked"
    } else if devices.iter().any(|d| d.state == "unauthorized") {
        "unauthorized"
    } else if has_error {
        "error"
    } else if devices.iter().any(|d| d.state == "device" && !d.allowed) {
        "pending"
    } else {
        "waiting"
    }
}

/// Runs forever. Spawn once at startup when `loopback_only` is on.
pub async fn run_usb_bridge(
    adb_path: Option<String>,
    apk_path: Option<String>,
    ws_port: u16,
    policy: UsbPolicy,
    app: tauri::AppHandle,
) {
    set_policy(policy);
    let apk = locate_apk(apk_path.as_deref(), &app);
    match &apk {
        Some(p) => println!("USB bridge: auto-install APK {}", p.display()),
        None => println!("USB bridge: no client APK found; auto-install disabled"),
    }
    loop {
        let Some(adb) = locate_adb(adb_path.as_deref()) else {
            let hint = match adb_path.as_deref() {
                Some(p) if !p.trim().is_empty() => {
                    format!("adbPath \"{}\" from server.json does not exist.", p)
                }
                _ => "adb not found on PATH or in a known Android SDK location. Install platform-tools or set adbPath in server.json.".to_string(),
            };
            publish(&app, status(None, "adb-missing", Vec::new(), Some(hint)));
            tokio::time::sleep(ADB_MISSING_RETRY).await;
            continue;
        };

        if let Err(e) = adb_command(&adb).arg("start-server").output().await {
            publish(
                &app,
                status(
                    Some(&adb),
                    "error",
                    Vec::new(),
                    Some(format!("adb start-server failed: {}", e)),
                ),
            );
            tokio::time::sleep(ADB_RESTART_DELAY).await;
            continue;
        }

        publish(&app, status(Some(&adb), "waiting", Vec::new(), None));

        let mut child = match adb_command(&adb)
            .arg("track-devices")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                publish(
                    &app,
                    status(
                        Some(&adb),
                        "error",
                        Vec::new(),
                        Some(format!("adb track-devices failed to start: {}", e)),
                    ),
                );
                tokio::time::sleep(ADB_RESTART_DELAY).await;
                continue;
            }
        };

        let Some(mut stdout) = child.stdout.take() else {
            let _ = child.kill().await;
            tokio::time::sleep(ADB_RESTART_DELAY).await;
            continue;
        };

        let mut buf: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 4096];
        let mut reversed: HashSet<String> = HashSet::new();
        // Last device list adb sent; replayed when the policy changes so an
        // "Allow" click links an already-plugged phone without a replug.
        let mut last_frame: Option<String> = None;

        loop {
            tokio::select! {
                read = stdout.read(&mut chunk) => {
                    let n = match read {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    buf.extend_from_slice(&chunk[..n]);
                    while let Some(frame) = take_frame(&mut buf) {
                        handle_device_list(&adb, apk.as_deref(), ws_port, &frame, &mut reversed, &app).await;
                        last_frame = Some(frame);
                    }
                }
                _ = USB_POLICY_CHANGED.notified() => {
                    if let Some(frame) = &last_frame {
                        handle_device_list(&adb, apk.as_deref(), ws_port, frame, &mut reversed, &app).await;
                    }
                }
            }
        }

        let _ = child.kill().await;
        publish(
            &app,
            status(
                Some(&adb),
                "waiting",
                Vec::new(),
                Some("adb server stopped; reconnecting.".to_string()),
            ),
        );
        tokio::time::sleep(ADB_RESTART_DELAY).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_frame_splits_length_prefixed_messages() {
        let body1 = "R58M12345\tdevice\n";
        let body2 = "";
        let mut buf = format!("{:04x}{}{:04x}{}", body1.len(), body1, body2.len(), body2).into_bytes();

        assert_eq!(take_frame(&mut buf).as_deref(), Some(body1));
        assert_eq!(take_frame(&mut buf).as_deref(), Some(""));
        assert_eq!(take_frame(&mut buf), None);
        assert!(buf.is_empty());
    }

    #[test]
    fn take_frame_waits_for_partial_message() {
        let body = "R58M12345\tunauthorized\n";
        let full = format!("{:04x}{}", body.len(), body);
        let (head, tail) = full.split_at(10);

        let mut buf = head.as_bytes().to_vec();
        assert_eq!(take_frame(&mut buf), None);
        assert_eq!(buf.len(), 10);

        buf.extend_from_slice(tail.as_bytes());
        assert_eq!(take_frame(&mut buf).as_deref(), Some(body));
    }

    #[test]
    fn take_frame_resyncs_on_garbage_prefix() {
        let mut buf = b"zzzz oops".to_vec();
        assert_eq!(take_frame(&mut buf), None);
        assert!(buf.is_empty());
    }

    #[test]
    fn parse_device_list_reads_serial_and_state() {
        let parsed = parse_device_list("R58M12345\tdevice\n192.168.1.5:5555\toffline\n\n");
        assert_eq!(
            parsed,
            vec![
                ("R58M12345".to_string(), "device".to_string()),
                ("192.168.1.5:5555".to_string(), "offline".to_string()),
            ]
        );
    }

    #[test]
    fn usb_serial_filter_excludes_tcp_and_emulators() {
        assert!(is_usb_serial("R58M12345"));
        assert!(!is_usb_serial("192.168.1.5:5555"));
        assert!(!is_usb_serial("emulator-5554"));
    }

    #[test]
    fn locate_adb_rejects_missing_configured_path() {
        assert_eq!(
            locate_adb(Some("/definitely/not/here/adb")),
            None
        );
    }

    #[test]
    fn status_serializes_camel_case() {
        let s = status(
            Some(Path::new("/usr/bin/adb")),
            "linked",
            vec![UsbDevice {
                serial: "R58M".to_string(),
                state: "device".to_string(),
                reversed: true,
                allowed: true,
            }],
            None,
        );
        let json = serde_json::to_value(s).unwrap();
        assert_eq!(json["adbPath"], "/usr/bin/adb");
        assert_eq!(json["state"], "linked");
        assert_eq!(json["devices"][0]["reversed"], true);
        assert_eq!(json["devices"][0]["allowed"], true);
    }

    fn device(serial: &str, state: &str, reversed: bool, allowed: bool) -> UsbDevice {
        UsbDevice {
            serial: serial.to_string(),
            state: state.to_string(),
            reversed,
            allowed,
        }
    }

    #[test]
    fn policy_allows_only_listed_serials() {
        let policy = UsbPolicy {
            allowed_devices: vec!["R58M12345".to_string()],
            auto_install: false,
        };
        assert!(policy.allows("R58M12345"));
        assert!(!policy.allows("PERSONAL999"));
        assert!(!UsbPolicy::default().allows("R58M12345"), "empty allowlist allows nothing");
    }

    #[test]
    fn normalize_serials_trims_and_dedupes_in_order() {
        let got = normalize_serials(vec![
            " B ".to_string(),
            "A".to_string(),
            "".to_string(),
            "B".to_string(),
        ]);
        assert_eq!(got, vec!["B".to_string(), "A".to_string()]);
    }

    #[test]
    fn summarize_state_reports_unallowed_phone_as_pending() {
        assert_eq!(summarize_state(&[], false), "waiting");
        assert_eq!(summarize_state(&[device("P", "device", false, false)], false), "pending");
        assert_eq!(
            summarize_state(
                &[device("P", "device", false, false), device("D", "device", true, true)],
                false
            ),
            "linked",
            "a linked deck wins over a pending phone"
        );
        assert_eq!(summarize_state(&[device("U", "unauthorized", false, true)], false), "unauthorized");
        assert_eq!(summarize_state(&[device("D", "device", false, true)], true), "error");
    }
}
