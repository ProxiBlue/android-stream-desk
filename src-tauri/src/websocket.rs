use crate::{pairing, ListenerBindStatus};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tauri::Emitter;
use tauri::Manager;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::sync::watch;
use tokio::sync::Mutex as TokioMutex;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::{header::ORIGIN, StatusCode};
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WSMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub payload: Option<Value>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct WsServerReadyPayload {
    port: u16,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct WsServerErrorPayload {
    port: u16,
    error: String,
}

lazy_static::lazy_static! {
    static ref WS_MUTEX: Arc<TokioMutex<Option<broadcast::Sender<String>>>> = Arc::new(TokioMutex::new(None));
    static ref CLIENT_COUNT: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    static ref WS_BIND_STATUS: StdMutex<ListenerBindStatus> = StdMutex::new(ListenerBindStatus::default());
    // Bumped by `disconnect_all_clients`; every per-client loop watches it.
    static ref SHUTDOWN_TX: watch::Sender<u64> = watch::channel(0).0;
}

const BROADCAST_CAPACITY: usize = 256;

const PAIRING_CLOSE_CODE: u16 = 4401;
const PAIRING_CLOSE_REASON: &str = "pairing required";
const AUTH_FAILED_MESSAGE: &str = r#"{"type":"auth_failed"}"#;
// How long a rejected client gets to acknowledge the close frame before the
// TCP connection is dropped.
const REJECT_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);

const TAURI_ORIGINS: [&str; 3] = [
    "http://tauri.localhost",
    "https://tauri.localhost",
    "tauri://localhost",
];
const DEV_SERVER_PORT: u16 = 1420;

/// Which browser origins may open a WS connection. Requests without an
/// `Origin` header (native clients, `websocat`) are not subject to it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OriginPolicy {
    web_port: u16,
    allow_dev_server: bool,
}

impl OriginPolicy {
    /// The Vite dev server origin is only trusted in debug builds.
    pub(crate) fn for_build(web_port: u16) -> Self {
        Self {
            web_port,
            allow_dev_server: cfg!(debug_assertions),
        }
    }

    fn allows(&self, origin: &str) -> bool {
        if TAURI_ORIGINS.contains(&origin) {
            return true;
        }
        match http_origin_port(origin) {
            Some(port) => {
                port == self.web_port || (self.allow_dev_server && port == DEV_SERVER_PORT)
            }
            None => false,
        }
    }
}

/// Port of an `http://<host>:<port>` origin; `None` for anything else
/// (other schemes, implicit port, `null`, paths or credentials).
fn http_origin_port(origin: &str) -> Option<u16> {
    let rest = origin.strip_prefix("http://")?;
    let (host, port) = rest.rsplit_once(':')?;
    if host.is_empty() || host.contains(['/', '@', '?', '#']) {
        return None;
    }
    if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    port.parse().ok()
}

fn token_from_query(query: Option<&str>) -> Option<String> {
    query?
        .split('&')
        .find_map(|pair| pair.strip_prefix("token="))
        .map(str::to_string)
}

fn forbidden_response() -> ErrorResponse {
    let mut response = ErrorResponse::new(Some("origin not allowed".to_string()));
    *response.status_mut() = StatusCode::FORBIDDEN;
    response
}

fn pairing_close_frame() -> CloseFrame<'static> {
    CloseFrame {
        code: CloseCode::from(PAIRING_CLOSE_CODE),
        reason: PAIRING_CLOSE_REASON.into(),
    }
}

/// Closes every open WS connection, e.g. after the pairing token rotated.
pub fn disconnect_all_clients() {
    SHUTDOWN_TX.send_modify(|generation| *generation = generation.wrapping_add(1));
}

