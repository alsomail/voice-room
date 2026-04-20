//! RoomState — 语音房间运行时内存状态
//!
//! 每个 active 房间对应一个 `RoomState` 实例，存储在 `RoomManager` 中。
//! 使用 `DashMap` 保证成员表的无锁并发读写；麦位用 `RwLock<Vec>` 保护。

use dashmap::{DashMap, DashSet};
use std::sync::RwLock;
use uuid::Uuid;

// ─── 错误类型 ─────────────────────────────────────────────────────────────────

/// `take_mic_slot` 原子操作的错误枚举
#[derive(Debug, PartialEq)]
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
    /// 已处理消息 ID（幂等去重，MVP 阶段不做大小限制）
    pub processed_msg_ids: DashSet<String>,
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
            processed_msg_ids: DashSet::new(),
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
