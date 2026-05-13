use uuid::Uuid;

use crate::common::error::AppError;

/// RBAC 权限枚举。
/// 对应 doc/protocol.md §3.3 权限矩阵的五个维度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    // 用户管理
    UserRead,
    UserWrite,
    // 房间管理
    RoomRead,
    RoomWrite,
    // 数据统计
    StatsRead,
    // 财务操作
    FinanceRead,
    FinanceWrite,
    // 系统管理
    SystemAdmin,
    // 房间强制关闭（T-10006）
    RoomForceClose,
    // 审计日志（T-10012）
    LogRead,
    // 钱包手动调整（T-10013）
    WalletAdjust,
    // 礼物管理读写（T-10014）：super_admin + operator
    GiftWrite,
    // 礼物软删除（T-10014）：仅 super_admin
    GiftDelete,
    // 治理日志查询（T-10016）：super_admin / operator / cs 可查；finance 禁止
    GovernanceRead,
    // 支付订单查询（T-10025）：super_admin / operator / finance
    PaymentRead,
    // 支付写操作（T-10026/27）：super_admin / operator
    PaymentWrite,
    // 支付报表（T-10028）：super_admin / finance
    PaymentReport,
    // 贵族 tier 只读（T-10030/32）：super_admin / operator
    NobleTierRead,
    // 贵族 tier 写操作（T-10030）：仅 super_admin
    NobleTierWrite,
}

/// 已鉴权的管理员上下文，由 `AdminAuthContext::from_request_parts` 注入。
///
/// 包含 admin_id 和 role，后续 handler 通过 `require_permission` 做 RBAC 校验。
#[derive(Clone, Debug)]
pub struct AdminAuthContext {
    pub admin_id: Uuid,
    pub role: String,
}

impl AdminAuthContext {
    pub fn new(admin_id: Uuid, role: impl Into<String>) -> Self {
        Self {
            admin_id,
            role: role.into(),
        }
    }

    /// 检查当前角色是否拥有指定权限。
    ///
    /// RBAC 矩阵（doc/protocol.md §3.3）：
    ///
    /// | 角色 | 用户管理 | 房间管理 | 数据统计 | 财务操作 | 系统管理 |
    /// |---------|---------|---------|---------|---------|---------|
    /// | super_admin | 读写 | 读写 | ✅ | ✅ | ✅ |
    /// | operator    | 读写 | 读写 | ✅ | ❌ | ❌ |
    /// | cs          | 只读 | 只读 | ❌ | ❌ | ❌ |
    /// | finance     |  ❌  |  ❌  | ✅ | ✅ | ❌ |
    pub fn has_permission(&self, permission: Permission) -> bool {
        match self.role.as_str() {
            "super_admin" => true,
            "operator" => matches!(
                permission,
                Permission::UserRead
                    | Permission::UserWrite
                    | Permission::RoomRead
                    | Permission::RoomWrite
                    | Permission::StatsRead
                    | Permission::RoomForceClose
                    | Permission::LogRead
                    | Permission::WalletAdjust
                    | Permission::GiftWrite
                    | Permission::GovernanceRead
                    | Permission::PaymentRead
                    | Permission::PaymentWrite
                    | Permission::NobleTierRead
            ),
            "cs" => matches!(
                permission,
                Permission::UserRead | Permission::RoomRead | Permission::GovernanceRead
            ),
            "finance" => matches!(
                permission,
                Permission::StatsRead
                    | Permission::FinanceRead
                    | Permission::FinanceWrite
                    | Permission::WalletAdjust
                    | Permission::PaymentRead
                    | Permission::PaymentReport
            ),
            _ => false,
        }
    }

