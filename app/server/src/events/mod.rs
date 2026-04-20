//! 管理事件模块
//!
//! 处理从 Redis `admin:events` 频道订阅的管理操作事件：
//! - `BanUser`         → 封禁用户（断开 WS 连接）
//! - `CloseRoom`       → 关闭房间（广播通知 + 断开所有成员）
//! - `BroadcastNotice` → 系统广播公告（推送至所有在线连接）

pub mod admin_event;
pub mod handler;
pub mod subscriber;

pub use handler::handle_admin_event;
pub use subscriber::start_admin_event_subscriber;
