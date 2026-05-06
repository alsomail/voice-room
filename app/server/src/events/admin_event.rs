//! AdminEvent — Redis `admin:events` 频道的事件类型定义（消费端）
//!
//! T-00105: 共享类型从 `voice_room_shared::admin_event` 重导出，
//! 确保发布端（adminServer）和消费端（server）使用完全相同的 enum 定义。

// ─── 共享类型重导出 ────────────────────────────────────────────────────────────
//
// 权威定义在 `app/shared/src/admin_event.rs`（Serialize + Deserialize + Clone + PartialEq）
pub use voice_room_shared::admin_event::{
    AdminEvent, BanUserPayload, BroadcastNoticePayload, CloseRoomPayload, UnbanUserPayload,
};
