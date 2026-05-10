# Phase 1 - 贵族会员体系 (E-09)

> **版本**: v1.1（v1.0 基础上扩充 §3.5 特权细则 13 小节）
> **创建日期**: 2026-05-09 / **修订**: 2026-05-10
> **负责人**: PM Agent
> **对应 Epic**: E-09 贵族体系
> **前置 Epic**: E-07 钱包闭环（✅ Done） / E-08 真支付（设计中，本 Epic 可在 E-08 沙箱跑通后并行启动）
> **状态**: 🟡 设计中

---

## 1. 战略定位

### 1.1 营收权重
参考 [competitors.md §Tier 2](./competitors.md)：
- **YoHo 贵族体系占营收 25%**（骑士 30$ → 国王 5000$）
- **Ahlan 守护勋章 + 贵族占 15%**（沙特大 R 复购率 > 60%）
- **Yalla VIP 占 20%**

> **结论**：贵族 = 第二大营收支柱（仅次于礼物），且**复购率最高**（月度续费）。E-09 与 E-07/E-08 协同后，预期 ARPU 可从 $4 提升到 $7-9。

### 1.2 贵族 = "可被看见的优越感"
中东大 R 用户的核心心理：
- **被识别**：进房就要"全房间所有人立刻知道我是贵族"
- **被尊重**：自动获得管理员部分权限（隐身、优先发言）
- **被服务**：专属客服、专属房间装扮、专属礼物折扣

E-09 必须在 **3 处视觉触点**让贵族身份"无法忽略"：
1. **进场特效**：全屏 Lottie + BGM（按等级时长 3-15s）
2. **聊天身份**：消息气泡贵族色 + 徽章
3. **资料卡**：永久贵族框 + 王冠

### 1.3 货币双通道：钻石 OR 真金
| 等级 | 月费（钻石） | 月费（USD，走 E-08） | 推荐通道 |
|------|------------|--------------------|---------|
| 骑士 (Knight) | 3000 💎 | $9.99 | 钻石（用礼物收入续费） |
| 男爵 (Baron) | 10,000 💎 | $29.99 | 钻石 |
| 子爵 (Viscount) | 30,000 💎 | $99.99 | 任一 |
| 伯爵 (Earl) | 100,000 💎 | $299.99 | 真金（避免大额扣钻提示风险） |
| 公爵 (Duke) | 300,000 💎 | $999.99 | 真金 |
| 国王 (King) | 1,000,000 💎 | $3999.99 | 仅真金（运营白名单审核） |

> **设计原则**：低端贵族鼓励"礼物收入续费"形成内循环；高端贵族走真金确保资金真实流入。

---

## 2. 范围边界 (Scope)

### 2.1 In Scope
| 领域 | 内容 |
|------|------|
| 贵族配置 | `noble_tiers` 表（tier_id / name_en / name_ar / level INT / monthly_diamonds / monthly_usd / privileges JSONB / icon_url / entrance_animation_url / badge_color / is_active） |
| 用户贵族 | `user_nobles` 表（user_id / tier_id / start_at / expire_at / auto_renew BOOL / source ENUM(diamond/google_play/admin_grant) / created_at） |
| 购买流程（钻石） | POST `/api/v1/nobles/purchase`：扣余额事务 + 写 user_nobles + 续期或新购 |
| 购买流程（真金） | 复用 E-08 `payment_orders` + 新 SKU 类型 `noble_pack` + 验签入账后写 user_nobles |
| 续费 | 自动续费（auto_renew=true 到期前 24h cron 扣款）+ 失败降级提示 |
| 过期 | cron 每小时扫 expire_at < now → 设 tier=null + 推送 `NobleExpired` |
| 特权 | 进场特效 / 徽章 / 字体颜色 / 隐身（房间观众席不显示） / 入房免密码（公爵+） / 优先抢麦（伯爵+） |
| Admin | 贵族 tier CRUD / 手动赠送 / 用户贵族查询 |
| Web | 贵族管理页 + 用户详情贵族 Tab |
| Android | 贵族中心 / 购买页 / 进场特效 / 徽章组件 / 资料卡贵族框 |

### 2.2 Out of Scope
| 领域 | 延后到 | 原因 |
|------|--------|------|
| 家族赠送贵族 | E-11 + 后续 | 依赖 E-11 家族体系 |
| 贵族任务（升级路径） | Phase 2 | MVP 仅靠"花钱"升级 |
| 贵族专属房 | Phase 2 | 房间体系扩展需独立 Epic |
| 退订/退款 | E-08 退款流程统一 | 共用 |

