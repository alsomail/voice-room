//! 快照定時任務模組
//!
//! `snapshot_task` 每 `interval_duration` 調用一次 `stats.take_snapshot()`。
//! 接受 `watch::Receiver<bool>` 作為優雅停機信號：發送 `true` 時退出循環。
//!
//! # 生產使用（推薦使用 `start_snapshot_task`，內建 60s 間隔）
//! ```ignore
//! let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
//! tokio::spawn(start_snapshot_task(
//!     state.stats_service.clone(),
//!     shutdown_rx,
//! ));
//! // 優雅停機時：
//! let _ = shutdown_tx.send(true);
//! ```

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

use crate::stats::StatsPort;

/// 定時快照任務（便捷入口，固定 60s 間隔）
///
/// 生產環境推薦使用此函數，避免在調用方硬編碼 interval。
pub async fn start_snapshot_task(
    stats: Arc<dyn StatsPort>,
    shutdown: watch::Receiver<bool>,
) {
    snapshot_task(stats, Duration::from_secs(60), shutdown).await;
}

/// 定時快照任務
///
/// 每個 `interval_duration` 週期調用 `stats.take_snapshot()`，
/// 快照失敗時記錄警告日誌（不退出）。
/// 收到 shutdown 信號（`true`）後優雅退出；
/// 若發送方意外 Drop，記錄 warn 日誌後同樣退出（防止 task 靜默消失）。
pub async fn snapshot_task(
    stats: Arc<dyn StatsPort>,
    interval_duration: Duration,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut interval = tokio::time::interval(interval_duration);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(e) = stats.take_snapshot().await {
                    tracing::warn!(error = %e, "stats snapshot failed, will retry next tick");
                }
            }
            // 驗證 changed() 結果：區分正常停機 vs 發送方意外 Drop
            result = shutdown.changed() => {
                match result {
                    Ok(_) => {
                        tracing::info!("snapshot_task: shutdown signal received, stopping");
                    }
                    Err(e) => {
                        tracing::warn!(
                            "snapshot_task: shutdown sender dropped unexpectedly: {:?}", e
                        );
                    }
                }
                return;
            }
        }
    }
}

// ─── 單元測試 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use tokio::sync::watch;

    use crate::stats::FakeStatsService;

    use super::*;

    // ST08: snapshot_task 在 1 個 tick 後調用 take_snapshot
    //
    // 設計：interval 首次 tick 立即觸發（tokio::time::interval 行為），
    // 因此 spawn 後短暫 sleep 即可斷言 snapshot_calls >= 1。
    #[tokio::test]
    async fn st08_snapshot_task_calls_take_snapshot() {
        let stats = Arc::new(FakeStatsService::default());
        let (_shutdown_tx, shutdown_rx) = watch::channel(false);

        let stats_clone = Arc::clone(&stats) as Arc<dyn StatsPort>;
        let handle = tokio::spawn(async move {
            snapshot_task(stats_clone, Duration::from_millis(1), shutdown_rx).await;
        });

        // 等待至少一個 tick 完成
        tokio::time::sleep(Duration::from_millis(20)).await;

        let calls = stats.snapshot_calls.load(Ordering::Relaxed);
        assert!(
            calls >= 1,
            "take_snapshot should be called at least once, but snapshot_calls = {calls}"
        );

        // 清理：中止 task，避免 test 掛起
        handle.abort();
    }

    // ST09: shutdown 信號後 task 退出
    //
    // 使用長達 60s 的 interval（確保不會自然 tick），
    // 發送 shutdown=true 後 task 應在 1s 內退出。
    #[tokio::test]
    async fn st09_snapshot_task_stops_on_shutdown() {
        let stats = Arc::new(FakeStatsService::default()) as Arc<dyn StatsPort>;
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let handle = tokio::spawn(async move {
            snapshot_task(stats, Duration::from_secs(60), shutdown_rx).await;
        });

        // 略等任務啟動，再發送停機信號
        tokio::time::sleep(Duration::from_millis(5)).await;
        shutdown_tx.send(true).expect("shutdown send must succeed");

        // task 應在 1 秒內正常退出（非 abort）
        let result = tokio::time::timeout(Duration::from_secs(1), handle).await;

        assert!(
            result.is_ok(),
            "snapshot_task should exit within 1s after receiving shutdown signal"
        );
        assert!(
            result.unwrap().is_ok(),
            "snapshot_task should not panic on shutdown"
        );
    }

    // ST10: 發送方 Drop 後 task 應退出（不靜默掛起）
    //
    // 模擬生產中 snapshot_shutdown_tx 被 Drop（非正常停機），
    // task 應記錄 warn 並在 1s 內退出，而非永久掛起。
    #[tokio::test]
    async fn st10_snapshot_task_exits_when_sender_dropped() {
        let stats = Arc::new(FakeStatsService::default()) as Arc<dyn StatsPort>;
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let handle = tokio::spawn(async move {
            snapshot_task(stats, Duration::from_secs(60), shutdown_rx).await;
        });

        // 略等任務啟動，然後 Drop 發送方（不發送任何值）
        tokio::time::sleep(Duration::from_millis(5)).await;
        drop(shutdown_tx); // 模擬發送方意外消失

        // task 應在 1 秒內退出
        let result = tokio::time::timeout(Duration::from_secs(1), handle).await;

        assert!(
            result.is_ok(),
            "snapshot_task should exit within 1s when sender is dropped"
        );
        assert!(
            result.unwrap().is_ok(),
            "snapshot_task should not panic when sender is dropped"
        );
    }
}
