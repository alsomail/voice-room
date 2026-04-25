//! 集成测试 — T-00027 观众席列表 API（含角色标签）
//!
//! 验收用例 M27-01 ~ M27-08：
//! - M27-01: 100 人房间内存查询 < 150ms
//! - M27-02: page=0 返回 40003（ValidationError）
//! - M27-03: page 超界返回空 items，total 真实
//! - M27-04: 麦上用户（slot≠null）始终置顶
//! - M27-05: slot ASC；slot=null 的按 joined_at DESC
//! - M27-06: 房主 role=owner；admin_user_id 匹配者 role=admin；其余 member
//! - M27-07: 非连接中的用户（非房间成员）请求 → 403
//! - M27-08: muted_mic/muted_chat 字段从内存状态读取正确

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use uuid::Uuid;

use voice_room_server::common::error::AppError;
use voice_room_server::modules::room::members_service::{
    MembersRoomRepo, MembersService, MembersUserRepo, RoomOwnerInfo, UserInfo,
};
use voice_room_server::room::manager::RoomManager;
use voice_room_server::room::state::MemberInfo;

// ─── Fake UserRepo ────────────────────────────────────────────────────────────

struct FakeUsersRepo {
    users: HashMap<Uuid, (String, Option<String>)>,
}

impl FakeUsersRepo {
    fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    fn add(&mut self, id: Uuid, nickname: &str) {
        self.users.insert(id, (nickname.to_string(), None));
    }
}

#[async_trait]
impl MembersUserRepo for FakeUsersRepo {
    async fn find_users_by_ids(&self, ids: &[Uuid]) -> Result<Vec<UserInfo>, AppError> {
        let result = ids
            .iter()
            .filter_map(|id| {
                self.users.get(id).map(|(n, a)| UserInfo {
                    id: *id,
                    nickname: n.clone(),
                    avatar: a.clone(),
                })
            })
            .collect();
        Ok(result)
    }
}

// ─── Fake RoomRepo ────────────────────────────────────────────────────────────

struct FakeRoomsRepo {
    rooms: HashMap<Uuid, RoomOwnerInfo>,
}

impl FakeRoomsRepo {
    fn new() -> Self {
        Self {
            rooms: HashMap::new(),
        }
    }

    fn add(&mut self, room_id: Uuid, owner_id: Uuid, admin_user_id: Option<Uuid>) {
        self.rooms.insert(
            room_id,
            RoomOwnerInfo {
                owner_id,
                admin_user_id,
            },
        );
    }
}

#[async_trait]
impl MembersRoomRepo for FakeRoomsRepo {
    async fn find_room_owner(&self, room_id: Uuid) -> Result<Option<RoomOwnerInfo>, AppError> {
        Ok(self.rooms.get(&room_id).cloned())
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// 向 RoomManager 中添加指定数量的成员，joined_at 依次递减（第 0 个最早）
fn seed_members(manager: &RoomManager, room_id: Uuid, count: usize) -> Vec<Uuid> {
    let state = manager.get_or_create_room(room_id);
    let mut ids = Vec::with_capacity(count);
    for i in 0..count {
        let uid = Uuid::new_v4();
        let mut info = MemberInfo::new(uid, format!("user_{i}"), None);
        // 第 i 个用户进房时间：count-i 秒前（索引越大进房越晚=时间越新）
        info.joined_at = Utc::now() - Duration::seconds((count - i) as i64);
        state.members.insert(uid, info);
        ids.push(uid);
    }
    ids
}

/// 构建 MembersService
fn make_service(
    manager: Arc<RoomManager>,
    user_repo: impl MembersUserRepo + 'static,
    room_repo: impl MembersRoomRepo + 'static,
) -> MembersService {
    MembersService::new(manager, Arc::new(user_repo), Arc::new(room_repo))
}

// ─── M27-01: 100 人房间 < 150ms ──────────────────────────────────────────────

/// M27-01: 100 人房间纯内存查询耗时 < 150ms
#[tokio::test]
async fn m27_01_hundred_members_under_150ms() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let manager = Arc::new(RoomManager::new());

    let ids = seed_members(&manager, room_id, 100);
    let caller_id = ids[0];

    // 预置用户信息
    let mut user_repo = FakeUsersRepo::new();
    for i in 0..100 {
        user_repo.add(ids[i], &format!("user_{i}"));
    }

    let mut room_repo = FakeRoomsRepo::new();
    room_repo.add(room_id, owner_id, None);

    let svc = make_service(manager, user_repo, room_repo);

    let start = Instant::now();
    let result = svc
        .list_members(room_id, caller_id, 1, 20)
        .await
        .expect("should succeed");
    let elapsed = start.elapsed();

    assert_eq!(result.total, 100, "M27-01: total should be 100");
    assert!(
        elapsed.as_millis() < 150,
        "M27-01: elapsed={:?} must be < 150ms",
        elapsed
    );
}

// ─── M27-02: page=0 返回 40003 ───────────────────────────────────────────────

/// M27-02: page=0 应返回 ValidationError（code 40003）
#[tokio::test]
async fn m27_02_page_zero_returns_validation_error() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let manager = Arc::new(RoomManager::new());
    let caller_id = owner_id;

