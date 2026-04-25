//! RoomState — 语音房间运行时内存状态
//!
//! 每个 active 房间对应一个 `RoomState` 实例，存储在 `RoomManager` 中。
//! 使用 `DashMap` 保证成员表的无锁并发读写；麦位用 `RwLock<Vec>` 保护。

use dashmap::{DashMap, DashSet};
use std::collections::{HashSet, VecDeque};
use std::sync::{Mutex, RwLock};
use uuid::Uuid;

// ─── BoundedMsgIdSet（P2-11：处理过 msg_id 的 FIFO 容量上限集合）──────────────

/// 处理过的 msg_id 容量上限：超过即按 FIFO 淘汰最早的条目，避免热门长直播房 OOM。
///
/// 选择 10_000 是基于：单房间正常 QPS ≤ 50/秒，10_000 ≈ 200 秒窗口，
/// 远大于客户端重发抖动窗口（5-10s），足以覆盖幂等去重需求。
pub const PROCESSED_MSG_IDS_CAPACITY: usize = 10_000;

/// 容量受限的 msg_id 去重集合（FIFO 淘汰）。
///
/// 用于 `RoomState.processed_msg_ids`，在保留幂等去重语义的同时，
/// 严格限制内存上界（`capacity` 个 `String`）。
///
/// 实现：`HashSet` 提供 O(1) `contains`；`VecDeque` 维护插入顺序，
/// 容量到达后弹出最旧条目并从 `HashSet` 同步移除。
pub struct BoundedMsgIdSet {
    inner: Mutex<BoundedMsgIdInner>,
    capacity: usize,
}

struct BoundedMsgIdInner {
    set: HashSet<String>,
    order: VecDeque<String>,
}

impl BoundedMsgIdSet {
    /// 创建容量为 `capacity` 的有界集合（capacity=0 视为禁用）。
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(BoundedMsgIdInner {
                set: HashSet::with_capacity(capacity.min(1024)),
                order: VecDeque::with_capacity(capacity.min(1024)),
            }),
            capacity,
        }
    }

    /// 检查 msg_id 是否已被处理过。
    pub fn contains(&self, id: &str) -> bool {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).set.contains(id)
    }

    /// 插入 msg_id；若已存在返回 false（不重复入队）；新插入返回 true。
    /// 当容量超过上限时，弹出队首并同步从 set 中移除。
    pub fn insert(&self, id: String) -> bool {
        if self.capacity == 0 {
            return false;
        }
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if !g.set.insert(id.clone()) {
            return false;
        }
        g.order.push_back(id);
        while g.order.len() > self.capacity {
            if let Some(evicted) = g.order.pop_front() {
                g.set.remove(&evicted);
            }
        }
        true
    }

    /// 当前条目数（主要用于测试与监控）。
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).order.len()
    }

    /// 是否为空（clippy 要求与 `len` 配套）。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 容量上限。
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Default for BoundedMsgIdSet {
    fn default() -> Self {
        Self::with_capacity(PROCESSED_MSG_IDS_CAPACITY)
    }
}

// ─── 错误类型 ─────────────────────────────────────────────────────────────────

/// `take_mic_slot` 原子操作的错误枚举
#[derive(Debug, PartialEq, Eq)]
pub enum TakeMicError {
    /// 该 user_id 已占用其他麦位
    AlreadyOnMic,
    /// 目标麦位已被其他用户占用
    SlotOccupied,
}

// ─── 数据结构 ─────────────────────────────────────────────────────────────────

/// 房间成员信息（存储在 RoomState.members 中）
#[derive(Clone, Debug)]
pub struct MemberInfo {
    pub user_id: Uuid,
    pub nickname: String,
    pub avatar: Option<String>,
    /// 加入房间的时间（UTC）— T-00027
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

impl MemberInfo {
    /// 构造新成员信息，joined_at 自动设为当前 UTC 时间。
    pub fn new(user_id: Uuid, nickname: String, avatar: Option<String>) -> Self {
        Self {
            user_id,
            nickname,
            avatar,
            joined_at: chrono::Utc::now(),
        }
    }
}

/// 单个房间的运行时状态
pub struct RoomState {
    /// 房间 ID
    pub room_id: Uuid,
    /// 当前成员表（key = user_id）
    pub members: DashMap<Uuid, MemberInfo>,
    /// 麦位列表（9 个槽，None 表示空闲，Some(user_id) 表示占用）
    pub mic_slots: RwLock<Vec<Option<Uuid>>>,
    /// 禁麦用户集合（在此集合中的用户不允许上麦）
    pub banned_mics: DashSet<Uuid>,
    /// 被禁言的用户集合（初始为空，管理员功能预留）
    pub muted_users: DashSet<Uuid>,
    /// 已处理消息 ID（幂等去重，FIFO 容量上限 — P2-11 修复 OOM 隐患）
    pub processed_msg_ids: BoundedMsgIdSet,
}

impl RoomState {
    /// 创建空房间状态（9 个麦位全为 None，禁麦列表为空）
    pub fn new(room_id: Uuid) -> Self {
        Self {
            room_id,
            members: DashMap::new(),
            mic_slots: RwLock::new(vec![None; 9]),
            banned_mics: DashSet::new(),
            muted_users: DashSet::new(),
            processed_msg_ids: BoundedMsgIdSet::default(),
        }
    }

