# Story 14.2 (S-SEC2): Xác thực client WS bằng pairing token + kiểm tra Origin

Status: ready-for-dev

## Bối cảnh

Sau S-SEC1, client chỉ bấm được button đã lưu, nhưng **ai cũng là client**: không có bước xác thực nào ở `handle_connection` (`accept_async` nhận mọi kết nối). Hai nhóm nguy cơ:

- **Thiết bị khác trong LAN** kết nối thẳng `ws://<ip>:8089`, nhận layout (gồm cả `commandValue`) và bấm button.
- **Trang web bất kỳ trong trình duyệt** (trên PC hoặc trên điện thoại trong LAN) mở `new WebSocket("ws://127.0.0.1:8089")`. Trình duyệt không áp CORS cho WebSocket, nên bind loopback (PR #7, `loopbackOnly`) không chặn được trường hợp này — gọi là cross-site WebSocket hijacking (CSWSH).

Story này thêm hai lớp, kiểm tra **trong handshake**, trước khi seed layout:

1. **Pairing token**: bí mật ngẫu nhiên do Companion sinh, chỉ phát qua kênh chủ máy kiểm soát (QR trên Dashboard, `adb` ở USB mode).
2. **Origin check**: nếu request có header `Origin` thì Origin đó phải nằm trong allowlist.

## Story

As a chủ máy chạy Companion,
I want chỉ thiết bị tôi đã ghép đôi mới kết nối và bấm được,
so that máy khác trong LAN hoặc một trang web độc hại không điều khiển được PC của tôi.

## Acceptance Criteria

1. **Given** Companion khởi động lần đầu,
   **Then** sinh token 32 byte ngẫu nhiên (CSPRNG), mã hoá base64url, lưu ở `app_config_dir()/pairing.json` (ghi atomic: `.tmp` + rename, cùng pattern `save_layout_config`). Lần sau đọc lại token cũ.
2. **Given** client mở WS tới `ws://host:port/?token=<token>`,
   **When** token khớp (so sánh constant-time) **và** Origin hợp lệ (AC 4),
   **Then** handshake thành công, client được seed layout như hiện tại.
3. **Given** token thiếu hoặc sai (Origin đã hợp lệ),
   **Then** Companion hoàn tất upgrade, gửi đúng một message `{ "type": "auth_failed" }`, rồi đóng với close code `4401` (reason `pairing required`). Trước khi đóng: không seed layout, không xử lý message nào từ client, không tăng `CLIENT_COUNT`. Log `ws rejected from <addr>: bad token`.
4. **Given** request có header `Origin`,
   **Then** chỉ chấp nhận nếu Origin thuộc allowlist:
   - `http://tauri.localhost`, `https://tauri.localhost`, `tauri://localhost` (app Tauri desktop/Android);
   - `http://<bất kỳ host>:<webPort>` — trang web client do chính Companion phục vụ ở port HTTP;
   - chỉ ở debug build: `http://localhost:1420`, `http://<bất kỳ host>:1420` (Vite dev).
   Ngoài allowlist → từ chối ngay trong handshake bằng HTTP `403` (không upgrade), log `ws rejected from <addr>: origin <origin>`.
   Request **không có** `Origin` (client native, `websocat`) chỉ cần qua token.
5. **Given** QR kết nối APK trên Dashboard,
   **Then** payload thành `android-stream-desk://connect?v=1&host=..&wsPort=..&token=..`. `parseApkConnectPayload` đọc thêm `token` (tuỳ chọn để không vỡ parse), client lưu token cùng `server_ip`/`server_port` trong localStorage và gắn vào URL WS.
6. **Given** web client URL / QR trên Dashboard,
   **Then** URL thành `http://<ip>:<webPort>/#token=<token>` (fragment — không đi vào request log hay header Referer). Web client đọc fragment, lưu localStorage, rồi `history.replaceState` xoá fragment khỏi thanh địa chỉ.
7. **Given** client nhận `auth_failed` hoặc close code `4401`,
   **Then** client dừng auto-reconnect, hiện trạng thái `Cần ghép đôi lại — quét QR trên Companion` và CTA quét QR (tái dùng CTA của S-REL3). Quét QR mới thì reset trạng thái này.
8. **Given** client chưa có token (cài cũ, chưa quét QR có token),
   **Then** vẫn thử kết nối không kèm token và rơi vào AC 7 — không có chế độ chuyển tiếp, mọi thiết bị quét QR lại một lần.
9. **Given** chủ máy bấm `Đặt lại mã ghép đôi` trong Settings → Network,
   **Then** sinh token mới, lưu, đóng mọi kết nối WS đang mở; QR trên Dashboard cập nhật. Có xác nhận trước khi đặt lại vì mọi thiết bị phải quét lại.
10. **Given** Dashboard,
   **Then** không bao giờ hiện token dạng text đầy đủ; chỉ nằm trong QR và nút `Sao chép link`.

## Tasks / Subtasks

- [ ] Task 1: Token store (AC: 1, 8)
  - [ ] Module `src-tauri/src/pairing.rs`: `load_or_create(config_dir)`, `rotate(config_dir)`, `verify(candidate) -> bool` (constant-time, ví dụ crate `subtle` hoặc so sánh tự viết không short-circuit).
  - [ ] Thêm dependency sinh số ngẫu nhiên (`getrandom` hoặc `rand`) — kiểm `cargo check` cả target Android.
  - [ ] Command `get_pairing_token` (chỉ Dashboard dùng) và `rotate_pairing_token`.
- [ ] Task 2: Handshake (AC: 2, 3, 4)
  - [ ] `websocket.rs`: `accept_async` → `accept_hdr_async`; callback kiểm `Origin` (trả `ErrorResponse` 403) và ghi nhận query `token` ra ngoài callback.
  - [ ] Sau upgrade: token sai → gửi `auth_failed`, đóng `CloseFrame { code: 4401 }`, return trước khi seed layout.
  - [ ] Truyền `web_port` vào `start_ws_server` để dựng allowlist Origin.
  - [ ] Tạo `ConnectionGuard` (tăng `CLIENT_COUNT`) **sau** khi token hợp lệ.
- [ ] Task 3: Rotate ngắt kết nối (AC: 9)
  - [ ] Thêm kênh shutdown (ví dụ `tokio::sync::watch`) mà vòng lặp per-client `select!` lắng nghe.
- [ ] Task 4: Frontend client (AC: 5, 6, 7, 8)
  - [ ] `apkConnectQr.ts`: build/parse `token`.
  - [ ] Thêm `'auth_failed'` vào union `WSMessage.type` trong `src/types/index.ts`.
  - [ ] `connection.ts`: lưu/đọc `server_token`; URL WS thêm `?token=`; xử lý `auth_failed` / `onclose` code `4401` → trạng thái `pairing-required`, dừng auto-reconnect.
  - [ ] Web client: đọc `#token=` khi khởi động.
- [ ] Task 5: Dashboard (AC: 5, 6, 9, 10)
  - [ ] `networkEndpointState.ts`: `buildApkEndpointPayload` / `buildWebClientUrl` nhận token.
  - [ ] Nút `Đặt lại mã ghép đôi` + dialog xác nhận trong `ModalSettings.vue`.
- [ ] Task 6: Test
  - [ ] Rust: token sinh ra đủ dài và khác nhau giữa hai lần; `verify` đúng/sai/rỗng; allowlist Origin (từng mục + `http://evil.example`, `null`, khác port); handshake callback trả 403 với Origin lạ; token sai nhận `auth_failed` + close 4401 và không nhận `sync_layout`.
  - [ ] Node: parse QR có/không token; `buildWebClientUrl` có fragment.
  - [ ] Đột biến kiểm chứng: cho `verify` luôn `true` → test handshake phải đỏ.
- [ ] Task 7: Manual QA
  - [ ] Android quét QR mới → kết nối; APK cũ không token → bị từ chối, hiện CTA ghép đôi lại.
  - [ ] Từ một trang HTML mở bằng trình duyệt trên PC, `new WebSocket('ws://127.0.0.1:8089/?token=<đúng>')` → vẫn bị `403` vì Origin.
  - [ ] `websocat` từ máy khác trong LAN không token → nhận `auth_failed`, close `4401`, không nhận layout.
  - [ ] Đặt lại mã → mọi client bị ngắt, quét lại được.

## Quyết định đã chốt (2026-10-11)

- **Q1 — Chuyển tiếp:** bắt buộc token ngay, không có `requirePairing`. Mọi thiết bị quét QR lại một lần.
- **Q2 — Báo lỗi:** Origin lạ → 403 trong handshake; token sai → upgrade, gửi `auth_failed`, đóng `4401`.
- **Q3 — USB mode (PR #7):** bridge giao token qua intent extra khi `am start`: `--es pairingToken <token>`; `MainActivity` chuyển cho JS qua bridge `AndroidScreen`, JS lưu vào cùng key `server_token`. PR #7 làm phần này khi rebase; story này chỉ đảm bảo `connection.ts` đọc token từ `server_token`.

## Dev Notes

- `tokio-tungstenite = "0.24"` có `accept_hdr_async` và `ErrorResponse`.
- Token nằm trong query string của URL WS: chỉ đi trên LAN/loopback, Companion không log URL. Không in token ra stdout ở bất kỳ log nào.
- `pairing.json` tách khỏi `server.json` có chủ đích: `docs/CONFIGURATION.md` (PR #7) hướng dẫn copy/dán `server.json`, không nên mang bí mật theo.
- `execute_button_action` (IPC) không bị ảnh hưởng — webview Dashboard không đi qua WS.
- Không thêm rate limit cho token sai: token 256 bit, thử vét cạn không khả thi; chỉ log.
- Không thuộc phạm vi: TLS (`wss://`), danh sách thiết bị/token theo từng thiết bị, thu hồi riêng từng thiết bị.