    // 把 owner 加入房间
    let state = manager.get_or_create_room(room_id);
    state.members.insert(
        caller_id,
        MemberInfo::new(caller_id, "Owner".to_string(), None),
    );

    let mut room_repo = FakeRoomsRepo::new();
    room_repo.add(room_id, owner_id, None);

    let svc = make_service(manager, FakeUsersRepo::new(), room_repo);

    let err = svc
        .list_members(room_id, caller_id, 0, 20)
        .await
        .expect_err("page=0 must fail");

    assert!(
        matches!(err, AppError::ValidationError(_)),
        "M27-02: page=0 must return ValidationError, got: {err:?}"
    );
}

// ─── M27-03: page 超界返回空 items，total 真实 ────────────────────────────────

/// M27-03: page 超过总页数时，items 为空，total 为真实成员总数
#[tokio::test]
async fn m27_03_out_of_range_page_returns_empty_items_with_real_total() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let manager = Arc::new(RoomManager::new());

    let ids = seed_members(&manager, room_id, 5);
    let caller_id = ids[0];

    let mut room_repo = FakeRoomsRepo::new();
    room_repo.add(room_id, owner_id, None);

    let svc = make_service(manager, FakeUsersRepo::new(), room_repo);

    // page=100, limit=20 → 远超 5 人
    let result = svc
        .list_members(room_id, caller_id, 100, 20)
        .await
        .expect("should succeed");

    assert_eq!(result.total, 5, "M27-03: total must be 5 (real count)");
    assert!(
        result.items.is_empty(),
        "M27-03: items must be empty for out-of-range page"
    );
}

// ─── M27-04: 麦上用户始终置顶 ────────────────────────────────────────────────

/// M27-04: 有麦位（slot != null）的用户始终出现在列表最前面
#[tokio::test]
async fn m27_04_mic_users_always_on_top() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let manager = Arc::new(RoomManager::new());

    let state = manager.get_or_create_room(room_id);

    // 添加 3 名观众
    let audience_ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
    for (i, uid) in audience_ids.iter().enumerate() {
        let mut info = MemberInfo::new(*uid, format!("audience_{i}"), None);
        info.joined_at = Utc::now() - Duration::seconds((10 - i) as i64);
        state.members.insert(*uid, info);
    }

    // 添加 2 名麦上用户，占 slot 2 和 slot 5
    let mic_user_1 = Uuid::new_v4();
    let mic_user_2 = Uuid::new_v4();
    state.members.insert(
        mic_user_1,
        MemberInfo::new(mic_user_1, "OnMic1".to_string(), None),
    );
    state.members.insert(
        mic_user_2,
        MemberInfo::new(mic_user_2, "OnMic2".to_string(), None),
    );
    state.take_mic_slot(2, mic_user_1).unwrap();
    state.take_mic_slot(5, mic_user_2).unwrap();

    let caller_id = audience_ids[0];
    let mut room_repo = FakeRoomsRepo::new();
    room_repo.add(room_id, owner_id, None);

    let svc = make_service(manager, FakeUsersRepo::new(), room_repo);

    let result = svc
        .list_members(room_id, caller_id, 1, 20)
        .await
        .expect("should succeed");

    assert_eq!(result.total, 5, "M27-04: total should be 5");

    // 前两个必须是麦上用户（slot != null）
    assert!(
        result.items[0].mic_slot.is_some(),
        "M27-04: first item must be on mic"
    );
    assert!(
        result.items[1].mic_slot.is_some(),
        "M27-04: second item must be on mic"
    );
    // 后三个必须是观众
    assert!(
        result.items[2].mic_slot.is_none(),
        "M27-04: third item must be audience"
    );
}

