# Spec: 贵族特权 (nobility_privileges)

> **状态**：活跃（覆盖 E-09 贵族特权应用与展示）
> **覆盖 Task 簇**：特权钩子（隐身/优先抢麦/礼物折扣/免密）、进场广播 + 徽章下发、Android 进场特效播放器/贵族徽章/资料卡贵族框
> **最后更新**：2026-05-15

---

## §1 关联 Task 簇

[`doc/tasks/模块11-E-09 贵族体系.md`](../tasks/模块11-E-09%20贵族体系.md)（共享）

| 端 | TaskID | 一句话职责 |
|---|---|---|
| server | T-00069 | 进场广播 + 徽章字段下发 |
| server | T-00070 | 特权钩子（隐身/优先抢麦/礼物折扣/免密）|
| android | T-30072 | 进场特效播放器 |
| android | T-30073 | 用户贵族徽章组件（全局）|
| android | T-30074 | 资料卡贵族框 |

---

## §2 事实源锚点

- 协议：[`protocol/nobility_api.md`](../protocol/nobility_api.md)、[`protocol/websocket_signals.md`](../protocol/websocket_signals.md)（UserEnteredRoom 增加 noble_tier 字段；MicQueueUpdated 增加 priority；GiftSent 包含 discount_applied）
- 状态机：[`state_machines.md#noble`](../product/state_machines.md#noble)（特权仅在 Active 与 GraceRenewal 生效）
- 旅程：[`user_journeys.md#j4-noble-renewal`](../product/user_journeys.md#j4-noble-renewal)
- 业务约束：
  - 各 tier 礼物折扣率 `NOBLE_GIFT_DISCOUNT_RATE_{TIER}`
  - 抢麦优先级权重 `NOBLE_MIC_PRIORITY_WEIGHT_{TIER}`
  - 隐身可绕过房间在线列表，但**不绕过审计**（INV-V2 / INV-NPR3）
  - 免密进房仅限非黑屋

---

## §3 流程图（裁剪后）

```mermaid
flowchart TD
    A[用户操作] --> B{触发特权钩子}
    B -->|进房| C[NoblePrivilegeHook::on_enter]
    B -->|抢麦| D[NoblePrivilegeHook::on_mic_take]
    B -->|送礼| E[NoblePrivilegeHook::on_gift_send]
    B -->|进密码房| F[NoblePrivilegeHook::on_password_room]
    C --> G{tier active?}
    G -->|是| H[隐身/特效广播]
    G -->|否| I[普通流程]
    D --> J[优先级 + 队列位置调整]
    E --> K[折扣 = price * (1 - rate)]
    F --> L[跳过密码校验（非黑屋）]
```

### 异常分支必覆清单
- [x] 用户已过期（state=Expired）→ 钩子无效 + 客户端徽章消失
- [x] 用户在 GraceRenewal → 特权保留 + 客户端展示"宽限期"标记
- [x] 隐身用户违规 → 仍记入 audit_logs（INV-V2 上位）
- [x] 折扣后金额 < 1 金币 → 兜底为 1 金币（避免免费）
- [x] 抢麦优先级冲突（同 tier）→ 按时间戳先后

---

## §4 边界不变量

- **INV-NPR1**：所有特权应用必须经 `NoblePrivilegeHook` 统一入口，禁止业务代码各自实现 if-tier 判断。
- **INV-NPR2**：特权计算的 tier 来源**必须**是 server 端 noble 表查询结果，禁止信任客户端传入。
- **INV-NPR3**：隐身仅影响**列表展示**，不影响审计 / 房主权限校验 / 拉黑列表。
- **INV-NPR4**：折扣应用必须在**送礼事务**内完成（同 gift_economy INV-G1），并在 `gift_transactions.discount_amount` 字段记录。
- **INV-NPR5**：客户端徽章渲染**必须**以 server 下发字段为准，禁止本地缓存 tier。

---

## §5 验收条款（GWT）

### GWT-NPR1（隐身进房）
- **Given** 用户 U 持有 Diamond tier（含隐身特权）+ 隐身开关 ON
- **When** U 进入房间 R
- **Then** UserEnteredRoom 广播 visibility=hidden；房间在线列表不展示 U；但房主在管理面板可见

### GWT-NPR2（礼物折扣）
- **Given** Gold tier 折扣率 0.9，礼物原价 100 金币
- **When** 用户送出
- **Then** 钱包扣 90 金币；gift_transactions.discount_amount=10；GiftSent 广播原价 + 折扣字段

### GWT-NPR3（抢麦优先）
- **Given** 队列中已有 [普通用户 X, 普通用户 Y]
- **When** Silver tier 用户 Z 加入队列
- **Then** 新队列顺序 = [Z, X, Y]；按 NOBLE_MIC_PRIORITY_WEIGHT_SILVER 插队；同 tier 后到者排在前到者之后

### GWT-NPR4（过期后徽章消失）
- **Given** 用户 U 在 Expired
- **When** 客户端打开资料卡
- **Then** server 不下发 noble_tier 字段；客户端徽章不渲染

### GWT-NPR5（钩子集中入口 grep）
- **Given** 代码审查
- **When** grep 业务代码 `noble_tier ==` 或 `tier.is_diamond()` 等散落判断
- **Then** 仅出现在 `NoblePrivilegeHook` 实现内；业务调用方一律 `hook.apply_*()`

---

## §6 变更记录

| 版本 | 日期 | 摘要 |
|------|------|------|
| v1.0 | 2026-05-15 | 初版 |
