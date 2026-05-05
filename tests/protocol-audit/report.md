# Protocol Binding Audit Report
Generated: 2026-05-05T15:34:43.624Z

## Summary

| Metric | Value |
|--------|-------|
| TDS Files Scanned | 144 |
| Total Binding Rows | 9 |
| **P0 Issues** | **2** |
| P1 Issues | 0 |
| P2 Info | 0 |

## ⛔ P0 Issues (Blocks CI)

- `/Users/yuanye/myWork/voice-room/doc/tds/android/T-30054.md` → **MISSING_CLIENT_CALL**: Client call not found for endpoint "SendMessage" (expected: app/android/app/src/main/java/com/voice/room/android/feature/room/RoomViewModel.kt::sendMessage) (server: `/Users/yuanye/myWork/voice-room/app/server/src/room/handler/mod.rs:18`)
- `/Users/yuanye/myWork/voice-room/doc/tds/server/T-00047.md` → **MISSING_CLIENT_CALL**: Client call not found for endpoint "SendMessage" (expected: app/android/app/src/main/java/com/voice/room/android/feature/room/RoomViewModel.kt::sendMessage) (server: `/Users/yuanye/myWork/voice-room/app/server/src/room/handler/mod.rs:18`)

## ⚠️ P1 Issues

No P1 warnings.

## Binding Coverage Matrix

| TDS File | Endpoint | Protocol | Server | Client |
|----------|----------|----------|--------|--------|
| T-30054.md | SendMessage | WS C→S | ✅ | ✅ |
| T-30054.md | POST /api/v1/chat-messages | HTTP REST | ✅ | N/A |
| T-00047.md | SendMessage | WS C→S | ✅ | ✅ |
| T-00047.md | RoomMessage | WS S→Room 广播 | ✅ | N/A |
| T-00047.md | POST /api/v1/chat-messages | HTTP REST | ✅ | N/A |
| T-00047.md | GET /api/v1/rooms/:room_id/messages | HTTP REST | ✅ | N/A |
| T-00048.md | SendMessage | WS C→S | ✅ | N/A |
| T-00048.md | RoomMessage | WS S→Room 广播 | ✅ | N/A |
| T-00048.md | POST /api/v1/chat-messages | HTTP REST | ✅ | N/A |