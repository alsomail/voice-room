//! TransferAdmin 信令处理 — T-00030
//!
//! ## 处理流程
//! 1. 解析 payload（room_id / target_user_id / action）
//! 2. 加载房间 Model → 权限校验（仅房主）
//! 3. assign：target != owner + DB 更新 admin_user_id + 广播 AdminChanged
//! 4. revoke：target == current_admin + DB 更新 NULL + 广播 AdminChanged
//!
//! ## 原子保障
//! DB 更新成功后才广播；任一步失败立即返回错误（不广播）。

use std::sync::Arc;
#[cfg(any(test, feature = "test-utils"))]
use std::{collections::HashMap, sync::Mutex};

use async_trait::async_trait;
use uuid::Uuid;

use crate::common::error::AppError;
use crate::modules::room::service::RoomService;
use crate::room::manager::RoomManager;
use crate::ws::registry::ConnectionRegistry;

// ─── TransferAdminRepo Trait ──────────────────────────────────────────────────

/// 管理员任命/撤销 DB 操作抽象。
///
/// 生产实现使用真实 Postgres；测试使用 `FakeTransferAdminRepo`（内存 HashMap）。
#[async_trait]
pub trait TransferAdminRepo: Send + Sync {
    /// 更新房间的 admin_user_id（None = 撤销管理员）。
    async fn set_admin_user_id(
        &self,
        room_id: Uuid,
        admin_user_id: Option<Uuid>,
    ) -> Result<(), AppError>;
}

/// Blanket impl：允许 `Arc<T: TransferAdminRepo>` 直接用作 `&dyn TransferAdminRepo`
#[async_trait]
impl<T: TransferAdminRepo + ?Sized> TransferAdminRepo for Arc<T> {
    async fn set_admin_user_id(
        &self,
        room_id: Uuid,
        admin_user_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        (**self).set_admin_user_id(room_id, admin_user_id).await
    }
}

// ─── FakeTransferAdminRepo（测试用内存实现）─────────────────────────────────

/// 内存 TransferAdmin DB（测试专用）。
#[cfg(any(test, feature = "test-utils"))]
pub struct FakeTransferAdminRepo {
    /// room_id → 最新的 admin_user_id
    data: Mutex<HashMap<Uuid, Option<Uuid>>>,
}

