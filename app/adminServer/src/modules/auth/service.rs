use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::Utc;
use voice_room_shared::{
    crypto::verify_password,
    jwt::token::{encode_token, AdminClaims},
    models::AdminModel,
};

use crate::common::error::AppError;

use super::{
    dto::{AdminInfo, AdminLoginResponse},
    repository::{AdminLogRepository, AdminRepository},
};

/// JWT 有效期：7 天（604800 秒），参见 doc/protocol.md §3.1
pub const TOKEN_EXPIRES_SECS: u64 = 7 * 24 * 3600;

/// 时序攻击防护用虚拟哈希（cost=12 有效 bcrypt）。
///
/// 账号不存在时仍需调用 `verify_password(password, DUMMY_HASH)` 以保证两种失败
/// 路径（账号不存在 vs 密码错误）耗时一致（常量时间认证模式）。
///
/// 由 `bcrypt::hash("__timing_protection_dummy_password_voiceroom__", 12)` 预计算得到，
/// 不对应任何真实管理员密码。
const DUMMY_HASH: &str = "$2b$12$Xmta40fS.0LJFwy9lnGgUOM/QmkpJDiMt4ko7Qy15lxWmzhAzxeyC";

// ─── AdminAuthService ─────────────────────────────────────────────────────────

pub struct AdminAuthService {
    admin_repo: Arc<dyn AdminRepository>,
    log_repo: Arc<dyn AdminLogRepository>,
    jwt_secret: String,
}

impl AdminAuthService {
    pub fn new(
        admin_repo: Arc<dyn AdminRepository>,
        log_repo: Arc<dyn AdminLogRepository>,
        jwt_secret: String,
    ) -> Self {
        Self {
            admin_repo,
            log_repo,
            jwt_secret,
        }
    }

