//! Analytics 核心模块 — T-00022
//!
//! 提供统一的 `EventWriter` 写入服务（HTTP 通道和 WS 通道共用）和
//! `PartitionScheduler` 分区自动创建任务。
//!
//! ## 子模块
//! - `writer`  — `EventWriter` / `EventWriterPort` / `FakeEventWriter`
//! - `scheduler` — `create_partition_if_not_exists` / `compensate_missing_partitions`

pub mod scheduler;
pub mod writer;
