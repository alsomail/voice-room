# 业务流程与规则

> 来源：原 `doc/product.md` 第二节  
> 最后更新：2026-04-20

---

## 2.1 核心业务流：手机号一键登录（Phase 0）

### ✅ 正向流程 (Happy Path)

```
1. 用户打开 App
   ↓
2. 输入手机号（自动识别国际区号 +966/+971 等）
   ↓
3. 点击"获取验证码" → App 请求 App Server: POST /auth/send-code
   ↓
4. App Server → Twilio → 用户手机收到 6 位验证码
   ↓
5. 用户输入验证码 → 点击"登录"
   ↓
6. App 请求 App Server: POST /auth/login {phone, code}
   ↓
7. App Server 校验验证码：
   ├─ 手机号已注册 → 查询 user 记录 → 签发 JWT
   └─ 手机号未注册 → 自动创建 user（默认昵称"用户XXXX"） → 签发 JWT
   ↓
8. App 保存 JWT 到本地 → 跳转到房间大厅
```

**设计要点**：
- 注册和登录合二为一，减少用户流失（参考 Yalla 的设计）
- 新用户无需设置密码，手机号即身份
- JWT 有效期 30 天，减少频繁登录

### ❌ 异常流程

**A. 短信发送失败**
- 原因: Twilio 限流、运营商拦截、手机号格式错误
- 方案: 返回具体错误信息（格式错误/频率过快/服务异常）；60 秒冷却期防刷；每日同一手机号限 10 次

**B. 验证码过期/错误**
- 过期: 5 分钟有效期，过期返回 401 + "验证码已过期，请重新获取"
- 错误: 最多尝试 5 次，超过后锁定 30 分钟
- 重发: 60 秒后可重新发送，生成新验证码，旧码失效

**C. 网络断开**
- 发送验证码时断网: 本地显示"网络异常，请检查连接"
- 登录请求时断网: 本地保留输入，恢复后自动重试 1 次
- 登录成功后断网: JWT 已保存，下次打开 App 直接进入大厅

**D. 设备被禁用/账号被封**
- 方案: App Server 返回 403 + 封禁原因 + 封禁到期时间
- 展示: 弹窗提示"您的账号因XXX已被封禁至YYYY-MM-DD"

---

## 2.2 核心业务流：后台管理员登录（Web 端）

### ✅ 正向流程

```
1. 管理员通过 VPN 访问后台地址
   ↓
2. 输入账号 + 密码
   ↓
3. Web 请求 Admin Server: POST /admin/login
   ↓
4. Admin Server 校验 bcrypt 密码 → 签发 JWT (含 role)
   ↓
5. Web 保存 JWT → 跳转数据看板
```

### ❌ 异常流程
- 密码错误 5 次: 账号锁定 30 分钟，记录 IP
- 非 VPN 访问: 网络层直接拒绝（Admin Server 仅内网暴露）
- 权限不足: 访问无权限页面时显示 403 提示

---

## 2.3 核心业务流：虚拟礼物打赏（Phase 1）

### ✅ 正向流程

```
1. 用户进入房间
   ↓
2. 点击"礼物"按钮 → 弹出礼物面板
   ↓
3. 选择礼物 (检查钻石余额) → 选择赠送对象
   ↓
4. 点击"赠送" → 客户端发送 SendGift 请求
   ↓
5. Server 验证 & 扣款 (事务) → 广播 GiftSent 事件
   ↓
6. 所有房间内用户收到广播 → 播放礼物特效动画
   ↓
7. 接收者魅力值增加，赠送者财富值增加
   ↓
8. 刷新排行榜
```

### ❌ 异常流程

**A. 网络断开/重连场景**
- Server 基于 `msg_id` 去重，重复请求返回相同结果
- 客户端重连后拉取最近 10 条事件补偿渲染
- 本地显示"赠送中..."状态，超时 5 秒自动重试 1 次

**B. 余额不足**
- Server 返回 `InsufficientBalance` 错误码
- 客户端弹窗提示"钻石不足，立即充值？" + 跳转充值页

**C. 赠送对象已下麦/离开房间**
- Server 检查目标用户是否仍在房间
- 客户端提示"该用户已离开，礼物未赠送，钻石未扣除"

**D. 并发抢购限量礼物**
- Redis 原子递减库存，售罄后返回 `SoldOut`
- 客户端显示实时剩余数量 (WebSocket 推送)

---

## 2.4 核心业务流：麦位管理

### ✅ 正向流程：用户主动上麦