pub fn current_ws_bind_status() -> ListenerBindStatus {
    WS_BIND_STATUS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn set_ws_bind_status(status: ListenerBindStatus) {
    *WS_BIND_STATUS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = status;
}

pub async fn start_ws_server(bind_host: &str, port: u16, web_port: u16, app_handle: tauri::AppHandle) {
    set_ws_bind_status(ListenerBindStatus::default());
    let addr = format!("{}:{}", bind_host, port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            let msg = format!("Failed to bind WebSocket TCP port {}: {}", port, e);
            eprintln!("{}", msg);
            set_ws_bind_status(ListenerBindStatus::bind_error(port, msg.clone()));
            let _ = app_handle.emit("server-error", WsServerErrorPayload { port, error: msg });
            return;
        }
    };
    println!("WebSocket server listening on ws://{}", addr);
    set_ws_bind_status(ListenerBindStatus::ready(port));
    let _ = app_handle.emit("server-ready", WsServerReadyPayload { port });

    let (tx, _) = broadcast::channel::<String>(BROADCAST_CAPACITY);
    {
        let mut global_tx = WS_MUTEX.lock().await;
        *global_tx = Some(tx.clone());
    }

    let origin_policy = OriginPolicy::for_build(web_port);

    while let Ok((stream, addr)) = listener.accept().await {
        let tx_clone = tx.clone();
        let app_handle_clone = app_handle.clone();

        tokio::spawn(async move {
            if let Err(e) =
                handle_connection(stream, addr, tx_clone, app_handle_clone, origin_policy).await
            {
                eprintln!("Error handling connection from {}: {}", addr, e);
            }
        });
    }
}

pub async fn broadcast_layout_to_clients(layout_json: Value) {
    broadcast_message("sync_layout", layout_json).await;
}

pub async fn broadcast_toast(payload: Value) {
    broadcast_message("toast", payload).await;
}

pub async fn broadcast_metrics(cpu_percent: f32, ram_percent: f32) {
    broadcast_message(
        "metric_update",
        serde_json::json!({ "cpu_percent": cpu_percent, "ram_percent": ram_percent }),
    )
    .await;
}

async fn broadcast_message(msg_type: &str, payload: Value) {
    if let Some(ref tx) = *WS_MUTEX.lock().await {
        let msg = WSMessage {
            msg_type: msg_type.to_string(),
            payload: Some(payload),
        };
        if let Ok(serialized) = serde_json::to_string(&msg) {
            let _ = tx.send(serialized);
        }
    }
}

async fn send_current_layout(
    ws_sender: &mut futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<TcpStream>,
        Message,
    >,
    app_handle: &tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let layout_val = get_cached_layout(app_handle).unwrap_or_else(|_| default_layout());
    let sync_msg = WSMessage {
        msg_type: "sync_layout".to_string(),
        payload: Some(layout_val),
    };
    let sync_str = serde_json::to_string(&sync_msg)?;
    ws_sender.send(Message::Text(sync_str)).await?;
    Ok(())
}

struct ConnectionGuard {
    app_handle: tauri::AppHandle,
}

impl ConnectionGuard {
    fn new(app_handle: tauri::AppHandle) -> Self {
        let count = CLIENT_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
        let _ = app_handle.emit(
            "client-count-changed",
            serde_json::json!({ "count": count }),
        );
        Self { app_handle }
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let count = CLIENT_COUNT.fetch_sub(1, Ordering::SeqCst) - 1;
        let _ = self.app_handle.emit(
            "client-count-changed",
            serde_json::json!({ "count": count }),
        );
    }
}

/// Runs the WS handshake and the pairing check. `Ok(None)` means the client
/// was rejected (403 for a foreign Origin, `auth_failed` + close 4401 for a
/// missing or wrong token) and the connection is finished.
async fn authenticate<S>(
    stream: S,
    addr: SocketAddr,
    policy: OriginPolicy,
) -> Result<Option<WebSocketStream<S>>, Box<dyn std::error::Error + Send + Sync>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut token: Option<String> = None;
    let mut origin_rejected = false;
    let callback = |req: &Request, response: Response| -> Result<Response, ErrorResponse> {
        if let Some(origin) = req.headers().get(ORIGIN) {
            let origin = origin.to_str().unwrap_or("<non-ascii>");
            if !policy.allows(origin) {
                eprintln!("ws rejected from {}: origin {}", addr, origin);
                origin_rejected = true;
                return Err(forbidden_response());
            }
        }
        // Never log the URI: the token travels in its query string.
        token = token_from_query(req.uri().query());
        Ok(response)
    };

    let mut ws_stream = match accept_hdr_async(stream, callback).await {
        Ok(ws_stream) => ws_stream,
        Err(_) if origin_rejected => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    if !pairing::verify(token.as_deref().unwrap_or("")) {
        eprintln!("ws rejected from {}: bad token", addr);
        reject_unpaired(&mut ws_stream).await;
        return Ok(None);
    }

    Ok(Some(ws_stream))
}

