//! T-10028: 财务报告 DTO

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── 查询参数 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ReportQuery {
    /// "day" or "month"
    pub granularity: String,
    /// YYYY-MM-DD
    pub from: String,
    /// YYYY-MM-DD (inclusive)
    pub to: String,
    /// ISO 4217 货币代码，用于汇总 GMV；默认 "USD"
    pub currency: Option<String>,
}

impl ReportQuery {
    /// 校验 granularity / from / to。
    pub fn validate(&self) -> Result<(NaiveDate, NaiveDate), String> {
        if self.granularity != "day" && self.granularity != "month" {
            return Err(format!(
                "granularity must be 'day' or 'month', got '{}'",
                self.granularity
            ));
        }
        let from = NaiveDate::parse_from_str(&self.from, "%Y-%m-%d")
            .map_err(|_| format!("from '{}' is not a valid YYYY-MM-DD date", self.from))?;
        let to = NaiveDate::parse_from_str(&self.to, "%Y-%m-%d")
            .map_err(|_| format!("to '{}' is not a valid YYYY-MM-DD date", self.to))?;
        if from > to {
            return Err(format!("from '{}' must not be after to '{}'", self.from, self.to));
        }
        Ok((from, to))
    }
}

// ─── 响应体 ───────────────────────────────────────────────────────────────────

/// 单个时间桶的数据。
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct ReportSeriesItem {
    /// YYYY-MM-DD 或 YYYY-MM
    pub date: String,
    /// GMV (USD) 字符串保留 2 位小数
    pub gmv_usd: String,
    /// 各货币 GMV 原始金额（字符串 2dp）
    pub gmv_by_currency: HashMap<String, String>,
    pub order_count: i64,
    pub refund_count: i64,
    /// 退款金额（负数，USD，字符串 2dp）
    pub refund_amount_usd: String,
    /// 平均客单价 USD（字符串 2dp）；order_count=0 时为 "0.00"
    pub avg_ticket_usd: String,
}

/// 全期间汇总。
#[derive(Debug, Serialize, Clone)]
pub struct ReportTotals {
    pub gmv_usd: String,
    pub order_count: i64,
    pub refund_count: i64,
    pub refund_amount_usd: String,
    pub avg_ticket_usd: String,
}

/// `GET /api/v1/admin/payments/reports` 响应体。
#[derive(Debug, Serialize)]
pub struct ReportResponse {
    pub granularity: String,
    pub from: String,
    pub to: String,
    pub series: Vec<ReportSeriesItem>,
    pub totals: ReportTotals,
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn make_query(gran: &str, from: &str, to: &str) -> ReportQuery {
        ReportQuery {
            granularity: gran.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            currency: None,
        }
    }

    // ── RP-01: 合法 day 查询 → Ok ─────────────────────────────────────────

    #[test]
    fn rp01_valid_day_granularity_ok() {
        let q = make_query("day", "2024-01-01", "2024-01-31");
        assert!(q.validate().is_ok());
    }

    /// month 粒度也 ok
    #[test]
    fn rp01b_valid_month_granularity_ok() {
        let q = make_query("month", "2024-01-01", "2024-06-01");
        assert!(q.validate().is_ok());
    }

    // ── RP-07: 非法 granularity → Err ────────────────────────────────────

    #[test]
    fn rp07_invalid_granularity_returns_error() {
        let q = make_query("week", "2024-01-01", "2024-01-31");
        assert!(q.validate().is_err(), "RP-07: 'week' is not valid");
    }

    // ── RP-08: from > to → Err ────────────────────────────────────────────

    #[test]
    fn rp08_from_after_to_returns_error() {
        let q = make_query("day", "2024-02-01", "2024-01-01");
        assert!(q.validate().is_err(), "RP-08: from > to should fail");
    }

    // ── RP-09: 非法日期格式 → Err ─────────────────────────────────────────

    #[test]
    fn rp09_invalid_date_format_returns_error() {
        let q = make_query("day", "01-01-2024", "2024-01-31"); // wrong format
        assert!(q.validate().is_err(), "RP-09: invalid date format");
    }

    /// from == to → Ok (single day query)
    #[test]
    fn rp_from_eq_to_ok() {
        let q = make_query("day", "2024-01-15", "2024-01-15");
        assert!(q.validate().is_ok());
    }
}
