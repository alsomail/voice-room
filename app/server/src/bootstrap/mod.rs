use std::sync::Arc;

use axum::{extract::Extension, middleware, routing::get, Json, Router};
use serde::Serialize;

use crate::{
    common::RequestContext,
    core::analytics::writer::EventWriterPort,
    infrastructure::{
        logging::request_context_middleware, redis_store::SmsCodeStore,
        third_party::sms::SmsProvider,
    },
    modules::{
        auth::{auth_routes, repository::UserRepository, service::AuthService},
        chat::{chat_routes, ChatRepository},
        events::events_routes,
        gift::{gift_routes, send_gift::SendGiftServicePort, service::GiftServicePort},
        governance::kick::{KickAuditDb, KickRedis},
        governance::mute::{MuteDb, MuteRedis},
        governance::transfer::TransferAdminRepo,
        ranking::{ranking_routes, RankingServicePort},
        room::{password::RoomPasswordRedis, repository::RoomRepository, room_routes, RoomService},
        wallet::{service::WalletServicePort, wallet_routes},
    },
    room::RoomManager,
    stats::StatsPort,
    ws::{ws_handler, ConnectionRegistry},
};

#[derive(Clone)]
pub struct AppState {
    pub auth_service: Arc<AuthService>,
    pub room_service: Arc<RoomService>,
    pub jwt_secret: String,
    /// WebSocket 連接注冊表（全局共享）
    pub ws_registry: Arc<ConnectionRegistry>,
    /// 在線統計服務（HyperLogLog + Set）
    pub stats_service: Arc<dyn StatsPort>,
    /// 房间运行时状态管理器（内存 DashMap）
    pub room_manager: Arc<RoomManager>,
    /// 钱包服务（余额查询、流水列表）
    pub wallet_service: Arc<dyn WalletServicePort>,
    /// 礼物配置服务（列表查询 + 内存缓存）
    pub gift_service: Arc<dyn GiftServicePort>,
    /// 送礼服务（T-00020 SendGift 事务 + 广播）
    pub send_gift_service: Arc<dyn SendGiftServicePort>,
    /// 榜单服务（T-00021 魅力/财富榜单 API）
    pub ranking_service: Arc<dyn RankingServicePort>,
    /// 事件写入服务（T-00022 埋点批量写入）
    pub event_writer: Arc<dyn EventWriterPort>,
    /// 密码房 Redis 操作（T-00026 失败计数 + 锁定）
    pub room_password_redis: Arc<dyn RoomPasswordRedis>,
    /// 踢人冷却 Redis（T-00028）
    pub kick_redis: Arc<dyn KickRedis>,
    /// 踢人审计 DB（T-00028）
    pub kick_audit_db: Arc<dyn KickAuditDb>,
    /// 禁麦/禁言 Redis（T-00029）
    pub mute_redis: Arc<dyn MuteRedis>,
    /// 禁麦/禁言审计 DB（T-00029）
    pub mute_db: Arc<dyn MuteDb>,
    /// 抢麦分布式锁（T-00014 #4 / P2-12）
    pub mic_lock: Arc<dyn crate::room::mic_lock::MicLock>,
    /// 管理员任命 DB（T-00030 TransferAdmin 信令）
    pub transfer_admin_repo: Arc<dyn TransferAdminRepo>,
    /// 聊天消息持久化（T-00043 SendMessage 落库 + REST 历史查询）
    pub chat_repo: Arc<dyn ChatRepository>,
}

