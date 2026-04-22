use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use rand::Rng;
use uuid::Uuid;
use voice_room_shared::{
    jwt::token::{encode_token, AppClaims},
    models::user::UserModel,
};

use crate::{
    common::error::AppError,
    infrastructure::{redis_store::SmsCodeStore, third_party::sms::SmsProvider},
};

use super::{
    dto::{LoginResponse, LoginUserInfo, SendCodeResponse, UserResponse},
    repository::UserRepository,
};

/// JWT 有效期：30 天
const TOKEN_EXPIRES_SECS: u64 = 30 * 24 * 3600;

pub struct AuthService {
    user_repo: Arc<dyn UserRepository>,
    code_store: Arc<dyn SmsCodeStore>,
    sms: Arc<dyn SmsProvider>,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        code_store: Arc<dyn SmsCodeStore>,
        sms: Arc<dyn SmsProvider>,
        jwt_secret: String,
    ) -> Self {
        Self {
            user_repo,
            code_store,
            sms,
            jwt_secret,
        }
    }

    /// T-00002: 发送短信验证码（save-first 消除并发 TOCTOU）
    pub async fn send_code(&self, phone: &str) -> Result<SendCodeResponse, AppError> {
        validate_phone(phone)?;
        let today = today_str();
        let code = generate_code();

        // 1. 原子写入 Redis（Lua 是真正的并发门控，检查冷却/日限并预留 code）
        self.code_store.save_code(phone, &code, &today).await?;

        // 2. 发送 SMS；失败时撤销预留（清除 code + cooldown，daily count 保留防滥用）
        if let Err(sms_err) = self.sms.send_verification_code(phone, &code).await {
            self.code_store.revoke_code(phone).await.ok();
            return Err(sms_err);
        }

        Ok(SendCodeResponse {
            expires_in: 300,
            cooldown: 60,
        })
    }

    /// T-00003: 验证码登录（验证 → 找 / 建用户 → 签发 JWT）
    pub async fn login(&self, phone: &str, code: &str) -> Result<LoginResponse, AppError> {
        validate_phone(phone)?;
        self.code_store.verify_and_consume(phone, code).await?;

        let (user, is_new) = match self.user_repo.find_by_phone(phone).await? {
            Some(u) => (u, false),
            None => {
                let suffix = &phone[phone.len().saturating_sub(4)..];
                let nickname = format!("User{suffix}");
                (self.user_repo.create(phone, &nickname).await?, true)
            }
        };

        if user.is_banned {
            return Err(AppError::Unauthorized);
        }

        let token = issue_token(&user, &self.jwt_secret)?;
        Ok(LoginResponse {
            token,
            expires_in: TOKEN_EXPIRES_SECS,
            user: LoginUserInfo::from((user, is_new)),
        })
    }

    /// T-00005: 获取当前登录用户信息
    pub async fn get_me(&self, user_id: Uuid) -> Result<UserResponse, AppError> {
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await?
            .ok_or_else(|| AppError::NotFound("user".into()))?;

        if user.is_banned {
            return Err(AppError::Unauthorized);
        }

        Ok(UserResponse::from(user))
    }

    /// T-00012: 按 user_id 获取用户信息（用于 JoinRoom 信令填充昵称/头像）
    pub async fn get_user_by_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<voice_room_shared::models::user::UserModel>, AppError> {
        self.user_repo.find_by_id(user_id).await
    }

    /// T-00027: 批量按 user_id 查询用户信息（单次 SQL，避免 N+1）
    pub async fn get_users_by_ids(
        &self,
        ids: &[Uuid],
    ) -> Result<Vec<voice_room_shared::models::user::UserModel>, AppError> {
        self.user_repo.find_by_ids(ids).await
    }
}

impl From<UserModel> for UserResponse {
    fn from(u: UserModel) -> Self {
        UserResponse {
            id: u.id.to_string(),
            phone: u.phone,
            nickname: u.nickname,
            avatar: u.avatar,
            coin_balance: u.coin_balance,
            vip_level: u.vip_level,
            created_at: u.created_at.to_rfc3339(),
        }
    }
}

