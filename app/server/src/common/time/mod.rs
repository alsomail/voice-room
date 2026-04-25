//! Asia/Riyadh 时区时间工具（缺陷 #3 P1）
//!
//! TDS T-00021 要求所有榜单 ZSet key、scheduler 触发判定、归档任务
//! 均使用 `Asia/Riyadh`（UTC+3，沙特无夏令时）作为时间锚点，确保榜单
//! 在沙特本地 00:00 切换、用户黄金活跃时段（凌晨）的写入归位正确。
//!
//! 本模块作为 server 端唯一时间锚点单一事实源；任何榜单/归档相关代码
//! 必须使用此处 API，禁止再调用 `chrono::Utc::now()` 计算 day/week key。

pub mod riyadh;

pub use riyadh::{
    format_day_riyadh, format_week_riyadh, last_week_riyadh_str, now_riyadh, today_riyadh_str,
    week_riyadh_str, yesterday_riyadh_str,
};