// ─── M27-05: slot ASC；slot=null 的按 joined_at DESC ─────────────────────────

/// M27-05: 麦上用户按 slot ASC 排序；观众按 joined_at DESC（最新进房在前）
#[tokio::test]
async fn m27_05_sorting_slot_asc_then_joined_at_desc() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let manager = Arc::new(RoomManager::new());

    let state = manager.get_or_create_room(room_id);

    // 添加观众（3人，joined_at 依次更晚）
    let early_uid = Uuid::new_v4();
    let mid_uid = Uuid::new_v4();
    let late_uid = Uuid::new_v4();

    let now = Utc::now();
    let mut early_info = MemberInfo::new(early_uid, "Early".to_string(), None);
    early_info.joined_at = now - Duration::seconds(30);
    let mut mid_info = MemberInfo::new(mid_uid, "Mid".to_string(), None);
    mid_info.joined_at = now - Duration::seconds(20);
    let mut late_info = MemberInfo::new(late_uid, "Late".to_string(), None);
    late_info.joined_at = now - Duration::seconds(10);

    state.members.insert(early_uid, early_info);
    state.members.insert(mid_uid, mid_info);
    state.members.insert(late_uid, late_info);

    // 添加麦上用户：slot=1 和 slot=0
    let slot0_uid = Uuid::new_v4();
    let slot1_uid = Uuid::new_v4();
    state.members.insert(
        slot0_uid,
        MemberInfo::new(slot0_uid, "Slot0".to_string(), None),
    );
    state.members.insert(
        slot1_uid,
        MemberInfo::new(slot1_uid, "Slot1".to_string(), None),
    );
    state.take_mic_slot(0, slot0_uid).unwrap();
    state.take_mic_slot(1, slot1_uid).unwrap();

    let caller_id = early_uid;
    let mut room_repo = FakeRoomsRepo::new();
    room_repo.add(room_id, owner_id, None);

    let svc = make_service(manager, FakeUsersRepo::new(), room_repo);

    let result = svc
        .list_members(room_id, caller_id, 1, 10)
        .await
        .expect("should succeed");

    // 前两个：slot 0 和 slot 1（ASC）
    assert_eq!(
        result.items[0].mic_slot,
        Some(0),
        "M27-05: first item must be slot=0"
    );
    assert_eq!(
        result.items[1].mic_slot,
        Some(1),
        "M27-05: second item must be slot=1"
    );

    // 后三个观众：joined_at DESC（最晚进房 → late, mid, early）
    let audience_ids: Vec<Uuid> = result.items[2..].iter().map(|it| it.user_id).collect();
    assert_eq!(
        audience_ids[0], late_uid,
        "M27-05: first audience must be latest joiner"
    );
    assert_eq!(
        audience_ids[1], mid_uid,
        "M27-05: second audience must be mid joiner"
    );
    assert_eq!(
        audience_ids[2], early_uid,
        "M27-05: third audience must be earliest joiner"
    );
}

// ─── M27-06: role 计算正确 ────────────────────────────────────────────────────