---

## 3. 特权矩阵

| 特权 | 骑士 | 男爵 | 子爵 | 伯爵 | 公爵 | 国王 |
|------|:---:|:---:|:---:|:---:|:---:|:---:|
| 专属徽章 | ✅ 银 | ✅ 蓝 | ✅ 紫 | ✅ 金 | ✅ 钻 | ✅ 红王冠 |
| 进场特效时长 | 3s | 5s | 7s | 10s | 12s | 15s |
| 进场 BGM | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 聊天气泡变色 | ❌ | ✅ 蓝 | ✅ 紫 | ✅ 金 | ✅ 钻 | ✅ 红 |
| 房间观众席置顶 | ❌ | ❌ | ✅ | ✅ | ✅ | ✅ |
| 隐身（不显示在线） | ❌ | ❌ | ❌ | ✅ | ✅ | ✅ |
| 入房免密码 | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ |
| 优先抢麦（同时间触发优先） | ❌ | ❌ | ❌ | ✅ | ✅ | ✅ |
| 礼物折扣 | -0% | -2% | -5% | -8% | -10% | -15% |
| 资料卡贵族框 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 全服上线广播 | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ |
| 专属客服 | ❌ | ❌ | ❌ | ✅ | ✅ | ✅ |

> **数据库存储**：privileges JSONB 字段直接存运算所需的 boolean / number 子集，便于配置化扩展。

---

## 3.5 特权细则（Privilege Specs · v2 强化）

> **目的**：为客户端 / 服务端实现提供**确定性参数**，杜绝口头描述带来的歧义。每条特权列出：① 触发点 ② 参数表 ③ 服务端校验位置 ④ 客户端渲染位置 ⑤ 异常降级。

### 3.5.1 专属徽章（Badge）

| 等级 | 徽章资源 | 颜色 token | 形状 |
|------|---------|----------|-----|
| 骑士 | `badge_knight.svg` | `#C0C0C0` 银 | 盾形 |
| 男爵 | `badge_baron.svg` | `#3B82F6` 蓝 | 盾形 + 双羽 |
| 子爵 | `badge_viscount.svg` | `#8B5CF6` 紫 | 盾形 + 月桂 |
| 伯爵 | `badge_earl.svg` | `#F59E0B` 金 | 王冠（小） |
| 公爵 | `badge_duke.svg` | `#06B6D4` 钻蓝 | 王冠 + 镶钻 |
| 国王 | `badge_king.svg` | `#DC2626` 红 + `#F59E0B` 金边 | 大王冠 + 光晕动画（Lottie 循环 2s） |

- **渲染点**：用户名前缀（公屏 / 排行榜 / 资料卡 / 麦位）；尺寸 16dp（公屏）/ 24dp（资料卡）/ 12dp（麦位）。
- **降级**：图片加载失败 → 显示纯色圆点 + 等级缩写（"K1"~"K6"）。

### 3.5.2 进场特效（Entry Effect）

| 等级 | 时长 | 范围 | BGM | 跑马灯 | 用户可关闭 |
|------|------|------|-----|--------|----------|
| 骑士 | 1.5s | 房间内仅顶部跑马灯 | ❌ | ✅ 蓝 | 自身 ✅ / 其他可关闭接收 |
| 男爵 | 2.0s | 跑马灯 | 6s 短 BGM | ✅ 紫 | 同上 |
| 子爵 | 2.5s | 半屏粒子 + 跑马灯 | 6s | ✅ 紫 | 同上 |
| 伯爵 | 4.0s | 全屏 Lottie | 8s | ✅ 金 | 同上 |
| 公爵 | 6.0s | 全屏 + 黄金尘埃 + 镜头光晕 | 10s（古阿拉伯小号）| ✅ 金 | 同上 |
| 国王 | 8.0s | 全屏 + 红毯 + 烟花 + 头像定格 | 12s 史诗 | ✅ 红金双语 | **不可关闭**（产品规则） |

- **服务端**：`NobleService.on_join_room()` 校验等级 → 广播 `NobleEntered` WS（含 level/资源 URL）。
- **客户端**：详见 [T-30072](../design/android/T-30072.md) 进场特效设计稿（队列、降级、低端机适配）。
- **频控**：同一用户 5 分钟内重复进同一房不再播放（服务端 Redis `noble:entry:{user}:{room}` TTL=300）。

### 3.5.3 聊天气泡（Chat Bubble）

