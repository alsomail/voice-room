# 业务约束字典 (Business Constraints)

> **作用**：产品边界类文档之三。本文件**唯一定义**所有"数字常量边界"：金额上限/字数上限/频率限制/时长上限/并发上限/TTL/重试次数等。
> **不重复**：业务流程 → `user_journeys.md`；状态语义 → `state_machines.md`；字段格式 → `doc/protocol/`。
> **引用方式**：Spec / TDS / test-design / 客户端校验 / 服务端校验**唯一以本文件常量名为准**。代码中常量命名必须与本表一致。
> **变更管控**：本文件任何数值变更必须走 ADR（`doc/adr/`）记录，并触发对应 Spec 的 §5 GWT 复检。

---

## 0. 命名规范

- 所有常量使用 `UPPER_SNAKE_CASE`。
- 单位后缀强制：`_SEC` / `_MS` / `_DAYS` / `_USD` / `_GOLD`（虚拟币）/ `_CHARS` / `_COUNT` / `_BYTES`。
- 数值如有"中东市场特殊值"，在备注列说明。

---

## 1. 金额与虚拟币 <a id="money"></a>

| 常量名 | 默认值 | 单位 | 描述 | 备注 |
|--------|--------|------|------|------|
| `DAILY_RECHARGE_CAP_USD` | 500 | USD | 单用户单日充值上限 | 风控阈值，超过转人工 |
| `MIN_RECHARGE_USD` | 0.99 | USD | 单笔最低充值 | 对齐 Google Play 最低档 |
| `MAX_RECHARGE_USD` | 199.99 | USD | 单笔最高充值 SKU | 大 R 专属 |
| `GIFT_MIN_PRICE_GOLD` | 1 | 金币 | 最低礼物档位 | "微心意"礼物 |
| `GIFT_MAX_PRICE_GOLD` | 100000 | 金币 | 最高礼物档位 | 顶级豪礼 |
| `GOLD_USD_RATE` | 100 | 金币/USD | 充值兑换比 | 1 USD = 100 金币 |
| `REFUND_NEGATIVE_BALANCE_LIMIT_GOLD` | -50000 | 金币 | 退款允许的最大负余额 | 超出需人工干预 |

---

## 2. 文本与媒体长度 <a id="text-length"></a>

| 常量名 | 默认值 | 单位 | 描述 |
|--------|--------|------|------|
| `NICKNAME_MAX_CHARS` | 20 | 字符 | 昵称最大长度（Unicode codepoint） |
| `ROOM_TITLE_MAX_CHARS` | 30 | 字符 | 房间标题 |
| `ROOM_NOTICE_MAX_CHARS` | 200 | 字符 | 房间公告 |
| `CHAT_MSG_MAX_CHARS` | 200 | 字符 | 公屏单条消息 |
| `BIO_MAX_CHARS` | 100 | 字符 | 个人简介 |
| `AVATAR_MAX_BYTES` | 2097152 | 字节 | 头像文件 2 MiB |
| `AVATAR_ALLOWED_MIME` | `image/jpeg,image/png,image/webp` | - | 白名单 |

---

## 3. 频率与速率限制 <a id="rate-limit"></a>

| 常量名 | 默认值 | 单位 | 描述 |
|--------|--------|------|------|
| `OTP_REQUEST_INTERVAL_SEC` | 60 | 秒 | 同手机号两次发码最小间隔 |
| `OTP_MAX_ATTEMPTS_PER_DAY` | 10 | 次 | 单手机号单日发码上限 |
| `OTP_VERIFY_MAX_FAILS` | 5 | 次 | 验证失败次数（连续）→ 锁 5 分钟 |
| `OTP_LOCK_DURATION_SEC` | 300 | 秒 | OTP 锁定时长 |
| `CHAT_MSG_RATE_PER_SEC` | 2 | 条/秒 | 公屏发送速率 |
| `CHAT_MSG_RATE_BURST` | 5 | 条 | 突发桶大小 |
| `GIFT_COMBO_MAX` | 99 | 次 | 单次连击最大次数 |
| `GIFT_COMBO_WINDOW_MS` | 3000 | 毫秒 | 连击合并窗口 |
| `LOGIN_FAIL_LOCK_THRESHOLD` | 5 | 次 | 连续登录失败锁定阈值 |
| `LOGIN_FAIL_LOCK_DURATION_SEC` | 900 | 秒 | 登录失败锁定时长 |

---

## 4. 时长与 TTL <a id="duration"></a>