async fn reject_unpaired<S>(ws_stream: &mut WebSocketStream<S>)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let _ = ws_stream
        .send(Message::Text(AUTH_FAILED_MESSAGE.to_string()))
        .await;
    let _ = ws_stream.close(Some(pairing_close_frame())).await;
    // Wait for the peer's close ack, discarding anything it sent meanwhile.
    let _ = tokio::time::timeout(REJECT_DRAIN_TIMEOUT, async {
        while let Some(Ok(_)) = ws_stream.next().await {}
    })
    .await;
}

async fn handle_connection(
    stream: TcpStream,
    addr: SocketAddr,
    tx: broadcast::Sender<String>,
    app_handle: tauri::AppHandle,
    origin_policy: OriginPolicy,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Subscribed before the token check so a rotation racing the handshake
    // still disconnects this client.
    let mut shutdown_rx = SHUTDOWN_TX.subscribe();
    let Some(ws_stream) = authenticate(stream, addr, origin_policy).await? else {
        return Ok(());
    };
    let _guard = ConnectionGuard::new(app_handle.clone());
    println!("New WebSocket connection established from: {}", addr);

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut rx = tx.subscribe();

    // Seed the new client with the current layout, falling back to default.
    let _ = send_current_layout(&mut ws_sender, &app_handle).await;

    let mut keep_running = true;

    while keep_running {
        tokio::select! {
            _ = shutdown_rx.changed() => {
                println!("Closing WebSocket client {}: pairing token rotated", addr);
                let _ = ws_sender.send(Message::Close(Some(pairing_close_frame()))).await;
                keep_running = false;
            }
            recv_result = rx.recv() => {
                match recv_result {
                    Ok(msg_str) => {
                        if ws_sender.send(Message::Text(msg_str)).await.is_err() {
                            keep_running = false;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        eprintln!("Client {} lagged by {} messages — resyncing layout.", addr, skipped);
                        let _ = send_current_layout(&mut ws_sender, &app_handle).await;
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        keep_running = false;
                    }
                }
            }
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(parsed_msg) = serde_json::from_str::<WSMessage>(&text) {
                            match parsed_msg.msg_type.as_str() {
                                "ping" => {
                                    let pong = WSMessage { msg_type: "pong".to_string(), payload: None };
                                    if let Ok(pong_str) = serde_json::to_string(&pong) {
                                        let _ = ws_sender.send(Message::Text(pong_str)).await;
                                    }
                                }
                                "press" => {
                                    // Chỉ đọc `id`; config thực thi luôn lấy từ layout đã lưu.
                                    // layout.json có nhưng đọc/parse lỗi → từ chối, không rơi về default_layout.
                                    let layout_result = get_cached_layout(&app_handle).map_err(|e| e.to_string());
                                    match resolve_press(layout_result, parsed_msg.payload.as_ref()) {
                                        Ok(button) => {
                                            println!("press received from {}: {}", addr, button.label);
                                            tauri::async_runtime::spawn(crate::run_resolved_button(
                                                app_handle.clone(),
                                                button,
                                            ));
                                        }
                                        Err(rejection) => {
                                            eprintln!("press rejected from {}: {}", addr, rejection.log_reason());
                                            if let Ok(toast_str) = serde_json::to_string(&rejection.toast()) {
                                                let _ = ws_sender.send(Message::Text(toast_str)).await;
                                            }
                                        }
                                    }
                                }
                                "device_info" => {
                                    if let Some(payload_val) = parsed_msg.payload {
                                        let _ = app_handle.emit("client-device-info", payload_val);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        println!("WebSocket client connection closed: {}", addr);
                        keep_running = false;
                    }
                    Some(Err(e)) => {
                        eprintln!("WebSocket error on connection {}: {}", addr, e);
                        keep_running = false;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn get_cached_layout(
    app_handle: &tauri::AppHandle,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let app_dir = app_handle.path().app_config_dir()?;
    let config_path = app_dir.join("layout.json");
    if config_path.exists() {
        let content = std::fs::read_to_string(config_path)?;
        let val: Value = serde_json::from_str(&content)?;
        Ok(val)
    } else {
        Ok(default_layout())
    }
}

fn default_layout() -> Value {
    let rows = 3u32;
    let cols = 3u32;
    let total = (rows * cols) as usize;
    let buttons: Vec<Value> = (0..total)
        .map(|i| {
            serde_json::json!({
                "id": format!("btn_{}", i),
                "label": format!("Button {}", i + 1),
                "emoji": "🎮",
                "backgroundColor": "#1e293b",
                "actionType": "shortcut",
                "shortcutValue": "Ctrl+Tab"
            })
        })
        .collect();
    serde_json::json!({
        "rows": rows,
        "cols": cols,
        "buttons": buttons,
    })
}

/// Mô tả `id` trong payload `press` để log/toast khi bị từ chối: id string
/// giữ nguyên, thiếu id → `<missing>`, kiểu khác → JSON của giá trị đó.
fn describe_press_id(payload: Option<&Value>) -> String {
    match payload.and_then(|p| p.get("id")) {
        Some(Value::String(id)) => id.clone(),
        Some(other) => other.to_string(),
        None => "<missing>".to_string(),
    }
}

/// Lý do một `press` bị từ chối; không lý do nào dẫn tới thực thi.
#[derive(Debug, PartialEq, Eq)]
enum PressRejection {
    /// `layout.json` tồn tại nhưng đọc/parse lỗi.
    LayoutUnreadable(String),
    /// Id không có trong layout, thiếu, hoặc không phải string (đã mô tả bằng `describe_press_id`).
    UnknownId(String),
}

impl PressRejection {
    fn log_reason(&self) -> String {
        match self {
            PressRejection::LayoutUnreadable(err) => format!("layout unreadable: {}", err),
            PressRejection::UnknownId(shown_id) => format!("unknown button id {}", shown_id),
        }
    }

    /// Toast lỗi gửi riêng cho client đã bấm (không broadcast).
    fn toast(&self) -> WSMessage {
        let message = match self {
            PressRejection::LayoutUnreadable(_) => {
                "Không đọc được layout đã lưu trên Companion, nút không được thực thi".to_string()
            }
            PressRejection::UnknownId(shown_id) => {
                format!("Không tìm thấy nút \"{}\" trong layout đã lưu", shown_id)
            }
        };
        WSMessage {
            msg_type: "toast".to_string(),
            payload: Some(serde_json::json!({ "kind": "error", "message": message })),
        }
    }
}

/// Phân giải `press` thành button đã lưu. `layout` là kết quả của
/// `get_cached_layout` (file không tồn tại đã được thay bằng `default_layout`);
/// lỗi đọc/parse thì từ chối thay vì fallback.
fn resolve_press(
    layout: Result<Value, String>,
    payload: Option<&Value>,
) -> Result<crate::ButtonConfig, PressRejection> {
    let layout = layout.map_err(PressRejection::LayoutUnreadable)?;
    payload
        .and_then(|p| p.get("id"))
        .and_then(Value::as_str)
        .and_then(|id| crate::find_button(&layout, id))
        .ok_or_else(|| PressRejection::UnknownId(describe_press_id(payload)))
}

/// Cho test ở `lib.rs` tra button trên đúng layout seed khi chưa có `layout.json` (AC 4).
#[cfg(test)]
pub(crate) fn default_layout_for_tests() -> Value {
    default_layout()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::http::HeaderValue;
    use tokio_tungstenite::tungstenite::Error as WsError;

    const TEST_WEB_PORT: u16 = 18090;

    fn release_policy() -> OriginPolicy {
        OriginPolicy {
            web_port: TEST_WEB_PORT,
            allow_dev_server: false,
        }
    }

    fn debug_policy() -> OriginPolicy {
        OriginPolicy {
            web_port: TEST_WEB_PORT,
            allow_dev_server: true,
        }
    }

    #[test]
    fn origin_allowlist_accepts_tauri_and_web_client_origins() {
        let policy = release_policy();
        for origin in TAURI_ORIGINS {
            assert!(policy.allows(origin), "{}", origin);
        }
        assert!(policy.allows("http://192.168.1.20:18090"));
        assert!(policy.allows("http://localhost:18090"));
        assert!(policy.allows("http://[::1]:18090"));
    }

    #[test]
    fn origin_allowlist_rejects_foreign_origins() {
        let policy = release_policy();
        for origin in [
            "http://evil.example",
            "null",
            "",
            "http://192.168.1.20:18091",
            "https://192.168.1.20:18090",
            "http://tauri.localhost.evil.example",
            "http://:18090",
            "http://user@evil.example:18090",
            "http://localhost:1420",
            "http://192.168.1.20:1420",
        ] {
            assert!(!policy.allows(origin), "{}", origin);
        }
    }

    #[test]
    fn dev_server_origin_only_allowed_when_enabled() {
        assert!(debug_policy().allows("http://localhost:1420"));
        assert!(debug_policy().allows("http://192.168.1.20:1420"));
        assert!(!debug_policy().allows("http://evil.example"));
        assert!(!release_policy().allows("http://localhost:1420"));
        assert_eq!(
            OriginPolicy::for_build(TEST_WEB_PORT).allows("http://localhost:1420"),
            cfg!(debug_assertions)
        );
    }

    #[test]
    fn token_is_read_from_query_string() {
        assert_eq!(token_from_query(Some("token=abc")), Some("abc".to_string()));
        assert_eq!(
            token_from_query(Some("v=1&token=abc")),
            Some("abc".to_string())
        );
        assert_eq!(token_from_query(Some("xtoken=abc")), None);
        assert_eq!(token_from_query(None), None);
    }

    /// Mirrors `handle_connection`: authenticated clients are seeded with a
    /// `sync_layout` message, rejected ones get nothing.
    async fn spawn_test_server(policy: OriginPolicy) -> SocketAddr {
        pairing::install_for_test(pairing::TEST_TOKEN);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((stream, peer)) = listener.accept().await {
                tokio::spawn(async move {
                    if let Ok(Some(mut ws)) = authenticate(stream, peer, policy).await {
                        let _ = ws
                            .send(Message::Text(
                                r#"{"type":"sync_layout","payload":{}}"#.to_string(),
                            ))
                            .await;
                        while let Some(Ok(_)) = ws.next().await {}
                    }
                });
            }
        });
        addr
    }

    async fn connect(
        addr: SocketAddr,
        query: &str,
        origin: Option<&str>,
    ) -> Result<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>, WsError> {
        let mut request = format!("ws://{}/{}", addr, query)
            .into_client_request()
            .unwrap();
        if let Some(origin) = origin {
            request
                .headers_mut()
                .insert(ORIGIN, HeaderValue::from_str(origin).unwrap());
        }
        tokio_tungstenite::connect_async(request)
            .await
            .map(|(ws, _)| ws)
    }

    /// Everything the server sends until it closes the connection.
    async fn collect_messages(
        mut ws: WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>,
    ) -> Vec<Message> {
        let mut messages = Vec::new();
        let _ = tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(Ok(msg)) = ws.next().await {
                let is_close = matches!(msg, Message::Close(_));
                messages.push(msg);
                if is_close {
                    break;
                }
            }
        })
        .await;
        messages
    }

    async fn first_message(
        ws: &mut WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>,
    ) -> Message {
        tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("server sent nothing")
            .expect("stream ended")
            .expect("ws error")
    }

    fn assert_pairing_rejection(messages: &[Message]) {
        assert_eq!(
            messages.first(),
            Some(&Message::Text(AUTH_FAILED_MESSAGE.to_string())),
            "first message must be auth_failed, got {:?}",
            messages
        );
        assert!(
            !messages
                .iter()
                .any(|m| matches!(m, Message::Text(t) if t.contains("sync_layout"))),
            "rejected client must not receive the layout: {:?}",
            messages
        );
        match messages.last() {
            Some(Message::Close(Some(frame))) => {
                assert_eq!(u16::from(frame.code), PAIRING_CLOSE_CODE);
                assert_eq!(frame.reason, PAIRING_CLOSE_REASON);
            }
            other => panic!("expected close 4401, got {:?}", other),
        }
        assert_eq!(messages.len(), 2, "{:?}", messages);
    }

    #[tokio::test]
    async fn handshake_with_valid_token_is_seeded() {
        let addr = spawn_test_server(release_policy()).await;
        let query = format!("?token={}", pairing::TEST_TOKEN);

        let mut native = connect(addr, &query, None).await.unwrap();
        assert!(
            matches!(first_message(&mut native).await, Message::Text(t) if t.contains("sync_layout"))
        );

        let mut app = connect(addr, &query, Some("http://tauri.localhost"))
            .await
            .unwrap();
        assert!(
            matches!(first_message(&mut app).await, Message::Text(t) if t.contains("sync_layout"))
        );
    }

    #[tokio::test]
    async fn handshake_with_wrong_token_gets_auth_failed_and_4401() {
        let addr = spawn_test_server(release_policy()).await;
        let ws = connect(addr, "?token=not-the-token", None).await.unwrap();
        assert_pairing_rejection(&collect_messages(ws).await);
    }

    #[tokio::test]
    async fn handshake_without_token_gets_auth_failed_and_4401() {
        let addr = spawn_test_server(release_policy()).await;
        let ws = connect(addr, "", Some("http://tauri.localhost"))
            .await
            .unwrap();
        assert_pairing_rejection(&collect_messages(ws).await);
    }

    #[tokio::test]
    async fn handshake_from_foreign_origin_gets_403_even_with_valid_token() {
        let addr = spawn_test_server(release_policy()).await;
        let query = format!("?token={}", pairing::TEST_TOKEN);
        for origin in ["http://evil.example", "null", "http://127.0.0.1:18091"] {
            match connect(addr, &query, Some(origin)).await {
                Err(WsError::Http(response)) => {
                    assert_eq!(response.status(), StatusCode::FORBIDDEN, "{}", origin)
                }
                Err(e) => panic!("expected HTTP 403 for {}, got {:?}", origin, e),
                Ok(_) => panic!("foreign origin {} was upgraded", origin),
            }
        }
    }

    #[tokio::test]
    async fn disconnect_all_clients_closes_open_connections() {
        let mut shutdown_rx = SHUTDOWN_TX.subscribe();
        disconnect_all_clients();
        tokio::time::timeout(Duration::from_secs(1), shutdown_rx.changed())
            .await
            .expect("shutdown not signalled")
            .unwrap();
    }

    #[test]
    fn resolve_press_rejects_when_layout_unreadable() {
        // btn_0 có trong default_layout: nếu lỗi layout rơi về fallback thì sẽ thực thi nhầm.
        let payload = serde_json::json!({ "id": "btn_0" });
        let result = resolve_press(Err("expected value at line 1".to_string()), Some(&payload));
        assert_eq!(
            result.unwrap_err(),
            PressRejection::LayoutUnreadable("expected value at line 1".to_string())
        );
    }

    #[test]
    fn resolve_press_uses_stored_layout_and_rejects_bad_ids() {
        let layout = default_layout();
        let ok = resolve_press(Ok(layout.clone()), Some(&serde_json::json!({ "id": "btn_1" })));
        assert_eq!(ok.unwrap().label, "Button 2");
        assert_eq!(
            resolve_press(Ok(layout.clone()), Some(&serde_json::json!({ "id": "nope" }))).unwrap_err(),
            PressRejection::UnknownId("nope".to_string())
        );
        assert_eq!(
            resolve_press(Ok(layout.clone()), Some(&serde_json::json!({ "id": 7 }))).unwrap_err(),
            PressRejection::UnknownId("7".to_string())
        );
        assert_eq!(
            resolve_press(Ok(layout), None).unwrap_err(),
            PressRejection::UnknownId("<missing>".to_string())
        );
    }

    #[test]
    fn press_rejection_toast_is_error_kind() {
        let toast = PressRejection::LayoutUnreadable("boom".to_string()).toast();
        assert_eq!(toast.msg_type, "toast");
        assert_eq!(toast.payload.unwrap()["kind"], "error");
    }

    #[test]
    fn ws_server_ready_payload_serializes_port() {
        let payload = WsServerReadyPayload { port: 18089 };

        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            serde_json::json!({ "port": 18089 })
        );
    }

    #[test]
    fn ws_server_error_payload_serializes_port_and_error() {
        let payload = WsServerErrorPayload {
            port: 18089,
            error: "address already in use".to_string(),
        };

        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            serde_json::json!({
                "port": 18089,
                "error": "address already in use"
            })
        );
    }

    #[test]
    fn ws_bind_status_can_store_ready_and_error_states() {
        set_ws_bind_status(ListenerBindStatus::ready(18089));
        assert_eq!(current_ws_bind_status(), ListenerBindStatus::ready(18089));

        set_ws_bind_status(ListenerBindStatus::bind_error(
            18090,
            "address already in use".to_string(),
        ));
        assert_eq!(
            current_ws_bind_status(),
            ListenerBindStatus::bind_error(18090, "address already in use".to_string())
        );
    }
}
