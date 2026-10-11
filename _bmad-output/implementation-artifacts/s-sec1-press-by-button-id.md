# Story 14.1 (S-SEC1): Companion phân giải press theo button id

Status: review

## Bối cảnh

Hiện tại client gửi `{ type: "press", payload: <ButtonConfig đầy đủ> }` và Companion thực thi đúng config nhận được (`websocket.rs` → emit `trigger-macro` → `execute_logic`). Nghĩa là bất kỳ ai mở được kết nối WS tới `:8089` đều có thể gửi `actionType: "command"` với `commandValue` tuỳ ý và chạy shell command trên máy Companion — layout đã lưu không giới hạn được gì.

Story này đổi nguồn tin cậy: client chỉ gửi **id**, Companion tra button trong `layout.json` đã lưu rồi mới thực thi. Story này không xác thực client (xem S-SEC2), nhưng giới hạn hành động thực thi được vào tập button chủ máy đã cấu hình.

Là tiền đề để land PR #7 (USB mode), theo cam kết trong comment review của PR #7.

## Story

As a chủ máy chạy Companion,
I want Companion chỉ thực thi các button đã lưu trong layout của tôi,
so that một client (hoặc trang web) kết nối được tới port WS không thể chạy lệnh tuỳ ý.

## Acceptance Criteria

1. **Given** client gửi `press` với payload `{ "id": "<id>" }`,
   **When** id tồn tại trong layout đã lưu (`buttons` hoặc `pages[].buttons`),
   **Then** Companion thực thi config **lấy từ layout đã lưu**, không lấy từ payload.
2. **Given** client cũ gửi payload là `ButtonConfig` đầy đủ,
   **When** Companion nhận,
   **Then** chỉ đọc trường `id`, bỏ qua mọi trường khác; nếu `commandValue`/`shortcutValue`/... trong payload khác bản đã lưu thì bản đã lưu thắng.
3. **Given** id không tồn tại trong layout, payload thiếu `id`, hoặc `id` không phải string,
   **When** Companion nhận `press`,
   **Then** không thực thi gì, log `press rejected from <addr>: unknown button id <id>`, và gửi `toast` lỗi về **riêng** client đó (không broadcast).
4. **Given** không có `layout.json` (đang dùng `default_layout`),
   **When** nhận `press`,
   **Then** tra trong `default_layout()` — cùng nguồn với dữ liệu đã seed cho client.
5. **Given** button tra được có `buttonKind == "monitor"`,
   **Then** giữ hành vi hiện tại (no-op).
6. Path IPC `execute_button_action` (Dashboard preview, chạy trong webview tin cậy) **không đổi** — editor vẫn cần thử button chưa lưu.
7. Event Tauri `trigger-macro` bị bỏ: `websocket.rs` gọi trực tiếp hàm thực thi. Lý do: event emit được từ frontend cũng tới listener này, là một đường vào thứ hai nhận `ButtonConfig` thô.

## Tasks / Subtasks

- [x] Task 1: Hàm tra button (AC: 1, 3, 4)
  - [x] `fn find_button(layout: &Value, id: &str) -> Option<ButtonConfig>` trong `lib.rs`: duyệt `pages[].buttons` rồi `buttons`; deserialize từng phần tử sang `ButtonConfig`, bỏ qua phần tử lỗi.
  - [x] Đọc layout bằng `get_cached_layout` (đã fallback `default_layout`) — đưa thành `pub(crate)` nếu cần.
- [x] Task 2: Đổi handler `press` (AC: 1, 2, 3, 7)
  - [x] `websocket.rs`: lấy `payload.id` → `find_button` → `tauri::async_runtime::spawn(execute_logic(...))`; log giữ format `macro ok:` / `macro FAILED:` hiện có.
  - [x] Id không hợp lệ: gửi `WSMessage { type: "toast", payload: { kind: "error", message } }` qua `ws_sender` của connection đó.
  - [x] Xoá `app.listen("trigger-macro", ...)` trong `run()`.
- [x] Task 3: Frontend (AC: 2)
  - [x] `layout.ts::pressButton`: gửi `{ type: 'press', payload: { id: button.id } }` trên path WS. Path IPC giữ nguyên `button`.
  - [x] Sửa các assertion trong `connection.test.mjs` nếu phụ thuộc shape payload.
- [x] Task 4: Test (AC: 1–5)
  - [x] Rust unit test cho `find_button`: id ở `buttons`, id ở `pages[1]`, id không tồn tại, phần tử hỏng bị bỏ qua, layout không có `pages`.
  - [x] Đột biến kiểm chứng: đổi `find_button` trả `None` cố định → test phải đỏ.
- [ ] Task 5: Manual QA
  - [ ] Android client mới + client cũ (APK v1.6.x) đều bấm được.
  - [ ] Gửi tay qua `websocat` một `press` có `commandValue` lạ với id hợp lệ → lệnh đã lưu chạy, lệnh lạ không chạy.

## Dev Notes

- Thứ tự ưu tiên khi trùng id giữa `pages` và `buttons`: lấy `pages` trước (multi-page là nguồn hiện hành; `buttons` là trường legacy). Cần kiểm lại trong `layout.ts` xem layout hiện ghi đồng bộ hai trường thế nào trước khi chốt.
- Đọc `layout.json` mỗi lần press là chấp nhận được (file nhỏ, tần suất bấm tay). Không thêm cache ở story này.
- Không thay đổi wire format của `sync_layout`. Layout seed cho client vẫn chứa `commandValue` — rò rỉ thông tin này được xử lý gián tiếp ở S-SEC2 (client chưa xác thực không nhận layout).
- Toast lỗi gửi riêng cho client: hiện `broadcast_toast` gửi tới mọi subscriber; ở đây dùng `ws_sender` trực tiếp.