/// M27-06: owner/admin/member 角色计算正确
#[tokio::test]
async fn m27_06_role_calculation_correct() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();
    let member_id = Uuid::new_v4();
    let manager = Arc::new(RoomManager::new());

    let state = manager.get_or_create_room(room_id);

    for (uid, name) in [
        (owner_id, "Owner"),
        (admin_id, "Admin"),
        (member_id, "Member"),
    ] {
        state
            .members
            .insert(uid, MemberInfo::new(uid, name.to_string(), None));
    }

    let mut room_repo = FakeRoomsRepo::new();
    room_repo.add(room_id, owner_id, Some(admin_id));

    let svc = make_service(manager, FakeUsersRepo::new(), room_repo);

    let result = svc
        .list_members(room_id, owner_id, 1, 20)
        .await
        .expect("should succeed");

    // 找到各用户的 role
    let find_role = |uid: Uuid| {
        result
            .items
            .iter()
            .find(|it| it.user_id == uid)
            .map(|it| it.role.clone())
            .expect("user should be in result")
    };

    assert_eq!(
        find_role(owner_id),
        "owner",
        "M27-06: owner_id must have role=owner"
    );
    assert_eq!(
        find_role(admin_id),
        "admin",
        "M27-06: admin_user_id must have role=admin"
    );
    assert_eq!(
        find_role(member_id),
        "member",
        "M27-06: other must have role=member"
    );
}

// ─── M27-07: 非成员请求 → 403 ────────────────────────────────────────────────

/// M27-07: 不在房间内的用户请求 → Forbidden (403)
#[tokio::test]
async fn m27_07_non_member_returns_403() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let outsider_id = Uuid::new_v4();
    let manager = Arc::new(RoomManager::new());

    let state = manager.get_or_create_room(room_id);
    state.members.insert(
        owner_id,
        MemberInfo::new(owner_id, "Owner".to_string(), None),
    );

    let mut room_repo = FakeRoomsRepo::new();
    room_repo.add(room_id, owner_id, None);

    let svc = make_service(manager, FakeUsersRepo::new(), room_repo);

    let err = svc
        .list_members(room_id, outsider_id, 1, 20)
        .await
        .expect_err("non-member must get error");

    assert!(
        matches!(err, AppError::Forbidden(_)),
        "M27-07: non-member must get Forbidden, got: {err:?}"
    );
}

// ─── M27-08: muted_mic/muted_chat 读取正确 ───────────────────────────────────

/// M27-08: banned_mics → muted_mic=true；muted_users → muted_chat=true
#[tokio::test]
async fn m27_08_muted_mic_and_chat_fields_correct() {
    let room_id = Uuid::new_v4();
    let owner_id = Uuid::new_v4();
    let mic_muted_id = Uuid::new_v4();
    let chat_muted_id = Uuid::new_v4();
    let normal_id = Uuid::new_v4();
    let manager = Arc::new(RoomManager::new());

    let state = manager.get_or_create_room(room_id);

    for (uid, name) in [
        (owner_id, "Owner"),
        (mic_muted_id, "MicMuted"),
        (chat_muted_id, "ChatMuted"),
        (normal_id, "Normal"),
    ] {
        state
            .members
            .insert(uid, MemberInfo::new(uid, name.to_string(), None));
    }

    // 设置禁麦/禁言
    state.banned_mics.insert(mic_muted_id);
    state.muted_users.insert(chat_muted_id);

    let mut room_repo = FakeRoomsRepo::new();
    room_repo.add(room_id, owner_id, None);

    let svc = make_service(manager, FakeUsersRepo::new(), room_repo);

    let result = svc
        .list_members(room_id, owner_id, 1, 20)
        .await
        .expect("should succeed");

    let find_item = |uid: Uuid| {
        result
            .items
            .iter()
            .find(|it| it.user_id == uid)
            .cloned()
            .expect("user should be in result")
    };

    let mic_muted = find_item(mic_muted_id);
    assert!(
        mic_muted.muted_mic,
        "M27-08: banned_mics user must have muted_mic=true"
    );
    assert!(
        !mic_muted.muted_chat,
        "M27-08: banned_mics user must have muted_chat=false"
    );

    let chat_muted = find_item(chat_muted_id);
    assert!(
        !chat_muted.muted_mic,
        "M27-08: muted_users user must have muted_mic=false"
    );
    assert!(
        chat_muted.muted_chat,
        "M27-08: muted_users user must have muted_chat=true"
    );

    let normal = find_item(normal_id);
    assert!(
        !normal.muted_mic,
        "M27-08: normal user must have muted_mic=false"
    );
    assert!(
        !normal.muted_chat,
        "M27-08: normal user must have muted_chat=false"
    );
}
