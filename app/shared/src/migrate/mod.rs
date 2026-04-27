//! T-0000M：双服务共库 Migration 表隔离工具
//!
//! 在共享 PostgreSQL 库中并行运行多个 sqlx 迁移源时，默认 `_sqlx_migrations`
//! 会被多套迁移文件互相覆盖与校验互掐。sqlx 0.8.x 既未提供
//! `Migrator::set_table_name`，`Migrator` 结构体也没有 `table_name` 字段
//! （详见 `sqlx-postgres-0.8.6/src/migrate.rs` L119-310，相关 SQL 中
//! `_sqlx_migrations` 表名为硬编码字符串）。本模块是 TDS §2.2 决议的 **保底方案**：
//!
//! - 复用 `sqlx::migrate!()` 宏在编译期嵌入的 `Migration` 列表（version /
//!   description / checksum / sql 全部由 sqlx 解析），
//! - 仅自管「迁移登记表」的建表/查询/写入 SQL，按调用方传入的 `table_name`
//!   注入；表名经 [`validate_table_name`] 严格校验（`^[A-Za-z_][A-Za-z0-9_]{0,62}$`），
//!   杜绝拼接型 SQL 注入。
//!
//! 引入此 helper 后：
//! - AppServer 用 `_sqlx_app_migrations`，AdminServer 用 `_sqlx_admin_migrations`，
//!   两服务在同一库内互不感知，启动顺序无关、重复启动幂等。
//! - 旧库残留的默认 `_sqlx_migrations` 表保持原样，不会污染新表。
//!
//! 演进：
//! - sqlx ≥ 0.9 一旦提供 `Migrator::set_table_name` / `with_table_name`，
//!   可以把本模块替换为薄封装。
//! - 切到 staging/prod「migrate-on-deploy」（TDS §6 阶段 C）时，本模块仍可作为
//!   sqlx-cli 的运行期等价物使用。

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::{Acquire, PgPool, Row};

/// 校验 sqlx 迁移登记表名，防止拼接 SQL 注入。
///
/// 接受 ASCII 字母 / 数字 / 下划线，且首字符不能是数字；长度 1..=63（PG 标识符上限）。
pub fn validate_table_name(name: &str) -> Result<(), MigrateTableError> {
    if name.is_empty() || name.len() > 63 {
        return Err(MigrateTableError::InvalidTableName(name.to_string()));
    }
    let mut chars = name.chars();
    let first = chars.next().expect("non-empty checked above");
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(MigrateTableError::InvalidTableName(name.to_string()));
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(MigrateTableError::InvalidTableName(name.to_string()));
    }
    Ok(())
}

/// helper 自有错误类型，包装 sqlx 的 [`MigrateError`] 并附加表名上下文，
/// 便于 N-2 / N-3 类用例从错误消息识别问题表。
#[derive(Debug, thiserror::Error)]
pub enum MigrateTableError {
    #[error("invalid migrations table name: {0:?}")]
    InvalidTableName(String),

    #[error(
        "migration {version} was previously applied (in `{table_name}`) but is missing in the resolved migrations"
    )]
    VersionMissing { table_name: String, version: i64 },

    #[error(
        "migration {version} (in `{table_name}`) checksum mismatch (file modified after apply)"
    )]
    VersionChecksumMismatch { table_name: String, version: i64 },

    #[error("sqlx migrate (table=`{table_name}`): {source}")]
    Sqlx {
        table_name: String,
        #[source]
        source: sqlx::Error,
    },

    #[error("sqlx migrate execute (table=`{table_name}`, version={version}): {source}")]
    ExecuteMigration {
        table_name: String,
        version: i64,
        #[source]
        source: sqlx::Error,
    },

    #[error(transparent)]
    Other(#[from] MigrateError),
}

impl MigrateTableError {
    fn sqlx(table_name: &str, source: sqlx::Error) -> Self {
        Self::Sqlx {
            table_name: table_name.to_string(),
            source,
        }
    }
}

/// 在 `pool` 上以自定义 `table_name` 执行 `migrator` 中编译期嵌入的全部迁移。
///
/// 行为对齐 sqlx 0.8 的 `Migrator::run`：
/// 1. 取 PG advisory lock（基于 table_name 派生 lock_id），并发安全。
/// 2. `CREATE TABLE IF NOT EXISTS <table_name>(...)`，schema 与 sqlx 默认一致。
/// 3. 列出已应用版本，校验「无遗失」「checksum 一致」（任一失败即报错）。
/// 4. 在事务内逐条 apply 未应用迁移 + INSERT 一行登记；失败回滚。
/// 5. 释放 advisory lock。
pub async fn run_migrations_with_table(
    pool: &PgPool,
    migrator: &Migrator,
    table_name: &str,
) -> Result<(), MigrateTableError> {
    validate_table_name(table_name)?;

    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| MigrateTableError::sqlx(table_name, e))?;

    // 1. advisory lock — 与 sqlx postgres 一致：使用 64-bit lock_id 的 advisory_lock。
    let lock_id = lock_id_for(table_name);
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(lock_id)
        .execute(&mut *conn)
        .await
        .map_err(|e| MigrateTableError::sqlx(table_name, e))?;

    let result = run_inner(&mut conn, migrator, table_name).await;

    // 无论成功失败都尝试解锁（连接归还连接池前不解锁会让其他实例被卡住）。
    let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(lock_id)
        .execute(&mut *conn)
        .await;

    result
}

