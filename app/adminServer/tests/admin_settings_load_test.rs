//! T-10020 集成测试：AdminSettings::load() 端到端启动行为
//!
//! 覆盖 TDS §3.2 I1~I5：
//! - I1：ADMIN_PROFILE=staging + 临时 staging.toml + 全部必填 ENV → 成功
//! - I2：ADMIN_PROFILE=staging + 无 staging.toml + 全部必填 ENV → 仍成功
//! - I3：ADMIN_PROFILE=prod + 缺 DATABASE_URL → Err 含 "DATABASE_URL"
//! - I4：ADMIN_PROFILE=prod + 缺 REDIS_URL → Err 含 "REDIS_URL must be set for non-dev profile"
//! - I5：ADMIN_PROFILE=dev + 缺 REDIS_URL → Ok，settings.redis_url is None（D-A1）

use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use voice_room_admin_server::infrastructure::config::AdminSettings;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn clear_relevant_env() {
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
        std::env::remove_var(k);
    }
}

fn make_tmp_config_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "voice-room-admin-cfg-{}-{}-{}",
        tag,
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("default.toml"),
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
    .unwrap();
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

    std::env::set_var("ADMIN_PROFILE", "staging");
    std::env::set_var("ADMIN_CONFIG_DIR", &dir);
    std::env::set_var("ADMIN_DOTENV_PATH", dir.join(".env.empty"));
    std::env::set_var("DATABASE_URL", "postgres://u:p@h:5432/db");
    std::env::set_var("REDIS_URL", "redis://r:6379");
    std::env::set_var("JWT_SECRET", "a-very-long-random-secret-1234567890");

    let settings = AdminSettings::load().expect("staging load should succeed");
    assert_eq!(settings.app.environment, "staging");
    assert_eq!(settings.app.name, "voice-room-admin-server");
    assert_eq!(settings.server.port, 3001);
    assert_eq!(settings.log.format, "json");
    assert!(settings.database.url.is_some());
    assert!(settings.redis_url.is_some());
}

#[test]
fn i2_load_without_staging_toml_still_succeeds() {
    let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
    clear_relevant_env();

    let dir = make_tmp_config_dir("i2"); // 故意不创建 staging.toml

    std::env::set_var("ADMIN_PROFILE", "staging");
    std::env::set_var("ADMIN_CONFIG_DIR", &dir);
    std::env::set_var("ADMIN_DOTENV_PATH", dir.join(".env.empty"));
    std::env::set_var("DATABASE_URL", "postgres://u:p@h:5432/db");
    std::env::set_var("REDIS_URL", "redis://r:6379");
    std::env::set_var("JWT_SECRET", "a-very-long-random-secret-1234567890");

    let settings = AdminSettings::load()
        .expect("staging load with missing staging.toml should still succeed (loader 容忍)");
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

    std::env::set_var("ADMIN_PROFILE", "prod");
    std::env::set_var("ADMIN_CONFIG_DIR", &dir);
    std::env::set_var("ADMIN_DOTENV_PATH", dir.join(".env.empty"));
    std::env::set_var("REDIS_URL", "redis://r:6379");
    std::env::set_var("JWT_SECRET", "a-very-long-random-secret-1234567890");

    let err = AdminSettings::load().expect_err("prod 缺 DATABASE_URL 应 fail-fast");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("DATABASE_URL"),
        "error must mention DATABASE_URL, got: {msg}"
    );
}

#[test]
fn i4_prod_missing_redis_url_fails_with_non_dev_hint() {
    let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
    clear_relevant_env();

    let dir = make_tmp_config_dir("i4");
    fs::write(
        dir.join("prod.toml"),
        r#"
[app]
environment = "prod"
"#,
    )
    .unwrap();

    std::env::set_var("ADMIN_PROFILE", "prod");
    std::env::set_var("ADMIN_CONFIG_DIR", &dir);
    std::env::set_var("ADMIN_DOTENV_PATH", dir.join(".env.empty"));
    std::env::set_var("DATABASE_URL", "postgres://u:p@h:5432/db");
    std::env::set_var("JWT_SECRET", "a-very-long-random-secret-1234567890");

    let err = AdminSettings::load().expect_err("prod 缺 REDIS_URL 应 fail-fast");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("REDIS_URL must be set for non-dev profile"),
        "got: {msg}"
    );
    assert!(msg.contains("profile=prod"), "missing profile hint: {msg}");
}

#[test]
fn i5_dev_missing_redis_url_returns_none() {
    let _g = env_lock().lock().unwrap_or_else(|e| e.into_inner());
    clear_relevant_env();

    let dir = make_tmp_config_dir("i5");

    std::env::set_var("ADMIN_PROFILE", "dev");
    std::env::set_var("ADMIN_CONFIG_DIR", &dir);
    std::env::set_var("ADMIN_DOTENV_PATH", dir.join(".env.empty"));
    std::env::set_var("DATABASE_URL", "postgres://u:p@h:5432/db");
    std::env::set_var("JWT_SECRET", "a-very-long-random-secret-1234567890");
    // 故意不设 REDIS_URL — D-A1 决策：dev 允许缺失 → None

    let settings = AdminSettings::load().expect("dev load without REDIS_URL should succeed");
    assert!(
        settings.redis_url.is_none(),
        "expected redis_url=None on dev profile"
    );
    assert_eq!(settings.app.environment, "dev");
}