```
1. 用户点击空闲麦位
   ↓
2. 客户端请求麦克风权限 (首次)
   ↓
3. 发送 TakeMic 请求 (携带麦位号)
   ↓
4. Server 检查麦位是否空闲 + 用户是否被禁麦
   ↓
5. 成功后广播 MicTaken 事件
   ↓
6. 所有客户端渲染头像到对应麦位
   ↓
7. 用户开启本地麦克风推流
```

### ❌ 异常流程

**A. 麦克风权限拒绝**: 弹窗引导"去设置"  
**B. 并发抢麦**: Redis 分布式锁，只有第一个请求成功  
**C. 房主抱用户上麦**: `ForceTakeMic` 请求 + 通知

---

## 2.5 核心业务流：贵族购买（Phase 1）

### ✅ 正向流程

```
1. 用户打开"贵族中心" → 选择等级 → 选择支付方式
   ↓
2. 跳转第三方支付 → 支付成功回调 → Server 验证订单 → 开通贵族
   ↓
3. 推送通知 + 全服广播 → 立即生效贵族特权
```

### ❌ 异常流程
- **支付成功但回调丢失**: Server 定时轮询，24小时内补偿
- **重复购买同等级**: 自动续期叠加时长
- **贵族过期**: 到期前 3 天提醒，3 天宽限期后失效

---

## 2.6 跨服务通信：Admin Server → App Server

**核心问题**: 管理员的某些操作需要实时影响 App Server 在线用户。

**Phase 0 方案：基于共享 Redis 的事件队列**

```
Admin Server ── PUBLISH admin:events ──▶ Redis ◀── SUBSCRIBE admin:events ── App Server
                                                                                │
                                                                          踢出用户/关闭WS
```

- **技术**: Redis Pub/Sub
- **频道**: `admin:events`
- **消息格式**: `{type, payload, admin_id, timestamp}`
- **支持事件**: `ban_user`, `unban_user`, `close_room`, `broadcast_notice`
- **Phase 1+ 可选升级**: 引入 NATS/Kafka

---

## 2.7 核心业务流：钱包与礼物闭环 MVP（Phase 1 E-07 细化）

> 本节是对 §2.3 的 MVP 级细化。关联文档: [phase1_gift_economy.md](./phase1_gift_economy.md)。
> **关键差异**：Phase 1 MVP 的充值通道为"运营手动调整"（Admin 后台），真实支付延后到 E-08。

### 2.7.1 钱包余额初始化与查询

```
1. 用户首次注册 → App Server 在 users 表设 diamond_balance = 0
   ↓
2. 用户登录/进入"我的"Tab
   ↓
3. Android 调用 GET /api/v1/wallet/balance → 返回 { diamond, last_updated_at }
   ↓
4. "我的"Tab 的钻石区域渲染余额
   ↓
5. WS 连接建立后，Server 在收到送礼/运营充值后主动推送 BalanceUpdated 事件 → 客户端刷新
```

### 2.7.2 运营手动充值（Admin Server）

```
1. 客服/运营在 Web Admin 用户详情页点击"调整余额"
   ↓
2. 填写 金额（正数=加/负数=扣）+ 原因（必填）
   ↓
3. Web → Admin Server: POST /api/v1/admin/users/:id/wallet/adjust
   ↓
4. Admin Server 事务：
   ├─ UPDATE users SET diamond_balance = diamond_balance + :amount WHERE id = :uid
   ├─ INSERT wallet_transactions (user_id, type='admin_adjust', amount, balance_after, operator_id, reason)
   └─ INSERT admin_logs (action='wallet_adjust', target_id, detail)
   ↓
5. Admin Server PUBLISH admin:events { type: "balance_updated", user_id, new_balance }
   ↓
6. App Server SUBSCRIBE → 找到该 user 的 WS 会话 → 推送 BalanceUpdated 事件
   ↓
7. Android 接收 → 更新本地余额显示
```

**异常流**:
- 扣减导致余额为负 → 返回 `INSUFFICIENT_BALANCE`，事务回滚
- 用户不在线 → 广播仍执行，下次登录调用 `/wallet/balance` 拉取最新值
- Redis Pub/Sub 丢失 → 无状态影响（DB 已落库），客户端下次刷新时矫正

### 2.7.3 送礼核心流程（强事务 + 幂等）

