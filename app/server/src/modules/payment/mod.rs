//! Payment 模块（E-08 Google Play 真支付）
//!
//! ## 子模块
//! - `dto` — 请求/响应 DTO（严格匹配 payment_api.md §9.3）
//! - `error` — PaymentError + PaymentErrorWithId（独立错误码系统）
//! - `repo` — PaymentRepo（payment_skus / payment_orders / rtdn_processed 数据访问）
//! - `risk` — RiskCheckService（风控：日失败 > 10 → 40903）
//! - `google_billing_port` — 防腐层 Trait（GooglePlayBillingPort + FakeGooglePlayBillingPort）
//! - `service` — PaymentOrderService（创建订单，T-00051）
//! - `verify_service` — PaymentVerifyService（验签 + 入账，T-00052）
//! - `rtdn_service` — PaymentRtdnService（RTDN 推送处理，T-00053）
//! - `cron` — 后台对账 cron（T-00054）
//! - `dev_mock` — Dev Mock 充值服务（T-00055，仅 dev_payment_mock feature）
//! - `controller` — HTTP handlers
//! - `routes` — 路由注册

pub mod controller;
pub mod cron;
pub mod dev_mock;
pub mod dto;
pub mod error;
pub mod google_billing_port;
pub mod repo;
pub mod risk;
pub mod routes;
pub mod rtdn_service;
pub mod service;
pub mod verify_service;

pub use routes::payment_routes;
pub use service::PaymentOrderServicePort;
pub use verify_service::PaymentVerifyServicePort;
pub use rtdn_service::PaymentRtdnServicePort;

// Fake 类型始终导出，允许在集成测试和生产默认（FakePayment）场景下使用
pub use google_billing_port::FakeGooglePlayBillingPort;
pub use risk::FakeRiskCheckService;
pub use service::FakePaymentOrderService;
pub use verify_service::FakePaymentVerifyService;
pub use rtdn_service::FakePaymentRtdnService;
pub use dev_mock::FakePaymentMockService;
