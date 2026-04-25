use async_trait::async_trait;
#[cfg(any(test, feature = "test-utils"))]
use chrono::Utc;
use sqlx::PgPool;
#[cfg(any(test, feature = "test-utils"))]
use std::{collections::HashMap, sync::Mutex};
use uuid::Uuid;
use voice_room_shared::models::user::UserModel;

use crate::common::error::AppError;

/// 用户持久化抽象，隔离真实 DB 与测试 Fake。
#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_phone(&self, phone: &str) -> Result<Option<UserModel>, AppError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<UserModel>, AppError>;
    /// 批量按 ID 查询用户（T-00027 N+1 修复）。
    ///
    /// 实现须使用 `WHERE id = ANY($1) AND deleted_at IS NULL` 单次 SQL，
    /// 返回的顺序不保证与 `ids` 一致，上层需自行建索引。
    async fn find_by_ids(&self, ids: &[Uuid]) -> Result<Vec<UserModel>, AppError>;
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
            "SELECT id, phone, nickname, avatar, coin_balance, diamond_balance, vip_level, is_banned, \
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
            "SELECT id, phone, nickname, avatar, coin_balance, diamond_balance, vip_level, is_banned, \
             created_at, updated_at, deleted_at \
             FROM users WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    async fn find_by_ids(&self, ids: &[Uuid]) -> Result<Vec<UserModel>, AppError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let users = sqlx::query_as::<_, UserModel>(
            "SELECT id, phone, nickname, avatar, coin_balance, diamond_balance, vip_level, is_banned, \
             created_at, updated_at, deleted_at \
             FROM users WHERE id = ANY($1) AND deleted_at IS NULL",
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;
        Ok(users)
    }

    async fn create(&self, phone: &str, nickname: &str) -> Result<UserModel, AppError> {
        let user = sqlx::query_as::<_, UserModel>(
            "INSERT INTO users (phone, nickname) VALUES ($1, $2) \
             RETURNING id, phone, nickname, avatar, coin_balance, diamond_balance, vip_level, is_banned, \
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

#[cfg(any(test, feature = "test-utils"))]
#[derive(Default)]
pub struct FakeUserRepository {
    users: Mutex<HashMap<Uuid, UserModel>>,
}

#[cfg(any(test, feature = "test-utils"))]
impl FakeUserRepository {
    /// 测试辅助：预置一个用户
    pub fn seed(&self, user: UserModel) {
        self.users.lock().unwrap().insert(user.id, user);
    }
}

#[cfg(any(test, feature = "test-utils"))]
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

    async fn find_by_ids(&self, ids: &[Uuid]) -> Result<Vec<UserModel>, AppError> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let guard = self.users.lock().unwrap();
        let result = ids
            .iter()
            .filter_map(|id| guard.get(id).filter(|u| u.deleted_at.is_none()).cloned())
            .collect();
        Ok(result)
    }

    async fn create(&self, phone: &str, nickname: &str) -> Result<UserModel, AppError> {
        let now = Utc::now();
        let user = UserModel {
            id: Uuid::new_v4(),
            phone: phone.to_string(),
            nickname: nickname.to_string(),
            avatar: None,
            coin_balance: 0,
            diamond_balance: 0,
            charm_balance: 0,
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

// ─── Failing Fake 实现（测试辅助：模拟 DB 错误）────────────────────────────────

/// 所有方法均返回 `AppError::Internal`，用于注入 DB 错误场景的单元测试。
#[cfg(any(test, feature = "test-utils"))]
#[derive(Default)]
pub struct FailingUserRepository;

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl UserRepository for FailingUserRepository {
    async fn find_by_phone(&self, _phone: &str) -> Result<Option<UserModel>, AppError> {
        Err(AppError::Internal("simulated db error".into()))
    }

    async fn find_by_id(&self, _id: Uuid) -> Result<Option<UserModel>, AppError> {
        Err(AppError::Internal("simulated db error".into()))
    }

    async fn find_by_ids(&self, _ids: &[Uuid]) -> Result<Vec<UserModel>, AppError> {
        Err(AppError::Internal("simulated db error".into()))
    }

    async fn create(&self, _phone: &str, _nickname: &str) -> Result<UserModel, AppError> {
        Err(AppError::Internal("simulated db error".into()))
    }
}

// ─── 单元测试：find_by_ids 批量语义 ─────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_user(phone: &str) -> UserModel {
        let now = Utc::now();
        UserModel {
            id: Uuid::new_v4(),
            phone: phone.to_string(),
            nickname: format!("User_{phone}"),
            avatar: None,
            coin_balance: 0,
            diamond_balance: 0,
            charm_balance: 0,
            vip_level: 0,
            is_banned: false,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    /// R01: find_by_ids 空切片 → 立即返回空 Vec（不触碰 HashMap）
    #[tokio::test]
    async fn r01_find_by_ids_empty_input_returns_empty() {
        let repo = FakeUserRepository::default();
        let result = repo.find_by_ids(&[]).await.unwrap();
        assert!(result.is_empty(), "R01: empty ids must return empty Vec");
    }

    /// R02: find_by_ids 返回所有请求 ID 的用户信息（批量语义）
    #[tokio::test]
    async fn r02_find_by_ids_returns_all_matching_users() {
        let repo = FakeUserRepository::default();
        let u1 = make_user("+8611111111111");
        let u2 = make_user("+8622222222222");
        let u3 = make_user("+8633333333333");
        let id1 = u1.id;
        let id2 = u2.id;
        repo.seed(u1);
        repo.seed(u2);
        repo.seed(u3.clone());

        let result = repo.find_by_ids(&[id1, id2]).await.unwrap();
        assert_eq!(result.len(), 2, "R02: must return exactly 2 users");
        let ids: Vec<Uuid> = result.iter().map(|u| u.id).collect();
        assert!(ids.contains(&id1), "R02: must contain id1");
        assert!(ids.contains(&id2), "R02: must contain id2");
    }

    /// R03: find_by_ids 不包含不存在的 ID（过滤语义）
    #[tokio::test]
    async fn r03_find_by_ids_skips_nonexistent_ids() {
        let repo = FakeUserRepository::default();
        let u1 = make_user("+8611111111111");
        let id1 = u1.id;
        let ghost_id = Uuid::new_v4(); // 不存在
        repo.seed(u1);

        let result = repo.find_by_ids(&[id1, ghost_id]).await.unwrap();
        assert_eq!(result.len(), 1, "R03: ghost ID must not appear in result");
        assert_eq!(result[0].id, id1, "R03: only the existing user must appear");
    }

    /// R04: find_by_ids 不返回软删除（deleted_at IS NOT NULL）的用户
    #[tokio::test]
    async fn r04_find_by_ids_excludes_soft_deleted_users() {
        let repo = FakeUserRepository::default();
        let mut deleted_user = make_user("+8644444444444");
        deleted_user.deleted_at = Some(Utc::now());
        let deleted_id = deleted_user.id;
        repo.seed(deleted_user);

        let result = repo.find_by_ids(&[deleted_id]).await.unwrap();
        assert!(
            result.is_empty(),
            "R04: soft-deleted user must not be returned"
        );
    }

    /// R05: FailingUserRepository.find_by_ids 模拟 DB 错误
    #[tokio::test]
    async fn r05_failing_repo_find_by_ids_returns_error() {
        let repo = FailingUserRepository::default();
        let err = repo.find_by_ids(&[Uuid::new_v4()]).await.unwrap_err();
        assert!(
            matches!(err, AppError::Internal(_)),
            "R05: failing repo must return AppError::Internal"
        );
    }
}
