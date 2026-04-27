//! 模块 8 R1 P0-1/P0-2 集成测试：治理模块真实 PG/Redis 仓储装配验证
//!
//! 覆盖：
//!   - GR01: RealKickAuditDb.insert_kick_record → SELECT 字段回读（room_id/operator/target/reason）
//!   - GR02: RealMuteDb.insert_mute_record → SELECT mute_type 列存在且匹配（P0-2 列名对齐回归）
//!   - GR03: RealTransferAdminRepo.set_admin_user_id Some/None → SELECT rooms.admin_user_id 回读
//!   - GR04: RealKickRedis.set_kicked → get_kick_remaining_sec 返回接近 600s
//!   - GR05: RealMuteRedis.set_mute → get_mute_ttl 返回接近 ttl_secs；del_mute 后变 None
//!
//! 运行前提：DATABASE_URL 指向已迁移 PG，REDIS_URL（或默认 redis://127.0.0.1:6379）可达。
//! 未配置时单测自动跳过（事件写入 / Wallet 集成测试沿用同款骨架）。

mod common;

use std::time::Duration;

use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;
use voice_room_server::modules::governance::{
    kick::{KickAuditDb, KickRedis, RealKickAuditDb, RealKickRedis},
    mute::{MuteDb, MuteRedis, RealMuteDb, RealMuteRedis},
    transfer::{RealTransferAdminRepo, TransferAdminRepo},
};

// ─── 辅助函数 ──────────────────────────────────────────────────────────────────

async fn test_pool() -> Option<PgPool> {
    let url = std::env::var("DATABASE_URL").ok()?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .ok()?;
    common::run_migrations(&pool).await.ok()?;
    Some(pool)
}

fn redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

async fn redis_available(url: &str) -> bool {
    let Ok(client) = redis::Client::open(url) else {
        return false;
    };
    let Ok(mut conn) = client.get_multiplexed_async_connection().await else {
        return false;
    };
    redis::cmd("PING")
        .query_async::<String>(&mut conn)
        .await
        .is_ok()
}

async fn insert_user(pool: &PgPool, phone: &str) -> Uuid {
    let id: Uuid =
        sqlx::query_scalar("INSERT INTO users (phone, nickname) VALUES ($1, $2) RETURNING id")
            .bind(phone)
            .bind("gov-real-test")
            .fetch_one(pool)
            .await
            .expect("insert user");
    id
}

/// 插入测试房间，返回 room_id。
async fn insert_room(pool: &PgPool, owner_id: Uuid) -> Uuid {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO rooms (owner_id, title, room_type) VALUES ($1, $2, 'normal') RETURNING id",
    )
    .bind(owner_id)
    .bind("gov-real-test-room")
    .fetch_one(pool)
    .await
    .expect("insert room");
    id
}

