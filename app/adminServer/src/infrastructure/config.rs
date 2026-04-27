//! AdminServer 配置加载（T-10020 / T-0000E §2.2 §2.4.2）
//!
//! 加载链：`default.toml → {ADMIN_PROFILE}.toml → ENV`，env 优先级最高。
//! 敏感字段（DATABASE_URL / REDIS_URL / ADMIN_JWT_SECRET / JWT_SECRET）**永不写入 TOML**，
//! 仅由 ENV 注入。启动期 fail-fast 的所有错误统一以 `CONFIG ERROR:` 前缀输出，
//! 退出码 78（EX_CONFIG）。
//!
//! 与 `voice-room-server::infrastructure::config` 模块边界 1:1 镜像，
//! 差异点全部冻结于 `doc/tds/adminServer/T-10020.md` §2.8 对称差异表。

use std::{
    env, fmt, fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use serde::Deserialize;

// =============================================================================
// Profile（T-0000E §2.2 加载链 + §2.4.2 字段冻结）
// =============================================================================

/// 运行 profile 白名单。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Dev,
    Test,
    Staging,
    Prod,
}

impl Profile {
    pub fn as_str(&self) -> &'static str {
        match self {
            Profile::Dev => "dev",
            Profile::Test => "test",
            Profile::Staging => "staging",
            Profile::Prod => "prod",
        }
    }

    /// dev 允许 REDIS_URL 缺失（`redis_url=None` + WARN，由 main.rs 装 NoopEventPublisher，
    /// 0 回归当前 main.rs:56~67 行为）；其他 profile 必须显式提供。
    /// 与 AppServer `allow_redis_fallback` 命名差异：AppServer dev 缺失走 fallback URL，
    /// AdminServer dev 缺失走 None（D-A1 决策，详见 TDS §2.8）。
    pub fn allow_redis_optional(&self) -> bool {
        matches!(self, Profile::Dev)
    }

    /// 由原始字符串解析（白名单内才接受）。
    pub fn parse_str(raw: &str) -> anyhow::Result<Self> {
        match raw.trim() {
            "dev" => Ok(Profile::Dev),
            "test" => Ok(Profile::Test),
            "staging" => Ok(Profile::Staging),
            "prod" => Ok(Profile::Prod),
            other => anyhow::bail!(
                "invalid ADMIN_PROFILE='{other}'; expected one of [dev,test,staging,prod]"
            ),
        }
    }

    /// 从 ENV 解析 profile：ADMIN_PROFILE > ADMIN_ENV > ADMIN__ENVIRONMENT > 默认 "dev"。
    pub fn from_env() -> anyhow::Result<Self> {
        let raw = env::var("ADMIN_PROFILE")
            .ok()
            .or_else(|| env::var("ADMIN_ENV").ok())
            .or_else(|| env::var("ADMIN__ENVIRONMENT").ok())
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "dev".to_owned());
        Self::parse_str(&raw)
    }
}

impl fmt::Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// =============================================================================
// JWT_SECRET 校验（ADMIN_JWT_SECRET 优先 + JWT_SECRET 兼容回落）
// =============================================================================

/// 校验并取出 JWT secret。
///
/// 优先级：`ADMIN_JWT_SECRET` > `JWT_SECRET`（向后兼容）。
/// - 任一来源命中后立即停止回落，并对该值做空白 / 占位符校验。
/// - 缺失 / 空白 / 占位符 `change-me-in-production` → 返回 `Err`。
pub(crate) fn require_admin_jwt_secret(
    admin: Option<String>,
    fallback: Option<String>,
) -> anyhow::Result<String> {
    let raw = admin.or(fallback).ok_or_else(|| {
        anyhow::anyhow!(
            "ADMIN_JWT_SECRET (or JWT_SECRET fallback) must be set; refusing to start \
             (see doc/protocol/auth_api.md)"
        )
    })?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        anyhow::bail!("JWT secret must not be empty");
    }
    if trimmed.eq_ignore_ascii_case("change-me-in-production") {
        anyhow::bail!(
            "JWT secret still equals the placeholder 'change-me-in-production'; \
             refuse to start"
        );
    }
    Ok(raw)
}

