# Server 端：贵族体系（E-09）

**Last Updated:** 2026-05-10 · **Status:** DoD ✅

## 模块职责

6 档贵族等级体系（knight/baron/viscount/earl/duke/king），涵盖购买/续费/升级强事务、进场广播、特权钩子、自动续费 cron。

## 目录结构

```
app/server/src/modules/nobility/
├── controller.rs           # HTTP handlers（list_tiers / get_me / purchase）
├── service.rs              # NobilityServicePort trait + FakeNobilityService
├── purchase.rs             # decide_purchase() 纯函数（升级补差公式）
├── cron.rs                 # decide_renew() + 3-strike 续费策略
├── privileges.rs           # 特权计算纯函数（隐身/折扣/免密/抢麦权重）
├── global_broadcast.rs     # GlobalBroadcastPort trait + FakeGlobalBroadcast stub
├── dto.rs                  # 所有请求/响应 DTO
├── routes.rs               # 路由注册
└── mod.rs

app/server/migrations/
└── 011_create_nobility.sql # noble_tiers / user_nobles / noble_history / noble_global_broadcast_log

app/shared/src/models/
└── nobility.rs             # NoblePrivileges struct（纯函数数学计算）
```

## 关键设计决策

### 1. 整数运算
- 钻石计算全程 `i64`，禁止 `f32`/`f64`
- 升级补差公式：`refund = old_monthly * remaining_days / 30`，`charge = max(0, new_monthly * duration_days / 30 - refund)`
- 月津贴计算：`stipend = monthly_diamonds * (privilege.stipend_percent / 100)`，保留整数部分

### 2. 续费叠加上限
- 同档续费条件：`remaining_days > 30`（严格大于），不允许续费则返回 40913
- 升级：始终允许，补差扣款
- 降级：始终拒绝，返回 40911

### 3. 向后兼容
`JoinRoomDeps.nobility_service: Option<Arc<dyn NobilityServicePort>>`：
- 生产：Some(Arc::new(NobilityService {...}))
- 历史测试：None，传统逻辑不受影响

### 4. 进场广播
- `GlobalBroadcastPort` trait 隔离 Redis Pub/Sub（生产实现待接入）
- 当前：`FakeGlobalBroadcast` stub 实现，记录日志但不发送
- LV5+(duke/king) 进场触发全服跑马灯 `NobleEntranceGlobal` 消息
- LV3+ 房间内触发 `NobleEntered` 信令

### 5. 隐身规则
- 持有贵族且 `privileges.invisible_enabled=true` 时，在观众席列表隐身
- 仅房主、管理员、自己可见
- 房间内其他地方（聊天、麦位）正常显示，不隐身

## 状态机（贵族生命周期）

```
None ──[purchase]──→ Active ──[expire_at < now]──→ GracePeriod ──[expire+7d]──→ None
          ↑                          ↑
          └──[renew/upgrade]────────┘
```

**关键约束**：
- `user_nobles.user_id` 为 PRIMARY KEY，保证一个用户最多持有一个贵族
- 过期清理 cron 每小时执行，无需手动触发
- 降级时直接拒绝，无中间状态

## 🔌 协议入口索引

