//! T-00040 集成测试：ServerSettings::load() 端到端启动行为
//!
//! 覆盖 TDS §3.2 I1~I3：
//! - I1：APP_PROFILE=staging + 临时 staging.toml + 全部必填 ENV → 成功
//! - I2：APP_PROFILE=staging + 无 staging.toml + 全部必填 ENV → 仍成功
//! - I3：APP_PROFILE=prod + 缺 DATABASE_URL → Err 含 "DATABASE_URL"

use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use voice_room_server::infrastructure::config::ServerSettings;

/// 串行化所有触碰进程级 ENV 的测试，避免并行污染。
fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// 清理 T-00040 关心的所有 ENV，避免上一个测试残留污染。
fn clear_relevant_env() {
    for k in [
        "APP_PROFILE",
        "APP_ENV",
        "APP__ENVIRONMENT",
        "APP_CONFIG_DIR",
        "APP_DOTENV_PATH",
        "DATABASE_URL",
        "REDIS_URL",
        "JWT_SECRET",
        "APP__SERVER__HOST",
        "APP__SERVER__PORT",
        "APP__LOG__LEVEL",
        "APP__LOG__FORMAT",
        "APP__JWT__EXPIRE_SECS",
        "APP__DATABASE__MAX_CONNECTIONS",
        "APP__DATABASE__CONNECT_TIMEOUT_SECS",
    ] {
        std::env::remove_var(k);
    }
}

fn make_tmp_config_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "voice-room-cfg-{}-{}-{}",
        tag,
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&dir).unwrap();
    // 写一个空的 default.toml 用于隔离 manifest 的 config/default.toml 影响
    fs::write(
        dir.join("default.toml"),
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
    .unwrap();
    // 创建一个空 .env 占位，避免 dotenvy 加载到仓库根 .env
    fs::write(dir.join(".env.empty"), "").unwrap();
    dir
}

#[test]
fn i1_load_with_staging_toml_and_all_env_succeeds() {
    let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
    clear_relevant_env();

    let dir = make_tmp_config_dir("i1");
    fs::write(
        dir.join("staging.toml"),
        r#"
[app]
environment = "staging"

[log]
level = "info"
format = "json"
"#,
    )
    .unwrap();

    std::env::set_var("APP_PROFILE", "staging");
    std::env::set_var("APP_CONFIG_DIR", &dir);
    std::env::set_var("APP_DOTENV_PATH", dir.join(".env.empty"));
    std::env::set_var("DATABASE_URL", "postgres://u:p@h:5432/db");
    std::env::set_var("REDIS_URL", "redis://r:6379");
    std::env::set_var("JWT_SECRET", "a-very-long-random-secret-1234567890");

    let settings = ServerSettings::load().expect("staging load should succeed");
    assert_eq!(settings.app.environment, "staging");
    assert_eq!(settings.log.format, "json");
    assert!(settings.database.url.is_some());
    assert!(settings.redis_url.is_some());
}

#[test]
fn i2_load_without_staging_toml_still_succeeds() {
    let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
    clear_relevant_env();

    let dir = make_tmp_config_dir("i2"); // 故意不创建 staging.toml

    std::env::set_var("APP_PROFILE", "staging");
    std::env::set_var("APP_CONFIG_DIR", &dir);
    std::env::set_var("APP_DOTENV_PATH", dir.join(".env.empty"));
    std::env::set_var("DATABASE_URL", "postgres://u:p@h:5432/db");
    std::env::set_var("REDIS_URL", "redis://r:6379");
    std::env::set_var("JWT_SECRET", "a-very-long-random-secret-1234567890");

    let settings = ServerSettings::load()
        .expect("staging load with missing staging.toml should still succeed (loader 容忍)");
    // profile 入口由 ENV 解析，覆盖 default.toml 内的 environment="dev"
    assert_eq!(settings.app.environment, "staging");
}

#[test]
fn i3_prod_missing_database_url_fails_with_hint() {
    let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
    clear_relevant_env();

    let dir = make_tmp_config_dir("i3");
    fs::write(
        dir.join("prod.toml"),
        r#"
[app]
environment = "prod"
"#,
    )
    .unwrap();

    std::env::set_var("APP_PROFILE", "prod");
    std::env::set_var("APP_CONFIG_DIR", &dir);
    std::env::set_var("APP_DOTENV_PATH", dir.join(".env.empty"));
    std::env::set_var("REDIS_URL", "redis://r:6379");
    std::env::set_var("JWT_SECRET", "a-very-long-random-secret-1234567890");
    // 故意不设 DATABASE_URL

    let err = ServerSettings::load().expect_err("prod 缺 DATABASE_URL 应 fail-fast");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("DATABASE_URL"),
        "error must mention DATABASE_URL, got: {msg}"
    );
}