/// 通用 require_env：缺失 / 空白 → Err。
fn require_env(var: &str, hint: &str) -> anyhow::Result<String> {
    match env::var(var) {
        Ok(v) if !v.trim().is_empty() => Ok(v),
        _ => anyhow::bail!("{hint}"),
    }
}

// =============================================================================
// AdminSettings & 子结构
// =============================================================================

#[derive(Clone, Debug)]
pub struct AdminSettings {
    pub profile: Profile,
    pub app: AppSettings,
    pub server: HttpServerSettings,
    pub log: LogSettings,
    pub database: DatabaseSettings,
    pub jwt: JwtSettings,
    pub storage: StorageSettings,
    /// JWT 密钥明文（仅运行时持有，永不写入 TOML / 日志）。
    pub jwt_secret: String,
    pub redis_url: Option<String>,
}

impl AdminSettings {
    pub fn load() -> anyhow::Result<Self> {
        // Step 1: dotenv（沿用 main.rs:29 行为）
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let dotenv_path = env::var("ADMIN_DOTENV_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| manifest_dir.join(".env"));
        let _ = dotenvy::from_path(&dotenv_path);

        // Step 2/3: 解析 + 白名单校验 profile
        let profile = Profile::from_env()?;

        // Step 4: 默认值
        let mut settings = Self::default_for(profile);

        // Step 5/6: TOML 分层加载
        let config_dir = env::var("ADMIN_CONFIG_DIR")
            .map(|value| resolve_path_from_base(value, &manifest_dir))
            .unwrap_or_else(|_| manifest_dir.join("config"));
        settings.apply(load_config_file(config_dir.join("default.toml"))?);
        settings.apply(load_config_file(
            config_dir.join(format!("{profile}.toml")),
        )?);
        // profile 入口由 ENV 解析，确保覆盖任何 toml 内的 environment 字段
        settings.app.environment = profile.as_str().to_owned();

        // Step 7: ENV override（非敏感字段 + 兼容别名 PORT / GIFT_UPLOAD_DIR）
        settings.apply_env_overrides();

        // Step 8: 注入敏感字段（fail-fast）
        let db_url = require_env(
            "DATABASE_URL",
            &format!(
                "DATABASE_URL must be set; refused to start (profile={profile})"
            ),
        )?;
        settings.database.url = Some(db_url);

        settings.jwt_secret = require_admin_jwt_secret(
            env::var("ADMIN_JWT_SECRET").ok(),
            env::var("JWT_SECRET").ok(),
        )?;

        settings.redis_url = match env::var("REDIS_URL") {
            Ok(v) if !v.trim().is_empty() => Some(v),
            _ if profile.allow_redis_optional() => {
                tracing::warn!(
                    profile = %profile,
                    "REDIS_URL not set; admin server will fall back to NoopEventPublisher"
                );
                None
            }
            _ => anyhow::bail!(
                "REDIS_URL must be set for non-dev profile (profile={profile})"
            ),
        };

        // Step 9: 启动摘要日志（脱敏）
        tracing::info!(
            target: "voice_room_admin_server::config",
            "{}",
            settings.format_summary()
        );

        Ok(settings)
    }

    /// 测试可见的 default 构造（含 profile）。
    pub(crate) fn default_for(profile: Profile) -> Self {
        Self {
            profile,
            app: AppSettings {
                name: "voice-room-admin-server".to_owned(),
                environment: profile.as_str().to_owned(),
            },
            server: HttpServerSettings {
                host: "0.0.0.0".to_owned(),
                port: 3001,
            },
            log: LogSettings {
                level: "info".to_owned(),
                format: "json".to_owned(),
                service_name: "voice-room-admin-server".to_owned(),
            },
            database: DatabaseSettings {
                url: None,
                max_connections: 10,
                connect_timeout_secs: 5,
            },
            jwt: JwtSettings { expire_secs: 86400 },
            storage: StorageSettings {
                gift_upload_dir: "./uploads/gifts".to_owned(),
            },
            jwt_secret: String::new(),
            redis_url: None,
        }
    }

    fn apply(&mut self, file: ConfigFile) {
        if let Some(app) = file.app {
            if let Some(name) = app.name {
                self.app.name = name;
            }
            if let Some(environment) = app.environment {
                self.app.environment = environment;
            }
        }
        if let Some(server) = file.server {
            if let Some(host) = server.host {
                self.server.host = host;
            }
            if let Some(port) = server.port {
                self.server.port = port;
            }
        }
        if let Some(log) = file.log {
            if let Some(level) = log.level {
                self.log.level = level;
            }
            if let Some(format) = log.format {
                self.log.format = format;
            }
            if let Some(service_name) = log.service_name {
                self.log.service_name = service_name;
            }
        }
        if let Some(database) = file.database {
            if let Some(max_connections) = database.max_connections {
                self.database.max_connections = max_connections;
            }
            if let Some(connect_timeout_secs) = database.connect_timeout_secs {
                self.database.connect_timeout_secs = connect_timeout_secs;
            }
        }
        if let Some(jwt) = file.jwt {
            if let Some(expire_secs) = jwt.expire_secs {
                self.jwt.expire_secs = expire_secs;
            }
        }
        if let Some(storage) = file.storage {
            if let Some(gift_upload_dir) = storage.gift_upload_dir {
                self.storage.gift_upload_dir = gift_upload_dir;
            }
        }
        // [redis] 章节当前仅为占位
        let _ = file.redis;
    }

    pub(crate) fn apply_env_overrides(&mut self) {
        if let Ok(host) = env::var("ADMIN__SERVER__HOST") {
            self.server.host = host;
        }
        // ADMIN__SERVER__PORT 优先；否则尝试 PORT 兼容别名
        if let Ok(port) = env::var("ADMIN__SERVER__PORT") {
            match port.parse() {
                Ok(p) => self.server.port = p,
                Err(e) => tracing::warn!(
                    raw = %port,
                    error = %e,
                    "ADMIN__SERVER__PORT parse failed; keeping toml default"
                ),
            }
        } else if let Ok(port) = env::var("PORT") {
            match port.parse() {
                Ok(p) => self.server.port = p,
                Err(e) => tracing::warn!(
                    raw = %port,
                    error = %e,
                    "PORT (legacy alias) parse failed; keeping toml default"
                ),
            }
        }
        if let Ok(level) = env::var("ADMIN__LOG__LEVEL") {
            self.log.level = level;
        }
        if let Ok(format) = env::var("ADMIN__LOG__FORMAT") {
            self.log.format = format;
        }
        if let Ok(secs) = env::var("ADMIN__JWT__EXPIRE_SECS") {
            match secs.parse() {
                Ok(v) => self.jwt.expire_secs = v,
                Err(e) => tracing::warn!(
                    raw = %secs,
                    error = %e,
                    "ADMIN__JWT__EXPIRE_SECS parse failed; keeping toml default"
                ),
            }
        }
        if let Ok(n) = env::var("ADMIN__DATABASE__MAX_CONNECTIONS") {
            match n.parse() {
                Ok(v) => self.database.max_connections = v,
                Err(e) => tracing::warn!(
                    raw = %n,
                    error = %e,
                    "ADMIN__DATABASE__MAX_CONNECTIONS parse failed; keeping toml default"
                ),
            }
        }
        if let Ok(n) = env::var("ADMIN__DATABASE__CONNECT_TIMEOUT_SECS") {
            match n.parse() {
                Ok(v) => self.database.connect_timeout_secs = v,
                Err(e) => tracing::warn!(
                    raw = %n,
                    error = %e,
                    "ADMIN__DATABASE__CONNECT_TIMEOUT_SECS parse failed; keeping toml default"
                ),
            }
        }
        // ADMIN__STORAGE__GIFT_UPLOAD_DIR 优先；否则 GIFT_UPLOAD_DIR 兼容别名
        if let Ok(dir) = env::var("ADMIN__STORAGE__GIFT_UPLOAD_DIR") {
            self.storage.gift_upload_dir = dir;
        } else if let Ok(dir) = env::var("GIFT_UPLOAD_DIR") {
            self.storage.gift_upload_dir = dir;
        }
    }

    /// 启动摘要日志（已脱敏）。返回单行可读字符串，方便测试。
    pub fn format_summary(&self) -> String {
        let db_redacted = self
            .database
            .url
            .as_deref()
            .map(redact_url)
            .unwrap_or_else(|| "<unset>".to_owned());
        let redis_redacted = self
            .redis_url
            .as_deref()
            .map(redact_url)
            .unwrap_or_else(|| "<none, NoopEventPublisher>".to_owned());
        format!(
            "admin server config loaded profile={profile} app.name={app} \
             server={host}:{port} \
             database.max_connections={db_max} database.connect_timeout_secs={db_to} \
             database.url={db_url} redis.url={redis} \
             jwt.expire_secs={jwt_exp} jwt.secret_len={jwt_len} \
             log.level={log_level} log.format={log_fmt} \
             storage.gift_upload_dir={gift_dir}",
            profile = self.profile,
            app = self.app.name,
            host = self.server.host,
            port = self.server.port,
            db_max = self.database.max_connections,
            db_to = self.database.connect_timeout_secs,
            db_url = db_redacted,
            redis = redis_redacted,
            jwt_exp = self.jwt.expire_secs,
            jwt_len = self.jwt_secret.len(),
            log_level = self.log.level,
            log_fmt = self.log.format,
            gift_dir = self.storage.gift_upload_dir,
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct AppSettings {
    pub name: String,
    pub environment: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HttpServerSettings {
    pub host: String,
    pub port: u16,
}

impl HttpServerSettings {
    pub fn bind_addr(&self) -> anyhow::Result<SocketAddr> {
        Ok(format!("{}:{}", self.host, self.port).parse()?)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct LogSettings {
    pub level: String,
    pub format: String,
    pub service_name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DatabaseSettings {
    #[serde(default)]
    pub url: Option<String>,
    pub max_connections: u32,
    pub connect_timeout_secs: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct JwtSettings {
    pub expire_secs: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct StorageSettings {
    pub gift_upload_dir: String,
}

// =============================================================================
// TOML 反序列化结构（容忍未知章节）
// =============================================================================

#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    app: Option<AppSettingsFile>,
    server: Option<HttpServerSettingsFile>,
    log: Option<LogSettingsFile>,
    database: Option<DatabaseSettingsFile>,
    jwt: Option<JwtSettingsFile>,
    storage: Option<StorageSettingsFile>,
    #[serde(default)]
    redis: Option<RedisSettingsFile>,
}

#[derive(Debug, Deserialize)]
struct AppSettingsFile {
    name: Option<String>,
    environment: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HttpServerSettingsFile {
    host: Option<String>,
    port: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct LogSettingsFile {
    level: Option<String>,
    format: Option<String>,
    service_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DatabaseSettingsFile {
    max_connections: Option<u32>,
    connect_timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct JwtSettingsFile {
    expire_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct StorageSettingsFile {
    gift_upload_dir: Option<String>,
}

/// `[redis]` 当前仅为字段冻结表的占位章节（密码 / URL 走 ENV）。
#[derive(Debug, Default, Deserialize)]
struct RedisSettingsFile {}

fn load_config_file<P>(path: P) -> anyhow::Result<ConfigFile>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if !path.exists() {
        return Ok(ConfigFile::default());
    }
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

fn resolve_path_from_base(path: String, base: &Path) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

/// 脱敏 URL：将 user:pass@ 部分替换为 `***@`，保留 scheme / host / port / path。
fn redact_url(url: &str) -> String {
    if let Some(scheme_end) = url.find("://") {
        let (scheme, rest) = url.split_at(scheme_end + 3);
        if let Some(at) = rest.find('@') {
            return format!("{scheme}***@{}", &rest[at + 1..]);
        }
        return format!("{scheme}{rest}");
    }
    "<redacted>".to_owned()
}

// =============================================================================
// 单元测试 — TDS §3.1 U1.* / U2.* / U3.* / U4.* / U5.* / U6.*
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn clear_env() {
        for k in [
            "ADMIN_PROFILE",
            "ADMIN_ENV",
            "ADMIN__ENVIRONMENT",
            "ADMIN_CONFIG_DIR",
            "ADMIN_DOTENV_PATH",
            "DATABASE_URL",
            "REDIS_URL",
            "JWT_SECRET",
            "ADMIN_JWT_SECRET",
            "PORT",
            "GIFT_UPLOAD_DIR",
            "ADMIN__SERVER__HOST",
            "ADMIN__SERVER__PORT",
            "ADMIN__LOG__LEVEL",
            "ADMIN__LOG__FORMAT",
            "ADMIN__JWT__EXPIRE_SECS",
            "ADMIN__DATABASE__MAX_CONNECTIONS",
            "ADMIN__DATABASE__CONNECT_TIMEOUT_SECS",
            "ADMIN__STORAGE__GIFT_UPLOAD_DIR",
        ] {
            env::remove_var(k);
        }
    }

    fn parse_default() -> ConfigFile {
        toml::from_str(
            r#"
[app]
name = "voice-room-admin-server"
environment = "dev"

[server]
host = "0.0.0.0"
port = 3001

[database]
max_connections = 10
connect_timeout_secs = 5

[jwt]
expire_secs = 86400

[log]
level = "info"
format = "json"
service_name = "voice-room-admin-server"

[storage]
gift_upload_dir = "./uploads/gifts"
"#,
        )
        .unwrap()
    }

    // ──────────────────────────────────────────
    // U1.* Profile 解析（5 case）
    // ──────────────────────────────────────────

    #[test]
    fn u1_1_admin_profile_dev() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        env::set_var("ADMIN_PROFILE", "dev");
        assert_eq!(Profile::from_env().unwrap(), Profile::Dev);
    }

    #[test]
    fn u1_2_admin_profile_staging() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        env::set_var("ADMIN_PROFILE", "staging");
        assert_eq!(Profile::from_env().unwrap(), Profile::Staging);
    }

    #[test]
    fn u1_3_admin_env_alias_when_admin_profile_missing() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        env::set_var("ADMIN_ENV", "test");
        assert_eq!(Profile::from_env().unwrap(), Profile::Test);
    }

    #[test]
    fn u1_4_invalid_profile_rejected() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        env::set_var("ADMIN_PROFILE", "invalid_xxx");
        let err = Profile::from_env().unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("expected one of [dev,test,staging,prod]"),
            "got: {msg}"
        );
        assert!(msg.contains("invalid_xxx"));
        assert!(msg.contains("ADMIN_PROFILE"));
    }

    #[test]
    fn u1_5_default_profile_is_dev_when_all_unset() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        assert_eq!(Profile::from_env().unwrap(), Profile::Dev);
    }

    // ──────────────────────────────────────────
    // U2.* 加载链优先级（R6 关键，5 case）
    // ──────────────────────────────────────────

    #[test]
    fn u2_1_default_only_port_3001() {
        let mut s = AdminSettings::default_for(Profile::Dev);
        s.apply(parse_default());
        assert_eq!(s.server.port, 3001);
        assert_eq!(s.app.name, "voice-room-admin-server");
    }

    #[test]
    fn u2_2_dev_toml_overrides_log_level() {
        let mut s = AdminSettings::default_for(Profile::Dev);
        s.apply(parse_default());
        let dev_file: ConfigFile = toml::from_str(
            r#"
[log]
level = "debug"
"#,
        )
        .unwrap();
        s.apply(dev_file);
        assert_eq!(s.log.level, "debug");
        assert_eq!(s.server.port, 3001);
        assert_eq!(s.log.format, "json");
    }

    #[test]
    fn u2_3_env_overrides_toml_port() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let mut s = AdminSettings::default_for(Profile::Test);
        s.apply(parse_default());
        let test_file: ConfigFile = toml::from_str(
            r#"
[server]
port = 4001
"#,
        )
        .unwrap();
        s.apply(test_file);
        assert_eq!(s.server.port, 4001);
        env::set_var("ADMIN__SERVER__PORT", "9999");
        s.apply_env_overrides();
        env::remove_var("ADMIN__SERVER__PORT");
        assert_eq!(s.server.port, 9999);
    }

    #[test]
    fn u2_4_prod_toml_log_format_json_and_environment() {
        let mut s = AdminSettings::default_for(Profile::Prod);
        s.apply(parse_default());
        let prod_file: ConfigFile = toml::from_str(
            r#"
[app]
environment = "prod"

[log]
level = "info"
format = "json"
"#,
        )
        .unwrap();
        s.apply(prod_file);
        assert_eq!(s.app.environment, "prod");
        assert_eq!(s.log.format, "json");
    }

    #[test]
    fn u2_5_missing_profile_toml_loader_tolerates() {
        let cfg = load_config_file(PathBuf::from(
            "/path/that/should/not/exist/__nope_admin__.toml",
        ))
        .expect("missing file should not error");
        let mut s = AdminSettings::default_for(Profile::Staging);
        s.apply(parse_default());
        s.apply(cfg);
        assert_eq!(s.server.port, 3001);
    }

    // ──────────────────────────────────────────
    // U3.* 敏感字段 fail-fast（5 case）
    // ──────────────────────────────────────────

    #[test]
    fn u3_1_jwt_secret_fallback_to_jwt_secret() {
        let s = require_admin_jwt_secret(
            None,
            Some("ok-secret-32-chars-long-string".to_owned()),
        )
        .unwrap();
        assert_eq!(s, "ok-secret-32-chars-long-string");
    }

    #[test]
    fn u3_2_admin_jwt_secret_priority_over_fallback() {
        let s = require_admin_jwt_secret(
            Some("admin-secret".to_owned()),
            Some("ignored".to_owned()),
        )
        .unwrap();
        assert_eq!(s, "admin-secret");
    }

    #[test]
    fn u3_3_both_missing_rejected() {
        let err = require_admin_jwt_secret(None, None).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("ADMIN_JWT_SECRET (or JWT_SECRET fallback)"),
            "got: {msg}"
        );
    }

    #[test]
    fn u3_4_empty_secret_rejected() {
        let err = require_admin_jwt_secret(Some(String::new()), None).unwrap_err();
        assert!(format!("{err:#}").contains("must not be empty"));
        let err = require_admin_jwt_secret(Some("   ".to_owned()), None).unwrap_err();
        assert!(format!("{err:#}").contains("must not be empty"));
    }

    #[test]
    fn u3_5_placeholder_secret_rejected() {
        let err =
            require_admin_jwt_secret(Some("change-me-in-production".to_owned()), None)
                .unwrap_err();
        assert!(format!("{err:#}").contains("placeholder"));
        let err = require_admin_jwt_secret(
            Some("CHANGE-ME-IN-PRODUCTION".to_owned()),
            None,
        )
        .unwrap_err();
        assert!(format!("{err:#}").contains("placeholder"));
    }

    // ──────────────────────────────────────────
    // U4.* ENV override 新增字段 + 兼容别名（5 case）
    // ──────────────────────────────────────────

    #[test]
    fn u4_1_jwt_expire_secs_env_override() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let mut s = AdminSettings::default_for(Profile::Dev);
        env::set_var("ADMIN__JWT__EXPIRE_SECS", "7200");
        s.apply_env_overrides();
        env::remove_var("ADMIN__JWT__EXPIRE_SECS");
        assert_eq!(s.jwt.expire_secs, 7200);
    }

    #[test]
    fn u4_2_database_max_connections_env_override() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let mut s = AdminSettings::default_for(Profile::Dev);
        env::set_var("ADMIN__DATABASE__MAX_CONNECTIONS", "20");
        s.apply_env_overrides();
        env::remove_var("ADMIN__DATABASE__MAX_CONNECTIONS");
        assert_eq!(s.database.max_connections, 20);
    }

    #[test]
    fn u4_3_legacy_port_alias_when_admin_port_unset() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let mut s = AdminSettings::default_for(Profile::Dev);
        env::set_var("PORT", "4242");
        s.apply_env_overrides();
        env::remove_var("PORT");
        assert_eq!(s.server.port, 4242);
    }

    #[test]
    fn u4_4_admin_port_priority_over_legacy_port() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let mut s = AdminSettings::default_for(Profile::Dev);
        env::set_var("ADMIN__SERVER__PORT", "5000");
        env::set_var("PORT", "4242");
        s.apply_env_overrides();
        env::remove_var("ADMIN__SERVER__PORT");
        env::remove_var("PORT");
        assert_eq!(s.server.port, 5000);
    }

    #[test]
    fn u4_5_legacy_gift_upload_dir_alias() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let mut s = AdminSettings::default_for(Profile::Dev);
        env::set_var("GIFT_UPLOAD_DIR", "/data/gifts");
        s.apply_env_overrides();
        env::remove_var("GIFT_UPLOAD_DIR");
        assert_eq!(s.storage.gift_upload_dir, "/data/gifts");
    }

