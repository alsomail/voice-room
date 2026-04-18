use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;
use voice_room_shared::models::user::UserModel;

use crate::common::error::AppError;

/// 用户持久化抽象，隔离真实 DB 与测试 Fake。
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_phone(&self, phone: &str) -> Result<Option<UserModel>, AppError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserModel>, AppError>;
    async fn create(&self, phone: &str, nickname: &str) -> Result<UserModel, AppError>;
}

// ─── Postgres 实现 ────────────────────────────────────────────────────────────

pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    async fn find_by_phone(&self, phone: &str) -> Result<Option<UserModel>, AppError> {
        let user = sqlx::query_as::<_, UserModel>(
            "SELECT id, phone, nickname, avatar, coin_balance, vip_level, is_banned, \
             created_at, updated_at, deleted_at \
             FROM users WHERE phone = $1 AND deleted_at IS NULL",
        )
        .bind(phone)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserModel>, AppError> {
        let user = sqlx::query_as::<_, UserModel>(
            "SELECT id, phone, nickname, avatar, coin_balance, vip_level, is_banned, \
             created_at, updated_at, deleted_at \
             FROM users WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    async fn create(&self, phone: &str, nickname: &str) -> Result<UserModel, AppError> {
        let user = sqlx::query_as::<_, UserModel>(
            "INSERT INTO users (phone, nickname) VALUES ($1, $2) \
             RETURNING id, phone, nickname, avatar, coin_balance, vip_level, is_banned, \
                       created_at, updated_at, deleted_at",
        )
        .bind(phone)
        .bind(nickname)
        .fetch_one(&self.pool)
        .await?;
        Ok(user)
    }
}

// ─── Fake 实现（内存，用于单元测试）─────────────────────────────────────────

#[derive(Default)]
pub struct FakeUserRepository {
    users: Mutex<HashMap<Uuid, UserModel>>,
}

impl FakeUserRepository {
    /// 测试辅助：预置一个用户
    pub fn seed(&self, user: UserModel) {
        self.users.lock().unwrap().insert(user.id, user);
    }
}

#[async_trait]
impl UserRepository for FakeUserRepository {
    async fn find_by_phone(&self, phone: &str) -> Result<Option<UserModel>, AppError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .values()
            .find(|u| u.phone == phone && u.deleted_at.is_none())
            .cloned())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserModel>, AppError> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .get(&id)
            .filter(|u| u.deleted_at.is_none())
            .cloned())
    }

    async fn create(&self, phone: &str, nickname: &str) -> Result<UserModel, AppError> {
        let now = Utc::now();
        let user = UserModel {
            id: Uuid::new_v4(),
            phone: phone.to_string(),
            nickname: nickname.to_string(),
            avatar: None,
            coin_balance: 0,
            vip_level: 0,
            is_banned: false,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        };
        self.users.lock().unwrap().insert(user.id, user.clone());
        Ok(user)
    }
}