    /// 断言当前角色有指定权限，权限不足时返回 `AppError::Forbidden`（HTTP 403 / 40301）。
    pub fn require_permission(&self, permission: Permission) -> Result<(), AppError> {
        if self.has_permission(permission) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    /// 断言当前角色等于指定角色（如 "super_admin"），不匹配时返回 `AppError::Forbidden`。
    /// 用于 T-10026 补单/退款 和 T-10031 贵族手动操作等仅 super_admin 可执行的操作。
    pub fn require_role(&self, role: &str) -> Result<(), AppError> {
        if self.role == role {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}

// ─── 单元测试（TDD T-10003 验收用例）────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(role: &str) -> AdminAuthContext {
        AdminAuthContext::new(Uuid::new_v4(), role)
    }

    // ── super_admin ─────────────────────────────────────────────────────────

    /// T-10003-R01: super_admin 拥有全部权限
    #[test]
    fn super_admin_has_all_permissions() {
        let c = ctx("super_admin");
        for p in [
            Permission::UserRead,
            Permission::UserWrite,
            Permission::RoomRead,
            Permission::RoomWrite,
            Permission::StatsRead,
            Permission::FinanceRead,
            Permission::FinanceWrite,
            Permission::SystemAdmin,
        ] {
            assert!(
                c.has_permission(p),
                "super_admin should have permission {p:?}"
            );
        }
    }

    // ── operator ────────────────────────────────────────────────────────────

    /// T-10003-R02: operator 有用户/房间/统计权限
    #[test]
    fn operator_has_user_room_stats_permissions() {
        let c = ctx("operator");
        for p in [
            Permission::UserRead,
            Permission::UserWrite,
            Permission::RoomRead,
            Permission::RoomWrite,
            Permission::StatsRead,
        ] {
            assert!(c.has_permission(p), "operator should have permission {p:?}");
        }
    }

    /// T-10003-R03: operator 无法访问财务接口
    #[test]
    fn operator_lacks_finance_permissions() {
        let c = ctx("operator");
        for p in [Permission::FinanceRead, Permission::FinanceWrite] {
            assert!(
                !c.has_permission(p),
                "operator must NOT have permission {p:?}"
            );
        }
    }

    /// T-10003-R04: operator 无系统管理权限
    #[test]
    fn operator_lacks_system_admin() {
        let c = ctx("operator");
        assert!(!c.has_permission(Permission::SystemAdmin));
    }

    // ── cs ──────────────────────────────────────────────────────────────────

    /// T-10003-R05: cs 只有用户只读 + 房间只读权限
    #[test]
    fn cs_has_user_read_and_room_read_permissions() {
        let c = ctx("cs");
        for p in [Permission::UserRead, Permission::RoomRead] {
            assert!(c.has_permission(p), "cs should have permission {p:?}");
        }
    }

    /// T-10003-R06: cs 无法执行用户写操作（如封禁）
    #[test]
    fn cs_lacks_user_write() {
        let c = ctx("cs");
        assert!(!c.has_permission(Permission::UserWrite));
    }

    /// T-10003-R07: cs 无房间写入/统计/财务/系统权限
    #[test]
    fn cs_lacks_room_write_stats_finance_system() {
        let c = ctx("cs");
        for p in [
            Permission::RoomWrite,
            Permission::StatsRead,
            Permission::FinanceRead,
            Permission::FinanceWrite,
            Permission::SystemAdmin,
        ] {
            assert!(!c.has_permission(p), "cs must NOT have permission {p:?}");
        }
    }

    // ── finance ─────────────────────────────────────────────────────────────

    /// T-10003-R08: finance 有统计和财务权限
    #[test]
    fn finance_has_stats_and_finance_permissions() {
        let c = ctx("finance");
        for p in [
            Permission::StatsRead,
            Permission::FinanceRead,
            Permission::FinanceWrite,
        ] {
            assert!(c.has_permission(p), "finance should have permission {p:?}");
        }
    }

    /// T-10003-R09: finance 无用户/房间/系统管理权限
    #[test]
    fn finance_lacks_user_room_system_permissions() {
        let c = ctx("finance");
        for p in [
            Permission::UserRead,
            Permission::UserWrite,
            Permission::RoomRead,
            Permission::RoomWrite,
            Permission::SystemAdmin,
        ] {
            assert!(
                !c.has_permission(p),
                "finance must NOT have permission {p:?}"
            );
        }
    }

    // ── 未知角色 ──────────────────────────────────────────────────────────

    /// T-10003-R10: 未知角色没有任何权限
    #[test]
    fn unknown_role_has_no_permissions() {
        let c = ctx("god");
        for p in [
            Permission::UserRead,
            Permission::UserWrite,
            Permission::RoomRead,
            Permission::RoomWrite,
            Permission::StatsRead,
            Permission::FinanceRead,
            Permission::FinanceWrite,
            Permission::SystemAdmin,
            Permission::RoomForceClose,
            Permission::LogRead,
        ] {
            assert!(
                !c.has_permission(p),
                "unknown role must NOT have permission {p:?}"
            );
        }
    }

    // ── require_permission ────────────────────────────────────────────────

    /// T-10003-R11: 有权限时 require_permission 返回 Ok
    #[test]
    fn require_permission_ok_when_allowed() {
        let c = ctx("operator");
        assert!(c.require_permission(Permission::UserWrite).is_ok());
    }

    /// T-10003-R12: 无权限时 require_permission 返回 Err(Forbidden)
    #[test]
    fn require_permission_err_when_denied() {
        let c = ctx("operator");
        let err = c.require_permission(Permission::FinanceRead).unwrap_err();
        assert!(matches!(err, AppError::Forbidden));
    }

    // ── T-10006 新增：RoomForceClose 权限矩阵验收 ─────────────────────────────

    /// T-10006-P01: super_admin 拥有 RoomForceClose 权限
    #[test]
    fn t10006_p01_super_admin_has_room_force_close() {
        let c = ctx("super_admin");
        assert!(
            c.has_permission(Permission::RoomForceClose),
            "super_admin 必须拥有 RoomForceClose 权限"
        );
    }

    /// T-10006-P02: operator 拥有 RoomForceClose 权限
    #[test]
    fn t10006_p02_operator_has_room_force_close() {
        let c = ctx("operator");
        assert!(
            c.has_permission(Permission::RoomForceClose),
            "operator 必须拥有 RoomForceClose 权限"
        );
    }

    /// T-10006-P03: cs 角色无 RoomForceClose 权限
    #[test]
    fn t10006_p03_cs_lacks_room_force_close() {
        let c = ctx("cs");
        assert!(
            !c.has_permission(Permission::RoomForceClose),
            "cs 不能拥有 RoomForceClose 权限"
        );
    }

    /// T-10006-P04: finance 角色无 RoomForceClose 权限
    #[test]
    fn t10006_p04_finance_lacks_room_force_close() {
        let c = ctx("finance");
        assert!(
            !c.has_permission(Permission::RoomForceClose),
            "finance 不能拥有 RoomForceClose 权限"
        );
    }

    // ── T-10012 新增：LogRead 权限矩阵验收 ───────────────────────────────────

    /// T-10012-L01: super_admin 拥有 LogRead 权限
    #[test]
    fn t10012_l01_super_admin_has_log_read() {
        let c = ctx("super_admin");
        assert!(
            c.has_permission(Permission::LogRead),
            "super_admin 必须拥有 LogRead 权限"
        );
    }

    /// T-10012-L02: operator 拥有 LogRead 权限
    #[test]
    fn t10012_l02_operator_has_log_read() {
        let c = ctx("operator");
        assert!(
            c.has_permission(Permission::LogRead),
            "operator 必须拥有 LogRead 权限"
        );
    }

    /// T-10012-L03: cs 角色无 LogRead 权限
    #[test]
    fn t10012_l03_cs_lacks_log_read() {
        let c = ctx("cs");
        assert!(
            !c.has_permission(Permission::LogRead),
            "cs 不能拥有 LogRead 权限"
        );
    }

    /// T-10012-L04: finance 角色无 LogRead 权限
    #[test]
    fn t10012_l04_finance_lacks_log_read() {
        let c = ctx("finance");
        assert!(
            !c.has_permission(Permission::LogRead),
            "finance 不能拥有 LogRead 权限"
        );
    }

    /// T-10012-L05: unknown role 无 LogRead 权限
    #[test]
    fn t10012_l05_unknown_role_lacks_log_read() {
        let c = ctx("god");
        assert!(
            !c.has_permission(Permission::LogRead),
            "未知角色不能拥有 LogRead 权限"
        );
    }
}