| 等级 | 气泡形状 | 背景渐变 | 边框 | 用户名颜色 |
|------|---------|---------|------|----------|
| 骑士 | 圆角矩形 | 默认（无变化） | 1px 银 | 默认 |
| 男爵 | 圆角矩形 | 浅蓝渐变 #E0F2FE → #BAE6FD | 1px 蓝 | #1E40AF |
| 子爵 | 圆角矩形 | 浅紫渐变 #F5F3FF → #DDD6FE | 1px 紫 | #6D28D9 |
| 伯爵 | 圆角矩形 + 左侧金边装饰 | 金色渐变 #FEF3C7 → #FCD34D | 2px 金 | #B45309 |
| 公爵 | 异形（盾形右上角）| 钻蓝渐变 + 镶嵌纹理 | 2px 钻 + 内描金 | #0E7490 |
| 国王 | 异形（旗帜形）| 红金虹彩动态渐变（CSS conic-gradient 旋转 8s）| 2px 金 + 外光晕 | #991B1B + 描金 |

- **降级**：渐变在低端机型禁用（`Build.VERSION.SDK_INT < 26`），仅保留边框颜色。

### 3.5.4 房间观众席置顶（Top Pin）

| 等级 | 置顶范围 | 置顶位置 | 互踢规则 |
|------|---------|---------|---------|
| 骑士/男爵 | ❌ | - | - |
| 子爵 | 自己进入的房间观众席 | Top 1-3（同档位按进房时间）| 同档先到先得 |
| 伯爵 | 同上 | Top 1-2 | 高档位顶替低档位 |
| 公爵 | 同上 + 大厅"在线贵族"卡片 | 永远 Top 1（同档位轮播）| 同上 |
| 国王 | 同上 + **全服首页公告位** | Top 1 | 国王永远在子爵/伯爵/公爵之前 |

- **服务端**：`AudienceListService.list()` 按 `(noble_level desc, joined_at asc)` 排序。
- **客户端**：观众列表无需特殊处理，按 server 顺序渲染。

### 3.5.5 隐身（Invisibility）

| 等级 | 隐身范围 | 仍可见者 |
|------|---------|---------|
| 骑士~子爵 | ❌ | - |
| 伯爵 | 麦下观众席不显示我 | 麦上用户 + 房主 + Admin |
| 公爵 | 麦下 + 麦上不显示我；进房无特效 | 房主 + Admin |
| 国王 | 全场所有人不可见进出（特效仍播但可关）| Admin only |

- **关键**：隐身**不影响 RTC 音频**（仍能收到声音/送礼）；仅影响 UI 列表与进场广播是否分发。
- **服务端**：在线列表查询时按当前观察者权限过滤（`InvisibilityFilter`）。

### 3.5.6 入房免密码（Bypass Password）

| 等级 | 范围 |
|------|-----|
| 骑士~伯爵 | ❌（必须输入正确密码）|
| 公爵 | 可绕过普通用户房间密码；**不绕过房主"贵族也需密码"开关** |
| 国王 | 同上，且自动加入房主白名单 |

- **服务端**：`JoinRoomService` 在密码校验前判断 `noble_level >= 5` AND `room.allow_noble_bypass = true`（房主开关，默认 ON）。

### 3.5.7 优先抢麦（Mic Priority）

> **本质**：抢麦不是"先到先得"，而是**带权重的队列**。

| 等级 | 抢麦权重 | 含义 |
|------|---------|-----|
| 普通用户 | 1.0 | 基线 |
| 骑士/男爵 | 1.0 | 无加成 |
| 子爵 | 1.0 | 无加成 |
| 伯爵 | 1.5 | 同时按按钮，有 60% 概率胜出 |
| 公爵 | 3.0 | 75% |
| 国王 | 10.0 | 91% |

- **算法**：500ms 滑动窗口收集所有抢麦请求 → 按权重 softmax 抽签（避免 100% 必胜引发投诉），日志记录抽签随机种子用于审计。
- **服务端**：`MicQueueService.try_acquire()` 内实现，与现有抢麦锁兼容。

### 3.5.8 礼物折扣（Gift Discount）

| 等级 | 折扣 | 应用范围 |
|------|------|---------|
| 骑士 | 0% | - |
| 男爵 | 2% | 全礼物 |
| 子爵 | 5% | 全礼物 |
| 伯爵 | 8% | 全礼物 |
| 公爵 | 10% | 全礼物 |
| 国王 | 15% | 全礼物 |