async fn cleanup(pool: &PgPool, room_id: Uuid, user_ids: &[Uuid]) {
    let _ = sqlx::query("DELETE FROM room_kick_records WHERE room_id = $1")
        .bind(room_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM room_mute_records WHERE room_id = $1")
        .bind(room_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("UPDATE rooms SET admin_user_id = NULL WHERE id = $1")
        .bind(room_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM rooms WHERE id = $1")
        .bind(room_id)
        .execute(pool)
        .await;
    for uid in user_ids {
        let _ = sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(uid)
            .execute(pool)
            .await;
    }
}

// ─── GR01 ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn gr01_real_kick_audit_db_inserts_record() {
    let Some(pool) = test_pool().await else {
        eprintln!("GR01 skipped: DATABASE_URL not set");
        return;
    };
    let owner = insert_user(&pool, &format!("+19{}", &Uuid::new_v4().simple().to_string()[..14])).await;
    let target = insert_user(&pool, &format!("+19{}", &Uuid::new_v4().simple().to_string()[..14])).await;
    let room_id = insert_room(&pool, owner).await;

    let repo = RealKickAuditDb::new(pool.clone());
    repo.insert_kick_record(room_id, owner, target, "spamming")
        .await
        .expect("insert kick record");

    let row =
        sqlx::query("SELECT room_id, operator_user_id, target_user_id, reason FROM room_kick_records WHERE room_id = $1")
            .bind(room_id)
            .fetch_one(&pool)
            .await
            .expect("select kick");
    assert_eq!(row.get::<Uuid, _>("room_id"), room_id);
    assert_eq!(row.get::<Uuid, _>("operator_user_id"), owner);
    assert_eq!(row.get::<Uuid, _>("target_user_id"), target);
    assert_eq!(row.get::<String, _>("reason"), "spamming".to_string());

    cleanup(&pool, room_id, &[owner, target]).await;
}

// ─── GR02 ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn gr02_real_mute_db_uses_mute_type_column() {
    let Some(pool) = test_pool().await else {
        eprintln!("GR02 skipped: DATABASE_URL not set");
        return;
    };
    let owner = insert_user(&pool, &format!("+19{}", &Uuid::new_v4().simple().to_string()[..14])).await;
    let target = insert_user(&pool, &format!("+19{}", &Uuid::new_v4().simple().to_string()[..14])).await;
    let room_id = insert_room(&pool, owner).await;

    let repo = RealMuteDb::new(pool.clone());
    repo.insert_mute_record(room_id, owner, target, "mic", 300, "yelling")
        .await
        .expect("insert mute record (mute_type=mic)");

    // P0-2 关键回归：直接以 mute_type 列名查询，若仍为 type 会编译期成功但 PG 报错
    let row = sqlx::query(
        "SELECT mute_type, duration_sec, reason FROM room_mute_records WHERE room_id = $1",
    )
    .bind(room_id)
    .fetch_one(&pool)
    .await
    .expect("select mute by mute_type");
    assert_eq!(row.get::<String, _>("mute_type"), "mic".to_string());
    assert_eq!(row.get::<i32, _>("duration_sec"), 300);
    assert_eq!(row.get::<String, _>("reason"), "yelling".to_string());

    cleanup(&pool, room_id, &[owner, target]).await;
}

// ─── GR03 ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn gr03_real_transfer_admin_repo_round_trip() {
    let Some(pool) = test_pool().await else {
        eprintln!("GR03 skipped: DATABASE_URL not set");
        return;
    };
    let owner = insert_user(&pool, &format!("+19{}", &Uuid::new_v4().simple().to_string()[..14])).await;
    let admin_user = insert_user(&pool, &format!("+19{}", &Uuid::new_v4().simple().to_string()[..14])).await;
    let room_id = insert_room(&pool, owner).await;

    let repo = RealTransferAdminRepo::new(pool.clone());

    // 任命管理员
    repo.set_admin_user_id(room_id, Some(admin_user))
        .await
        .expect("set admin");
    let v: Option<Uuid> = sqlx::query_scalar("SELECT admin_user_id FROM rooms WHERE id = $1")
        .bind(room_id)
        .fetch_one(&pool)
        .await
        .expect("read admin_user_id");
    assert_eq!(v, Some(admin_user));

    // 撤销
    repo.set_admin_user_id(room_id, None)
        .await
        .expect("clear admin");
    let v2: Option<Uuid> = sqlx::query_scalar("SELECT admin_user_id FROM rooms WHERE id = $1")
        .bind(room_id)
        .fetch_one(&pool)
        .await
        .expect("read admin_user_id again");
    assert_eq!(v2, None);

    cleanup(&pool, room_id, &[owner, admin_user]).await;
}

// ─── GR04 ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn gr04_real_kick_redis_sets_and_reads_ttl() {
    let url = redis_url();
    if !redis_available(&url).await {
        eprintln!("GR04 skipped: REDIS unavailable");
        return;
    }
    let redis = RealKickRedis::new(&url).expect("kick redis client");
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    redis
        .set_kicked(room_id, user_id, "abuse")
        .await
        .expect("set_kicked");
    let ttl = redis
        .get_kick_remaining_sec(room_id, user_id)
        .await
        .expect("get ttl");
    let ttl = ttl.expect("kick key should exist");
    // 默认 600s，允许有少量误差
    assert!(
        (550..=600).contains(&ttl),
        "ttl out of range: {ttl}"
    );

    // 清理
    let mut conn = redis::Client::open(url.as_str())
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .unwrap();
    let _: () = redis::cmd("DEL")
        .arg(format!("voiceroom:kick:{}:{}", room_id, user_id))
        .query_async(&mut conn)
        .await
        .unwrap();
}

// ─── GR05 ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn gr05_real_mute_redis_set_get_del() {
    let url = redis_url();
    if !redis_available(&url).await {
        eprintln!("GR05 skipped: REDIS unavailable");
        return;
    }
    let redis = RealMuteRedis::new(&url).expect("mute redis client");
    let room_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    redis
        .set_mute("mic", room_id, user_id, 120, "loud")
        .await
        .expect("set_mute");
    let ttl = redis
        .get_mute_ttl("mic", room_id, user_id)
        .await
        .expect("get_mute_ttl");
    let ttl = ttl.expect("mute key should exist");
    assert!((100..=120).contains(&ttl), "ttl out of range: {ttl}");

    redis
        .del_mute("mic", room_id, user_id)
        .await
        .expect("del_mute");
    let after = redis
        .get_mute_ttl("mic", room_id, user_id)
        .await
        .expect("get_mute_ttl after del");
    assert!(after.is_none(), "mute key should be gone after del_mute");
}