```
1. 用户在房间点击底部 🎁 → 礼物面板 BottomSheet 弹出
   ↓
2. Android 调用 GET /api/v1/gifts/list → 渲染礼物网格 + 顶部余额条
   ↓
3. 用户选礼物 → 选接收者（麦位列表，默认主麦） → 选数量（1/10/66/520/786/1314）
   ↓
4. 前端预校验：余额 >= gift.price * count ? 否则跳到 2.7.4 引导流
   ↓
5. 点击"送出" → 按钮置 loading → 生成 msg_id (UUID v4)
   ↓
6. WS → Server: SendGift { gift_id, receiver_id, count, room_id, msg_id }
   ↓
7. Server 开启 SQLx Transaction：
   ├─ ① 查 (sender_id, msg_id) 已存在 → 直接返回缓存结果（幂等）
   ├─ ② SELECT FOR UPDATE users WHERE id=sender_id → 校验余额
   ├─ ③ 余额不足 → ROLLBACK，返回 INSUFFICIENT_BALANCE
   ├─ ④ UPDATE 发送者 diamond_balance -= total
   ├─ ⑤ UPDATE 接收者 charm_value += total
   ├─ ⑥ INSERT wallet_transactions (sender: 'gift_send', receiver: 'gift_receive')
   ├─ ⑦ INSERT gift_records (sender, receiver, gift_id, count, room_id, msg_id, created_at)
   ├─ ⑧ Redis ZINCRBY ranking:charm:day:{date} receiver_id total
   ├─ ⑨ Redis ZINCRBY ranking:wealth:day:{date} sender_id total
   └─ COMMIT
   ↓
8. Server 对当前房间所有 WS 客户端广播 GiftReceived { sender, receiver, gift, count, effect_level }
   ↓
9. Server 单独向发送者推送 BalanceUpdated { new_balance }
   ↓
10. Android 所有客户端：
    ├─ 聊天区追加礼物弹幕（昵称金色 + 礼物图标 + "送给 xxx x N"）
    ├─ 根据 effect_level 播放特效：L1(气泡) / L2(麦位光圈) / L3(全屏Lottie)
    └─ 发送者额外：更新面板顶部余额
```

### 2.7.4 关键异常流程

| # | 场景 | Server 处理 | Android UI |
|---|------|-----------|-----------|
| A | 余额不足 | 事务 ROLLBACK，返回 `INSUFFICIENT_BALANCE` | 弹 `InsufficientBalanceDialog`："钻石不足，去充值" → 跳钱包页（充值按钮 Phase 1 显示"即将上线"占位） |
| B | 接收者不在房间/已下麦 | 返回 `RECEIVER_UNAVAILABLE` | Toast"对方已离开" + 礼物面板不关闭 |
| C | 重复 msg_id（客户端重试） | 查缓存，幂等返回首次结果 | 正常收到广播，不二次渲染（基于 msg_id 去重） |
| D | WS 断开重连 | 重连后客户端调 `/api/v1/gifts/recent?room_id=` 补最近 20 条 | 按时序补渲染，但不回放动画（仅文字气泡） |
| E | Server 事务异常（DB/Redis） | ROLLBACK，返回 `INTERNAL_ERROR` | 弹"赠送失败，请重试"，按钮恢复可点 |
| F | 连击送礼（3s 内同礼物） | 聚合为一次广播（count 累加） | 动画仅播一次，数量徽章显示 `x N` |
| G | 礼物已下架（Admin 关闭） | 查 gift.is_active=false → 返回 `GIFT_UNAVAILABLE` | 面板自动刷新，Toast"该礼物已下架" |
| H | 礼物面板打开时余额被 Admin 扣减 | WS 推 BalanceUpdated | 面板顶部余额条实时刷新 |

### 2.7.5 榜单查询与展示

```
1. 用户打开"排行榜"页（房间内 or 大厅入口）
   ↓
2. Tab 切换：魅力榜 / 财富榜 × 日榜 / 周榜
   ↓
3. Android → App Server: GET /api/v1/ranking?type=charm&period=day&limit=50
   ↓
4. Server 从 Redis ZREVRANGE 读取 Top N + 关联用户信息（昵称/头像/VIP标识）
   ↓
5. 返回列表 + 当前用户排名（ZREVRANK）
   ↓
6. Android 渲染：Top3 金银铜光圈，Top1 王冠；底部固定"我的排名"
```

**时区切换异常**:
- 凌晨 00:00 Asia/Riyadh 后刷新 → 自动读取新 key `ranking:charm:day:{today}`
- 定时任务失败 → 旧榜仍可读，但新榜 key 不存在时返回空榜（不报错）

