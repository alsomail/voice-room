//! ConnectionRegistry — 管理所有活跃 WebSocket 连接句柄
//!
//! 使用 DashMap 实现线程安全的无锁并发读写，无需外部 Mutex。
//! 每个连接以独立的 connection_id (UUID) 为键存储，支持同一用户同时建立多条连接。
//! user_id 仅作为 handle 的元数据，不再作为 key。

use std::sync::{Arc, RwLock};
use std::time::Instant;

use dashmap::DashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

// ─── 数据结构 ─────────────────────────────────────────────────────────────────

/// 单条连接的持有句柄，存储在 Registry 中。
pub struct ConnectionHandle {
    /// 本次连接的唯一 ID（与 user_id 解耦，同一用户多连接互不干扰）
    pub connection_id: Uuid,
    /// 归属用户
    pub user_id: Uuid,
    /// 归属房间（可选，用户未加入房间时为 None）
    pub room_id: Option<Uuid>,
    /// 向该连接的写入 task 发送消息的通道
    pub sender: mpsc::UnboundedSender<String>,
    /// 最近一次心跳时间（由 ping 消息更新）
    pub last_heartbeat: Arc<RwLock<Instant>>,
}

/// 全局连接注册表，线程安全，可跨 task/线程共享。
pub struct ConnectionRegistry {
    pub(crate) connections: DashMap<Uuid, ConnectionHandle>, // key = connection_id
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionRegistry {
    /// 创建空注册表
    pub fn new() -> Self {
        Self {
            connections: DashMap::new(),
        }
    }

    /// 注册一个新连接（以 connection_id 为 key，支持同一 user_id 多连接）
    pub fn register(&self, handle: ConnectionHandle) {
        self.connections.insert(handle.connection_id, handle);
    }

    /// 注销指定连接（仅删除该 connection_id，不影响同一用户的其他连接）
    pub fn unregister(&self, connection_id: Uuid) {
        self.connections.remove(&connection_id);
    }

    /// 检查 connection_id 是否存在，存在则返回 Some(())
    pub fn get(&self, connection_id: Uuid) -> Option<()> {
        self.connections.contains_key(&connection_id).then_some(())
    }

    /// 获取连接的消息发送端（克隆 sender，不持有引用）
    pub fn get_sender(&self, connection_id: Uuid) -> Option<mpsc::UnboundedSender<String>> {
        self.connections.get(&connection_id).map(|h| h.sender.clone())
    }

    /// 向指定连接发送消息；连接不存在或 channel 已关闭时返回 false
    pub fn send_to(&self, connection_id: Uuid, message: &str) -> bool {
        match self.connections.get(&connection_id) {
            Some(handle) => handle.sender.send(message.to_string()).is_ok(),
            None => false,
        }
    }

    /// 按 user_id 获取该用户所有连接的 (connection_id, sender) 对
    /// （用于向某用户的全部设备发消息，同时保留 connection_id 以便注销）
    pub fn get_by_user_id(&self, user_id: Uuid) -> Vec<(Uuid, mpsc::UnboundedSender<String>)> {
        self.connections
            .iter()
            .filter(|entry| entry.user_id == user_id)
            .map(|entry| (entry.connection_id, entry.sender.clone()))
            .collect()
    }

    /// 按 room_id 获取该房间内所有连接的 (connection_id, sender) 对
    /// （用于房间级广播或批量断开）
    pub fn get_connections_in_room(&self, room_id: Uuid) -> Vec<(Uuid, mpsc::UnboundedSender<String>)> {
        self.connections
            .iter()
            .filter(|entry| entry.room_id == Some(room_id))
            .map(|entry| (entry.connection_id, entry.sender.clone()))
            .collect()
    }

    /// 设置连接所属房间（JoinRoom 信令成功后调用）
    ///
    /// 若 connection_id 不存在则静默忽略（连接可能已断开）。
    pub fn set_room_id(&self, connection_id: Uuid, room_id: Uuid) {
        if let Some(mut handle) = self.connections.get_mut(&connection_id) {
            handle.room_id = Some(room_id);
        }
    }

    /// 获取连接当前所在房间 ID
    ///
    /// 若 connection_id 不存在或未加入房间，返回 None。
    pub fn get_room_id(&self, connection_id: Uuid) -> Option<Uuid> {
        self.connections.get(&connection_id).and_then(|h| h.room_id)
    }

    /// 清除连接的房间关联（LeaveRoom 信令或断线时调用）
    ///
    /// 若 connection_id 不存在则静默忽略。
    pub fn clear_room_id(&self, connection_id: Uuid) {
        if let Some(mut handle) = self.connections.get_mut(&connection_id) {
            handle.room_id = None;
        }
    }

    /// 向所有连接广播消息；sender 已断开的连接自动从 map 中移除并记录日志
    pub fn broadcast_to_all(&self, message: &str) {
        self.connections.retain(|connection_id, handle| {
            if handle.sender.send(message.to_string()).is_err() {
                tracing::info!(
                    %connection_id,
                    user_id = %handle.user_id,
                    "removing dead connection during broadcast"
                );
                false
            } else {
                true
            }
        });
    }

    /// 当前连接数
    pub fn count(&self) -> usize {
        self.connections.len()
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_handle(user_id: Uuid) -> (ConnectionHandle, mpsc::UnboundedReceiver<String>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let handle = ConnectionHandle {
            connection_id: Uuid::new_v4(),
            user_id,
            room_id: None,
            sender: tx,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        };
        (handle, rx)
    }

    // R01: 注册连接，成功注册（count 变为 1）
    #[tokio::test]
    async fn r01_register_connection_success() {
        let registry = ConnectionRegistry::new();
        let user_id = Uuid::new_v4();
        let (handle, _rx) = make_handle(user_id);

        registry.register(handle);

        assert_eq!(registry.count(), 1, "registry should contain 1 connection after register");
    }

    // R02: 注册后可通过 connection_id 查找到 sender
    #[tokio::test]
    async fn r02_lookup_registered_connection() {
        let registry = ConnectionRegistry::new();
        let user_id = Uuid::new_v4();
        let (handle, _rx) = make_handle(user_id);
        let conn_id = handle.connection_id;

        registry.register(handle);

        let sender = registry.get_sender(conn_id);
        assert!(sender.is_some(), "registered connection_id should return a sender");
    }

    // R03: 注销连接，查找返回 None
    #[tokio::test]
    async fn r03_unregister_removes_connection() {
        let registry = ConnectionRegistry::new();
        let user_id = Uuid::new_v4();
        let (handle, _rx) = make_handle(user_id);
        let conn_id = handle.connection_id;

        registry.register(handle);
        assert_eq!(registry.count(), 1);

        registry.unregister(conn_id);
        assert_eq!(registry.count(), 0, "count should be 0 after unregister");
        assert!(
            registry.get_sender(conn_id).is_none(),
            "get_sender should return None after unregister"
        );
    }

    // R04: 并发注册 100 个连接，无 panic
    #[tokio::test]
    async fn r04_concurrent_registration() {
        use std::sync::Arc;

        let registry = Arc::new(ConnectionRegistry::new());
        let mut handles = Vec::new();

        for _ in 0..100 {
            let reg = registry.clone();
            let h = tokio::spawn(async move {
                let uid = Uuid::new_v4();
                let (handle, _rx) = make_handle(uid);
                reg.register(handle);
            });
            handles.push(h);
        }

        for h in handles {
            h.await.expect("task should not panic");
        }

        assert_eq!(
            registry.count(),
            100,
            "all 100 concurrent registrations should succeed"
        );
    }

    // R05: 向连接发送消息，receiver 收到
    #[tokio::test]
    async fn r05_send_message_to_connection() {
        let registry = ConnectionRegistry::new();
        let user_id = Uuid::new_v4();
        let (handle, mut rx) = make_handle(user_id);
        let conn_id = handle.connection_id;

        registry.register(handle);

        let sent = registry.send_to(conn_id, r#"{"type":"test"}"#);
        assert!(sent, "send_to should return true for existing connection");

        let msg = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("should not time out")
            .expect("channel should not be closed");
        assert_eq!(msg, r#"{"type":"test"}"#);
    }

    // R06: 同一用户第二个连接不会删除第一个连接的 handle
    #[tokio::test]
    async fn r06_same_user_second_connection_does_not_remove_first() {
        let registry = Arc::new(ConnectionRegistry::new());

        let user_id = Uuid::new_v4();

        // 注册同一用户的第一个连接
        let conn_id_1 = Uuid::new_v4();
        let (tx1, _rx1) = mpsc::unbounded_channel::<String>();
        registry.register(ConnectionHandle {
            connection_id: conn_id_1,
            user_id,
            room_id: None,
            sender: tx1,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });
        assert_eq!(registry.count(), 1);

        // 注册同一用户的第二个连接
        let conn_id_2 = Uuid::new_v4();
        let (tx2, _rx2) = mpsc::unbounded_channel::<String>();
        registry.register(ConnectionHandle {
            connection_id: conn_id_2,
            user_id,
            room_id: None,
            sender: tx2,
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
        });
        assert_eq!(registry.count(), 2);

        // 注销第一个连接（模拟旧连接断开的清理）
        registry.unregister(conn_id_1);

        // 第二个连接必须仍然存在
        assert!(
            registry.get(conn_id_2).is_some(),
            "second connection should still exist after first connection is unregistered"
        );
        assert_eq!(registry.count(), 1, "only 1 connection should remain");
    }

    // R08: clear_room_id 后 get_room_id 为 None，且 get_connections_in_room 不含该连接
    #[tokio::test]
    async fn r08_clear_room_id_removes_connection_from_room() {
        let registry = ConnectionRegistry::new();
        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();

        let (handle, _rx) = make_handle(user_id);
        let conn_id = handle.connection_id;

        registry.register(handle);
        registry.set_room_id(conn_id, room_id);

        // 确认连接在房间内
        assert_eq!(registry.get_connections_in_room(room_id).len(), 1);
        assert_eq!(registry.get_room_id(conn_id), Some(room_id));

        // 清除 room_id
        registry.clear_room_id(conn_id);

        // get_room_id 应返回 None
        assert!(
            registry.get_room_id(conn_id).is_none(),
            "get_room_id should return None after clear_room_id"
        );
        // get_connections_in_room 不应再包含该连接
        assert!(
            registry.get_connections_in_room(room_id).is_empty(),
            "get_connections_in_room should be empty after clear_room_id"
        );
    }

    // R07: set_room_id 后 get_connections_in_room 能找到该连接
    #[tokio::test]
    async fn r07_set_room_id_makes_connection_visible_in_room() {
        let registry = ConnectionRegistry::new();
        let user_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();

        let (handle, _rx) = make_handle(user_id);
        let conn_id = handle.connection_id;

        registry.register(handle);

        // 注册时 room_id = None，此时房间内无连接
        assert!(
            registry.get_connections_in_room(room_id).is_empty(),
            "no connections should be in room before set_room_id"
        );

        // 设置 room_id
        registry.set_room_id(conn_id, room_id);

        // 现在应该能找到
        let conns = registry.get_connections_in_room(room_id);
        assert_eq!(conns.len(), 1, "should find 1 connection after set_room_id");
        assert_eq!(
            conns[0].0, conn_id,
            "the found connection should be the one we set room_id for"
        );
    }
}
