//! Admin Server library crate.
//!
//! This module contains shared business logic for the Admin Server,
//! including role validation, model re-exports, and migration content tests.

pub mod bootstrap;
pub mod common;
pub mod infrastructure;
pub mod modules;

/// Valid admin role strings, mirroring the CHECK constraint in the admins table.
///
/// These values must stay in sync with:
/// - `migrations/001_create_admins.sql` (CHK_ADMIN_ROLE constraint)
/// - `doc/protocol.md` §3.3 RBAC 权限矩阵
pub const VALID_ADMIN_ROLES: &[&str] = &["super_admin", "operator", "cs", "finance"];

/// Returns `true` if `role` is one of the four allowed admin role strings.
///
/// # Examples
/// ```
/// use voice_room_admin_server::is_valid_admin_role;
/// assert!(is_valid_admin_role("super_admin"));
/// assert!(!is_valid_admin_role("god"));
/// ```
pub fn is_valid_admin_role(role: &str) -> bool {
    VALID_ADMIN_ROLES.contains(&role)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;
    use voice_room_shared::models::AdminModel;

    // ────────────────────────────────────────────────────────────────────────
    // T-10001-U01  AdminModel 结构体字段完整性
    // ────────────────────────────────────────────────────────────────────────

    /// Verifies that `AdminModel` exposes every column defined in 001_create_admins.sql.
    /// If the struct is missing a field this test will NOT COMPILE (compile-time guard).
    #[test]
    fn admin_model_has_all_required_fields() {
        let model = AdminModel {
            id: Uuid::new_v4(),
            username: "test_admin".to_string(),
            password_hash: "$2b$12$placeholder_hash_value_here_60ch".to_string(),
            role: "operator".to_string(),
            display_name: Some("测试管理员".to_string()),
            is_active: true,
            last_login_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert_eq!(model.username, "test_admin");
        assert_eq!(model.role, "operator");
        assert!(model.is_active, "is_active should default to true");
        assert!(model.display_name.is_some(), "display_name should be optional");
        assert!(model.last_login_at.is_none(), "last_login_at should start as None");
    }

    /// AdminModel fields must carry the right types (UUID, String, Option<String>,
    /// bool, Option<DateTime<Utc>>, DateTime<Utc>).
    #[test]
    fn admin_model_field_types_are_correct() {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let model = AdminModel {
            id,
            username: "finance_mgr".to_string(),
            password_hash: "$2b$12$hash".to_string(),
            role: "finance".to_string(),
            display_name: None,
            is_active: false,
            last_login_at: Some(now),
            created_at: now,
            updated_at: now,
        };
        // UUID round-trip
        assert_eq!(model.id, id);
        // last_login_at is Some when set
        assert_eq!(model.last_login_at, Some(now));
        // is_active set to false
        assert!(!model.is_active);
    }

    // ────────────────────────────────────────────────────────────────────────
    // T-10001-U02  is_valid_admin_role 角色枚举校验
    // ────────────────────────────────────────────────────────────────────────

    /// All four valid roles must pass validation.
    #[test]
    fn valid_roles_are_accepted() {
        for role in ["super_admin", "operator", "cs", "finance"] {
            assert!(
                is_valid_admin_role(role),
                "role '{role}' must be valid per the admins table CHECK constraint"
            );
        }
    }

    /// Invalid or misspelled roles must be rejected.
    #[test]
    fn invalid_roles_are_rejected() {
        for role in [
            "admin",
            "user",
            "superadmin",
            "SUPER_ADMIN",
            "Operator",
            "",
            "root",
            "god",
            "super admin", // space
        ] {
            assert!(
                !is_valid_admin_role(role),
                "role '{role}' must be rejected"
            );
        }
    }

    /// Exactly four roles are valid — not more, not fewer.
    #[test]
    fn valid_roles_count_is_exactly_four() {
        assert_eq!(
            VALID_ADMIN_ROLES.len(),
            4,
            "VALID_ADMIN_ROLES must contain exactly 4 values"
        );
    }

    // ────────────────────────────────────────────────────────────────────────
    // T-10001-U03  bcrypt 密码 hash 格式验证
    // ────────────────────────────────────────────────────────────────────────

    /// password_hash 使用 bcrypt 算法，输出以 $2b$ 或 $2a$ 开头，长度 ≥ 60 字符。
    #[test]
    fn password_hash_uses_bcrypt_format() {
        use voice_room_shared::crypto::hash_password;

        let hash = hash_password("super_admin_password")
            .expect("bcrypt hash should succeed");

        assert!(
            hash.starts_with("$2b$") || hash.starts_with("$2a$"),
            "password_hash must use bcrypt format (starts with $2b$ or $2a$), got: {hash}"
        );
        assert!(
            hash.len() >= 60,
            "bcrypt hash length should be ≥ 60 chars, got: {}",
            hash.len()
        );
    }

    /// bcrypt hash 必须可以通过 verify_password 验证。
    #[test]
    fn password_hash_is_verifiable() {
        use voice_room_shared::crypto::{hash_password, verify_password};

        let plain = "admin_password_change_me";
        let hash = hash_password(plain).expect("hash should succeed");

        assert!(
            verify_password(plain, &hash).expect("verify should succeed"),
            "correct password must verify against its own hash"
        );
        assert!(
            !verify_password("wrong_password", &hash).expect("verify should succeed"),
            "wrong password must NOT verify"
        );
    }

    // ────────────────────────────────────────────────────────────────────────
    // T-10001-M01  Migration 001: admins 表 DDL 结构验证
    // ────────────────────────────────────────────────────────────────────────

    /// Migration 001 must CREATE TABLE admins with all required columns.
    #[test]
    fn migration_001_creates_admins_table() {
        let sql = include_str!("../migrations/001_create_admins.sql");

        assert!(
            sql.contains("CREATE TABLE") && sql.contains("admins"),
            "migration 001 must create the admins table"
        );
        for col in ["id", "username", "password_hash", "role", "display_name",
                    "is_active", "last_login_at", "created_at", "updated_at"] {
            assert!(
                sql.contains(col),
                "admins table must have column '{col}'"
            );
        }
    }

    /// username 必须有 UNIQUE 约束（直接在列定义上或通过 UNIQUE INDEX）。
    #[test]
    fn migration_001_username_has_unique_constraint() {
        let sql = include_str!("../migrations/001_create_admins.sql");
        let sql_upper = sql.to_uppercase();

        assert!(
            sql_upper.contains("UNIQUE"),
            "admins.username must have a UNIQUE constraint, found no UNIQUE keyword"
        );
    }

    /// role 字段必须有 CHECK 约束，且包含全部四个枚举值。
    #[test]
    fn migration_001_role_has_check_constraint_with_all_values() {
        let sql = include_str!("../migrations/001_create_admins.sql");
        let sql_upper = sql.to_uppercase();

        assert!(
            sql_upper.contains("CHECK"),
            "role column must have a CHECK constraint"
        );
        for role in ["super_admin", "operator", "cs", "finance"] {
            assert!(
                sql.contains(role),
                "CHECK constraint must include role value '{role}'"
            );
        }
    }

    /// password_hash 列类型应足够存储 bcrypt hash（≥ 60 字符）。
    #[test]
    fn migration_001_password_hash_column_is_text_or_varchar_200() {
        let sql = include_str!("../migrations/001_create_admins.sql");
        let sql_upper = sql.to_uppercase();

        // 列类型为 TEXT 或 VARCHAR(n) where n>=60
        // 简单断言: 文件包含 "VARCHAR(200)" 或 "TEXT"
        let has_varchar200 = sql_upper.contains("VARCHAR(200)");
        let has_text = sql_upper.contains("TEXT");
        assert!(
            has_varchar200 || has_text,
            "password_hash column must be VARCHAR(200) or TEXT to store a 60-char bcrypt hash"
        );
    }

    // ────────────────────────────────────────────────────────────────────────
    // T-10001-M02  Migration 002: admin_logs 表 DDL 结构验证
    // ────────────────────────────────────────────────────────────────────────

    /// Migration 002 must CREATE TABLE admin_logs with required columns.
    #[test]
    fn migration_002_creates_admin_logs_table() {
        let sql = include_str!("../migrations/002_create_admin_logs.sql");

        assert!(
            sql.contains("CREATE TABLE") && sql.contains("admin_logs"),
            "migration 002 must create admin_logs table"
        );
        for col in ["id", "admin_id", "action", "created_at"] {
            assert!(
                sql.contains(col),
                "admin_logs must have column '{col}'"
            );
        }
    }

    /// admin_id 必须外键引用 admins(id)。
    #[test]
    fn migration_002_admin_id_references_admins() {
        let sql = include_str!("../migrations/002_create_admin_logs.sql");

        assert!(
            sql.contains("REFERENCES admins"),
            "admin_logs.admin_id must REFERENCES admins(id) for referential integrity"
        );
    }

    /// admin_logs 表必须有索引提升查询性能。
    #[test]
    fn migration_002_has_indexes_on_admin_logs() {
        let sql = include_str!("../migrations/002_create_admin_logs.sql");
        let sql_upper = sql.to_uppercase();

        assert!(
            sql_upper.contains("CREATE INDEX"),
            "migration 002 must create indexes on admin_logs for query performance"
        );
    }

    // ────────────────────────────────────────────────────────────────────────
    // T-10001-M03  Migration 003: 种子数据 super_admin
    // ────────────────────────────────────────────────────────────────────────

    /// Migration 003 must INSERT a default super_admin account.
    #[test]
    fn migration_003_seeds_default_super_admin() {
        let sql = include_str!("../migrations/003_seed_super_admin.sql");
        let sql_upper = sql.to_uppercase();

        assert!(
            sql_upper.contains("INSERT"),
            "seed migration must contain an INSERT statement"
        );
        assert!(
            sql.contains("super_admin"),
            "seed migration must insert a super_admin role account"
        );
    }

    /// 种子数据的 password_hash 必须是 bcrypt 格式（$2b$ 或 $2a$ 前缀）。
    #[test]
    fn migration_003_seed_password_hash_uses_bcrypt_prefix() {
        let sql = include_str!("../migrations/003_seed_super_admin.sql");

        assert!(
            sql.contains("$2b$") || sql.contains("$2a$"),
            "seed password_hash must use bcrypt format ($2b$ or $2a$ prefix)"
        );
    }
}