---

## 2.8 核心业务流：房间治理（Phase 1.5 E-10）

> 关联文档: [phase1_room_governance.md](./phase1_room_governance.md)

### 2.8.1 创建房间（含封面/分类/密码）

```
1. 大厅 FAB "+" → CreateRoomScreen
   ↓
2. 用户填写：房间名（必填）+ 封面（8张预设选1）+ 分类（6项选1）+ 公告（可选）+ 密码（可选，6位数字）
   ↓
3. 提交 → POST /api/v1/rooms { name, cover_url, category, announcement, password }
   ↓
4. Server：
   ├─ 密码若有值，用 bcrypt hash 存 password_hash
   ├─ INSERT rooms (owner_id=me, admin_user_id=NULL, ...)
   └─ 创建房间内存状态（RoomManager）
   ↓
5. 返回 room_id → Android 自动 JoinRoom 进入该房间
```

**异常**:
- 密码非 6 位数字 → 前端校验拒绝
- 该 owner 已有一个活跃房间（003 迁移的 unique 约束）→ 409 返回"你已有活跃房间"
- 封面 URL 非白名单（MVP 仅预设图） → 400

### 2.8.2 密码房进房校验

```
1. 大厅列表房间卡片显示 🔒 图标（密码房）
   ↓
2. 用户点击 → 弹出 PasswordInputDialog
   ↓
3. 输入 6 位密码 → POST /api/v1/rooms/:id/verify-password { password }
   ↓
4. Server bcrypt 比对：
   ├─ 正确 → 返回 short-live token (60s 内允许 join)
   └─ 错误 → 累计错误计数
       - 5 次错误锁定 30 分钟（Redis pwd_lock:{user_id}:{room_id} TTL 1800）
       - 返回 401 + 剩余次数
   ↓
5. 正确后 → WS JoinRoom 携带 password_token → 进入房间
```

### 2.8.3 观众席打开与交互

```
1. 房间顶部在线人数点击 → AudienceBottomSheet 弹出
   ↓
2. Android → GET /api/v1/rooms/:id/members?page=1&limit=20
   ↓
3. Server 从 RoomManager 内存读取成员 + 批量查 users 表补头像/昵称
   ↓
4. 返回列表（麦上用户置顶 + 观众按进房时间倒序 + 角色字段 owner/admin/member）
   ↓
5. 渲染列表，角色显示 👑 / 🛡️ / 无标识
   ↓
6. 用户点击某用户 → UserActionBottomSheet 弹出
   根据（当前用户角色，目标用户角色）动态显示可用操作：
     - 我是房主 + 目标是普通 → [抱上麦, 禁麦, 禁言, 踢出, 任命管理员]
     - 我是管理员 + 目标是普通 → [抱上麦, 禁麦, 禁言, 踢出]
     - 我是普通 + 目标是任意 → [查看资料(占位), 举报]
     - 我是任意 + 目标是房主 → [查看资料(占位), 举报]
```

### 2.8.4 房主任命管理员

```
1. 房主在观众席点击用户 → 选择"任命管理员"
   ↓
2. 确认弹窗："将 XXX 任命为管理员？"
   ↓
3. WS → TransferAdmin { room_id, target_user_id, action: "assign" | "revoke" }
   ↓
4. Server：
   ├─ 校验操作者是房主
   ├─ 若 assign 且已有管理员 → 先隐式 revoke 旧管理员
   ├─ UPDATE rooms SET admin_user_id = target_user_id (或 NULL)
   └─ 广播 AdminChanged { room_id, new_admin_user_id }
   ↓
5. 房间所有人收到广播 → 刷新角色徽章
```

### 2.8.5 踢出用户（核心）

```
1. 房主/管理员点用户 → "踢出"
   ↓
2. KickReasonDialog：预设 4 个原因 [骚扰/刷屏/辱骂/其他] + 自定义（可选）
   ↓
3. WS → KickUser { room_id, target_user_id, reason }
   ↓
4. Server 强事务：
   ├─ ① 校验权限：操作者是 owner 或 admin
   ├─ ② 校验目标：目标不是 owner（管理员不能踢房主）
   ├─ ③ Redis SETEX kicked:{room_id}:{user_id} 600 reason
   ├─ ④ INSERT room_kick_records
   ├─ ⑤ RoomManager 移除目标用户 + 若在麦上自动下麦
   └─ ⑥ 广播：
        ├─ 仅向目标：UserKicked { room_id, reason, cooldown_sec: 600 }
        └─ 向其他人：UserLeft { user_id, kicked_by: operator_id, reason }
   ↓
5. 目标客户端收到 UserKicked → 弹窗"你被移出房间，原因: XXX，10 分钟后可再次进入" → 自动回到大厅
   其他客户端收到 UserLeft → 列表移除该用户
```

