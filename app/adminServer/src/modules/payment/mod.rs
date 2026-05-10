//! T-10025~28: Admin Payment 模块
//!
//! - `dto`           : 订单查询 DTO
//! - `repo`          : PaymentOrderRepo trait + Pg/Fake 实现
//! - `controller`    : 订单查询 handler（GET /orders, GET /orders/:id）
//! - `admin_service` : 补单/退款原子事务（T-10026）
//! - `sku_dto`        : SKU CRUD DTO
//! - `sku_repo`       : SkuRepository trait
//! - `sku_service`    : SKU 业务逻辑
//! - `sku_controller` : SKU CRUD handler
//! - `report_dto`     : 财务报表 DTO
//! - `report_query`   : DB 聚合 SQL
//! - `report_service` : FX 折算 + 汇总构建
//! - `report_controller`: 报表 handler

pub mod admin_service;
pub mod controller;
pub mod dto;
pub mod repo;
pub mod report_controller;
pub mod report_dto;
pub mod report_query;
pub mod report_service;
pub mod sku_controller;
pub mod sku_dto;
pub mod sku_repo;
pub mod sku_service;