#[cfg(any(test, feature = "test-utils"))]
impl Default for FakeTransferAdminRepo {
    fn default() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl FakeTransferAdminRepo {
    /// 测试辅助：读取指定房间当前存储的 admin_user_id。
    ///
    /// 返回 `Some(Some(uuid))` 表示管理员被设为 uuid，
    /// 返回 `Some(None)` 表示管理员已被撤销（设为 NULL），
    /// 返回 `None` 表示该房间从未被写入。
    pub fn get_admin(&self, room_id: Uuid) -> Option<Option<Uuid>> {
        self.data.lock().unwrap().get(&room_id).cloned()
    }
}

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl TransferAdminRepo for FakeTransferAdminRepo {
    async fn set_admin_user_id(
        &self,
        room_id: Uuid,
        admin_user_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        self.data.lock().unwrap().insert(room_id, admin_user_id);
        Ok(())
    }
}

// ─── FailingTransferAdminRepo（测试用失败实现）──────────────────────────────

/// 总是返回 DB 错误的 TransferAdminRepo（用于原子性测试 TA30-14）。
#[cfg(any(test, feature = "test-utils"))]
pub struct FailingTransferAdminRepo;

#[cfg(any(test, feature = "test-utils"))]
#[async_trait]
impl TransferAdminRepo for FailingTransferAdminRepo {
    async fn set_admin_user_id(
        &self,
        _room_id: Uuid,
        _admin_user_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        Err(AppError::DatabaseError("simulated db failure".to_string()))
    }
}

// ─── RealTransferAdminRepo（生产 DB 实现）───────────────────────────────────

/// 生产 TransferAdmin DB 实现（基于 sqlx PgPool）。
///
/// 使用行锁（`FOR UPDATE`）保证并发安全；MVP 阶段用简单 UPDATE。
pub struct RealTransferAdminRepo {
    pool: sqlx::PgPool,
}

impl RealTransferAdminRepo {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TransferAdminRepo for RealTransferAdminRepo {
    async fn set_admin_user_id(
        &self,
        room_id: Uuid,
        admin_user_id: Option<Uuid>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE rooms SET admin_user_id = $1, updated_at = now() \
             WHERE id = $2 AND deleted_at IS NULL",
        )
        .bind(admin_user_id)
        .bind(room_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }
}

// ─── TransferAdminDeps ────────────────────────────────────────────────────────

/// `handle_transfer_admin` 所需的全部服务依赖。
pub struct TransferAdminDeps {
    /// 房间运行时状态管理器（成员表检查，预留未来扩展）
    pub room_manager: Arc<RoomManager>,
    /// 房间服务（权限校验：owner_id + admin_user_id）
    pub room_service: Arc<RoomService>,
    /// 管理员任命 DB 操作
    pub room_repo: Arc<dyn TransferAdminRepo>,
    /// WS 连接注册表（广播 AdminChanged）
    pub registry: Arc<ConnectionRegistry>,
}

// ─── 辅助函数 ─────────────────────────────────────────────────────────────────

fn transfer_error(msg_id: Option<String>, code: i64, message: &str) -> String {
    crate::ws::broadcaster::build_outbound_result(
        "TransferAdminResult",
        msg_id,
        code,
        Some(serde_json::json!({ "message": message })),
    )
}

fn transfer_success(msg_id: Option<String>) -> String {
    crate::ws::broadcaster::build_outbound_result("TransferAdminResult", msg_id, 0, None)
}

// ─── handle_transfer_admin ────────────────────────────────────────────────────

/// 处理 TransferAdmin 信令，返回 JSON 字符串响应。
///
/// ## 原子性保障
/// DB 更新成功后才广播；DB 失败时立即返回 50000，**不广播**。
pub async fn handle_transfer_admin(
    payload: Option<serde_json::Value>,
    msg_id: Option<String>,
    operator_user_id: Uuid,
    deps: &TransferAdminDeps,
) -> String {
    let TransferAdminDeps {
        room_manager,
        room_service,
        room_repo,
        registry,
    } = deps;

    // ── 1. 解析 payload ────────────────────────────────────────────────────────
    let room_id = match payload
        .as_ref()
        .and_then(|p| p.get("room_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return transfer_error(msg_id, 40002, "missing room_id"),
    };

    let target_user_id = match payload
        .as_ref()
        .and_then(|p| p.get("target_user_id"))
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return transfer_error(msg_id, 40002, "missing target_user_id"),
    };

    let action = match payload
        .as_ref()
        .and_then(|p| p.get("action"))
        .and_then(|v| v.as_str())
        .filter(|s| *s == "assign" || *s == "revoke")
    {
        Some(a) => a.to_string(),
        None => return transfer_error(msg_id, 40002, "action must be 'assign' or 'revoke'"),
    };

    // ── 2. 加载房间 Model ──────────────────────────────────────────────────────
    let room = match room_service.get_active_room_model(room_id).await {
        Ok(Some(r)) => r,
        Ok(None) => return transfer_error(msg_id, 40400, "room not found or closed"),
        Err(e) => {
            tracing::error!("get_active_room_model error: {e}");
            return transfer_error(msg_id, 50000, "internal error");
        }
    };

    // ── 3. 权限校验：仅房主可操作 ──────────────────────────────────────────────
    if room.owner_id != operator_user_id {
        return transfer_error(
            msg_id,
            40301,
            "permission denied: only owner can transfer admin",
        );
    }

    let previous_admin_id = room.admin_user_id;

    if action == "assign" {
        // ── 4a. assign 分支 ────────────────────────────────────────────────────

        // target 不能是房主
        if target_user_id == room.owner_id {
            return transfer_error(msg_id, 40302, "cannot assign owner as admin");
        }

        // ── 5a. DB 原子更新 admin_user_id ─────────────────────────────────────
        if let Err(e) = room_repo
            .set_admin_user_id(room_id, Some(target_user_id))
            .await
        {
            tracing::error!("set_admin_user_id failed: {e}");
            return transfer_error(msg_id, 50000, "internal error");
        }

        // ── 6a. DB 成功后广播 AdminChanged — 走统一出口 broadcast_to_room ─────
        let admin_changed_envelope = serde_json::json!({
            "type": "AdminChanged",
            "payload": {
                "room_id": room_id.to_string(),
                "admin_user_id": target_user_id.to_string(),
                "previous_admin_id": previous_admin_id.map(|id| id.to_string()),
                "operator_id": operator_user_id.to_string(),
            },
            "timestamp": chrono::Utc::now().timestamp(),
        });
        if let Some(rs) = room_manager.get_room(room_id) {
            crate::ws::broadcaster::broadcast_to_room(registry, &rs, admin_changed_envelope);
        } else {
            // 房间状态尚未注册到内存（e.g. 治理 API 在 JoinRoom 之前触发）— 降级广播，
            // 不写 recent_broadcasts 缓冲（无法被 last_msg_id 续传，但保证连接收到事件）
            crate::ws::broadcaster::broadcast_to_room_no_state(
                registry,
                room_id,
                admin_changed_envelope,
            );
        }
    } else {
        // action == "revoke"

        // ── 4b. target 必须是当前管理员 ──────────────────────────────────────
        if previous_admin_id != Some(target_user_id) {
            return transfer_error(msg_id, 40404, "target is not current admin");
        }

        // ── 5b. DB 原子更新 admin_user_id = NULL ──────────────────────────────
        if let Err(e) = room_repo.set_admin_user_id(room_id, None).await {
            tracing::error!("set_admin_user_id failed: {e}");
            return transfer_error(msg_id, 50000, "internal error");
        }

        // ── 6b. DB 成功后广播 AdminChanged — 走统一出口 broadcast_to_room ─────
        let admin_changed_envelope = serde_json::json!({
            "type": "AdminChanged",
            "payload": {
                "room_id": room_id.to_string(),
                "admin_user_id": null,
                "previous_admin_id": previous_admin_id.map(|id| id.to_string()),
                "operator_id": operator_user_id.to_string(),
            },
            "timestamp": chrono::Utc::now().timestamp(),
        });
        if let Some(rs) = room_manager.get_room(room_id) {
            crate::ws::broadcaster::broadcast_to_room(registry, &rs, admin_changed_envelope);
        } else {
            crate::ws::broadcaster::broadcast_to_room_no_state(
                registry,
                room_id,
                admin_changed_envelope,
            );
        }
    }

    transfer_success(msg_id)
}
