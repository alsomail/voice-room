# Spec: 排行榜 (ranking_leaderboard)

> **状态**：已归档
> **覆盖 Epic**：E-07 礼物经济 - 排行榜衍生
> **最后更新**：2026-05-15

---

## §1 关联 Task 簇

模块6 中排行榜相关 Task：日榜 / 周榜 / 房间贡献榜 / 守护榜 / 实时上榜 WS 推送。

---

## §2 事实源锚点

- 协议：[`protocol/ranking_api.md`](../protocol/ranking_api.md)（如无，并入 `gift_api.md` 排行榜章节）、[`protocol/websocket_signals.md`](../protocol/websocket_signals.md)（RankingUpdated）
- 状态机：N/A（排行榜为 Redis ZSET，无业务状态机）
- 旅程：[`user_journeys.md#j1-recharge-gift-noble`](../product/user_journeys.md#j1-recharge-gift-noble)（上榜动机）
- 业务约束：`LEADERBOARD_TOP_N` / `WEEKLY_RESET_DAY` / `DEFAULT_TIMEZONE`

---

## §3 流程图（裁剪后）

```mermaid
flowchart LR
    A[GiftTransaction.Settled] -->|异步| B[Redis ZINCRBY rank:daily:{uid}]
    A --> C[Redis ZINCRBY rank:weekly:{uid}]
    A --> D[Redis ZINCRBY rank:room:{rid}:{uid}]
    E[Cron 每日 03:00 Asia/Riyadh] --> F[快照 daily → daily_history; 清零 daily]
    G[Cron 每周六 03:00] --> H[快照 weekly → weekly_history; 清零 weekly]
    I[Client] -->|HTTP GET /ranking/daily?top=100| J[Server: ZREVRANGE]
```

### 异常分支必覆清单
- [x] Redis 故障 → 降级查 `daily_history`（仅历史榜）；当日实时榜返回 503 + 文案
- [x] 重置时机以 `Asia/Riyadh` 时区为准，禁止以 UTC
- [x] 退款回退：礼物退款必须**同步**回退榜单分数（ZINCRBY 负值）
- [x] TopN 越界：客户端请求 top > `LEADERBOARD_TOP_N` → 截断到上限

---

## §4 边界不变量

- **INV-L1**：所有时间维度（日/周）以 `DEFAULT_TIMEZONE` = `Asia/Riyadh` 为准。
- **INV-L2**：周榜重置日 = `WEEKLY_RESET_DAY` = Saturday（中东周末习惯）。
- **INV-L3**：榜单写入只在 `GiftTransaction.Settled` 后触发，禁止在 Deducted 阶段写（避免事务回滚不一致）。
- **INV-L4**：客户端展示**至多** `LEADERBOARD_TOP_N` 条，禁止"获取全量"接口。

---

## §5 验收条款（GWT）

### GWT-L1（实时上榜）
- **Given** 用户 U 当前日榜 rank=11，差第 10 名 100 金币
- **When** U 送出价值 150 金币的礼物且 Settled
- **Then** ≤ 1s 内 WS `RankingUpdated`(uid, new_rank=10) 推送给房间内所有用户

### GWT-L2（周重置）
- **Given** 当前为 周五 23:59:59 `Asia/Riyadh`
- **When** 时间跨入 周六 03:00:00
- **Then** `rank:weekly:*` 被快照到 `weekly_history:{yyyyww}`；ZSET 清零；广播一条 `WeeklyRankReset`

### GWT-L3（退款回退）
- **Given** 用户 U 因礼物 G 上榜 rank=5
- **When** G 关联订单被 Google 退款触发回退
- **Then** ZINCRBY 负值；U 的 rank 立即重算；若 rank > `LEADERBOARD_TOP_N` 则离榜

### GWT-L4（Redis 故障降级）
- **Given** Redis 不可用
- **When** 客户端查日榜
- **Then** Server 返回 503 + 文案 `ranking_unavailable`；客户端展示"榜单恢复中"

---

## §6 变更记录

| 版本 | 日期 | 摘要 |
|------|------|------|
| v1.0 | 2026-05-15 | 初版归档 |