**10 分钟内重进校验**:
```
1. 被踢用户再次点该房间 → WS JoinRoom
   ↓
2. Server 查 Redis kicked:{room_id}:{user_id}
   ├─ 存在 → 返回 KICKED_COOLDOWN { remaining_sec }
   └─ 不存在 → 正常进入
```

### 2.8.6 禁麦 / 禁言

```
1. 房主/管理员点用户 → "禁麦 5 min" (或 15 / 30)
   ↓
2. WS → MuteUser { room_id, target_user_id, type: "mic", duration_sec: 300 }
   ↓
3. Server：
   ├─ 校验权限 + 目标非房主
   ├─ Redis SETEX mic_muted:{room_id}:{user_id} 300 operator_id
   ├─ INSERT room_mute_records
   ├─ 若 type="mic" 且目标在麦上 → 强制下麦（广播 MicLeft）
   └─ 广播 UserMuted { type, duration_sec, operator_id }
   ↓
4. 目标客户端：
   ├─ type=mic → 麦位按钮置灰 + Toast "你已被禁麦 5 分钟"
   └─ type=chat → 输入框置灰 + 发送按钮置灰
```

**禁言用户发消息拦截**（双重校验）:
```
1. 前端：输入框置灰，发送按钮 disabled
2. 后端（防刷）：SendMessage 处理前查 Redis chat_muted:* → 存在则拒绝返回 CHAT_MUTED
```

### 2.8.7 抱上麦 / 抱下麦

```
1. 抱上麦：房主/管理员点观众 → "抱上麦 到 X 号位"（选择空麦位）
   ↓
2. WS → ForceTakeMic { room_id, target_user_id, slot_index }
   ↓
3. Server：
   ├─ 校验操作者权限
   ├─ 校验目标非禁麦
   ├─ 校验麦位空闲
   ├─ 更新 RoomManager 麦位状态
   └─ 广播 MicTaken { user_id, slot_index, forced_by: operator_id }
   ↓
4. 目标客户端收到广播 → UI 自动显示"你已被 X 抱上 N 号麦位" + 自动请求麦克风权限
   若被抱用户拒绝麦克风权限 → 客户端自动发 MicLeave（因无法推流）
```

**抱下麦**（同理，信令 `ForceLeaveMic`）

### 2.8.8 被禁用户的完整客户端反馈

| 状态 | Android UI 反馈 |
|------|---------------|
| 被踢 | 全屏 Dialog + 自动退出房间 + 10min 内大厅进房按钮灰 |
| 禁麦 | 麦位"+"按钮灰 + 已在麦时自动下麦 + Toast 倒计时 |
| 禁言 | 输入框灰 + 占位文本"你已被禁言 X 分钟" + 倒计时 |
| 解禁 | 收到 `UserMuted { duration: 0 }` 广播后恢复 UI |

---

## 2.9 埋点事件字典（Phase 1 E-07.5）

> 关联文档: [phase1_observability.md](./phase1_observability.md)

### 2.9.1 事件上报协议

**公共字段**（所有事件必带）:
| 字段 | 类型 | 来源 | 说明 |
|------|------|------|------|
| `event_name` | string | 客户端 | 事件名（见下表） |
| `client_ts` | int64 | 客户端 | 毫秒时间戳 |
| `session_id` | UUID | 客户端 | 本次启动会话 ID |
| `device_id` | UUID | 客户端 | 设备唯一标识（首次启动生成） |
| `user_id` | UUID? | 客户端 | 未登录时为 null，Server 侧可回填 |
| `app_version` | string | 客户端 | 如 "1.0.3" |
| `os_version` | string | 客户端 | 如 "Android 14" |
| `locale` | string | 客户端 | 如 "ar_SA" |
| `network_type` | string | 客户端 | "wifi" / "4g" / "5g" / "offline" |

**自定义字段**：每事件的 `properties` JSONB 自带。

### 2.9.2 核心事件列表

#### 应用生命周期

| event_name | 触发时机 | properties |
|-----------|---------|-----------|
| `app_launch` | 冷启动 | launch_time_ms |
| `app_foreground` | 前台 | — |
| `app_background` | 后台 | foreground_duration_ms |
| `splash_to_main` | Splash 跳转 | has_jwt (bool) |