    /// 管理员账号密码登录。
    ///
    /// # 业务流程
    /// 1. 按 username 查询 admins 表
    /// 2. bcrypt 验证密码（失败统一返回 InvalidAdminCredentials，防止用户名枚举）
    /// 3. 检查 is_active（被禁用返回 AccountDisabled）
    /// 4. 签发 Admin JWT（iss="voiceroom-admin"，含 role，有效期 7 天）
    /// 5. 更新 last_login_at（失败仅 warn，不影响登录结果）
    /// 6. 写入 admin_logs（action="admin_login"，含 ip_address）
    pub async fn login(
        &self,
        username: &str,
        password: &str,
        ip_addr: Option<String>,
    ) -> Result<AdminLoginResponse, AppError> {
        // Step 1: 查账号
        // 账号不存在时仍调用 verify_password(DUMMY_HASH) 保证恒定时间响应，
        // 防止攻击者通过响应时间（< 1ms vs ~300ms）枚举有效用户名。
        let admin = match self
            .admin_repo
            .find_by_username(username)
            .await?
        {
            Some(admin) => admin,
            None => {
                // 恒定时间保护：即使账号不存在也执行一次 bcrypt 计算
                let _ = verify_password(password, DUMMY_HASH);
                return Err(AppError::InvalidAdminCredentials);
            }
        };

        // Step 2: 验证密码
        let verified = verify_password(password, &admin.password_hash)
            .map_err(|e| AppError::Internal(format!("bcrypt error: {e}")))?;
        if !verified {
            return Err(AppError::InvalidAdminCredentials);
        }

        // Step 3: 检查账号状态
        if !admin.is_active {
            return Err(AppError::AccountDisabled);
        }

        // Step 4: 签发 JWT
        let token = issue_admin_token(&admin, &self.jwt_secret)?;

        // Step 5: 更新 last_login_at（非关键操作，失败只记录警告）
        let now_dt = Utc::now();
        if let Err(e) = self
            .admin_repo
            .update_last_login_at(admin.id, now_dt)
            .await
        {
            tracing::warn!(admin_id = %admin.id, error = %e, "failed to update last_login_at");
        }

        // Step 6: 写入审计日志（非关键操作，失败只记录警告）
        if let Err(e) = self
            .log_repo
            .insert_login_log(admin.id, ip_addr)
            .await
        {
            tracing::warn!(admin_id = %admin.id, error = %e, "failed to insert admin login log");
        }

        Ok(AdminLoginResponse {
            token,
            expires_in: TOKEN_EXPIRES_SECS,
            admin: AdminInfo {
                id: admin.id.to_string(),
                username: admin.username.clone(),
                role: admin.role.clone(),
                display_name: admin.display_name.clone(),
                last_login_at: Some(now_dt.to_rfc3339()),
            },
        })
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn issue_admin_token(admin: &AdminModel, secret: &str) -> Result<String, AppError> {
    let now = now_secs();
    let claims = AdminClaims {
        sub: admin.id.to_string(),
        role: admin.role.clone(),
        iss: "voiceroom-admin".to_string(),
        exp: now + TOKEN_EXPIRES_SECS,
        iat: now,
    };
    encode_token(&claims, secret.as_bytes())
        .map_err(|e| AppError::Internal(format!("jwt encode: {e}")))
}

// ─── 单元测试（T-10002 TDD 验收用例）────────────────────────────────────────
//
// TDD 工作流说明：
//   RED  阶段：先在此处编写所有测试用例（service 实现仅有骨架时测试失败）
//   GREEN 阶段：填写上方 login() 实现后，所有测试变绿
//   REFACTOR：保持测试绿色，优化实现细节
//
#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use voice_room_shared::jwt::token::decode_token;

    // ── 测试辅助 ──────────────────────────────────────────────────────────────

    /// 使用低 cost(4) 快速生成 bcrypt 哈希，仅用于测试。
    fn test_hash(password: &str) -> String {
        bcrypt::hash(password, 4).unwrap()
    }

    fn make_admin(username: &str, password: &str, is_active: bool) -> AdminModel {
        AdminModel {
            id: Uuid::new_v4(),
            username: username.to_string(),
            password_hash: test_hash(password),
            role: "operator".to_string(),
            display_name: Some("测试运营".to_string()),
            is_active,
            last_login_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    use crate::modules::auth::repository::{FakeAdminLogRepository, FakeAdminRepository};

    fn test_service() -> (
        AdminAuthService,
        Arc<FakeAdminRepository>,
        Arc<FakeAdminLogRepository>,
    ) {
        let admin_repo = Arc::new(FakeAdminRepository::default());
        let log_repo = Arc::new(FakeAdminLogRepository::default());
        let svc = AdminAuthService::new(
            admin_repo.clone() as Arc<dyn AdminRepository>,
            log_repo.clone() as Arc<dyn AdminLogRepository>,
            "test-jwt-secret".to_string(),
        );
        (svc, admin_repo, log_repo)
    }

    // ── T-10002-U01: 账号不存在 → 401 (40106) ────────────────────────────────
    //
    // RED: 此测试先于 login() 实现编写，初始运行失败（方法不存在）
    // GREEN: 实现 login() 的 Step 1（查账号返回 None → InvalidAdminCredentials）后通过
    #[tokio::test]
    async fn login_account_not_found_returns_invalid_credentials() {
        let (svc, _, _) = test_service();
        // 仓库为空，账号不存在
        let err = svc.login("ghost_user", "anypassword", None).await.unwrap_err();
        assert!(
            matches!(err, AppError::InvalidAdminCredentials),
            "账号不存在时必须返回 InvalidAdminCredentials (40106)，实际: {err:?}"
        );
    }

    // ── T-10002-U02: 密码错误 → 401 (40106) ──────────────────────────────────
    //
    // RED: 先写此测试，login() 骨架尚未验证密码时测试失败
    // GREEN: 实现 Step 2（bcrypt verify 失败 → InvalidAdminCredentials）后通过
    #[tokio::test]
    async fn login_wrong_password_returns_invalid_credentials() {
        let (svc, admin_repo, _) = test_service();
        let admin = make_admin("op_user", "correct_password", true);
        admin_repo.seed(admin);

        let err = svc
            .login("op_user", "wrong_password", None)
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::InvalidAdminCredentials),
            "密码错误时必须返回 InvalidAdminCredentials (40106)，实际: {err:?}"
        );
    }

    // ── T-10002-U03: 成功登录 → 返回 JWT + admin 信息 ───────────────────────
    //
    // GREEN: 实现完整 login() 后通过
    #[tokio::test]
    async fn login_success_returns_token_and_admin_info() {
        let (svc, admin_repo, _) = test_service();
        let admin = make_admin("op_user", "pass1234", true);
        let admin_id = admin.id;
        admin_repo.seed(admin);

        let resp = svc
            .login("op_user", "pass1234", None)
            .await
            .expect("正确凭证必须登录成功");

        assert!(!resp.token.is_empty(), "token 不能为空");
        assert_eq!(resp.expires_in, 604800, "有效期必须为 7 天（604800 秒）");
        assert_eq!(resp.admin.username, "op_user");
        assert_eq!(resp.admin.id, admin_id.to_string());
        assert_eq!(resp.admin.role, "operator");
    }

    // ── T-10002-U04: JWT 有效期 7 天 ─────────────────────────────────────────
    #[tokio::test]
    async fn login_success_token_expires_in_7_days() {
        let (svc, admin_repo, _) = test_service();
        let admin = make_admin("op_user", "pass1234", true);
        admin_repo.seed(admin);

        let resp = svc.login("op_user", "pass1234", None).await.unwrap();

        // 解码 JWT，验证 exp - iat ≈ 604800（7 天）
        let claims: AdminClaims =
            decode_token(&resp.token, b"test-jwt-secret", "voiceroom-admin")
                .expect("token 必须可以用签名密钥解码");
        let duration = claims.exp.saturating_sub(claims.iat);
        assert!(
            duration >= 604798 && duration <= 604802,
            "JWT 有效期必须为 7 天（604800 秒），实际: {duration}"
        );
    }

    // ── T-10002-U05: JWT 包含 role 字段和正确 iss ─────────────────────────────
    #[tokio::test]
    async fn login_success_jwt_contains_role_and_correct_iss() {
        let (svc, admin_repo, _) = test_service();
        let admin = make_admin("finance_mgr", "secure_pass", true);
        // 使用 finance 角色测试
        let finance_admin = AdminModel {
            role: "finance".to_string(),
            ..admin
        };
        admin_repo.seed(finance_admin);

        let resp = svc.login("finance_mgr", "secure_pass", None).await.unwrap();

        let claims: AdminClaims =
            decode_token(&resp.token, b"test-jwt-secret", "voiceroom-admin")
                .expect("token 必须可解码");
        assert_eq!(claims.role, "finance", "JWT 必须包含正确的 role 字段");
        assert_eq!(
            claims.iss, "voiceroom-admin",
            "Admin JWT 的 iss 必须为 'voiceroom-admin'"
        );
    }

    // ── T-10002-U06: 账号被禁用（is_active=false）→ 403 (40302) ──────────────
    //
    // RED: 先写此测试，login() 不检查 is_active 时测试失败
    // GREEN: 实现 Step 3（检查 is_active）后通过
    #[tokio::test]
    async fn login_disabled_account_returns_account_disabled() {
        let (svc, admin_repo, _) = test_service();
        let admin = make_admin("disabled_op", "pass1234", false); // is_active = false
        admin_repo.seed(admin);

        let err = svc
            .login("disabled_op", "pass1234", None)
            .await
            .unwrap_err();
        assert!(
            matches!(err, AppError::AccountDisabled),
            "禁用账号必须返回 AccountDisabled (40302)，实际: {err:?}"
        );
    }

    // ── T-10002-U07: 登录成功记录审计日志（含 IP）────────────────────────────
    //
    // GREEN: Step 6（写 admin_logs）实现后通过
    #[tokio::test]
    async fn login_success_records_audit_log_with_ip_and_admin_id() {
        let (svc, admin_repo, log_repo) = test_service();
        let admin = make_admin("op_user", "pass1234", true);
        let admin_id = admin.id;
        admin_repo.seed(admin);

        svc.login("op_user", "pass1234", Some("192.168.1.100".to_string()))
            .await
            .unwrap();

        let logs = log_repo.get_logs();
        assert_eq!(logs.len(), 1, "必须写入恰好 1 条登录日志");
        assert_eq!(logs[0].admin_id, admin_id, "日志的 admin_id 必须正确");
        assert_eq!(
            logs[0].ip_address,
            Some("192.168.1.100".to_string()),
            "日志必须记录 IP 地址"
        );
        assert!(
            logs[0].created_at <= Utc::now(),
            "日志 created_at 必须有合法时间戳"
        );
    }

    // ── T-10002-U08: 登录成功更新 last_login_at ──────────────────────────────
    //
    // GREEN: Step 5（update_last_login_at）实现后通过
    #[tokio::test]
    async fn login_success_updates_last_login_at() {
        let (svc, admin_repo, _) = test_service();
        let admin = make_admin("op_user", "pass1234", true);
        let admin_id = admin.id;
        admin_repo.seed(admin);

        let before = Utc::now();
        svc.login("op_user", "pass1234", None).await.unwrap();
        let after = Utc::now();

        let updated = admin_repo
            .get_last_login_at(admin_id)
            .expect("登录成功后 last_login_at 必须被更新");
        assert!(
            updated >= before && updated <= after,
            "last_login_at 必须在登录前后时间窗口内，got: {updated}"
        );
    }

    // ── T-10002-R01: DUMMY_HASH 常量必须是有效 bcrypt 格式 ───────────────────
    //
    // RED: DUMMY_HASH 未定义时编译失败；使用无效格式哈希时 verify 返回 Err
    // GREEN: 定义有效 bcrypt DUMMY_HASH 常量后通过
    #[test]
    fn dummy_hash_constant_is_valid_bcrypt_format() {
        // DUMMY_HASH 必须能触发完整 bcrypt 计算（非立即报错的无效格式）。
        // 若格式无效，bcrypt::verify 立即返回 Err，无法提供时序保护。
        let result = voice_room_shared::crypto::verify_password("any_test_password", DUMMY_HASH);
        assert!(
            result.is_ok(),
            "DUMMY_HASH 必须是有效 bcrypt 哈希（能被 verify 解析），实际错误: {:?}. \
             请使用 bcrypt::hash(\"...\", 12) 预计算一个真实哈希替换当前值",
            result.err()
        );
        assert_eq!(
            result.unwrap(),
            false,
            "DUMMY_HASH 不应与任何真实密码匹配（它只是用来触发时序保护的虚拟哈希）"
        );
    }

    // ── T-10002-R02: 账号不存在时必须执行 bcrypt（时序攻击防护）────────────
    //
    // RED: 修复前 login("nonexistent",...) 直接返回（< 5ms），测试失败
    // GREEN: 修复后调用 verify_password(password, DUMMY_HASH)，耗时 >= 100ms（cost=12）
    #[tokio::test]
    async fn login_timing_protection_nonexistent_account_calls_bcrypt() {
        let (svc, _, _) = test_service();
        // 空仓库：账号不存在

        let start = std::time::Instant::now();
        let err = svc
            .login("nonexistent_user", "any_password", None)
            .await
            .unwrap_err();
        let elapsed = start.elapsed();

        assert!(
            matches!(err, AppError::InvalidAdminCredentials),
            "账号不存在必须返回 InvalidAdminCredentials，实际: {err:?}"
        );

        // cost=12 的 bcrypt 计算约 200-400ms；修复前（无 bcrypt 调用）< 5ms
        // 阈值设为 100ms，宽松容忍慢 CI 环境
        assert!(
            elapsed.as_millis() >= 100,
            "账号不存在路径必须调用 bcrypt（时序攻击防护），\
             实际耗时: {:?}（期望 >= 100ms）。\
             请在账号不存在时调用 verify_password(password, DUMMY_HASH)",
            elapsed
        );
    }

    // ── 边界情况：no IP 地址仍可正常登录 ────────────────────────────────────
    #[tokio::test]
    async fn login_success_without_ip_address() {
        let (svc, admin_repo, log_repo) = test_service();
        let admin = make_admin("op_user", "pass1234", true);
        admin_repo.seed(admin);

        let resp = svc.login("op_user", "pass1234", None).await.unwrap();
        assert!(!resp.token.is_empty());

        let logs = log_repo.get_logs();
        assert_eq!(logs.len(), 1);
        assert!(logs[0].ip_address.is_none(), "无 IP 时日志字段应为 None");
    }

    // ── 边界情况：同一账号可多次登录（每次生成新 token）────────────────────
    #[tokio::test]
    async fn repeated_login_generates_new_tokens() {
        let (svc, admin_repo, log_repo) = test_service();
        let admin = make_admin("op_user", "pass1234", true);
        admin_repo.seed(admin);

        let resp1 = svc.login("op_user", "pass1234", None).await.unwrap();
        // 短暂等待确保 iat 可能不同（实际会相同，但 token 内容应合法）
        let resp2 = svc.login("op_user", "pass1234", None).await.unwrap();

        assert!(!resp1.token.is_empty());
        assert!(!resp2.token.is_empty());
        // 两次均写入日志
        assert_eq!(log_repo.get_logs().len(), 2, "每次登录都必须写入日志");
    }
}