| # | 协议类型 | 客户端入口 | URL / 信令 | 服务端处理函数 | protocol/ 锚点 |
|---|---------|---------|-----------|--------------|---------------|
| 1 | HTTP GET | Android `NobleApi.kt::getTiers` ⭐ | `GET /api/v1/nobles/tiers` | `nobility::controller::list_tiers_handler` | [nobility_api.md §10.3.1](../protocol/nobility_api.md#1031-贵族列表-get-apiv1noblestiers) |
| 2 | HTTP GET | Android `NobleApi.kt::getMe` ⭐ | `GET /api/v1/nobles/me` | `nobility::controller::get_me_handler` | [nobility_api.md §10.3.2](../protocol/nobility_api.md#1032-我的贵族-get-apiv1noblesme) |
| 3 | HTTP POST | Android `NobleApi.kt::purchase` ⭐ | `POST /api/v1/nobles/purchase` | `nobility::controller::purchase_handler` | [nobility_api.md §10.3.3](../protocol/nobility_api.md#1033-购买续费升级-post-apiv1noblespurchase) |
| 4 | WS S→C 单播 | — | `BalanceUpdated` reason=`noble_purchase/renew/upgrade_proration` | wallet 触发 | [websocket_signals.md §6.14.2](../protocol/websocket_signals.md#61402-balanceupdatedsc-单播) |
| 5 | WS S→C 单播 | — | `NobleChanged`（单播 + 房间广播）| nobility controller | [nobility_api.md §10.4.1](../protocol/nobility_api.md#1041-noblechangedsc-单播) |
| 6 | WS S→C 单播 | — | `NobleRenewSuccess / NobleRenewFailed / NobleExpired` | cron 触发 | [nobility_api.md §10.4.2-4](../protocol/nobility_api.md#1042-noblerenewsuccesssc-单播) |
| 7 | WS S→Room 广播 | — | `NobleEntered`（LV3+ 进场特效）| `room::handler::lifecycle` | [nobility_api.md §10.4.5](../protocol/nobility_api.md#1045-nobleenteredroom-广播) |
| 8 | Redis Pub/Sub | — | `NobleEntranceGlobal`（LV5+ 全服跑马灯）| `nobility::global_broadcast::GlobalBroadcastPort` | [nobility_api.md §10.4.6](../protocol/nobility_api.md#1046-nobleentranceglobalpubsub) |
| 9 | WS S→C / S→Room | — | `UserJoined.noble` 扩展字段 | `room::handler::lifecycle` | [nobility_api.md §10.4.7](../protocol/nobility_api.md#1047-userjoinednoble-字段扩展) |
| 10 | HTTP POST | Admin `AdminNobleService.kt::grantTier` ⭐ | `POST /api/v1/admin/users/:id/noble/grant` | `admin::handler::grant_noble_handler` | [nobility_api.md §10.5](../protocol/nobility_api.md#105-admin-rest) |

## 错误码

| 代码 | HTTP | 含义 | 触发场景 |
|------|------|------|---------|
| 40911 | 409 | DOWNGRADE_NOT_ALLOWED | 尝试购买低档贵族但已持有更高档 |
| 40912 | 422 | INSUFFICIENT_BALANCE | 钻石不足（含升级补差） |
| 40913 | 409 | SAME_TIER_RENEWAL_OVERLAP | 同档续费但剩余天数 > 30d（避免无限囤） |
| 40914 | 404 | TIER_NOT_ACTIVE | tier 已下架 |
| 40915 | 409 | NOBLE_PRIVILEGE_BLOCKED | 特权被禁用（如风控临时关闭某用户的全服广播） |
| 40916 | 409 | RENEW_REMINDER_ACK_INVALID | event_id 不存在或已确认 |
| 40917 | 422 | PRIVILEGES_SCHEMA_INVALID | Admin 提交的 privileges JSON 不符合 schema |

详见 [nobility_api.md §10.6](../protocol/nobility_api.md#106-错误码表e-09)

## 特权钩子绑定表

| 特权 | 服务端模块 | 实现文件 | 约束条件 |
|------|----------|---------|--------|
| 徽章/气泡/资料框 | RenderContextBuilder | `nobility::controller.rs` | 所有返回用户对象的 API/WS 响应序列化前调用 |
| 进场特效 | RoomService::on_join | `room::handler::lifecycle.rs` | LV3+ 房间内广播 NobleEntered，LV5+ 全服广播 NobleEntranceGlobal |
| 观众席置顶 | AudienceListService | `room::handler.rs` | 返回观众席时按 (noble_level desc, joined_at asc) 排序 |
| 隐身 | PresenceService::list_visible_users | `room::handler.rs` | invisible_enabled=true 用户在观众席不可见，仅房主/管理员可见 |
| 免密（免密房） | JoinRoomService::check_password_or_bypass | `room::handler::lifecycle.rs` | LV4+ 进入密码房返回 200，跳过密码检验 |
| 优先抢麦 | MicQueueService | `room/handler/mic.rs` 或 Lua | 100ms 内多人抢同一麦位时，贵族优先成功（权重 1.0→10.0） |
| 礼物折扣 | GiftService::send | `modules/gift/send_gift/handler.rs` | 结算价 = 原价 * (1 - discount_percent / 100)，向下取整 |
| 全服广播 | NobilityService::on_entrance | `nobility/global_broadcast.rs` | LV5+ 进场向 Redis `noble:global` Pub/Sub channel 发送 NobleEntranceGlobal |
| 月津贴 | NobilityRenewalService | `nobility/cron.rs` | renew/purchase 事务尾部追加 stipend_transaction(user_id, amount) |
| 过期降级 | NobilityExpiryCron | `nobility/cron.rs` | 每小时扫描 expire_at < now，清理 user_nobles + 推送 NobleExpired |

详见 [nobility_api.md §10.7](../protocol/nobility_api.md#107-特权钩子绑定服务端-enforcement-map引用产品-3513)

## 延后项（非阻塞）

- **GlobalBroadcastPort 生产 Redis 实现**
  - 当前：`FakeGlobalBroadcast` stub 仅记录日志
  - 待续：接入 Redis Pub/Sub channel `noble:global`，服务启动时注入真实实现

- **cron 真实 DB 扫描**
  - 当前：`run_renew_phase()` / `run_expire_phase()` 标记为 stub
  - 待续：实现完整 SQL 扫描 + 批量更新（含 3-strike 失败计数与 auto_renew 关闭）

- **礼物折扣全量接入**
  - 当前：GiftService 内部已预留 discount 字段
  - 待续：`GiftSendService.send()` 调用 `NobleService.get_discount()` 计算折扣，写流水时记录 discount_pct

- **优先抢麦 Lua 权重接入**
  - 当前：抢麦 Lua 脚本为原 FIFO，无权重概念
  - 待续：传入 `noble_level` 参数，按 softmax 权重扩展为权重抽签

## 关联文档

- 产品方向：[phase1_nobility.md](../product/phase1_nobility.md)（§3.5 特权细则为字段值唯一来源）
- 协议规范：[nobility_api.md](../protocol/nobility_api.md)
- 任务子表：[模块11-贵族体系 (E-09).md](../tasks/模块11-贵族体系%20(E-09).md)
- 依赖模块：[payment.md](./payment.md)（真金通道 SKU）