impl AppState {
    /// 测试 / `test-utils` feature 专用构造器：使用进程内 Fake 实现初始化所有
    /// governance / password / mic_lock 后端依赖。
    ///
    /// **生产路径请勿使用** — 调用 `new_with_managers` 注入真实仓储。
    #[cfg(any(test, feature = "test-utils"))]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        code_store: Arc<dyn SmsCodeStore>,
        sms: Arc<dyn SmsProvider>,
        jwt_secret: String,
        room_repo: Arc<dyn RoomRepository>,
        stats_service: Arc<dyn StatsPort>,
        wallet_service: Arc<dyn WalletServicePort>,
        gift_service: Arc<dyn GiftServicePort>,
        send_gift_service: Arc<dyn SendGiftServicePort>,
        ranking_service: Arc<dyn RankingServicePort>,
        event_writer: Arc<dyn EventWriterPort>,
    ) -> Self {
        let auth_service = Arc::new(AuthService::new(
            user_repo,
            code_store,
            sms,
            jwt_secret.clone(),
        ));
        let room_service = Arc::new(RoomService::new(room_repo));
        Self {
            auth_service,
            room_service,
            jwt_secret,
            ws_registry: Arc::new(ConnectionRegistry::new()),
            stats_service,
            room_manager: Arc::new(RoomManager::new()),
            wallet_service,
            gift_service,
            send_gift_service,
            ranking_service,
            event_writer,
            room_password_redis: Arc::new(crate::modules::room::FakeRoomPasswordRedis::default()),
            kick_redis: Arc::new(crate::modules::governance::kick::FakeKickRedis::default()),
            kick_audit_db: Arc::new(crate::modules::governance::kick::FakeKickAuditDb::default()),
            mute_redis: Arc::new(crate::modules::governance::mute::FakeMuteRedis::default()),
            mute_db: Arc::new(crate::modules::governance::mute::FakeMuteDb::default()),
            mic_lock: Arc::new(crate::room::mic_lock::FakeMicLock::default()),
            transfer_admin_repo: Arc::new(
                crate::modules::governance::transfer::FakeTransferAdminRepo::default(),
            ),
            chat_repo: Arc::new(crate::modules::chat::FakeChatRepository::default()),
        }
    }

    /// 生产环境装配入口 — 调用方必须显式注入 **全部** 治理 / 密码房 / 抢麦锁
    /// 真实仓储。任何 Fake* 实现 **不会** 出现在该构造器编译产物中（参见
    /// `Cargo.toml [features] test-utils`）。
    ///
    /// `main.rs` 在启动期完成一次装配；DB / Redis 连接失败 → fail-fast。
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_managers(
        user_repo: Arc<dyn UserRepository>,
        code_store: Arc<dyn SmsCodeStore>,
        sms: Arc<dyn SmsProvider>,
        jwt_secret: String,
        room_repo: Arc<dyn RoomRepository>,
        stats_service: Arc<dyn StatsPort>,
        wallet_service: Arc<dyn WalletServicePort>,
        gift_service: Arc<dyn GiftServicePort>,
        send_gift_service: Arc<dyn SendGiftServicePort>,
        ranking_service: Arc<dyn RankingServicePort>,
        ws_registry: Arc<ConnectionRegistry>,
        room_manager: Arc<RoomManager>,
        event_writer: Arc<dyn EventWriterPort>,
        // ── R1 P0-1: 治理 / 密码房 / 抢麦锁仓储均为必填，生产装配显式注入 ──
        room_password_redis: Arc<dyn RoomPasswordRedis>,
        kick_redis: Arc<dyn KickRedis>,
        kick_audit_db: Arc<dyn KickAuditDb>,
        mute_redis: Arc<dyn MuteRedis>,
        mute_db: Arc<dyn MuteDb>,
        mic_lock: Arc<dyn crate::room::mic_lock::MicLock>,
        transfer_admin_repo: Arc<dyn TransferAdminRepo>,
        chat_repo: Arc<dyn ChatRepository>,
    ) -> Self {
        let auth_service = Arc::new(AuthService::new(
            user_repo,
            code_store,
            sms,
            jwt_secret.clone(),
        ));
        let room_service = Arc::new(RoomService::new(room_repo));
        Self {
            auth_service,
            room_service,
            jwt_secret,
            ws_registry,
            stats_service,
            room_manager,
            wallet_service,
            gift_service,
            send_gift_service,
            ranking_service,
            event_writer,
            room_password_redis,
            kick_redis,
            kick_audit_db,
            mute_redis,
            mute_db,
            mic_lock,
            transfer_admin_repo,
            chat_repo,
        }
    }

    /// 设置生产环境真实 Redis（用于密码房校验），替换默认的 FakeRoomPasswordRedis。
    pub fn with_room_password_redis(mut self, redis: Arc<dyn RoomPasswordRedis>) -> Self {
        self.room_password_redis = redis;
        self
    }

    /// 设置生产环境真实 KickRedis（T-00028）。
    pub fn with_kick_redis(mut self, redis: Arc<dyn KickRedis>) -> Self {
        self.kick_redis = redis;
        self
    }

    /// 设置生产环境真实 KickAuditDb（T-00028）。
    pub fn with_kick_audit_db(mut self, db: Arc<dyn KickAuditDb>) -> Self {
        self.kick_audit_db = db;
        self
    }

    /// 设置生产环境真实 MuteRedis（T-00029）。
    pub fn with_mute_redis(mut self, redis: Arc<dyn MuteRedis>) -> Self {
        self.mute_redis = redis;
        self
    }

    /// 设置生产环境真实 MuteDb（T-00029）。
    pub fn with_mute_db(mut self, db: Arc<dyn MuteDb>) -> Self {
        self.mute_db = db;
        self
    }

    /// 设置生产环境真实 MicLock（T-00014 #4 / P2-12）。
    pub fn with_mic_lock(mut self, lock: Arc<dyn crate::room::mic_lock::MicLock>) -> Self {
        self.mic_lock = lock;
        self
    }

    /// 设置生产环境真实 TransferAdminRepo（T-00030）。
    pub fn with_transfer_admin_repo(mut self, repo: Arc<dyn TransferAdminRepo>) -> Self {
        self.transfer_admin_repo = repo;
        self
    }

    /// 设置生产环境真实 ChatRepository（T-00043）。
    pub fn with_chat_repo(mut self, repo: Arc<dyn ChatRepository>) -> Self {
        self.chat_repo = repo;
        self
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub fn for_test() -> Self {
        use crate::core::analytics::writer::FakeEventWriter;
        use crate::infrastructure::{
            redis_store::FakeCodeStore, third_party::sms::MockSmsProvider,
        };
        use crate::modules::auth::repository::FakeUserRepository;
        use crate::modules::gift::send_gift::FakeSendGiftService;
        use crate::modules::gift::service::FakeGiftService;
        use crate::modules::ranking::FakeRankingService;
        use crate::modules::room::FakeRoomRepository;
        use crate::modules::wallet::service::FakeWalletService;
        use crate::stats::FakeStatsService;
        Self::new(
            Arc::new(FakeUserRepository::default()),
            Arc::new(FakeCodeStore::default()),
            Arc::new(MockSmsProvider),
            "test-secret".to_string(),
            Arc::new(FakeRoomRepository::default()),
            Arc::new(FakeStatsService::default()),
            Arc::new(FakeWalletService),
            Arc::new(FakeGiftService),
            Arc::new(FakeSendGiftService),
            Arc::new(FakeRankingService),
            Arc::new(FakeEventWriter::default()),
        )
    }

    /// 測試輔助：注入預置數據的 FakeRoomRepository（用于集成測試 T-00009）
    #[cfg(any(test, feature = "test-utils"))]
    pub fn for_test_with_room_repo(
        room_repo: Arc<crate::modules::room::FakeRoomRepository>,
    ) -> Self {
        use crate::core::analytics::writer::FakeEventWriter;
        use crate::infrastructure::{
            redis_store::FakeCodeStore, third_party::sms::MockSmsProvider,
        };
        use crate::modules::auth::repository::FakeUserRepository;
        use crate::modules::gift::send_gift::FakeSendGiftService;
        use crate::modules::gift::service::FakeGiftService;
        use crate::modules::ranking::FakeRankingService;
        use crate::modules::wallet::service::FakeWalletService;
        use crate::stats::FakeStatsService;
        Self::new(
            Arc::new(FakeUserRepository::default()),
            Arc::new(FakeCodeStore::default()),
            Arc::new(MockSmsProvider),
            "test-secret".to_string(),
            room_repo,
            Arc::new(FakeStatsService::default()),
            Arc::new(FakeWalletService),
            Arc::new(FakeGiftService),
            Arc::new(FakeSendGiftService),
            Arc::new(FakeRankingService),
            Arc::new(FakeEventWriter::default()),
        )
    }

    /// 測試輔助：注入真實 WalletService（DB 集成測試 T-00018）
    /// LOW-1: 参数类型改为 Arc<dyn WalletServicePort>（接口抽象，而非具体类型）
    #[cfg(any(test, feature = "test-utils"))]
    pub fn for_test_with_wallet(wallet_service: Arc<dyn WalletServicePort>) -> Self {
        use crate::core::analytics::writer::FakeEventWriter;
        use crate::infrastructure::{
            redis_store::FakeCodeStore, third_party::sms::MockSmsProvider,
        };
        use crate::modules::auth::repository::FakeUserRepository;
        use crate::modules::gift::send_gift::FakeSendGiftService;
        use crate::modules::gift::service::FakeGiftService;
        use crate::modules::ranking::FakeRankingService;
        use crate::modules::room::FakeRoomRepository;
        use crate::stats::FakeStatsService;
        Self::new(
            Arc::new(FakeUserRepository::default()),
            Arc::new(FakeCodeStore::default()),
            Arc::new(MockSmsProvider),
            "test-secret".to_string(),
            Arc::new(FakeRoomRepository::default()),
            Arc::new(FakeStatsService::default()),
            wallet_service,
            Arc::new(FakeGiftService),
            Arc::new(FakeSendGiftService),
            Arc::new(FakeRankingService),
            Arc::new(FakeEventWriter::default()),
        )
    }

    /// 測試輔助：注入真實 EventWriter（DB 集成測試 T-00022）
    #[cfg(any(test, feature = "test-utils"))]
    pub fn for_test_with_event_writer(event_writer: Arc<dyn EventWriterPort>) -> Self {
        use crate::infrastructure::{
            redis_store::FakeCodeStore, third_party::sms::MockSmsProvider,
        };
        use crate::modules::auth::repository::FakeUserRepository;
        use crate::modules::gift::send_gift::FakeSendGiftService;
        use crate::modules::gift::service::FakeGiftService;
        use crate::modules::ranking::FakeRankingService;
        use crate::modules::room::FakeRoomRepository;
        use crate::modules::wallet::service::FakeWalletService;
        use crate::stats::FakeStatsService;
        Self::new(
            Arc::new(FakeUserRepository::default()),
            Arc::new(FakeCodeStore::default()),
            Arc::new(MockSmsProvider),
            "test-secret".to_string(),
            Arc::new(FakeRoomRepository::default()),
            Arc::new(FakeStatsService::default()),
            Arc::new(FakeWalletService),
            Arc::new(FakeGiftService),
            Arc::new(FakeSendGiftService),
            Arc::new(FakeRankingService),
            event_writer,
        )
    }
}

pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/ping", get(ping))
        .route("/health", get(health))
        .route("/ws", get(ws_handler))
        .merge(auth_routes())
        .merge(chat_routes())
        .merge(room_routes())
        .merge(wallet_routes())
        .merge(gift_routes())
        .merge(ranking_routes())
        .merge(events_routes())
        .layer(middleware::from_fn(request_context_middleware))
        .with_state(state)
}

#[derive(Serialize)]
struct PingResponse {
    status: &'static str,
    request_id: String,
}

async fn ping(Extension(request_context): Extension<RequestContext>) -> Json<PingResponse> {
    tracing::info!(request_id = %request_context.request_id(), "handled ping request");

    Json(PingResponse {
        status: "ok",
        request_id: request_context.request_id().to_owned(),
    })
}

// ─── T-0000N: 统一 /health 端点 ────────────────────────────────────────────────
//
// 与 `/ping` 同层挂载，零鉴权、零 AppState 读取、零下游探测。
// 用于 wait-on / preflight / 监控探针；语义 = 「进程存活并能响应 HTTP」。

/// `/health` 响应体。结构由协议约束（status/service/version 三字段）。
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
}

/// AppServer 健康探活 handler，纯静态 JSON，零参数零依赖。
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "app-server",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::*;

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// H-01: 错误响应 body 中的 request_id 应与 X-Request-Id header 一致
    #[tokio::test]
    async fn error_response_body_contains_request_id() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/verification-codes")
                    .header("content-type", "application/json")
                    .header("x-request-id", "test-req-id-42")
                    .body(Body::from(r#"{"phone":"invalid"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let json = body_json(response).await;
        assert_eq!(
            json["request_id"], "test-req-id-42",
            "error body must echo the X-Request-Id"
        );
    }

    /// 成功响应中 request_id 已正确（回归保护）
    #[tokio::test]
    async fn success_response_body_contains_request_id() {
        // 直接构造一个 send-code 请求（MockSmsProvider 不报错）
        let response = build_app(AppState::for_test())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/verification-codes")
                    .header("content-type", "application/json")
                    .header("x-request-id", "req-ok-1")
                    .body(Body::from(r#"{"phone":"+8613800138000"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["request_id"], "req-ok-1");
    }

    // ── C-02: 无 token 访问 POST /api/v1/rooms → 401 ─────────────────────────

    /// C-02: 未携带 Authorization header → HTTP 401
    #[tokio::test]
    async fn c02_create_room_no_token_returns_401() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("content-type", "application/json")
                    .header("x-request-id", "req-room-401")
                    .body(Body::from(r#"{"title":"Test","room_type":"normal"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["code"], 40101, "should be Unauthorized error code");
    }

    // ── C-12: request_id 透传 ────────────────────────────────────────────────

    /// C-12: 无 token 时错误响应中的 request_id 必须与 X-Request-Id header 一致
    #[tokio::test]
    async fn c12_create_room_request_id_echoed_in_error_response() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/rooms")
                    .header("content-type", "application/json")
                    .header("x-request-id", "room-req-id-xyz")
                    .body(Body::from(r#"{"title":"Test","room_type":"normal"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            json["request_id"], "room-req-id-xyz",
            "request_id in error body must match X-Request-Id header"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // T-00008: GET /api/v1/rooms 集成测试（L-20 ~ L-24）
    // ═══════════════════════════════════════════════════════════════════════

    /// L-20: GET /api/v1/rooms 无数据时返回 200 + 空 items
    #[tokio::test]
    async fn l20_list_rooms_empty_returns_200() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/rooms")
                    .header("x-request-id", "list-req-001")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["total"], 0);
        assert_eq!(json["data"]["page"], 1);
        assert_eq!(json["data"]["size"], 20);
        assert!(json["data"]["items"].as_array().unwrap().is_empty());
    }

    /// L-21: GET /api/v1/rooms?size=200 → HTTP 400（超出上限）
    #[tokio::test]
    async fn l21_list_rooms_size_200_returns_400() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/rooms?size=200")
                    .header("x-request-id", "list-req-002")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let json = body_json(response).await;
        assert_eq!(json["code"], 40003, "should be ValidationError code");
    }

    /// L-22: GET /api/v1/rooms?page=0 → HTTP 400
    #[tokio::test]
    async fn l22_list_rooms_page_0_returns_400() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/rooms?page=0")
                    .header("x-request-id", "list-req-003")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let json = body_json(response).await;
        assert_eq!(json["code"], 40003, "should be ValidationError code");
    }

    /// L-23: request_id 透传 — GET /api/v1/rooms 响应中 request_id 与 header 一致
    #[tokio::test]
    async fn l23_list_rooms_request_id_echoed() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/rooms")
                    .header("x-request-id", "list-trace-abc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(
            json["request_id"], "list-trace-abc",
            "request_id must be echoed from X-Request-Id header"
        );
    }

    /// L-24: 无 Authorization header 也返回 200（list_rooms 不需要鉴权）
    #[tokio::test]
    async fn l24_list_rooms_no_auth_returns_200() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/rooms")
                    // 故意不携带 Authorization header
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "list_rooms must be accessible without authentication"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // T-00009: GET /api/v1/rooms/:id 集成测试（D-20 ~ D-27）
    // ═══════════════════════════════════════════════════════════════════════

    /// 测试辅助：构建带有预置 active 房间的 app，返回 (app, room_id_string)
    fn build_app_with_active_room() -> (axum::Router, String) {
        use crate::modules::room::FakeRoomRepository;
        use chrono::Utc;
        use uuid::Uuid;
        use voice_room_shared::models::room::RoomModel;

        let repo = std::sync::Arc::new(FakeRoomRepository::default());
        let owner_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();

        repo.seed_user(
            owner_id,
            "Owner User".to_string(),
            Some("https://img.example.com/av.png".to_string()),
        );
        let now = Utc::now();
        repo.seed(RoomModel {
            id: room_id,
            owner_id,
            title: "Live Room".to_string(),
            room_type: "normal".to_string(),
            member_count: 3,
            status: "active".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });

        let app = build_app(AppState::for_test_with_room_repo(repo));
        (app, room_id.to_string())
    }

    /// D-20: GET /api/v1/rooms/:id 房间存在 → 200 + 正确 data
    #[tokio::test]
    async fn d20_get_room_exists_returns_200() {
        let (app, room_id) = build_app_with_active_room();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/rooms/{room_id}"))
                    .header("x-request-id", "detail-req-001")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["code"], 0);
        assert_eq!(json["data"]["room_id"], room_id);
        assert_eq!(json["data"]["title"], "Live Room");
        assert_eq!(json["data"]["room_type"], "normal");
        assert_eq!(json["data"]["member_count"], 3);
        assert_eq!(json["data"]["max_members"], 50);
        assert_eq!(json["data"]["owner"]["nickname"], "Owner User");
    }

    /// D-21: 房间不存在 → 404，code=40400
    #[tokio::test]
    async fn d21_get_room_not_found_returns_404() {
        let app = build_app(AppState::for_test());
        let nonexistent_id = uuid::Uuid::new_v4();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/rooms/{nonexistent_id}"))
                    .header("x-request-id", "detail-req-002")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let json = body_json(response).await;
        assert_eq!(json["code"], 40400, "should be NotFound error code");
    }

    /// D-22: GET /api/v1/rooms/not-a-uuid → 400，code=40003
    #[tokio::test]
    async fn d22_get_room_invalid_uuid_returns_400() {
        let app = build_app(AppState::for_test());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/rooms/not-a-uuid")
                    .header("x-request-id", "detail-req-003")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let json = body_json(response).await;
        assert_eq!(json["code"], 40003, "should be ValidationError code");
    }

    /// D-23: closed 房间 → 404
    #[tokio::test]
    async fn d23_get_closed_room_returns_404() {
        use crate::modules::room::FakeRoomRepository;
        use chrono::Utc;
        use uuid::Uuid;
        use voice_room_shared::models::room::RoomModel;

        let repo = std::sync::Arc::new(FakeRoomRepository::default());
        let owner_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();
        repo.seed_user(owner_id, "Owner".to_string(), None);
        let now = Utc::now();
        repo.seed(RoomModel {
            id: room_id,
            owner_id,
            title: "Closed Room".to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            status: "closed".to_string(), // 已关闭
            password_hash: None,
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });

        let app = build_app(AppState::for_test_with_room_repo(repo));
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/rooms/{room_id}"))
                    .header("x-request-id", "detail-req-004")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let json = body_json(response).await;
        assert_eq!(json["code"], 40400);
    }

    /// D-24: soft-deleted 房间 → 404
    #[tokio::test]
    async fn d24_get_soft_deleted_room_returns_404() {
        use crate::modules::room::FakeRoomRepository;
        use chrono::Utc;
        use uuid::Uuid;
        use voice_room_shared::models::room::RoomModel;

        let repo = std::sync::Arc::new(FakeRoomRepository::default());
        let owner_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();
        repo.seed_user(owner_id, "Owner".to_string(), None);
        let now = Utc::now();
        repo.seed(RoomModel {
            id: room_id,
            owner_id,
            title: "Deleted Room".to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            status: "active".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at: Some(now), // 已软删除
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });

        let app = build_app(AppState::for_test_with_room_repo(repo));
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/rooms/{room_id}"))
                    .header("x-request-id", "detail-req-005")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let json = body_json(response).await;
        assert_eq!(json["code"], 40400);
    }

    /// D-25: 无 Authorization header → 200（公开接口）
    #[tokio::test]
    async fn d25_get_room_no_auth_returns_200() {
        let (app, room_id) = build_app_with_active_room();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/rooms/{room_id}"))
                    // 故意不携带 Authorization header
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::OK,
            "get_room must be accessible without authentication"
        );
    }

    /// D-26: request_id 透传 — 响应中 request_id 与 X-Request-Id header 一致
    #[tokio::test]
    async fn d26_get_room_request_id_echoed() {
        let (app, room_id) = build_app_with_active_room();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/rooms/{room_id}"))
                    .header("x-request-id", "trace-xyz-789")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(
            json["request_id"], "trace-xyz-789",
            "request_id must be echoed from X-Request-Id header"
        );
    }

    /// D-27: mic_slots 为空数组
    #[tokio::test]
    async fn d27_get_room_mic_slots_is_empty_array() {
        let (app, room_id) = build_app_with_active_room();
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/rooms/{room_id}"))
                    .header("x-request-id", "detail-req-027")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert!(
            json["data"]["mic_slots"].as_array().unwrap().is_empty(),
            "mic_slots should be empty array in MVP"
        );
    }

    // ═══════════════════════════════════════════════════════════════════════
    // T-00010: DELETE /api/v1/rooms/:id 集成测试（I-C-01 ~ I-C-08）
    // ═══════════════════════════════════════════════════════════════════════

    use std::time::{SystemTime, UNIX_EPOCH};
    use voice_room_shared::jwt::token::{encode_token, AppClaims};

    /// 用于 T-00010 测试的 JWT 生成辅助
    fn make_token_for(user_id: uuid::Uuid) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claims = AppClaims {
            sub: user_id.to_string(),
            iss: "voiceroom".into(),
            exp: now + 3600,
            iat: now,
        };
        encode_token(&claims, b"test-secret").unwrap()
    }

    /// 过期 token 辅助（exp 设为过去）
    fn make_expired_token_for(user_id: uuid::Uuid) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let claims = AppClaims {
            sub: user_id.to_string(),
            iss: "voiceroom".into(),
            exp: now - 3600, // 已过期
            iat: now - 7200,
        };
        encode_token(&claims, b"test-secret").unwrap()
    }

    /// 测试辅助：构建含有 active 房间的 app，返回 (app, owner_id, room_id_str)
    fn build_app_with_room_for_close() -> (axum::Router, uuid::Uuid, String) {
        use crate::modules::room::FakeRoomRepository;
        use chrono::Utc;
        use uuid::Uuid;
        use voice_room_shared::models::room::RoomModel;

        let repo = std::sync::Arc::new(FakeRoomRepository::default());
        let owner_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();

        repo.seed_user(owner_id, "Owner".to_string(), None);
        let now = Utc::now();
        repo.seed(RoomModel {
            id: room_id,
            owner_id,
            title: "Test Room".to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            status: "active".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });

        let app = build_app(AppState::for_test_with_room_repo(repo));
        (app, owner_id, room_id.to_string())
    }

    /// 测试辅助：构建含有 closed 房间的 app，返回 (app, owner_id, room_id_str)
    fn build_app_with_closed_room() -> (axum::Router, uuid::Uuid, String) {
        use crate::modules::room::FakeRoomRepository;
        use chrono::Utc;
        use uuid::Uuid;
        use voice_room_shared::models::room::RoomModel;

        let repo = std::sync::Arc::new(FakeRoomRepository::default());
        let owner_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();

        repo.seed_user(owner_id, "Owner".to_string(), None);
        let now = Utc::now();
        repo.seed(RoomModel {
            id: room_id,
            owner_id,
            title: "Closed Room".to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            status: "closed".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });

        let app = build_app(AppState::for_test_with_room_repo(repo));
        (app, owner_id, room_id.to_string())
    }

    /// I-C-01: 有效 JWT(房主) + active 房间 → 200 + code=0 + data=null
    #[tokio::test]
    async fn ic01_owner_closes_active_room_returns_200() {
        let (app, owner_id, room_id) = build_app_with_room_for_close();
        let token = make_token_for(owner_id);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/rooms/{room_id}"))
                    .header("authorization", format!("Bearer {token}"))
                    .header("x-request-id", "close-req-001")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["code"], 0, "success code must be 0");
        assert!(json["data"].is_null(), "data must be null on success");
    }

    /// I-C-02: 有效 JWT(房主) + 不存在房间 → 404 / code=40400
    #[tokio::test]
    async fn ic02_owner_closes_nonexistent_room_returns_404() {
        let app = build_app(AppState::for_test());
        let owner_id = uuid::Uuid::new_v4();
        let nonexistent_id = uuid::Uuid::new_v4();
        let token = make_token_for(owner_id);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/rooms/{nonexistent_id}"))
                    .header("authorization", format!("Bearer {token}"))
                    .header("x-request-id", "close-req-002")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let json = body_json(response).await;
        assert_eq!(json["code"], 40400, "not found code must be 40400");
    }

    /// I-C-03: 有效 JWT(非房主) + active 房间 → 403 / code=40301
    #[tokio::test]
    async fn ic03_non_owner_returns_403() {
        let (app, _owner_id, room_id) = build_app_with_room_for_close();
        let other_user = uuid::Uuid::new_v4();
        let token = make_token_for(other_user);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/rooms/{room_id}"))
                    .header("authorization", format!("Bearer {token}"))
                    .header("x-request-id", "close-req-003")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let json = body_json(response).await;
        assert_eq!(json["code"], 40301, "forbidden code must be 40301");
    }

    /// I-C-04: 有效 JWT(房主) + closed 房间 → 409 / code=40901
    #[tokio::test]
    async fn ic04_owner_closes_already_closed_room_returns_409() {
        let (app, owner_id, room_id) = build_app_with_closed_room();
        let token = make_token_for(owner_id);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/rooms/{room_id}"))
                    .header("authorization", format!("Bearer {token}"))
                    .header("x-request-id", "close-req-004")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
        let json = body_json(response).await;
        assert_eq!(
            json["code"], 40901,
            "room already closed code must be 40901"
        );
    }

    /// I-C-05: 有效 JWT + /api/v1/rooms/not-a-uuid → 400 / code=40003
    #[tokio::test]
    async fn ic05_invalid_uuid_returns_400() {
        let app = build_app(AppState::for_test());
        let owner_id = uuid::Uuid::new_v4();
        let token = make_token_for(owner_id);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/v1/rooms/not-a-uuid")
                    .header("authorization", format!("Bearer {token}"))
                    .header("x-request-id", "close-req-005")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let json = body_json(response).await;
        assert_eq!(json["code"], 40003, "invalid uuid code must be 40003");
    }

    /// I-C-06: 无 Authorization header → 401 / code=40101
    #[tokio::test]
    async fn ic06_no_auth_returns_401() {
        let room_id = uuid::Uuid::new_v4();
        let app = build_app(AppState::for_test());

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/rooms/{room_id}"))
                    .header("x-request-id", "close-req-006")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(response).await;
        assert_eq!(json["code"], 40101, "unauthorized code must be 40101");
    }

    /// I-C-07: 过期 JWT → 401 / code=40102
    #[tokio::test]
    async fn ic07_expired_token_returns_401() {
        let room_id = uuid::Uuid::new_v4();
        let user_id = uuid::Uuid::new_v4();
        let expired_token = make_expired_token_for(user_id);
        let app = build_app(AppState::for_test());

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/rooms/{room_id}"))
                    .header("authorization", format!("Bearer {expired_token}"))
                    .header("x-request-id", "close-req-007")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let json = body_json(response).await;
        assert_eq!(json["code"], 40102, "expired token code must be 40102");
    }

    /// I-C-08: 关闭成功后 GET /api/v1/rooms/:id → 404 / code=40400
    #[tokio::test]
    async fn ic08_after_close_get_room_returns_404() {
        use crate::modules::room::FakeRoomRepository;
        use chrono::Utc;
        use uuid::Uuid;
        use voice_room_shared::models::room::RoomModel;

        // 构建一个共享 repo（需要先关闭再查询，共用同一个 repo 实例）
        let repo = std::sync::Arc::new(FakeRoomRepository::default());
        let owner_id = Uuid::new_v4();
        let room_id = Uuid::new_v4();

        repo.seed_user(owner_id, "Owner".to_string(), None);
        let now = Utc::now();
        repo.seed(RoomModel {
            id: room_id,
            owner_id,
            title: "Room To Close".to_string(),
            room_type: "normal".to_string(),
            member_count: 0,
            status: "active".to_string(),
            password_hash: None,
            max_members: 50,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            cover_url: String::new(),
            category: "chat".to_string(),
            announcement: None,
            admin_user_id: None,
        });

        let token = make_token_for(owner_id);
        let app = build_app(AppState::for_test_with_room_repo(repo));

        // 先关闭房间
        let close_resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/v1/rooms/{room_id}"))
                    .header("authorization", format!("Bearer {token}"))
                    .header("x-request-id", "close-req-008a")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            close_resp.status(),
            StatusCode::OK,
            "close must succeed first"
        );

        // 再 GET 该房间 → 应返回 404
        let get_resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/api/v1/rooms/{room_id}"))
                    .header("x-request-id", "close-req-008b")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
        let json = body_json(get_resp).await;
        assert_eq!(json["code"], 40400, "room should not be found after close");
    }
}
