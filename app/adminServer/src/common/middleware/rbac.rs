//! RBAC 辅助模块，配合 `AdminAuthContext::require_permission` 使用。
//!
//! 典型用法（在 handler 中）：
//! ```rust,ignore
//! use crate::common::auth::Permission;
//!
//! async fn ban_user_handler(ctx: AdminAuthContext, ...) -> impl IntoResponse {
//!     ctx.require_permission(Permission::UserWrite)?;
//!     // ... 业务逻辑
//! }
//! ```
//!
//! 文件预留：后续可在此添加路由级别的 RBAC 中间件 Layer。
//!
//! 目前 RBAC 逻辑已内置于 `AdminAuthContext::has_permission` / `require_permission`。
//! 参见 `src/common/auth/context.rs`。
