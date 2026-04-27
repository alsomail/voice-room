//! AppServer 配置加载（T-00040 / T-0000E §2.2 §2.4.2）
//!
//! 加载链：`default.toml → {APP_PROFILE}.toml → ENV`，env 优先级最高。
//! 敏感字段（DATABASE_URL / REDIS_URL / JWT_SECRET）**永不写入 TOML**，仅由 ENV 注入。
//! 启动期 fail-fast 的所有错误统一以 `CONFIG ERROR:` 前缀输出，退出码 78（EX_CONFIG）。

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

    /// dev 允许 REDIS_URL 缺失走 fallback；其他 profile 必须显式提供。
    pub fn allow_redis_fallback(&self) -> bool {
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
                "invalid APP_PROFILE='{other}'; expected one of [dev,test,staging,prod]"
            ),
        }
    }

    /// 从 ENV 解析 profile：APP_PROFILE > APP_ENV > APP__ENVIRONMENT > 默认 "dev"。
    pub fn from_env() -> anyhow::Result<Self> {
        let raw = env::var("APP_PROFILE")
            .ok()
            .or_else(|| env::var("APP_ENV").ok())
            .or_else(|| env::var("APP__ENVIRONMENT").ok())
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
// JWT_SECRET 校验（已存在契约 + 0 回归）
// =============================================================================

/// 校验并取出 JWT_SECRET 字符串。
///
/// 要求：必须由环境变量提供，**禁止使用任何硬编码默认值**。
/// - 缺失 / 空白 / 占位符 `change-me-in-production` → 返回 `Err`。
pub(crate) fn require_jwt_secret(value: Option<String>) -> anyhow::Result<String> {
    let raw = value.ok_or_else(|| {
        anyhow::anyhow!(
            "JWT_SECRET must be set; refusing to fall back to a hardcoded default \
             (see doc/protocol/auth_api.md)"
        )
    })?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        anyhow::bail!("JWT_SECRET must not be empty");
    }
    if trimmed.eq_ignore_ascii_case("change-me-in-production") {
        anyhow::bail!(
            "JWT_SECRET still equals the placeholder 'change-me-in-production'; \
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
// ServerSettings & 子结构
// =============================================================================

#[derive(Clone, Debug)]
pub struct ServerSettings {
    pub profile: Profile,
    pub app: AppSettings,
    pub server: HttpServerSettings,
    pub log: LogSettings,
    pub database: DatabaseSettings,
    pub jwt: JwtSettings,
    /// JWT 密钥明文（仅运行时持有，永不写入 TOML / 日志）。
    pub jwt_secret: String,
    pub redis_url: Option<String>,
}

impl ServerSettings {
    pub fn load() -> anyhow::Result<Self> {
        // Step 1: dotenv（已有）
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let dotenv_path = env::var("APP_DOTENV_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| manifest_dir.join(".env"));
        let _ = dotenvy::from_path(&dotenv_path);

        // Step 2/3: 解析 + 白名单校验 profile
        let profile = Profile::from_env()?;

        // Step 4: 默认值
        let mut settings = Self::default_for(profile);

        // Step 5/6: TOML 分层加载
        let config_dir = env::var("APP_CONFIG_DIR")
            .map(|value| resolve_path_from_base(value, &manifest_dir))
            .unwrap_or_else(|_| manifest_dir.join("config"));
        settings.apply(load_config_file(config_dir.join("default.toml"))?);
        settings.apply(load_config_file(
            config_dir.join(format!("{profile}.toml")),
        )?);
        // profile 入口由 ENV 解析，确保覆盖任何 toml 内的 environment 字段
        settings.app.environment = profile.as_str().to_owned();

        // Step 7: ENV override（非敏感字段）
        settings.apply_env_overrides();

        // Step 8: 注入敏感字段（fail-fast）
        let db_url = require_env(
            "DATABASE_URL",
            &format!(
                "DATABASE_URL must be set; refused to start (profile={profile})"
            ),
        )?;
        settings.database.url = Some(db_url);

        settings.jwt_secret = require_jwt_secret(env::var("JWT_SECRET").ok())?;

        settings.redis_url = match env::var("REDIS_URL") {
            Ok(v) if !v.trim().is_empty() => Some(v),
            _ if profile.allow_redis_fallback() => {
                tracing::warn!(
                    "REDIS_URL not set, fallback to redis://127.0.0.1:6379 (dev only)"
                );
                Some("redis://127.0.0.1:6379".to_owned())
            }
            _ => anyhow::bail!(
                "REDIS_URL must be set for non-dev profile (profile={profile})"
            ),
        };

        // Step 9: 启动摘要日志（脱敏）
        tracing::info!(target: "voice_room_server::config", "{}", settings.format_summary());

        Ok(settings)
    }

    /// 测试可见的 default 构造（含 profile）。
    pub(crate) fn default_for(profile: Profile) -> Self {
        Self {
            profile,
            app: AppSettings {
                name: "voice-room-server".to_owned(),
                environment: profile.as_str().to_owned(),
            },
            server: HttpServerSettings {
                host: "0.0.0.0".to_owned(),
                port: 3000,
            },
            log: LogSettings {
                level: "info".to_owned(),
                format: "json".to_owned(),
                service_name: "voice-room-server".to_owned(),
            },
            database: DatabaseSettings {
                url: None,
                max_connections: 10,
                connect_timeout_secs: 5,
            },
            jwt: JwtSettings { expire_secs: 86400 },
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
        // [redis] 章节当前仅为占位，无可覆盖字段；保留 ConfigFile.redis 以容忍 toml 出现 [redis] 段
        let _ = file.redis;
    }

    pub(crate) fn apply_env_overrides(&mut self) {
        if let Ok(host) = env::var("APP__SERVER__HOST") {
            self.server.host = host;
        }
        if let Ok(port) = env::var("APP__SERVER__PORT") {
            match port.parse() {
                Ok(p) => self.server.port = p,
                Err(e) => tracing::warn!(
                    raw = %port,
                    error = %e,
                    "APP__SERVER__PORT parse failed; keeping toml default"
                ),
            }
        }
        if let Ok(level) = env::var("APP__LOG__LEVEL") {
            self.log.level = level;
        }
        if let Ok(format) = env::var("APP__LOG__FORMAT") {
            self.log.format = format;
        }
        if let Ok(secs) = env::var("APP__JWT__EXPIRE_SECS") {
            match secs.parse() {
                Ok(v) => self.jwt.expire_secs = v,
                Err(e) => tracing::warn!(
                    raw = %secs,
                    error = %e,
                    "APP__JWT__EXPIRE_SECS parse failed; keeping toml default"
                ),
            }
        }
        if let Ok(n) = env::var("APP__DATABASE__MAX_CONNECTIONS") {
            match n.parse() {
                Ok(v) => self.database.max_connections = v,
                Err(e) => tracing::warn!(
                    raw = %n,
                    error = %e,
                    "APP__DATABASE__MAX_CONNECTIONS parse failed; keeping toml default"
                ),
            }
        }
        if let Ok(n) = env::var("APP__DATABASE__CONNECT_TIMEOUT_SECS") {
            match n.parse() {
                Ok(v) => self.database.connect_timeout_secs = v,
                Err(e) => tracing::warn!(
                    raw = %n,
                    error = %e,
                    "APP__DATABASE__CONNECT_TIMEOUT_SECS parse failed; keeping toml default"
                ),
            }
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
            .unwrap_or_else(|| "<unset>".to_owned());
        format!(
            "server config loaded profile={profile} app.name={app} \
             server={host}:{port} \
             database.max_connections={db_max} database.connect_timeout_secs={db_to} \
             database.url={db_url} redis.url={redis} \
             jwt.expire_secs={jwt_exp} jwt.secret_len={jwt_len} \
             log.level={log_level} log.format={log_fmt}",
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

/// `[redis]` 当前仅为字段冻结表的占位章节（密码 / URL 走 ENV）。
/// 保留结构以容忍 toml 中出现该段但无字段。
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
/// 非 URL 形态则返回 `<redacted>`，保证不会泄漏。
fn redact_url(url: &str) -> String {
    // 寻找 scheme://
    if let Some(scheme_end) = url.find("://") {
        let (scheme, rest) = url.split_at(scheme_end + 3);
        if let Some(at) = rest.find('@') {
            return format!("{scheme}***@{}", &rest[at + 1..]);
        }
        // 无 userinfo，原样返回（无敏感信息）
        return format!("{scheme}{rest}");
    }
    "<redacted>".to_owned()
}

// =============================================================================
// 单元测试
// =============================================================================

#[cfg(test)]
mod tests {
    //! T-00040 单元测试 — 覆盖 TDS §3.1 U1.* / U2.* / U3.* / U4.* / U5.*
    //! + 历史 require_jwt_secret 4 case（0 回归）。

    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn clear_env() {
        for k in [
            "APP_PROFILE",
            "APP_ENV",
            "APP__ENVIRONMENT",
            "APP__SERVER__HOST",
            "APP__SERVER__PORT",
            "APP__LOG__LEVEL",
            "APP__LOG__FORMAT",
            "APP__JWT__EXPIRE_SECS",
            "APP__DATABASE__MAX_CONNECTIONS",
            "APP__DATABASE__CONNECT_TIMEOUT_SECS",
        ] {
            env::remove_var(k);
        }
    }

    fn parse_default() -> ConfigFile {
        toml::from_str(
            r#"
[app]
name = "voice-room-server"
environment = "dev"

[server]
host = "0.0.0.0"
port = 3000

[database]
max_connections = 10
connect_timeout_secs = 5

[jwt]
expire_secs = 86400

[log]
level = "info"
format = "json"
service_name = "voice-room-server"
"#,
        )
        .unwrap()
    }

    // ──────────────────────────────────────────
    // U1.* Profile 解析
    // ──────────────────────────────────────────

    #[test]
    fn u1_1_app_profile_dev() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        env::set_var("APP_PROFILE", "dev");
        assert_eq!(Profile::from_env().unwrap(), Profile::Dev);
    }

    #[test]
    fn u1_2_app_profile_staging() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        env::set_var("APP_PROFILE", "staging");
        assert_eq!(Profile::from_env().unwrap(), Profile::Staging);
    }

    #[test]
    fn u1_3_app_env_alias_when_app_profile_missing() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        env::set_var("APP_ENV", "test");
        assert_eq!(Profile::from_env().unwrap(), Profile::Test);
    }

    #[test]
    fn u1_4_invalid_profile_rejected() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        env::set_var("APP_PROFILE", "invalid_xxx");
        let err = Profile::from_env().unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("expected one of [dev,test,staging,prod]"),
            "got: {msg}"
        );
        assert!(msg.contains("invalid_xxx"));
    }

    #[test]
    fn u1_5_default_profile_is_dev_when_all_unset() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        assert_eq!(Profile::from_env().unwrap(), Profile::Dev);
    }

    // ──────────────────────────────────────────
    // U2.* 加载链优先级（R6 关键）
    // ──────────────────────────────────────────

    #[test]
    fn u2_1_default_only_port_3000() {
        let mut s = ServerSettings::default_for(Profile::Dev);
        s.apply(parse_default());
        assert_eq!(s.server.port, 3000);
    }

    #[test]
    fn u2_2_dev_toml_overrides_log_level() {
        let mut s = ServerSettings::default_for(Profile::Dev);
        s.apply(parse_default());
        // 模拟 dev.toml 覆写
        let dev_file: ConfigFile = toml::from_str(
            r#"
[log]
level = "debug"
"#,
        )
        .unwrap();
        s.apply(dev_file);
        assert_eq!(s.log.level, "debug");
        // 其他字段保持 default
        assert_eq!(s.server.port, 3000);
        assert_eq!(s.log.format, "json");
    }

    #[test]
    fn u2_3_env_overrides_toml_port() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let mut s = ServerSettings::default_for(Profile::Test);
        s.apply(parse_default());
        let test_file: ConfigFile = toml::from_str(
            r#"
[server]
port = 4000
"#,
        )
        .unwrap();
        s.apply(test_file);
        assert_eq!(s.server.port, 4000);
        env::set_var("APP__SERVER__PORT", "9999");
        s.apply_env_overrides();
        assert_eq!(s.server.port, 9999);
    }

    #[test]
    fn u2_4_prod_toml_log_format_json_and_environment() {
        let mut s = ServerSettings::default_for(Profile::Prod);
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
        // load_config_file 缺文件返回空 ConfigFile：apply 后字段保持原值
        let cfg = load_config_file(PathBuf::from(
            "/path/that/should/not/exist/__nope__.toml",
        ))
        .expect("missing file should not error");
        let mut s = ServerSettings::default_for(Profile::Staging);
        s.apply(parse_default());
        s.apply(cfg);
        assert_eq!(s.server.port, 3000);
    }

    // ──────────────────────────────────────────
    // U3.* 敏感字段 fail-fast
    // ──────────────────────────────────────────

    #[test]
    fn u3_1_require_jwt_secret_four_cases() {
        // 缺失
        assert!(require_jwt_secret(None).is_err());
        // 空白
        assert!(require_jwt_secret(Some(String::new())).is_err());
        assert!(require_jwt_secret(Some("   ".to_owned())).is_err());
        // 占位符
        assert!(require_jwt_secret(Some("change-me-in-production".to_owned())).is_err());
        assert!(require_jwt_secret(Some("CHANGE-ME-IN-PRODUCTION".to_owned())).is_err());
        // 合法
        assert_eq!(
            require_jwt_secret(Some("a-very-long-random-secret".to_owned())).unwrap(),
            "a-very-long-random-secret"
        );
    }

    #[test]
    fn u3_2_require_env_database_url_missing() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        env::remove_var("DATABASE_URL");
        let err = require_env("DATABASE_URL", "DATABASE_URL must be set; refused").unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("DATABASE_URL must be set"), "got: {msg}");
    }

    #[test]
    fn u3_3_require_env_database_url_blank_rejected() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        env::set_var("DATABASE_URL", "   ");
        let err = require_env("DATABASE_URL", "DATABASE_URL must be set").unwrap_err();
        env::remove_var("DATABASE_URL");
        assert!(format!("{err:#}").contains("DATABASE_URL must be set"));
    }

    #[test]
    fn u3_4_staging_redis_url_required() {
        // 直接验证 Profile::Staging.allow_redis_fallback() == false
        assert!(!Profile::Staging.allow_redis_fallback());
        assert!(!Profile::Prod.allow_redis_fallback());
        assert!(!Profile::Test.allow_redis_fallback());
    }

    #[test]
    fn u3_5_dev_allows_redis_fallback() {
        assert!(Profile::Dev.allow_redis_fallback());
    }

    // ──────────────────────────────────────────
    // U4.* ENV override 新增字段
    // ──────────────────────────────────────────

    #[test]
    fn u4_1_jwt_expire_secs_env_override() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let mut s = ServerSettings::default_for(Profile::Dev);
        env::set_var("APP__JWT__EXPIRE_SECS", "7200");
        s.apply_env_overrides();
        env::remove_var("APP__JWT__EXPIRE_SECS");
        assert_eq!(s.jwt.expire_secs, 7200);
    }

    #[test]
    fn u4_2_database_max_connections_env_override() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let mut s = ServerSettings::default_for(Profile::Dev);
        env::set_var("APP__DATABASE__MAX_CONNECTIONS", "20");
        s.apply_env_overrides();
        env::remove_var("APP__DATABASE__MAX_CONNECTIONS");
        assert_eq!(s.database.max_connections, 20);
    }

    #[test]
    fn u4_3_database_connect_timeout_env_override() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let mut s = ServerSettings::default_for(Profile::Dev);
        env::set_var("APP__DATABASE__CONNECT_TIMEOUT_SECS", "15");
        s.apply_env_overrides();
        env::remove_var("APP__DATABASE__CONNECT_TIMEOUT_SECS");
        assert_eq!(s.database.connect_timeout_secs, 15);
    }

    #[test]
    fn u4_4_invalid_port_keeps_toml_default() {
        let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
        clear_env();
        let mut s = ServerSettings::default_for(Profile::Dev);
        s.server.port = 4000;
        env::set_var("APP__SERVER__PORT", "not_a_number");
        s.apply_env_overrides();
        env::remove_var("APP__SERVER__PORT");
        // 解析失败：保留原值，不 fail
        assert_eq!(s.server.port, 4000);
    }

    // ──────────────────────────────────────────
    // U5.* 启动摘要日志脱敏
    // ──────────────────────────────────────────

    fn make_sample_settings() -> ServerSettings {
        let mut s = ServerSettings::default_for(Profile::Staging);
        s.database.url =
            Some("postgres://app_user:app_server_pass@stg-pg.example.com:5432/voice_room".to_owned());
        s.redis_url =
            Some("redis://default:supersecret@stg-redis.example.com:6379".to_owned());
        s.jwt_secret = "a-very-long-random-secret-1234567890".to_owned();
        s
    }

    #[test]
    fn u5_1_summary_no_jwt_secret_plaintext() {
        let s = make_sample_settings();
        let line = s.format_summary();
        assert!(
            !line.contains(&s.jwt_secret),
            "summary leaked jwt secret: {line}"
        );
        // 要求只输出长度
        assert!(line.contains("jwt.secret_len="));
    }

    #[test]
    fn u5_2_summary_redacts_database_password() {
        let s = make_sample_settings();
        let line = s.format_summary();
        assert!(!line.contains("app_server_pass"), "leaked db pass: {line}");
        assert!(!line.contains("supersecret"), "leaked redis pass: {line}");
        // host/port 仍可见，便于运维核对
        assert!(line.contains("stg-pg.example.com:5432"));
    }

    #[test]
    fn u5_3_summary_contains_profile_host_port() {
        let s = make_sample_settings();
        let line = s.format_summary();
        assert!(line.contains("profile=staging"), "missing profile: {line}");
        assert!(line.contains("server=0.0.0.0:3000"), "missing host:port: {line}");
    }

    // ──────────────────────────────────────────
    // 历史 require_jwt_secret 单测（0 回归）
    // ──────────────────────────────────────────

    #[test]
    fn require_jwt_secret_rejects_missing_env() {
        let err = require_jwt_secret(None).unwrap_err();
        assert!(format!("{err}").contains("JWT_SECRET"));
    }

    #[test]
    fn require_jwt_secret_rejects_empty_string() {
        assert!(require_jwt_secret(Some(String::new())).is_err());
        assert!(require_jwt_secret(Some("   ".to_owned())).is_err());
    }

    #[test]
    fn require_jwt_secret_rejects_placeholder_default() {
        assert!(require_jwt_secret(Some("change-me-in-production".to_owned())).is_err());
        assert!(require_jwt_secret(Some("CHANGE-ME-IN-PRODUCTION".to_owned())).is_err());
    }

    #[test]
    fn require_jwt_secret_accepts_real_secret() {
        let s = require_jwt_secret(Some(
            "a-very-long-random-base64-secret-1234567890".to_owned(),
        ))
        .unwrap();
        assert_eq!(s, "a-very-long-random-base64-secret-1234567890");
    }

    #[test]
    fn default_for_does_not_embed_hardcoded_secret() {
        let s = ServerSettings::default_for(Profile::Dev);
        assert!(s.jwt_secret.is_empty());
    }

    // redact_url 单测（脱敏函数）
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