    /// 当前成员数
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// 获取麦位快照（克隆，用于序列化响应）
    ///
    /// 使用 `unwrap_or_else(|e| e.into_inner())` 防御毒化锁（PoisonError）：
    /// 即使持有写锁的线程 panic，仍可安全读取最后一次写入的数据。
    pub fn mic_slots_snapshot(&self) -> Vec<Option<Uuid>> {
        self.mic_slots
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// 从麦位列表移除指定用户（自动下麦）。
    ///
    /// 返回 `true` 表示用户确实在麦上（有槽位被置 None）。
    /// 使用 `unwrap_or_else(|e| e.into_inner())` 防御 PoisonError。
    pub fn remove_from_mic_slots(&self, user_id: Uuid) -> bool {
        let mut slots = self.mic_slots.write().unwrap_or_else(|e| e.into_inner());
        let mut was_on_mic = false;
        for slot in slots.iter_mut() {
            if *slot == Some(user_id) {
                *slot = None;
                was_on_mic = true;
            }
        }
        was_on_mic
    }

    /// 原子查找并清除用户的麦位。
    ///
    /// 持写锁期间完成"查找 + 清除"，保证并发安全。
    /// 不跨越任何 `.await` 点，写锁在函数返回时立即释放。
    ///
    /// 返回 `Some(mic_index)` 表示成功下麦并返回麦位索引，
    /// `None` 表示用户不在任何麦位。
    pub fn leave_mic_slot(&self, user_id: Uuid) -> Option<usize> {
        let mut slots = self.mic_slots.write().unwrap_or_else(|e| e.into_inner());
        for (i, slot) in slots.iter_mut().enumerate() {
            if *slot == Some(user_id) {
                *slot = None;
                return Some(i);
            }
        }
        None
    }

    /// 原子检查并占用麦位。
    ///
    /// 持写锁期间完成"检查 + 设置"，保证并发抢麦安全性。
    /// 不跨越任何 `.await` 点，写锁在函数返回时立即释放。
    ///
    /// # Errors
    /// - `TakeMicError::AlreadyOnMic`：该 `user_id` 已占用其他麦位
    /// - `TakeMicError::SlotOccupied`：目标 `mic_index` 已被占用
    pub fn take_mic_slot(&self, mic_index: usize, user_id: Uuid) -> Result<(), TakeMicError> {
        let mut slots = self.mic_slots.write().unwrap_or_else(|e| e.into_inner());
        // 检查用户是否已在任意麦位（防重复上麦）
        if slots.contains(&Some(user_id)) {
            return Err(TakeMicError::AlreadyOnMic);
        }
        // 检查目标麦位是否被其他用户占用
        if slots[mic_index].is_some() {
            return Err(TakeMicError::SlotOccupied);
        }
        slots[mic_index] = Some(user_id);
        Ok(())
    }
}

// ─── 单元测试（P2-11 BoundedMsgIdSet）─────────────────────────────────────────

#[cfg(test)]
mod bounded_msg_id_tests {
    use super::*;

    // BMS-01: 容量内插入 → contains 命中，长度递增
    #[test]
    fn bms01_insert_within_capacity_is_remembered() {
        let s = BoundedMsgIdSet::with_capacity(3);
        assert!(s.insert("a".to_string()));
        assert!(s.insert("b".to_string()));
        assert!(s.contains("a"));
        assert!(s.contains("b"));
        assert_eq!(s.len(), 2);
    }

    // BMS-02: 重复插入返回 false，长度不变
    #[test]
    fn bms02_duplicate_insert_returns_false() {
        let s = BoundedMsgIdSet::with_capacity(3);
        assert!(s.insert("a".to_string()));
        assert!(!s.insert("a".to_string()));
        assert_eq!(s.len(), 1);
    }

    // BMS-03: 超出容量后 FIFO 淘汰最早条目
    #[test]
    fn bms03_evicts_oldest_when_capacity_exceeded() {
        let s = BoundedMsgIdSet::with_capacity(2);
        s.insert("a".to_string());
        s.insert("b".to_string());
        s.insert("c".to_string()); // 应淘汰 "a"
        assert!(!s.contains("a"), "最早的 'a' 应被淘汰");
        assert!(s.contains("b"));
        assert!(s.contains("c"));
        assert_eq!(s.len(), 2);
    }

    // BMS-04: 严格守恒 — 插入 N+10 条，长度恒为 N
    #[test]
    fn bms04_strict_capacity_invariant_under_load() {
        let cap = 100;
        let s = BoundedMsgIdSet::with_capacity(cap);
        for i in 0..cap + 10 {
            s.insert(format!("msg-{i}"));
        }
        assert_eq!(s.len(), cap, "长度必须严格 == capacity");
        // 头 10 条已被淘汰
        for i in 0..10 {
            assert!(!s.contains(&format!("msg-{i}")));
        }
        // 末 cap 条仍然在
        for i in 10..cap + 10 {
            assert!(s.contains(&format!("msg-{i}")));
        }
    }

    // BMS-05: capacity=0 → 永远 contains=false（禁用）
    #[test]
    fn bms05_zero_capacity_is_disabled() {
        let s = BoundedMsgIdSet::with_capacity(0);
        assert!(!s.insert("x".to_string()));
        assert!(!s.contains("x"));
        assert_eq!(s.len(), 0);
    }

    // BMS-06: RoomState 默认使用 PROCESSED_MSG_IDS_CAPACITY 容量
    #[test]
    fn bms06_room_state_uses_default_bounded_capacity() {
        let st = RoomState::new(Uuid::new_v4());
        assert_eq!(
            st.processed_msg_ids.capacity(),
            PROCESSED_MSG_IDS_CAPACITY,
            "RoomState 默认容量必须为 PROCESSED_MSG_IDS_CAPACITY"
        );
    }
}