    // 额外：parse 失败 WARN 不 fail（与 T-00040 镜像）
    #[test]
    fn u4_6_invalid_port_keeps_toml_default() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let mut s = AdminSettings::default_for(Profile::Dev);
        s.server.port = 4001;
        env::set_var("ADMIN__SERVER__PORT", "not_a_number");
        s.apply_env_overrides();
        env::remove_var("ADMIN__SERVER__PORT");
        assert_eq!(s.server.port, 4001);
    }

    // ──────────────────────────────────────────
    // U5.* 启动摘要日志脱敏（3 case）
    // ──────────────────────────────────────────

    fn make_sample_settings() -> AdminSettings {
        let mut s = AdminSettings::default_for(Profile::Staging);
        s.database.url = Some(
            "postgres://app_user:app_server_pass@stg-pg.example.com:5432/voice_room"
                .to_owned(),
        );
        s.redis_url =
            Some("redis://default:supersecret@stg-redis.example.com:6379".to_owned());
        s.jwt_secret = "secret-from-test-1234567890abcdef".to_owned();
        s
    }

    #[test]
    fn u5_1_summary_no_jwt_secret_plaintext() {
        let s = make_sample_settings();
        let line = s.format_summary();
        assert!(
            !line.contains("secret-from-test"),
            "summary leaked jwt secret: {line}"
        );
        assert!(line.contains("jwt.secret_len="));
    }

    #[test]
    fn u5_2_summary_redacts_database_password() {
        let s = make_sample_settings();
        let line = s.format_summary();
        assert!(!line.contains("app_server_pass"), "leaked db pass: {line}");
        assert!(!line.contains("supersecret"), "leaked redis pass: {line}");
        assert!(line.contains("***@"), "expected redaction marker: {line}");
        assert!(line.contains("stg-pg.example.com:5432"));
    }

    #[test]
    fn u5_3_summary_none_redis_marker() {
        let mut s = make_sample_settings();
        s.redis_url = None;
        let line = s.format_summary();
        assert!(line.contains("redis.url="), "missing redis.url field: {line}");
        assert!(
            line.contains("<none, NoopEventPublisher>"),
            "expected NoopEventPublisher marker: {line}"
        );
    }

    // 额外：profile/host/port 可见
    #[test]
    fn u5_4_summary_contains_profile_host_port_and_storage() {
        let s = make_sample_settings();
        let line = s.format_summary();
        assert!(line.contains("profile=staging"));
        assert!(line.contains("server=0.0.0.0:3001"));
        assert!(line.contains("storage.gift_upload_dir="));
    }

    // ──────────────────────────────────────────
    // U6.* NoopEventPublisher 容忍（D-A1，2 case）
    // ──────────────────────────────────────────

    #[test]
    fn u6_1_dev_allows_redis_optional() {
        assert!(Profile::Dev.allow_redis_optional());
    }

    #[test]
    fn u6_2_non_dev_rejects_redis_optional() {
        assert!(!Profile::Test.allow_redis_optional());
        assert!(!Profile::Staging.allow_redis_optional());
        assert!(!Profile::Prod.allow_redis_optional());
    }

    // ──────────────────────────────────────────
    // require_env / default_for 兜底
    // ──────────────────────────────────────────

    #[test]
    fn require_env_database_url_missing() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        env::remove_var("DATABASE_URL");
        let err =
            require_env("DATABASE_URL", "DATABASE_URL must be set; refused").unwrap_err();
        assert!(format!("{err:#}").contains("DATABASE_URL must be set"));
    }

    #[test]
    fn require_env_database_url_blank_rejected() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        env::set_var("DATABASE_URL", "   ");
        let err = require_env("DATABASE_URL", "DATABASE_URL must be set").unwrap_err();
        env::remove_var("DATABASE_URL");
        assert!(format!("{err:#}").contains("DATABASE_URL must be set"));
    }

    #[test]
    fn default_for_does_not_embed_hardcoded_secret() {
        let s = AdminSettings::default_for(Profile::Dev);
        assert!(s.jwt_secret.is_empty());
    }

    #[test]
    fn redact_url_strips_userinfo() {
        assert_eq!(
            redact_url("postgres://u:p@h:5432/db"),
            "postgres://***@h:5432/db"
        );
        assert_eq!(redact_url("redis://r:6379"), "redis://r:6379");
        assert_eq!(redact_url("not-a-url"), "<redacted>");
    }
}
