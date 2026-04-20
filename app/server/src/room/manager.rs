//! RoomManager — 全局房间运行时状态管理
//!
//! 维护一张 `DashMap<Uuid, Arc<RoomState>>`，保证对同一 room_id 的并发请求
//! 只创建一个 `RoomState` 实例（entry API 保证原子性）。

use std::sync::Arc;

use dashmap::DashMap;
use uuid::Uuid;

use super::state::RoomState;

// ─── RoomManager ──────────────────────────────────────────────────────────────

/// 全局房间运行时状态管理器，线程安全，可跨 task 共享。
pub struct RoomManager {
    rooms: DashMap<Uuid, Arc<RoomState>>,
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RoomManager {
    /// 创建空管理器
    pub fn new() -> Self {
        Self {
            rooms: DashMap::new(),
        }
    }

    /// 获取或创建指定 room_id 的 RoomState。
    ///
    /// 若已存在，返回现有 Arc；否则原子插入新 RoomState 并返回其 Arc。
    pub fn get_or_create_room(&self, room_id: Uuid) -> Arc<RoomState> {
        self.rooms
            .entry(room_id)
            .or_insert_with(|| Arc::new(RoomState::new(room_id)))
            .clone()
    }

    /// 获取已存在的 RoomState（不创建），若不存在返回 None。
    pub fn get_room(&self, room_id: Uuid) -> Option<Arc<RoomState>> {
        self.rooms.get(&room_id).map(|r| r.clone())
    }

    /// 从管理器中删除指定房间（房间关闭时调用）。
    pub fn remove_room(&self, room_id: Uuid) {
        self.rooms.remove(&room_id);
    }

    /// 当前活跃房间数
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // J01: get_or_create_room 首次调用为新 room_id 创建 RoomState
    #[test]
    fn j01_get_or_create_room_creates_new() {
        let manager = RoomManager::new();
        let room_id = Uuid::new_v4();

        assert_eq!(manager.room_count(), 0, "manager should start empty");

        let state = manager.get_or_create_room(room_id);

        assert_eq!(
            state.room_id, room_id,
            "created RoomState should carry the correct room_id"
        );
        assert_eq!(
            manager.room_count(),
            1,
            "room_count should be 1 after first call"
        );
        assert_eq!(
            state.member_count(),
            0,
            "newly created room should have 0 members"
        );
    }

    // J02: 同一 room_id 两次调用返回同一 Arc（ptr_eq）
    #[test]
    fn j02_get_or_create_room_returns_existing() {
        let manager = RoomManager::new();
        let room_id = Uuid::new_v4();

        let first = manager.get_or_create_room(room_id);
        let second = manager.get_or_create_room(room_id);

        assert!(
            Arc::ptr_eq(&first, &second),
            "both calls for the same room_id must return the same Arc (ptr_eq)"
        );
        assert_eq!(
            manager.room_count(),
            1,
            "room_count must still be 1 after two calls for same room_id"
        );
    }
}