- **结算**：服务端 `GiftService.send()` 在扣钻前应用 `floor(price × (1 - discount))`，保证主播收益**不打折**（差价由平台补贴）。
- **审计**：`gift_transactions` 表新增 `discount_pct` 字段，便于财务对账。

### 3.5.9 全服上线广播（Global Login Broadcast）

| 等级 | 广播形式 | 频率限制 |
|------|---------|---------|
| 骑士~伯爵 | ❌ | - |
| 公爵 | 大厅顶部跑马灯（蓝）| 每日 1 次（首次登录）|
| 国王 | **全服所有房间** + 大厅顶部跑马灯（红金）+ Push 通知关注者 | 每日 1 次 |

- **服务端**：登录 hook 触发，写 `daily_broadcast_log` 防重复。
- **客户端**：跑马灯沿用 §3.5.2 进场跑马灯组件 + 加 `scope=global` 字段区分样式。

### 3.5.10 专属客服（VIP Support）

| 等级 | SLA 首响 | 通道 |
|------|---------|------|
| 骑士~子爵 | 24h | 通用 ticket |
| 伯爵 | 2h（工作时间）| VIP 邮箱 + Slack 通道 |
| 公爵 | 30min | 专属客服经理（实名）|
| 国王 | 即时（< 5min）| 专属经理 + WhatsApp 直联 |

- **客户端**：客服入口按等级显示不同文案（"联系客服" → "联系您的专属经理 王小妹"）。

### 3.5.11 月度钻石返还（Monthly Stipend）

> 用户购买/续费贵族当月，平台返还一定比例钻石作为"贵族月津贴"，提升续费动力。

| 等级 | 返还比例 | 计算基数 | 发放节奏 |
|------|---------|---------|---------|
| 骑士 | 5% | 月费钻石值 = 150 💎 | 续费成功立即发放 |
| 男爵 | 8% | 800 💎 | 同上 |
| 子爵 | 10% | 3000 💎 | 同上 |
| 伯爵 | 12% | 12000 💎 | 同上 |
| 公爵 | 15% | 45000 💎 | 同上 |
| 国王 | 20% | 200000 💎 | 同上 |

- **审计**：`wallet_transactions` 写 `source = noble_stipend`，便于剔除该笔流水的财务计算。

### 3.5.12 续费过期与降档（Expiry & Grace）

| 时间点 | 行为 |
|--------|------|
| T-3 天 | Push + IM 系统通知"贵族即将到期" |
| T-1 天 | 进房弹窗 + 提醒续费 |
| T 0 时刻 | 进入 **7 天宽限期**：所有特权保留但不再发放新月津贴 |
| T+7 天 | 正式失效：徽章 / 进场特效 / 气泡 / 折扣全部移除 |
| T+7 ~ T+37 天 | "贵族保留期"：资料卡仍显示**灰色历史徽章**（仅本人可见），续费可恢复连续等级（防止间隔丢失"连续 N 月贵族"成就）|
| T+37 天 | 全部清除 |

- **降级算法**：续费失败时检查是否存在更低有效等级（用户同时持有骑士+伯爵 → 伯爵到期降骑士）。

### 3.5.13 特权钩子（Server Enforcement Map）

> 实现侧规约：每条特权对应**唯一的服务端拦截点**，杜绝散落。

| 特权 | 服务端模块 | 函数 |
|------|----------|------|
| 徽章/气泡/资料框 | `RenderContextBuilder` | `build_user_render_ctx(user_id) -> UserRenderCtx { noble_level, badge_url, bubble_style }` |
| 进场特效 | `RoomService.on_join` | `broadcast_noble_entry()` |
| 观众席置顶 | `AudienceListService` | `list_with_priority()` |
| 隐身 | `PresenceService.list_visible_users` | `apply_invisibility_filter()` |
| 免密 | `JoinRoomService` | `check_password_or_bypass()` |
| 优先抢麦 | `MicQueueService` | `try_acquire_with_priority()` |
| 礼物折扣 | `GiftService.send` | `apply_noble_discount()` |
| 全服广播 | `LoginService.on_success` | `maybe_global_broadcast()` |
| 月津贴 | `NobilityRenewalService` | `grant_stipend()` |
| 过期降级 | `NobilityExpiryCron`（每小时）| `expire_and_downgrade()` |

---

## 4. 业务流程