| 常量名 | 默认值 | 单位 | 描述 |
|--------|--------|------|------|
| `JWT_ACCESS_TTL_SEC` | 7200 | 秒 | Access Token 有效期 2h |
| `JWT_REFRESH_TTL_SEC` | 1209600 | 秒 | Refresh Token 14 天 |
| `OTP_CODE_TTL_SEC` | 300 | 秒 | 验证码 5 分钟有效 |
| `ROOM_EMPTY_TTL_SEC` | 7200 | 秒 | 空房保留 2h 后关闭 |
| `WS_HEARTBEAT_INTERVAL_SEC` | 25 | 秒 | 客户端心跳间隔 |
| `WS_HEARTBEAT_TIMEOUT_SEC` | 60 | 秒 | 服务端判定离线阈值 |
| `MIC_KICK_BAN_HOURS` | 24 | 小时 | 被踢用户同房禁入时长 |
| `NOBLE_RENEW_WINDOW_DAYS` | 3 | 天 | 到期前提前续费窗口 |
| `NOBLE_GRACE_DAYS` | 3 | 天 | 续费失败宽限期 |
| `ORDER_VERIFY_TIMEOUT_SEC` | 30 | 秒 | Google verifyPurchase 超时 |
| `WS_RECONNECT_MAX_BACKOFF_SEC` | 30 | 秒 | 重连指数退避上限 |

---

## 5. 并发与容量 <a id="capacity"></a>

| 常量名 | 默认值 | 单位 | 描述 |
|--------|--------|------|------|
| `ROOM_MAX_USERS` | 500 | 人 | 单房间在线人数上限 |
| `ROOM_MAX_MIC_SEATS` | 9 | 个 | 麦位数（房主 1 + 嘉宾 8） |
| `USER_MAX_ROOM_FOLLOW` | 200 | 个 | 单用户关注房间数 |
| `USER_MAX_FRIENDS` | 1000 | 个 | 好友上限 |
| `LEADERBOARD_TOP_N` | 100 | 名 | 排行榜展示位 |
| `CHAT_HISTORY_KEEP_COUNT` | 200 | 条 | 进房拉取历史消息条数 |
| `GIFT_RECORD_QUERY_PAGE_SIZE` | 50 | 条/页 | 礼物流水分页 |

---

## 6. 重试与超时 <a id="retry"></a>

| 常量名 | 默认值 | 单位 | 描述 |
|--------|--------|------|------|
| `HTTP_REQUEST_TIMEOUT_SEC` | 10 | 秒 | 客户端 HTTP 默认超时 |
| `HTTP_RETRY_MAX_ATTEMPTS` | 3 | 次 | 客户端可重试次数 |
| `RTC_PUBLISH_RETRY_MAX` | 3 | 次 | RTC 推流失败重试 |
| `GOOGLE_VERIFY_RETRY_MAX` | 3 | 次 | 服务端校验 Google 重试 |
| `GOOGLE_VERIFY_RETRY_BACKOFF_MS` | 1000,2000,4000 | 毫秒 | 指数退避序列 |

---

## 7. 治理与风控 <a id="governance"></a>

| 常量名 | 默认值 | 单位 | 描述 |
|--------|--------|------|------|
| `MUTE_DURATION_L1_MIN` | 5 | 分钟 | L1 警告禁言 |
| `MUTE_DURATION_L2_MIN` | 30 | 分钟 | L2 禁麦 |
| `BAN_DURATION_L4_DAYS` | 7 | 天 | L4 封号 |
| `BAN_DURATION_L5_DAYS` | -1 | - | L5 永封（-1 哨兵） |
| `KICK_COUNT_24H_AUTOBAN_THRESHOLD` | 3 | 次 | 24h 内被踢 N 次自动封号 24h |
| `REPORT_COOLDOWN_SEC` | 60 | 秒 | 同人举报冷却 |

---

## 8. 中东市场特殊值 <a id="mena"></a>

| 常量名 | 默认值 | 描述 |
|--------|--------|------|
| `DEFAULT_LOCALE` | `ar-SA` | 默认阿拉伯语（沙特） |
| `SUPPORTED_LOCALES` | `ar,en` | 支持的语言 |
| `RTL_LOCALES` | `ar,he,fa,ur` | 启用 RTL 布局的语言 |
| `DEFAULT_TIMEZONE` | `Asia/Riyadh` | 默认时区（UTC+3） |
| `WEEKLY_RESET_DAY` | `Saturday` | 排行榜周重置日（中东周末为五六） |

---

## 9. 验证规则正则 <a id="regex"></a>

| 常量名 | 模式 | 描述 |
|--------|------|------|
| `PHONE_REGEX_MENA` | `^\+(966|971|965|974|973|968|962)\d{8,9}$` | 中东主要国家手机号 |
| `NICKNAME_REGEX` | `^[\p{L}\p{N}_\u0600-\u06FF]{1,20}$` | 允许阿拉伯字符 |
| `ROOM_TITLE_REGEX` | `^[\p{L}\p{N}\p{P}\s]{1,30}$` | 允许标点 |

---

## 10. 变更记录

| 版本 | 日期 | 摘要 |
|------|------|------|
| v1.0 | 2026-05-15 | 初版：9 大类约束统一抽出，作为唯一事实源 |

---

## 附录 A：使用规范

1. **服务端**：在 `app/shared/src/constants.rs`（如不存在则新建）维护对应常量；任何业务校验必须 import，禁止字面量。
2. **客户端**：Android 在 `BuildConfig` 或 `Constants.kt`；Web 在 `src/constants/business.ts`。
3. **测试**：测试用例边界值必须直接引用本文件常量名，禁止硬编码。
4. **变更流程**：修改 → ADR → 同步代码常量 → 触发相关 Spec §5 GWT 复检 → tasks/index.md changelog。
