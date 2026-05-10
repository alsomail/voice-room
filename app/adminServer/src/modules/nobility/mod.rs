//! Admin Nobility Module (T-10030 ~ T-10032)
//!
//! - `dto`        : DTO 定义
//! - `repository` : NobilityRepo trait + PgNobilityRepo + FakeNobilityRepo
//! - `service`    : NobilityService（业务逻辑 + 单元测试）
//! - `controller` : HTTP handlers

pub mod controller;
pub mod dto;
pub mod repository;
pub mod service;

pub use service::NobilityService;

#[cfg(any(test, feature = "test-utils"))]
pub use repository::FakeNobilityRepo;
