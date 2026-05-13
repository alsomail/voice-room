//! T-10028: ReportService — 财务报告业务逻辑

use std::{collections::HashMap, sync::Arc};

use chrono::NaiveDate;

use crate::common::error::AppError;

use super::{
    report_dto::{ReportResponse, ReportSeriesItem, ReportTotals},
    report_query::ReportQueryRepo,
};

#[cfg(any(test, feature = "test-utils"))]
use super::report_query::FakeReportQuery;

// ─── 汇率表 ───────────────────────────────────────────────────────────────────

/// 各货币对 USD 的汇率（1 货币 = ? USD）。
/// 来自配置 `[payment.exchange_rates]`；USD=1.0 默认内置。
#[derive(Clone, Debug)]
pub struct ExchangeRates(pub HashMap<String, f64>);

impl ExchangeRates {
    /// 返回 `currency` 对应的 USD 汇率；未配置时默认 1.0。
    pub fn to_usd(&self, currency: &str) -> f64 {
        if currency == "USD" {
            return 1.0;
        }
        *self.0.get(currency).unwrap_or(&1.0)
    }
}

// ─── Service ─────────────────────────────────────────────────────────────────

pub struct ReportService {
    repo: Arc<dyn ReportQueryRepo>,
    rates: ExchangeRates,
}

impl ReportService {
    pub fn new(repo: Arc<dyn ReportQueryRepo>, rates: ExchangeRates) -> Self {
        Self { repo, rates }
    }

    /// 构建财务报告。
    pub async fn build_report(
        &self,
        granularity: &str,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<ReportResponse, AppError> {
        let raw_rows = self.repo.aggregate(granularity, from, to).await?;

        // 按 date 聚合
        let mut buckets: HashMap<String, BucketAccum> = HashMap::new();

        for row in &raw_rows {
            let bucket = buckets.entry(row.date.clone()).or_default();
            let rate = self.rates.to_usd(&row.currency);

            bucket.gmv_usd += row.revenue_sum * rate;
            bucket.order_count += row.order_count;
            bucket.refund_count += row.refund_count;
            bucket.refund_amount_usd -= row.refund_sum * rate; // negative

            let cur_gmv = bucket
                .gmv_by_currency
                .entry(row.currency.clone())
                .or_insert(0.0);
            *cur_gmv += row.revenue_sum;
        }

        // 排序（lexicographic = chronological for YYYY-MM-DD / YYYY-MM）
        let mut dates: Vec<String> = buckets.keys().cloned().collect();
        dates.sort();

        let series: Vec<ReportSeriesItem> = dates
            .iter()
            .map(|date| {
                let b = &buckets[date];
                let avg = if b.order_count > 0 {
                    b.gmv_usd / b.order_count as f64
                } else {
                    0.0
                };
                ReportSeriesItem {
                    date: date.clone(),
                    gmv_usd: format_usd(b.gmv_usd),
                    gmv_by_currency: b
                        .gmv_by_currency
                        .iter()
                        .map(|(k, v)| (k.clone(), format_usd(*v)))
                        .collect(),
                    order_count: b.order_count,
                    refund_count: b.refund_count,
                    refund_amount_usd: format_usd(b.refund_amount_usd),
                    avg_ticket_usd: format_usd(avg),
                }
            })
            .collect();

        // 全期汇总
        let total_gmv: f64 = series.iter().map(|s| parse_f64(&s.gmv_usd)).sum();
        let total_orders: i64 = series.iter().map(|s| s.order_count).sum();
        let total_refunds: i64 = series.iter().map(|s| s.refund_count).sum();
        let total_refund_usd: f64 = series.iter().map(|s| parse_f64(&s.refund_amount_usd)).sum();
        let total_avg = if total_orders > 0 {
            total_gmv / total_orders as f64
        } else {
            0.0
        };

        Ok(ReportResponse {
            granularity: granularity.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            series,
            totals: ReportTotals {
                gmv_usd: format_usd(total_gmv),
                order_count: total_orders,
                refund_count: total_refunds,
                refund_amount_usd: format_usd(total_refund_usd),
                avg_ticket_usd: format_usd(total_avg),
            },
        })
    }
}

// ─── 辅助 ─────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct BucketAccum {
    gmv_usd: f64,
    order_count: i64,
    refund_count: i64,
    refund_amount_usd: f64,
    gmv_by_currency: HashMap<String, f64>,
}

