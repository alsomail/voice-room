use std::{
    collections::HashMap,
    sync::Mutex,
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;
use voice_room_shared::models::AdminModel;

use crate::common::error::AppError;

// ─── AdminRepository trait ────────────────────────────────────────────────────

/// admins 表持久化抽象，隔离真实 DB 与测试 Fake。
#[async_trait]
pub trait AdminRepository: Send + Sync {
    /// 按用户名查找管理员（用于登录）。
    async fn find_by_username(&self, username: &str) -> Result<Option<AdminModel>, AppError>;

    /// 更新 last_login_at 时间戳（登录成功后调用）。
    async fn update_last_login_at(
        &self,
        admin_id: Uuid,
        time: DateTime<Utc>,
    ) -> Result<(), AppError>;
}

// ─── AdminLogRepository trait ─────────────────────────────────────────────────

/// admin_logs 表持久化抽象，用于写入登录审计日志。
#[async_trait]
pub trait AdminLogRepository: Send + Sync {
    /// 写入登录日志（action = "admin_login"）。
    async fn insert_login_log(
        &self,
        admin_id: Uuid,
        ip_address: Option<String>,
    ) -> Result<(), AppError>;
}

// ─── Postgres 实现 ───────────────────────────────────────────────────────────

/// 基于 SQLx + PostgreSQL 的 AdminRepository 生产实现。
pub struct PgAdminRepository {
    pool: PgPool,
}

impl PgAdminRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AdminRepository for PgAdminRepository {
    /// 按用户名查找未删除的管理员记录。
    async fn find_by_username(&self, username: &str) -> Result<Option<AdminModel>, AppError> {
        let admin = sqlx::query_as::<_, AdminModel>(
            "SELECT id, username, password_hash, role, display_name, \
             is_active, last_login_at, created_at, updated_at \
             FROM admins WHERE username = $1 AND deleted_at IS NULL",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(admin)
    }

    /// 登录成功后更新 last_login_at 与 updated_at。
    async fn update_last_login_at(
        &self,
        admin_id: Uuid,
        time: DateTime<Utc>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE admins SET last_login_at = $1, updated_at = $1 WHERE id = $2",
        )
        .bind(time)
        .bind(admin_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

/// 基于 SQLx + PostgreSQL 的 AdminLogRepository 生产实现。
pub struct PgAdminLogRepository {
    pool: PgPool,
}

impl PgAdminLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AdminLogRepository for PgAdminLogRepository {
    /// 写入 admin_logs 表，action = "admin_login"。
    ///
    /// ip_address 字段类型为 INET，使用字符串绑定（PostgreSQL 会自动转型）。
    async fn insert_login_log(
        &self,
        admin_id: Uuid,
        ip_address: Option<String>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO admin_logs (admin_id, action, ip_address) \
             VALUES ($1, 'admin_login', $2::inet)",
        )
        .bind(admin_id)
        .bind(ip_address.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

// ─── Fake 实现（内存，用于单元 / 集成测试）────────────────────────────────────

/// 登录日志条目（供测试断言使用）
#[derive(Debug, Clone)]
pub struct LoginLogEntry {
    pub admin_id: Uuid,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 内存版 AdminRepository，用于单元测试。
#[derive(Default)]
pub struct FakeAdminRepository {
    admins: Mutex<HashMap<Uuid, AdminModel>>,
    /// 记录每次 update_last_login_at 的调用结果，key=admin_id
    last_login_updates: Mutex<HashMap<Uuid, DateTime<Utc>>>,
}

impl FakeAdminRepository {
    /// 测试辅助：预置一条管理员记录。
    pub fn seed(&self, admin: AdminModel) {
        self.admins.lock().unwrap().insert(admin.id, admin);
    }

    /// 测试辅助：获取某管理员的最后登录更新时间。
    pub fn get_last_login_at(&self, id: Uuid) -> Option<DateTime<Utc>> {
        self.last_login_updates.lock().unwrap().get(&id).copied()
    }
}

#[async_trait]
impl AdminRepository for FakeAdminRepository {
    async fn find_by_username(&self, username: &str) -> Result<Option<AdminModel>, AppError> {
        let guard = self.admins.lock().unwrap();
        Ok(guard.values().find(|a| a.username == username).cloned())
    }

    async fn update_last_login_at(
        &self,
        admin_id: Uuid,
        time: DateTime<Utc>,
    ) -> Result<(), AppError> {
        self.last_login_updates
            .lock()
            .unwrap()
            .insert(admin_id, time);
        // 同步更新存储的 model
        if let Some(admin) = self.admins.lock().unwrap().get_mut(&admin_id) {
            admin.last_login_at = Some(time);
        }
        Ok(())
    }
}

/// 内存版 AdminLogRepository，用于单元测试。
#[derive(Default)]
pub struct FakeAdminLogRepository {
    logs: Mutex<Vec<LoginLogEntry>>,
}

impl FakeAdminLogRepository {
    /// 测试辅助：获取所有已记录的登录日志。
    pub fn get_logs(&self) -> Vec<LoginLogEntry> {
        self.logs.lock().unwrap().clone()
    }
}

#[async_trait]
impl AdminLogRepository for FakeAdminLogRepository {
    async fn insert_login_log(
        &self,
        admin_id: Uuid,
        ip_address: Option<String>,
    ) -> Result<(), AppError> {
        self.logs.lock().unwrap().push(LoginLogEntry {
            admin_id,
            ip_address,
            created_at: Utc::now(),
        });
        Ok(())
    }
}