### 4.1 钻石购买流程
```
Android 用户进入"贵族中心" → GET /api/v1/nobles/tiers 拉等级列表
  → 选档位 → "立即开通"
  → POST /api/v1/nobles/purchase { tier_id, source=diamond, auto_renew=true }
  → Server 强事务：① 扣 users.diamond_balance ② upsert user_nobles（同等级延期，升级覆盖且按比例补差）③ 写 wallet_transactions(type='noble_purchase')
  → 推送 BalanceUpdated + NobleChanged
  → Android：贵族中心刷新到期时间 + 全房间用户看到该用户徽章升级
```

### 4.2 真金购买流程（复用 E-08）
```
选择"真金购买" → POST /api/v1/payments/orders { sku_id=noble_xxx_30d }
  → 走 E-08 完整 Google Play 验签链路 → CREDITED 时除了加余额逻辑外，
    payment_orders.metadata = { type: "noble_pack", tier_id, days } 触发贵族写入分支
  → 写 user_nobles + NobleChanged 广播
```

### 4.3 自动续费
```
cron 每小时：SELECT * FROM user_nobles WHERE auto_renew=true AND expire_at < now() + INTERVAL '24h' AND renewed_for_next=false
  → 尝试扣钻续费（钻石源贵族）→ 成功：expire_at += 30d，标记 renewed=true；推送 NobleRenewSuccess
  → 失败（余额不足）：推送 NobleRenewFailed + 24h 后再次尝试 + 客户端弹"续费失败请充值"
真金源贵族：不自动续费（依赖 Google Play Subscription，Phase 1.5 接入）
```

### 4.4 过期
```
cron 每小时：SELECT * FROM user_nobles WHERE expire_at < now()
  → DELETE 或软删 + 推送 NobleExpired
  → Android 收到后：徽章组件 LaunchedEffect 重组消失 + 弹窗"贵族已到期，是否续费？"
```

### 4.5 关键异常

| 场景 | Server 行为 | Android UI |
|------|-----------|-----------|
| 钻石余额不足 | 返回 INSUFFICIENT_BALANCE | 跳钻石充值（E-08）|
| 已是更高级贵族购低级 | 返回 40911 NOBLE_DOWNGRADE_FORBIDDEN | "您已是 X，无需购买更低等级" |
| 重复请求（同 msg_id） | 幂等返回 | UI 不重复显示购买动画 |
| 自动续费失败连续 3 次 | 关闭 auto_renew + 写 admin_logs | 弹窗 + 黄条提示 |
| 真金订单退款 | E-08 RTDN 退款 → 撤销当期贵族（保留至 expire_at 或立即下架由风控决定） | 弹窗"贵族因订单退款已下架" |

---

## 5. 关键技术约束

1. **特权下发单一事实源**：用户登录返回 + WS 连上后下发 `current_noble`，房间内其他用户通过 `UserJoined`/`MemberSnapshot` 携带 `noble_tier_id` 字段获取（红线 #1）
2. **进场特效防腐**：复用 T-30031 `ILottiePlayer`，特效 URL 由 Server 下发，**严禁**客户端硬编码（红线 #4）
3. **隐身实现**：观众席列表 + UserJoined 广播过滤 `noble_tier.invisible=true` 用户；管理员/房主依然可见（保留治理能力）
4. **优先抢麦**：T-00012 抢麦 Lua 脚本增加贵族权重排序，伯爵+ 在 100ms 内并发请求时优先成功
5. **强事务**（红线 #2）：钻石扣款 + user_nobles 写入 + wallet_transactions = 同一 SQLx Transaction
6. **配置化**：所有等级/特权/特效在 `noble_tiers` 表内运营可改，**严禁**硬编码到代码

---

## 6. 验收指标

| 类别 | 指标 | 目标 |
|------|------|------|
| 功能 | 6 档贵族购买/续费/过期/降级保护全跑通 | 100% |
| 视觉 | 进场特效 P95 加载延迟 | < 1s |
| 一致性 | 贵族过期 cron 滞后 | < 5min |
| 业务 | Phase 1 上线 30 天贵族开通率（DAU） | ≥ 1.5% |
| 业务 | 续费率（月） | ≥ 40% |
| 营收 | 贵族收入占总营收比 | ≥ 15% |

---

## 7. 关联文档

- [Tasks 模块 11 - E-09 贵族体系](../tasks/模块11-贵族体系%20(E-09).md)
- [phase1_payment_billing.md - E-08 真支付](./phase1_payment_billing.md)
- [Android 设计稿 - T-30070 贵族中心](../design/android/T-30070.md)
- [Architecture - 防腐层](../architecture/anticorruption_layer.md)