impl From<(UserModel, bool)> for LoginUserInfo {
    fn from((u, is_new): (UserModel, bool)) -> Self {
        LoginUserInfo {
            id: u.id.to_string(),
            phone: u.phone,
            nickname: u.nickname,
            avatar: u.avatar,
            coin_balance: u.coin_balance,
            vip_level: u.vip_level,
            is_new,
            created_at: u.created_at.to_rfc3339(),
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// E.164 格式校验：`+` 后跟 6–14 位数字
pub fn validate_phone(phone: &str) -> Result<(), AppError> {
    let stripped = phone
        .strip_prefix('+')
        .ok_or(AppError::InvalidPhoneNumber)?;
    if stripped.len() < 6
        || stripped.len() > 14
        || !stripped.chars().all(|c| c.is_ascii_digit())
    {
        return Err(AppError::InvalidPhoneNumber);
    }
    Ok(())
}

fn generate_code() -> String {
    let mut rng = rand::rng();
    format!("{:06}", rng.random_range(0u32..1_000_000u32))
}

fn today_str() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn issue_token(user: &UserModel, secret: &str) -> Result<String, AppError> {
    let now = now_secs();
    let claims = AppClaims {
        sub: user.id.to_string(),
        iss: "voiceroom".to_string(),
        exp: now + TOKEN_EXPIRES_SECS,
        iat: now,
    };
    encode_token(&claims, secret.as_bytes())
        .map_err(|e| AppError::Internal(format!("jwt encode: {e}")))
}

// ─── 单元测试（T-00002 / T-00003 / T-00005 验收用例）────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use chrono::Utc;
    use uuid::Uuid;
    use voice_room_shared::models::user::UserModel;

    use crate::infrastructure::{
        redis_store::FakeCodeStore, third_party::sms::MockSmsProvider,
    };
    use crate::modules::auth::repository::FakeUserRepository;

    fn test_service() -> (AuthService, Arc<FakeCodeStore>, Arc<FakeUserRepository>) {
        let code_store = Arc::new(FakeCodeStore::default());
        let user_repo = Arc::new(FakeUserRepository::default());
        let sms = Arc::new(MockSmsProvider::default());
        let svc = AuthService::new(
            user_repo.clone(),
            code_store.clone(),
            sms,
            "test-secret".to_string(),
        );
        (svc, code_store, user_repo)
    }

    fn dummy_user(phone: &str, banned: bool) -> UserModel {
        let now = Utc::now();
        UserModel {
            id: Uuid::new_v4(),
            phone: phone.to_string(),
            nickname: "TestUser".into(),
            avatar: None,
            coin_balance: 0,
            diamond_balance: 0,
            charm_balance: 0,
            vip_level: 0,
            is_banned: banned,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    // ── validate_phone ────────────────────────────────────────────────────────
    #[test]
    fn valid_e164_passes() {
        assert!(validate_phone("+8613800138000").is_ok());
        assert!(validate_phone("+97150123456").is_ok());
    }

    #[test]
    fn phone_without_plus_fails() {
        assert!(matches!(
            validate_phone("8613800138000"),
            Err(AppError::InvalidPhoneNumber)
        ));
    }

    #[test]
    fn phone_too_short_fails() {
        assert!(matches!(
            validate_phone("+123"),
            Err(AppError::InvalidPhoneNumber)
        ));
    }

    #[test]
    fn phone_too_long_fails() {
        assert!(matches!(
            validate_phone("+123456789012345"),
            Err(AppError::InvalidPhoneNumber)
        ));
    }

    #[test]
    fn phone_with_non_digits_fails() {
        assert!(matches!(
            validate_phone("+8613abc00138"),
            Err(AppError::InvalidPhoneNumber)
        ));
    }

    // ── T-00002: send_code ────────────────────────────────────────────────────
    #[tokio::test]
    async fn send_code_invalid_phone_returns_error() {
        let (svc, _, _) = test_service();
        let err = svc.send_code("bad-phone").await.unwrap_err();
        assert!(matches!(err, AppError::InvalidPhoneNumber));
    }

    #[tokio::test]
    async fn send_code_during_cooldown_returns_error() {
        let (svc, code_store, _) = test_service();
        code_store.set_cooldown("+8613800138000", true);
        let err = svc.send_code("+8613800138000").await.unwrap_err();
        assert!(matches!(err, AppError::VerificationCodeCooldown));
    }

    #[tokio::test]
    async fn send_code_daily_limit_returns_error() {
        let (svc, code_store, _) = test_service();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        code_store.set_daily_count("+8613800138000", &today, 10);
        let err = svc.send_code("+8613800138000").await.unwrap_err();
        assert!(matches!(err, AppError::VerificationCodeDailyLimit));
    }

    #[tokio::test]
    async fn send_code_success_stores_and_sends() {
        let (svc, _, _) = test_service();
        let resp = svc.send_code("+8613800138000").await.unwrap();
        assert_eq!(resp.expires_in, 300);
        assert_eq!(resp.cooldown, 60);
    }

    // ── T-00003: login ────────────────────────────────────────────────────────
    #[tokio::test]
    async fn login_correct_code_creates_user_and_returns_token() {
        let (svc, code_store, user_repo) = test_service();
        code_store.seed_code("+8613800138000", "123456");
        let resp = svc.login("+8613800138000", "123456").await.unwrap();
        assert!(!resp.token.is_empty());
        assert!(resp.user.is_new, "should be new user");
        assert!(!resp.user.created_at.is_empty());
        assert!(user_repo
            .find_by_phone("+8613800138000")
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn login_wrong_code_returns_invalid_code() {
        let (svc, code_store, _) = test_service();
        code_store.seed_code("+8613800138000", "999999");
        let err = svc.login("+8613800138000", "000000").await.unwrap_err();
        assert!(matches!(err, AppError::InvalidVerificationCode));
    }

    #[tokio::test]
    async fn login_no_code_returns_expired() {
        let (svc, _, _) = test_service();
        let err = svc.login("+8613800138000", "123456").await.unwrap_err();
        assert!(matches!(err, AppError::VerificationCodeExpired));
    }

    #[tokio::test]
    async fn login_existing_user_reuses_account() {
        let (svc, code_store, user_repo) = test_service();
        let user = dummy_user("+8613800138000", false);
        let uid = user.id;
        user_repo.seed(user);
        code_store.seed_code("+8613800138000", "123456");
        let resp = svc.login("+8613800138000", "123456").await.unwrap();
        assert_eq!(resp.user.id, uid.to_string());
        assert!(!resp.user.is_new, "should be existing user");
    }

    #[tokio::test]
    async fn login_banned_user_returns_unauthorized() {
        let (svc, code_store, user_repo) = test_service();
        let user = dummy_user("+8613800138000", true);
        user_repo.seed(user);
        code_store.seed_code("+8613800138000", "123456");
        let err = svc.login("+8613800138000", "123456").await.unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    // ── T-00005: get_me ───────────────────────────────────────────────────────
    #[tokio::test]
    async fn get_me_returns_correct_user() {
        let (svc, _, user_repo) = test_service();
        let user = dummy_user("+8613800138000", false);
        let uid = user.id;
        let nickname = user.nickname.clone();
        user_repo.seed(user);
        let resp = svc.get_me(uid).await.unwrap();
        assert_eq!(resp.id, uid.to_string());
        assert_eq!(resp.nickname, nickname);
        assert!(!resp.created_at.is_empty());
    }

    #[tokio::test]
    async fn get_me_not_found_returns_404() {
        let (svc, _, _) = test_service();
        let err = svc.get_me(Uuid::new_v4()).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn get_me_banned_user_returns_unauthorized() {
        let (svc, _, user_repo) = test_service();
        let user = dummy_user("+8613800138000", true);
        let uid = user.id;
        user_repo.seed(user);
        let err = svc.get_me(uid).await.unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    // ── H-02/M-02: SMS 失败后冷却必须撤销，日计数保留（防滥用）─────────────
    #[tokio::test]
    async fn send_code_sms_failure_revokes_cooldown_keeps_daily_count() {
        use crate::infrastructure::third_party::sms::FailingSmsProvider;
        let code_store = Arc::new(FakeCodeStore::default());
        let user_repo = Arc::new(FakeUserRepository::default());
        let svc = AuthService::new(
            user_repo,
            code_store.clone(),
            Arc::new(FailingSmsProvider::default()),
            "test-secret".to_string(),
        );

        let err = svc.send_code("+8613800138000").await.unwrap_err();
        assert!(
            matches!(err, AppError::SmsSendFailed(_)),
            "should propagate SMS error"
        );

        // 冷却期必须通过 revoke_code 清除，用户可以立即重试
        let in_cooldown = code_store.is_in_cooldown("+8613800138000").await.unwrap();
        assert!(!in_cooldown, "cooldown must be revoked after SMS failure");

        // daily count 保留 1：save_code 已消耗日额度（防止无限重试滥用 SMS API）
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let count = code_store.daily_count("+8613800138000", &today).await.unwrap();
        assert_eq!(count, 1, "daily count must increment even on SMS failure to prevent abuse");
    }

    // ── H-01: 同一 OTP 只能被消费一次（行为契约 / 防双重登录）────────────────
    #[tokio::test]
    async fn login_reuse_code_returns_expired() {
        let (svc, code_store, _) = test_service();
        code_store.seed_code("+8613800138000", "123456");

        // 第一次登录成功
        svc.login("+8613800138000", "123456").await.unwrap();

        // 第二次使用相同 OTP 必须失败
        let err = svc.login("+8613800138000", "123456").await.unwrap_err();
        assert!(
            matches!(err, AppError::VerificationCodeExpired),
            "same OTP must not be reused; got: {err:?}"
        );
    }

    // ── T-00027: get_users_by_ids 批量查询 ───────────────────────────────────

    /// S01: get_users_by_ids 空切片 → 返回空 Vec
    #[tokio::test]
    async fn get_users_by_ids_empty_returns_empty() {
        let (svc, _, _) = test_service();
        let result = svc.get_users_by_ids(&[]).await.unwrap();
        assert!(result.is_empty(), "S01: empty ids must return empty Vec");
    }

    /// S02: get_users_by_ids 批量返回所有存在的用户
    #[tokio::test]
    async fn get_users_by_ids_returns_all_matching_users() {
        let (svc, _, user_repo) = test_service();
        let u1 = dummy_user("+8611000000001", false);
        let u2 = dummy_user("+8611000000002", false);
        let id1 = u1.id;
        let id2 = u2.id;
        user_repo.seed(u1);
        user_repo.seed(u2);

        let result = svc.get_users_by_ids(&[id1, id2]).await.unwrap();
        assert_eq!(result.len(), 2, "S02: must return 2 users");
        let ids: Vec<Uuid> = result.iter().map(|u| u.id).collect();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    /// S03: get_users_by_ids 不存在的 ID 不在返回结果中
    #[tokio::test]
    async fn get_users_by_ids_skips_nonexistent() {
        let (svc, _, user_repo) = test_service();
        let u1 = dummy_user("+8611000000003", false);
        let id1 = u1.id;
        user_repo.seed(u1);

        let ghost = Uuid::new_v4();
        let result = svc.get_users_by_ids(&[id1, ghost]).await.unwrap();
        assert_eq!(result.len(), 1, "S03: ghost id must not appear");
        assert_eq!(result[0].id, id1);
    }
}
