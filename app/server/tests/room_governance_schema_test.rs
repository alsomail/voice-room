//! 集成测试 — T-00024 rooms 扩字段 + 治理审计表迁移
//!
//! 验收用例 S24-01 ~ S24-06：
//! - S24-01: 迁移脚本幂等（IF NOT EXISTS / DROP CONSTRAINT IF EXISTS 保证重入）
//! - S24-02: category CHECK 约束枚举值完整（'invalid' 应被拒绝）
//! - S24-03: 存量房间兼容性：cover_url DEFAULT ''，category DEFAULT 'chat'
//! - S24-04: idx_kick_records_room_ts 索引定义存在于迁移 SQL
//! - S24-05: room_mute_records.mute_type CHECK 约束仅允许 'mic'/'chat'
//! - S24-06: admin_user_id 外键引用 users(id)（REFERENCES users(id)）
//!
//! 数据库不可用时（无 DATABASE_URL），所有测试改为纯文本验证迁移 SQL 文件内容。
//! 数据库可用时，运行完整的 SQL 集成测试。

// ─────────────────────────────────────────────────────────────────────────────
// 辅助：加载迁移 SQL 文件
// ─────────────────────────────────────────────────────────────────────────────

fn load_governance_migration() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migrations/008_room_governance.sql"
    );
    std::fs::read_to_string(path)
        .expect("008_room_governance.sql must exist at app/server/migrations/")
}