async fn run_inner(
    conn: &mut sqlx::PgConnection,
    migrator: &Migrator,
    table_name: &str,
) -> Result<(), MigrateTableError> {
    // 2. ensure table — schema 严格对齐 sqlx-postgres-0.8.6/src/migrate.rs L119-126
    let create_sql = format!(
        "CREATE TABLE IF NOT EXISTS {table_name} (\n\
            version BIGINT PRIMARY KEY,\n\
            description TEXT NOT NULL,\n\
            installed_on TIMESTAMPTZ NOT NULL DEFAULT now(),\n\
            success BOOLEAN NOT NULL,\n\
            checksum BYTEA NOT NULL,\n\
            execution_time BIGINT NOT NULL\n\
        )"
    );
    sqlx::query(&create_sql)
        .execute(&mut *conn)
        .await
        .map_err(|e| MigrateTableError::sqlx(table_name, e))?;

    // 3. list applied
    let list_sql = format!("SELECT version, checksum FROM {table_name} ORDER BY version");
    let rows = sqlx::query(&list_sql)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| MigrateTableError::sqlx(table_name, e))?;
    let applied: HashMap<i64, Vec<u8>> = rows
        .into_iter()
        .map(|r| {
            let v: i64 = r.get(0);
            let c: Vec<u8> = r.get(1);
            (v, c)
        })
        .collect();

    // 4. validate — 缺失即视为「文件被回滚」按 N-3 触发显式错误（含表名）
    let known: HashSet<i64> = migrator.iter().map(|m| m.version).collect();
    for v in applied.keys() {
        if !known.contains(v) {
            return Err(MigrateTableError::VersionMissing {
                table_name: table_name.to_string(),
                version: *v,
            });
        }
    }
    for m in migrator.iter() {
        if let Some(applied_chk) = applied.get(&m.version) {
            if applied_chk.as_slice() != m.checksum.as_ref() {
                return Err(MigrateTableError::VersionChecksumMismatch {
                    table_name: table_name.to_string(),
                    version: m.version,
                });
            }
        }
    }

    // 5. apply pending
    let insert_sql = format!(
        "INSERT INTO {table_name} \
        (version, description, success, checksum, execution_time) \
        VALUES ($1, $2, TRUE, $3, $4)"
    );
    for m in migrator.iter() {
        if applied.contains_key(&m.version) {
            continue;
        }
        let start = Instant::now();
        let mut tx = conn
            .begin()
            .await
            .map_err(|e| MigrateTableError::sqlx(table_name, e))?;

        sqlx::raw_sql(m.sql.as_ref())
            .execute(&mut *tx)
            .await
            .map_err(|e| MigrateTableError::ExecuteMigration {
                table_name: table_name.to_string(),
                version: m.version,
                source: e,
            })?;

        let elapsed_ns = i64::try_from(start.elapsed().as_nanos()).unwrap_or(i64::MAX);
        sqlx::query(&insert_sql)
            .bind(m.version)
            .bind(m.description.as_ref())
            .bind(m.checksum.as_ref())
            .bind(elapsed_ns)
            .execute(&mut *tx)
            .await
            .map_err(|e| MigrateTableError::sqlx(table_name, e))?;

        tx.commit()
            .await
            .map_err(|e| MigrateTableError::sqlx(table_name, e))?;
    }

    Ok(())
}

/// 基于 table_name 派生 64-bit lock_id（FNV-1a 64-bit），避免与 sqlx 默认
/// 按 database name 派生的 lock 冲突。
fn lock_id_for(table_name: &str) -> i64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in table_name.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0001_0000_01b3);
    }
    hash as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_accepts_standard_names() {
        validate_table_name("_sqlx_app_migrations").unwrap();
        validate_table_name("_sqlx_admin_migrations").unwrap();
        validate_table_name("a").unwrap();
        validate_table_name("Mig_2").unwrap();
    }

    #[test]
    fn validate_rejects_empty() {
        assert!(matches!(
            validate_table_name(""),
            Err(MigrateTableError::InvalidTableName(_))
        ));
    }

    #[test]
    fn validate_rejects_leading_digit() {
        assert!(matches!(
            validate_table_name("1bad"),
            Err(MigrateTableError::InvalidTableName(_))
        ));
    }

    #[test]
    fn validate_rejects_special_chars() {
        for bad in [
            "bad-name", "bad name", "bad;DROP", "bad\"x", "bad'x", "bad.x",
        ] {
            assert!(
                matches!(
                    validate_table_name(bad),
                    Err(MigrateTableError::InvalidTableName(_))
                ),
                "{bad} should be rejected"
            );
        }
    }

    #[test]
    fn validate_rejects_overlong() {
        let long = "a".repeat(64);
        assert!(matches!(
            validate_table_name(&long),
            Err(MigrateTableError::InvalidTableName(_))
        ));
    }

    #[test]
    fn validate_accepts_max_length() {
        let max = "a".repeat(63);
        validate_table_name(&max).unwrap();
    }

    #[test]
    fn lock_id_is_deterministic_and_unique_per_name() {
        let a = lock_id_for("_sqlx_app_migrations");
        let b = lock_id_for("_sqlx_admin_migrations");
        assert_eq!(a, lock_id_for("_sqlx_app_migrations"));
        assert_ne!(a, b);
    }
}