#### 认证

| event_name | properties |
|-----------|-----------|
| `login_sms_request` | phone_region |
| `login_sms_sent` | — |
| `login_sms_fail` | fail_reason |
| `login_verify_success` | is_new_user |
| `login_verify_fail` | fail_reason, attempt_count |
| `logout` | trigger ("manual" / "token_expired") |

#### 大厅与房间

| event_name | properties |
|-----------|-----------|
| `hall_view` | tab, page |
| `hall_scroll` | max_position |
| `room_card_click` | room_id, position, source ("hall" / "ranking") |
| `create_room_click` | from |
| `create_room_submit` | category, has_password (bool), has_cover (bool) |
| `create_room_success` | room_id |
| `create_room_fail` | fail_reason |
| `password_room_prompt` | room_id |
| `password_room_pass` | room_id, attempts |
| `password_room_fail` | room_id, attempts |
| `room_enter` | room_id, source |
| `room_leave` | room_id, duration_sec, reason ("self" / "kicked" / "closed") |

#### 麦位

| event_name | properties |
|-----------|-----------|
| `mic_take_click` | slot_index |
| `mic_take_success` | slot_index, duration_to_stream_ms |
| `mic_take_fail` | slot_index, fail_reason |
| `mic_leave` | slot_index, duration_on_mic_sec |
| `mic_permission_request` | — |
| `mic_permission_grant` | — |
| `mic_permission_deny` | — |

#### 聊天

| event_name | properties |
|-----------|-----------|
| `chat_send` | msg_len, room_id |
| `chat_send_fail` | fail_reason (含 "muted") |
| `chat_receive` | msg_type ("text" / "system" / "gift") |

#### 礼物（核心）

| event_name | properties |
|-----------|-----------|
| `gift_panel_open` | room_id, source ("bottom_bar" / ...) |
| `gift_panel_close` | duration_ms, bought (bool) |
| `gift_select` | gift_id, tier |
| `gift_recipient_select` | target_user_id, target_mic_slot |
| `gift_count_select` | count |
| `gift_send_click` | gift_id, count, total_price, recipient_id |
| `gift_send_success` | gift_id, count, total_price, effect_level, elapsed_ms |
| `gift_send_fail` | gift_id, count, fail_reason (含 INSUFFICIENT_BALANCE / RECEIVER_UNAVAILABLE 等) |
| `gift_send_combo` | gift_id, combo_count |
| `gift_receive` | gift_id, count, sender_id, effect_level |

#### 钱包（核心）

| event_name | properties |
|-----------|-----------|
| `wallet_view` | source, balance |
| `wallet_transactions_load` | page, count_loaded |
| `balance_update_received` | old_balance, new_balance, delta, source ("gift_send" / "gift_receive" / "admin_adjust") |
| `insufficient_balance_dialog_shown` | required, current, gift_id |
| `insufficient_balance_dialog_action` | action ("recharge" / "cancel") |
| `recharge_click` | source, placeholder (bool=true while E-08 未完成) |

#### 榜单

| event_name | properties |
|-----------|-----------|
| `ranking_view` | source |
| `ranking_tab_switch` | type ("charm" / "wealth"), period ("day" / "week") |
| `ranking_item_click` | rank, target_user_id |

#### 房间治理（E-10 同步）

| event_name | properties |
|-----------|-----------|
| `audience_view` | room_id, audience_count |
| `user_action_menu_open` | target_role, my_role |
| `kick_user_click` | reason |
| `kick_user_success` | target_user_id |
| `mute_user_click` | type ("mic" / "chat"), duration_sec |
| `mute_user_success` | target_user_id |
| `transfer_admin_click` | action ("assign" / "revoke") |
| `force_take_mic_click` | slot_index |
| `user_kicked_received` | room_id, reason, cooldown_sec |
| `user_muted_received` | type, duration_sec |

#### 合规

| event_name | properties |
|-----------|-----------|
| `privacy_dialog_shown` | — |
| `privacy_dialog_action` | action ("agree" / "crash_only") |

#### 异常（Sentry 自动）

由 `io.sentry.Sentry.captureException` 自动捕获，无需手动埋点。

### 2.9.3 事件命名规范

- `snake_case`
- 动词/名词清晰：`gift_send_success` 而非 `send_success`
- 状态切换一律成对：`xxx_click` / `xxx_success` / `xxx_fail`
- 接收类事件用 `_received`：`user_kicked_received`

