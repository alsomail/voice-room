//! Asia/Riyadh 时区辅助函数（沙特本地时间 = UTC+3，无夏令时）
//!
//! ## 提供的 API
//! - `now_riyadh()` —— 当前 Riyadh 本地时间
//! - `today_riyadh_str()` —— 今日日期串 YYYY-MM-DD（Riyadh）
//! - `yesterday_riyadh_str()` —— 昨日日期串（Riyadh）
//! - `week_riyadh_str()` —— 本周编号 YYYY-WW（Riyadh）
//! - `format_day_riyadh(dt)` / `format_week_riyadh(dt)` —— 测试可注入版本

use chrono::{DateTime, Datelike, Duration, Utc};
use chrono_tz::Asia::Riyadh;
use chrono_tz::Tz;

/// 返回当前 Riyadh 本地时间。
pub fn now_riyadh() -> DateTime<Tz> {
    Utc::now().with_timezone(&Riyadh)
}

/// 将任意 UTC `DateTime` 转换为 Riyadh 时区下的 YYYY-MM-DD 字符串。
///
/// 暴露此函数主要为测试可注入边界时间（如 Riyadh 23:59）。
pub fn format_day_riyadh(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Riyadh).format("%Y-%m-%d").to_string()
}

/// 将任意 UTC `DateTime` 转换为 Riyadh 时区下的 YYYY-WW（年-周）字符串。
pub fn format_week_riyadh(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Riyadh).format("%Y-%W").to_string()
}

/// 当前 Riyadh 本地日期 YYYY-MM-DD。
pub fn today_riyadh_str() -> String {
    format_day_riyadh(Utc::now())
}

/// 昨日 Riyadh 本地日期 YYYY-MM-DD（Riyadh 当前时间 - 1 天）。
pub fn yesterday_riyadh_str() -> String {
    let yesterday = Utc::now() - Duration::days(1);
    format_day_riyadh(yesterday)
}

/// 当前 Riyadh 本地年-周 YYYY-WW（%W = 周日为周首日的周编号，与现有 UTC 实现保持一致格式）。
pub fn week_riyadh_str() -> String {
    format_week_riyadh(Utc::now())
}

/// 上周 Riyadh 本地年-周 YYYY-WW（Riyadh 当前时间 - 7 天）。
pub fn last_week_riyadh_str() -> String {
    let last_week = Utc::now() - Duration::weeks(1);
    format_week_riyadh(last_week)
}

/// （留给上层判定"是否跨日"的便捷函数）当前 Riyadh 日的 ISO 编号 (year, ord_day)。
pub fn riyadh_year_ordinal() -> (i32, u32) {
    let now = now_riyadh();
    (now.year(), now.ordinal())
}

/// 给 scheduler 用：将一个 UTC 瞬时映射到 Riyadh 的"日期边界"。
/// 返回该瞬时所属的 Riyadh 本地日（00:00 起算）。
pub fn riyadh_date_at(dt: DateTime<Utc>) -> chrono::NaiveDate {
    dt.with_timezone(&Riyadh).date_naive()
}

/// 直接构造一个 Riyadh 本地时间 DateTime（测试辅助）。
#[cfg(test)]
pub fn make_riyadh(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Tz> {
    use chrono::TimeZone;
    Riyadh
        .with_ymd_and_hms(y, mo, d, h, mi, 0)
        .single()
        .expect("valid Riyadh datetime")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;
    use chrono::TimeZone;

    // RYD-01: 同一 UTC 瞬时在 Riyadh 表示晚 3 小时（沙特无夏令时）
    #[test]
    fn ryd01_offset_is_plus_three_hours_year_round() {
        // 1 月（北半球冬）
        let winter = Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap();
        let r = winter.with_timezone(&Riyadh);
        assert_eq!(r.hour(), 15, "Winter: Riyadh = UTC+3");

        // 7 月（北半球夏） —— 沙特无 DST，仍 +3
        let summer = Utc.with_ymd_and_hms(2026, 7, 15, 12, 0, 0).unwrap();
        let rs = summer.with_timezone(&Riyadh);
        assert_eq!(rs.hour(), 15, "Summer: Riyadh remains UTC+3 (no DST)");
    }

    // RYD-02: UTC 23:00 跨 Riyadh 02:00 →（next day）日期串应是"明天"
    #[test]
    fn ryd02_format_day_riyadh_crosses_midnight() {
        // 2026-04-25 23:00 UTC = 2026-04-26 02:00 Riyadh
        let utc = Utc.with_ymd_and_hms(2026, 4, 25, 23, 0, 0).unwrap();
        assert_eq!(format_day_riyadh(utc), "2026-04-26");
    }

    // RYD-03: Riyadh 23:59 还在"今天"，UTC 时间则已是次日 02:59
    #[test]
    fn ryd03_riyadh_late_night_still_same_day() {
        // Riyadh 2026-04-25 23:59 = UTC 2026-04-25 20:59
        let utc = Utc.with_ymd_and_hms(2026, 4, 25, 20, 59, 0).unwrap();
        assert_eq!(format_day_riyadh(utc), "2026-04-25");
    }

    // RYD-04: today_riyadh_str / yesterday_riyadh_str 长度 10 (YYYY-MM-DD)
    #[test]
    fn ryd04_today_yesterday_format_length() {
        assert_eq!(today_riyadh_str().len(), 10);
        assert_eq!(yesterday_riyadh_str().len(), 10);
    }

    // RYD-05: week_riyadh_str 形如 YYYY-WW
    #[test]
    fn ryd05_week_format() {
        let s = week_riyadh_str();
        assert_eq!(s.len(), 7);
        assert!(s.contains('-'));
    }

    // RYD-06: scheduler 在 UTC 21:00（=Riyadh 次日 00:00）触发归档
    //         此时 yesterday_riyadh_str 应等于"前一天的 Riyadh 日期"
    #[test]
    fn ryd06_scheduler_boundary_at_utc_21() {
        // UTC 2026-04-25 21:00 → Riyadh 2026-04-26 00:00
        let utc = Utc.with_ymd_and_hms(2026, 4, 25, 21, 0, 0).unwrap();
        let yesterday = utc - Duration::days(1);
        assert_eq!(format_day_riyadh(yesterday), "2026-04-25");
        assert_eq!(format_day_riyadh(utc), "2026-04-26");
    }

    // RYD-07: 假定夏季（7 月）仍 +3，验证未受 DST 影响（chrono-tz 数据正确性）
    #[test]
    fn ryd07_no_dst_in_summer() {
        let utc = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
        // 即 Riyadh 03:00 同日
        assert_eq!(format_day_riyadh(utc), "2026-07-01");
    }
}