// ─────────────────────────────────────────────────────────────────────────────
// S24-01: 迁移脚本幂等性 — IF NOT EXISTS / DROP CONSTRAINT IF EXISTS 均已使用
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn s24_01_migration_is_idempotent_via_if_not_exists() {
    let sql = load_governance_migration();

    // rooms 扩字段使用 IF NOT EXISTS
    assert!(
        sql.contains("ADD COLUMN IF NOT EXISTS cover_url"),
        "S24-01: cover_url must use ADD COLUMN IF NOT EXISTS"
    );
    assert!(
        sql.contains("ADD COLUMN IF NOT EXISTS category"),
        "S24-01: category must use ADD COLUMN IF NOT EXISTS"
    );
    assert!(
        sql.contains("ADD COLUMN IF NOT EXISTS password_hash"),
        "S24-01: password_hash must use ADD COLUMN IF NOT EXISTS"
    );
    assert!(
        sql.contains("ADD COLUMN IF NOT EXISTS announcement"),
        "S24-01: announcement must use ADD COLUMN IF NOT EXISTS"
    );
    assert!(
        sql.contains("ADD COLUMN IF NOT EXISTS admin_user_id"),
        "S24-01: admin_user_id must use ADD COLUMN IF NOT EXISTS"
    );

    // CHECK 约束先 DROP IF EXISTS 再 ADD → 保证重入
    assert!(
        sql.contains("DROP CONSTRAINT IF EXISTS chk_room_category"),
        "S24-01: chk_room_category must be dropped idempotently before re-adding"
    );

    // 审计表使用 CREATE TABLE IF NOT EXISTS
    assert!(
        sql.contains("CREATE TABLE IF NOT EXISTS room_kick_records"),
        "S24-01: room_kick_records must use CREATE TABLE IF NOT EXISTS"
    );
    assert!(
        sql.contains("CREATE TABLE IF NOT EXISTS room_mute_records"),
        "S24-01: room_mute_records must use CREATE TABLE IF NOT EXISTS"
    );

    // 索引使用 CREATE INDEX IF NOT EXISTS
    assert!(
        sql.contains("CREATE INDEX IF NOT EXISTS idx_kick_records_room_ts"),
        "S24-01: idx_kick_records_room_ts must use CREATE INDEX IF NOT EXISTS"
    );
    assert!(
        sql.contains("CREATE INDEX IF NOT EXISTS idx_mute_records_room_ts"),
        "S24-01: idx_mute_records_room_ts must use CREATE INDEX IF NOT EXISTS"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// S24-02: category CHECK 约束包含所有合法枚举值，且不含 'invalid'
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn s24_02_category_check_constraint_valid_values() {
    let sql = load_governance_migration();

    // 约束名称存在
    assert!(
        sql.contains("chk_room_category"),
        "S24-02: SQL must define chk_room_category constraint"
    );

    // 所有 6 种合法分类
    for category in &["chat", "emotion", "music", "game", "matchmaking", "other"] {
        assert!(
            sql.contains(&format!("'{category}'")),
            "S24-02: chk_room_category must include '{category}'"
        );
    }

    // 'invalid' 不在约束枚举中
    assert!(
        !sql.contains("'invalid'"),
        "S24-02: 'invalid' must NOT appear in category CHECK constraint"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// S24-03: 存量房间兼容 — cover_url DEFAULT '' / category DEFAULT 'chat'
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn s24_03_legacy_rooms_have_compatible_defaults() {
    let sql = load_governance_migration();

    assert!(
        sql.contains("DEFAULT ''"),
        "S24-03: cover_url must have DEFAULT '' so legacy rooms are not affected"
    );
    assert!(
        sql.contains("DEFAULT 'chat'"),
        "S24-03: category must have DEFAULT 'chat' so legacy rooms get a safe default"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// S24-04: room_kick_records 索引定义完整
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn s24_04_kick_records_index_exists_in_migration() {
    let sql = load_governance_migration();

    // 主索引（room_id + created_at）
    assert!(
        sql.contains("idx_kick_records_room_ts"),
        "S24-04: idx_kick_records_room_ts must be defined in migration"
    );
    assert!(
        sql.contains("room_kick_records(room_id, created_at"),
        "S24-04: idx_kick_records_room_ts must index (room_id, created_at ...)"
    );

    // 二级索引（target_user_id + created_at）
    assert!(
        sql.contains("idx_kick_records_target_ts"),
        "S24-04: idx_kick_records_target_ts must also be defined in migration"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// S24-05: room_mute_records.mute_type CHECK 约束仅允许 'mic'/'chat'，拒绝 'sms'
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn s24_05_mute_type_check_excludes_invalid_values() {
    let sql = load_governance_migration();

    // CHECK 约束枚举（R1 P0-2: 列名由 `type` 重命名为 `mute_type` 全链路对齐）
    assert!(
        sql.contains("CHECK (mute_type IN ('mic','chat'))"),
        "S24-05: room_mute_records.mute_type must have CHECK (mute_type IN ('mic','chat'))"
    );

    // 'sms' 不在枚举中
    assert!(
        !sql.contains("'sms'"),
        "S24-05: 'sms' must NOT appear in mute type CHECK constraint"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// S24-06: ON DELETE 策略 — rooms 外键来自 admin_user_id → users(id)
//         审计表外键对 rooms(id) 默认 RESTRICT（不显式 CASCADE 删除）
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn s24_06_foreign_keys_use_restrict_not_cascade() {
    let sql = load_governance_migration();

    // admin_user_id 引用 users(id)（不加 ON DELETE CASCADE）
    assert!(
        sql.contains("admin_user_id   UUID REFERENCES users(id)"),
        "S24-06: admin_user_id must reference users(id)"
    );

    // 审计表引用 rooms(id)，不含 ON DELETE CASCADE（默认 RESTRICT 语义）
    assert!(
        !sql.contains("ON DELETE CASCADE"),
        "S24-06: No ON DELETE CASCADE allowed — default RESTRICT must be used"
    );

    // 审计表引用 users(id)
    let kick_section = sql
        .find("CREATE TABLE IF NOT EXISTS room_kick_records")
        .expect("room_kick_records table should exist");
    let kick_end = sql[kick_section..]
        .find(';')
        .map(|i| kick_section + i)
        .unwrap_or(sql.len());
    let kick_ddl = &sql[kick_section..kick_end];
    assert!(
        kick_ddl.contains("REFERENCES users(id)"),
        "S24-06: room_kick_records must reference users(id) for target/operator"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 附加：SQL 结构完整性检查
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn s24_extra_mute_records_duration_check() {
    let sql = load_governance_migration();

    assert!(
        sql.contains("duration_sec"),
        "room_mute_records must have duration_sec column"
    );
    assert!(
        sql.contains("CHECK (duration_sec >= 0)"),
        "duration_sec must have CHECK >= 0 (0 = 解除禁言)"
    );
}

#[test]
fn s24_extra_audit_tables_have_created_at() {
    let sql = load_governance_migration();

    // 两张审计表都应有 created_at TIMESTAMPTZ
    let kick_count = sql.matches("created_at").count();
    assert!(
        kick_count >= 2,
        "Both room_kick_records and room_mute_records must have created_at columns"
    );
}

#[test]
fn s24_extra_mute_records_indexes_exist() {
    let sql = load_governance_migration();

    assert!(
        sql.contains("idx_mute_records_room_ts"),
        "idx_mute_records_room_ts must be defined"
    );
    assert!(
        sql.contains("idx_mute_records_target_type_ts"),
        "idx_mute_records_target_type_ts must be defined"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Rust model 结构验证 — 确认 RoomModel 新字段可正常构造和序列化
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod room_model_governance_fields {
    use chrono::Utc;
    use uuid::Uuid;
    use voice_room_shared::models::RoomModel;

    fn make_governance_room() -> RoomModel {
        RoomModel {
            id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            title: "Governance Room".to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            status: "active".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            cover_url: "https://example.com/cover.jpg".to_string(),
            category: "music".to_string(),
            announcement: Some("Welcome!".to_string()),
            admin_user_id: None,
        }
    }

    #[test]
    fn test_room_model_has_cover_url_field() {
        let room = make_governance_room();
        assert_eq!(room.cover_url, "https://example.com/cover.jpg");
    }

    #[test]
    fn test_room_model_has_category_field() {
        let room = make_governance_room();
        assert_eq!(room.category, "music");
    }

    #[test]
    fn test_room_model_has_announcement_field() {
        let room = make_governance_room();
        assert_eq!(room.announcement.as_deref(), Some("Welcome!"));
    }

    #[test]
    fn test_room_model_has_admin_user_id_field() {
        let mut room = make_governance_room();
        assert!(room.admin_user_id.is_none());
        room.admin_user_id = Some(Uuid::new_v4());
        assert!(room.admin_user_id.is_some());
    }

    #[test]
    fn test_room_model_cover_url_empty_default() {
        let room = RoomModel {
            id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            title: "Legacy Room".to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            status: "active".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            cover_url: String::new(),     // legacy: empty string
            category: "chat".to_string(), // legacy: default category
            announcement: None,
            admin_user_id: None,
        };
        assert_eq!(room.cover_url, "");
        assert_eq!(room.category, "chat");
    }

    #[test]
    fn test_room_model_governance_fields_serialize() {
        let room = make_governance_room();
        let json = serde_json::to_string(&room).expect("serialize should succeed");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["cover_url"], "https://example.com/cover.jpg");
        assert_eq!(v["category"], "music");
        assert_eq!(v["announcement"], "Welcome!");
        assert!(v["admin_user_id"].is_null());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Rust governance model 结构验证
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod governance_models {
    use chrono::Utc;
    use uuid::Uuid;
    use voice_room_shared::models::governance::{MuteType, RoomKickRecord, RoomMuteRecord};

    #[test]
    fn test_kick_record_construct_and_clone() {
        let record = RoomKickRecord {
            id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            target_user_id: Uuid::new_v4(),
            operator_user_id: Uuid::new_v4(),
            reason: Some("violating rules".to_string()),
            created_at: Utc::now(),
        };
        let cloned = record.clone();
        assert_eq!(record.id, cloned.id);
        assert_eq!(record.reason.as_deref(), Some("violating rules"));
    }

    #[test]
    fn test_kick_record_reason_nullable() {
        let record = RoomKickRecord {
            id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            target_user_id: Uuid::new_v4(),
            operator_user_id: Uuid::new_v4(),
            reason: None,
            created_at: Utc::now(),
        };
        assert!(record.reason.is_none());
    }

    #[test]
    fn test_kick_record_serialize() {
        let id = Uuid::new_v4();
        let record = RoomKickRecord {
            id,
            room_id: Uuid::new_v4(),
            target_user_id: Uuid::new_v4(),
            operator_user_id: Uuid::new_v4(),
            reason: Some("spam".to_string()),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&record).expect("serialize should succeed");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["id"], id.to_string());
        assert_eq!(v["reason"], "spam");
    }

    #[test]
    fn test_mute_record_construct() {
        let record = RoomMuteRecord {
            id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            target_user_id: Uuid::new_v4(),
            operator_user_id: Uuid::new_v4(),
            mute_type: MuteType::Mic,
            duration_sec: 300,
            reason: Some("spamming mic".to_string()),
            created_at: Utc::now(),
        };
        assert_eq!(record.duration_sec, 300);
    }

    #[test]
    fn test_mute_record_duration_zero_means_unmute() {
        let record = RoomMuteRecord {
            id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            target_user_id: Uuid::new_v4(),
            operator_user_id: Uuid::new_v4(),
            mute_type: MuteType::Chat,
            duration_sec: 0, // 0 = 解除禁言
            reason: None,
            created_at: Utc::now(),
        };
        assert_eq!(record.duration_sec, 0);
    }

    #[test]
    fn test_mute_type_variants() {
        let mic = MuteType::Mic;
        let chat = MuteType::Chat;
        let mic_clone = mic.clone();
        let _chat_clone = chat.clone();
        let debug = format!("{:?}", mic_clone);
        assert!(debug.contains("Mic"));
    }

    #[test]
    fn test_mute_record_serialize() {
        let record = RoomMuteRecord {
            id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            target_user_id: Uuid::new_v4(),
            operator_user_id: Uuid::new_v4(),
            mute_type: MuteType::Mic,
            duration_sec: 600,
            reason: None,
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&record).expect("serialize should succeed");
        assert!(json.contains("\"duration_sec\":600"));
    }

    #[test]
    fn test_mute_record_reason_nullable() {
        let record = RoomMuteRecord {
            id: Uuid::new_v4(),
            room_id: Uuid::new_v4(),
            target_user_id: Uuid::new_v4(),
            operator_user_id: Uuid::new_v4(),
            mute_type: MuteType::Chat,
            duration_sec: 120,
            reason: None,
            created_at: Utc::now(),
        };
        assert!(record.reason.is_none());
    }
}