fn format_usd(v: f64) -> String {
    // Normalize -0.0 → 0.0: IEEE 754 comparison treats -0.0 == 0.0
    format!("{:.2}", if v == 0.0 { 0.0 } else { v })
}

fn parse_f64(s: &str) -> f64 {
    s.parse().unwrap_or(0.0)
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::payment::report_query::ReportDbRow;
    use chrono::NaiveDate;

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn make_rates(pairs: &[(&str, f64)]) -> ExchangeRates {
        let mut m = HashMap::new();
        for (k, v) in pairs {
            m.insert(k.to_string(), *v);
        }
        ExchangeRates(m)
    }

    fn make_service_with_rows(rows: Vec<ReportDbRow>, rates: ExchangeRates) -> ReportService {
        let fake = Arc::new(FakeReportQuery::default());
        for r in rows {
            fake.seed(r);
        }
        ReportService::new(fake, rates)
    }

    // ── RP-02: 空数据 → series 为空, totals 全零 ─────────────────────────

    #[tokio::test]
    async fn rp02_empty_data_returns_zero_totals() {
        let svc = make_service_with_rows(vec![], ExchangeRates(HashMap::new()));
        let report = svc
            .build_report("day", date("2024-01-01"), date("2024-01-31"))
            .await
            .unwrap();
        assert!(report.series.is_empty());
        assert_eq!(report.totals.gmv_usd, "0.00");
        assert_eq!(report.totals.order_count, 0);
    }

    // ── RP-03: 汇率 SAR → USD 换算 ───────────────────────────────────────

    #[tokio::test]
    async fn rp03_sar_converted_to_usd() {
        // SAR 1 = 0.2666 USD  →  37.48 SAR ≈ 9.99 USD
        let rows = vec![ReportDbRow {
            date: "2024-01-15".to_string(),
            currency: "SAR".to_string(),
            revenue_sum: 37.48,
            order_count: 1,
            refund_count: 0,
            refund_sum: 0.0,
        }];
        let rates = make_rates(&[("SAR", 0.2666)]);
        let svc = make_service_with_rows(rows, rates);
        let report = svc
            .build_report("day", date("2024-01-15"), date("2024-01-15"))
            .await
            .unwrap();
        let item = &report.series[0];
        // 37.48 * 0.2666 ≈ 9.99 USD
        let gmv: f64 = item.gmv_usd.parse().unwrap();
        assert!((gmv - 9.99).abs() < 0.02, "RP-03: gmv_usd expected ~9.99, got {gmv}");
    }

    // ── RP-04: 多货币同一天聚合 ──────────────────────────────────────────

    #[tokio::test]
    async fn rp04_multi_currency_same_day_aggregation() {
        let rows = vec![
            ReportDbRow {
                date: "2024-01-15".to_string(),
                currency: "USD".to_string(),
                revenue_sum: 9.99,
                order_count: 1,
                refund_count: 0,
                refund_sum: 0.0,
            },
            ReportDbRow {
                date: "2024-01-15".to_string(),
                currency: "SAR".to_string(),
                revenue_sum: 37.48,
                order_count: 1,
                refund_count: 0,
                refund_sum: 0.0,
            },
        ];
        let rates = make_rates(&[("SAR", 0.2666)]);
        let svc = make_service_with_rows(rows, rates);
        let report = svc
            .build_report("day", date("2024-01-15"), date("2024-01-15"))
            .await
            .unwrap();
        assert_eq!(report.series.len(), 1);
        let item = &report.series[0];
        assert_eq!(item.order_count, 2);
        assert!(item.gmv_by_currency.contains_key("USD"));
        assert!(item.gmv_by_currency.contains_key("SAR"));
    }

    // ── RP-05: 退款金额 → 负数 ───────────────────────────────────────────

    #[tokio::test]
    async fn rp05_refund_amount_is_negative() {
        let rows = vec![ReportDbRow {
            date: "2024-01-15".to_string(),
            currency: "USD".to_string(),
            revenue_sum: 0.0,
            order_count: 0,
            refund_count: 1,
            refund_sum: 9.99,
        }];
        let svc = make_service_with_rows(rows, ExchangeRates(HashMap::new()));
        let report = svc
            .build_report("day", date("2024-01-15"), date("2024-01-15"))
            .await
            .unwrap();
        let item = &report.series[0];
        let refund: f64 = item.refund_amount_usd.parse().unwrap();
        assert!(refund < 0.0, "RP-05: refund_amount_usd must be negative, got {refund}");
    }

    // ── RP-06: avg_ticket 计算 ────────────────────────────────────────────

    #[tokio::test]
    async fn rp06_avg_ticket_usd_correct() {
        let rows = vec![ReportDbRow {
            date: "2024-01-15".to_string(),
            currency: "USD".to_string(),
            revenue_sum: 29.97, // 3 orders x $9.99
            order_count: 3,
            refund_count: 0,
            refund_sum: 0.0,
        }];
        let svc = make_service_with_rows(rows, ExchangeRates(HashMap::new()));
        let report = svc
            .build_report("day", date("2024-01-15"), date("2024-01-15"))
            .await
            .unwrap();
        let item = &report.series[0];
        let avg: f64 = item.avg_ticket_usd.parse().unwrap();
        assert!((avg - 9.99).abs() < 0.01, "RP-06: avg_ticket expected 9.99, got {avg}");
    }

    // ── RP-06b: order_count=0 → avg=0.00 ────────────────────────────────

    #[tokio::test]
    async fn rp06b_avg_ticket_zero_when_no_orders() {
        let rows = vec![ReportDbRow {
            date: "2024-01-15".to_string(),
            currency: "USD".to_string(),
            revenue_sum: 0.0,
            order_count: 0,
            refund_count: 1,
            refund_sum: 9.99,
        }];
        let svc = make_service_with_rows(rows, ExchangeRates(HashMap::new()));
        let report = svc
            .build_report("day", date("2024-01-15"), date("2024-01-15"))
            .await
            .unwrap();
        let item = &report.series[0];
        assert_eq!(item.avg_ticket_usd, "0.00", "RP-06b: avg must be 0.00 when no orders");
    }

    // ── RP-10: 多天系列按日期排序 ─────────────────────────────────────────

    #[tokio::test]
    async fn rp10_series_sorted_chronologically() {
        let rows = vec![
            ReportDbRow {
                date: "2024-01-20".to_string(),
                currency: "USD".to_string(),
                revenue_sum: 9.99,
                order_count: 1,
                refund_count: 0,
                refund_sum: 0.0,
            },
            ReportDbRow {
                date: "2024-01-10".to_string(),
                currency: "USD".to_string(),
                revenue_sum: 19.98,
                order_count: 2,
                refund_count: 0,
                refund_sum: 0.0,
            },
        ];
        let svc = make_service_with_rows(rows, ExchangeRates(HashMap::new()));
        let report = svc
            .build_report("day", date("2024-01-01"), date("2024-01-31"))
            .await
            .unwrap();
        assert_eq!(report.series[0].date, "2024-01-10", "first date should be earliest");
        assert_eq!(report.series[1].date, "2024-01-20");
    }

    // ── ExchangeRates: 未知货币 → fallback 1.0 ───────────────────────────

    #[test]
    fn rates_unknown_currency_fallback_to_one() {
        let rates = make_rates(&[("SAR", 0.2666)]);
        assert_eq!(rates.to_usd("XYZ"), 1.0);
        assert_eq!(rates.to_usd("USD"), 1.0);
    }
}
