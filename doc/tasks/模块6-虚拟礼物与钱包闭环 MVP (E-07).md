# 模块 6: 虚拟礼物与钱包闭环 MVP (E-07)

> 返回 [任务总索引](./index.md)

## Phase 1: 核心营收闭环

> **说明**：Phase 1 聚焦营收打通。E-07 采用"封闭内循环"策略——充值通道为 Admin 手动调整（快速打通闭环），真实支付延后到 E-08。详见 [phase1_gift_economy.md](../product/phase1_gift_economy.md)。
> **产品流程规范**: [business_flows.md §2.7](../product/business_flows.md)


## 模块 6: 虚拟礼物与钱包闭环 MVP (E-07)

> **依赖关系图**:
> ```
> T-00017 (钱包schema) ──┬─► T-00018 (余额API) ──► T-30027 (Android钱包)
>                        └─► T-10013 (Admin充值) ──► T-20012 (Web充值UI)
> T-00019 (礼物表+API) ──► T-10014 (Admin礼物CRUD) ──► T-30028 (礼物面板)
> T-00017 + T-00019 ──► T-00020 (SendGift事务) ──► T-30029~T-30031 (送礼UI+动画)
> T-00020 ──► T-00021 (榜单) ──► T-30033 (榜单页)
> T-30028 + T-30027 ──► T-30032 (余额不足引导)
> ```

#### App Server

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-00017** | App Server | Wallet | 钱包 Schema 与迁移 [TDS](../tds/server/T-00017.md) | T-0000B | users 表增 `diamond_balance BIGINT DEFAULT 0`（CHECK >= 0）+ 新建 `wallet_transactions` 流水表（user_id/type/amount/balance_after/ref_id/reason/created_at） | 1. 迁移脚本可幂等执行<br>2. CHECK 约束防止余额为负<br>3. 流水表带 (user_id, created_at) 索引<br>4. 新注册用户默认 0 | 3h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | ✅ N/A | ✅ Released |
| **T-00018** | App Server | Wallet | 余额查询 API + WS 推送 [TDS](../tds/server/T-00018.md) | T-00017 | GET `/api/v1/wallet/balance`、GET `/api/v1/wallet/transactions`（分页）；新增 WS 信令 `BalanceUpdated { msg_id, diamond_balance, delta, reason, ref_id, timestamp }`，在余额变化时推送给当前用户所有会话；支持 Redis PubSub 跨进程推送 | 1. 查询返回最新余额<br>2. 流水按时间倒序分页<br>3. 余额变化后 <500ms 内 WS 推送<br>4. 同一用户多连接全部收到推送，每条消息独立 msg_id<br>5. 离线用户重连后主动拉刷新 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | ✅ N/A | ✅ Released |
| **T-00019** | App Server | Gift | 礼物配置表 + 列表 API [TDS](../tds/server/T-00019.md) | T-0000B | 新建 `gifts` 表（id/name_en/name_ar/icon_url/price/tier/effect_level/animation_url/is_active/sort_order）；GET `/api/v1/gifts/list` 返回上架礼物列表（按 tier+sort_order 排序） | 1. 迁移脚本创建表并插入 8 款 MVP 礼物种子数据<br>2. 列表只返回 is_active=true<br>3. 支持 Accept-Language 切换 name_en/name_ar<br>4. 响应时间 <50ms（加缓存） | 5h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | ✅ N/A | ✅ Released |
| **T-00020** | App Server | Gift | SendGift 事务 + 广播 [TDS](../tds/server/T-00020.md) | T-00017, T-00019, T-00016 | 新增 WS 信令 `SendGift { gift_id, receiver_id, count, msg_id }`；SQLx 事务：查余额→扣发送者→加接收者魅力值→写流水→写 gift_records→Redis ZINCRBY 日/周榜；广播 `GiftReceived { sender, receiver, gift, count, effect_level, total }` 给房间所有人；发送者单独推送 BalanceUpdated | 1. 并发 20 QPS 无超扣/脏数据<br>2. 重复 msg_id 幂等返回首次结果<br>3. 余额不足返回 INSUFFICIENT_BALANCE 并回滚<br>4. 接收者离线返回 RECEIVER_UNAVAILABLE<br>5. 全链路落 4 张表 + 2 个 Redis key | 10h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | ✅ N/A | ✅ Released |
| **T-00021** | App Server | Ranking | 魅力/财富榜单 API [TDS](../tds/server/T-00021.md) | T-00020 | GET `/api/v1/ranking?type=charm\|wealth&period=day\|week&limit=50`；读取 Redis ZSet 返回 Top N + 当前用户排名；定时任务：每日 00:00 Riyadh 切换日榜 key，每周六切换周榜 key | 1. Top 50 返回 <100ms<br>2. 返回值包含 Top3 金银铜标识字段<br>3. 当前用户未入榜时返回 rank=null<br>4. 时区切换任务可补偿执行<br>5. 旧榜归档到 ranking_archive | 6h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | ✅ N/A | ✅ Released |
| **T-00044** | App Server | Gift | 礼物 REST 端点 POST /api/v1/gifts/send [TDS](../tds/server/T-00044.md) | T-00020 | 新增 HTTP 礼物发送接口，复用 WS SendGift 事务逻辑 | 1. HTTP 送礼成功扣款 + 写 DB<br>2. 幂等性（重复请求返回同一 record_id）<br>3. 错误码与 WS 一致（40290/40402/40403）<br>4. 弱网场景降级可用 | 4h | Dod | ✅ Done | [✅ Passed](../review/模块3-6-8-架构阻塞修复.md) | ✅ N/A | ✅ Released |

#### Admin Server

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-10013** | Admin Server | Wallet | 手动调整余额 API [TDS](../tds/adminServer/T-10013.md) | T-00017, T-10012 | POST `/api/v1/admin/users/:id/wallet/adjust { amount, reason }`；事务：改 users 余额 + 写 wallet_transactions (type='admin_adjust', operator_id) + 写 admin_logs；Redis PUBLISH admin:events {type:'balance_updated', user_id, new_balance} 通知 App Server 推 WS | 1. amount 正数=加，负数=扣<br>2. reason 必填且写入日志<br>3. 导致余额<0 返回 400<br>4. 事务原子性（任一步失败整体回滚）<br>5. Redis 事件已发布 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | ✅ N/A | ✅ Released |
| **T-10014** | Admin Server | Gift | 礼物 CRUD 管理 API [TDS](../tds/adminServer/T-10014.md) | T-00019, T-10012 | `/api/v1/admin/gifts` CRUD（GET 列表含未上架 / POST 新增 / PUT 更新 / DELETE 软删）；图片/Lottie 上传走对象存储或本地静态目录；所有操作写 admin_logs | 1. 上架/下架通过 is_active 字段切换<br>2. 删除为软删（is_deleted=true）<br>3. 上传文件类型白名单校验<br>4. 价格必须 >=1<br>5. 所有写操作落 admin_logs | 6h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | ✅ N/A | ✅ Released |

#### Web Admin

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 | UI设计文档 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|------------|
| **T-20012** | Web | User | 余额调整弹窗 + 礼物管理页 [TDS](../tds/web/T-20012.md) | T-10013, T-10014, T-20007 | 用户详情页新增"调整余额"按钮→`AdjustBalanceModal`（金额/原因/确认）；新增"礼物管理"菜单页：列表 + 新增弹窗 + 编辑 + 上下架开关 + 软删 | 1. 调整成功后用户余额实时刷新<br>2. 原因必填校验<br>3. 负数显示红色二次确认<br>4. 礼物列表可筛选 tier/状态<br>5. 上传图片预览 | 8h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending | 完成 DoD 文档同步：web/user-management.md + web/gift-management.md 新增；product/index.md E-07 进度 8/15 |

#### Android

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 | UI设计文档 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|------------|
| **T-30027** | Android | Wallet | 钱包页（余额 + 流水） [TDS](../tds/android/T-30027.md) | T-00018, T-30024 | 新建 `WalletScreen`：顶部大卡片显示钻石余额 + "充值"按钮（占位 Toast"即将上线"）；下方 LazyColumn 流水列表（收入绿色/支出红色 + 图标 + 时间）；WS `BalanceUpdated` 自动刷新；个人中心"钻石余额"项点击跳转进入 | 1. 余额大号金色显示<br>2. 下拉刷新拉最新余额<br>3. 流水分页加载<br>4. 收到 BalanceUpdated 事件即时更新<br>5. 空状态占位"暂无流水" | 6h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending | [T-30027.md](../design/android/T-30027.md) |
| **T-30028** | Android | Gift | 礼物面板 Bottom Sheet [TDS](../tds/android/T-30028.md) | T-00019, T-30026 | `GiftPanelBottomSheet` Composable：顶部余额条 + 礼物网格（4列）+ 分类 Tab（热门/全部）+ 数量选择器（1/10/66/520/786/1314）+ 发送按钮；房间页 🎁 按钮点击弹出（替换 T-30026 的 Toast 占位） | 1. 面板占屏幕 55% 高度<br>2. 余额实时显示（WS 更新）<br>3. 选中礼物有金色边框<br>4. 数量按钮吉祥数档位<br>5. 余额不足时"送出"置灰 | 7h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending | [T-30028.md](../design/android/T-30028.md) |
| **T-30029** | Android | Gift | 接收者选择器 [TDS](../tds/android/T-30029.md) | T-30028 | 礼物面板顶部横向滚动的麦位头像条：默认选中 1 号主麦；点击切换；选中项金色光圈；空麦位不显示 | 1. 显示所有在麦用户<br>2. 默认主麦<br>3. 选中高亮<br>4. 无人在麦时发送按钮禁用 + 提示<br>5. 麦位变化实时刷新 | 4h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending | [T-30029.md](../design/android/T-30029.md) |
| **T-30030** | Android | Gift | SendGift 客户端 + 幂等 [TDS](../tds/android/T-30030.md) | T-30028, T-30029, T-00020 | 点"送出"生成 UUID msg_id → WS 发送 SendGift → 按钮 loading → 收到 GiftReceived 或错误后还原；同礼物 3s 内连击累加 count，最终只发一次；错误码对应处理（余额不足弹窗/接收者不可用 Toast） | 1. msg_id 每次唯一<br>2. 3s 连击聚合<br>3. 超时 5s 自动失败<br>4. 余额不足跳 T-30032 弹窗<br>5. 成功后面板不自动关 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending | [T-30030.md](../design/android/T-30030.md) |
| **T-30031** | Android | Gift | 送礼特效播放器 + 弹幕样式 [TDS](../tds/android/T-30031.md) | T-30030 | 分层特效：L1 聊天区气泡（礼物图标+文字）；L2 接收者麦位金色光圈闪烁 2s；L3 全屏 Lottie 动画覆盖层（使用 airbnb/lottie-compose），动画期间可继续交互但不可点击覆盖层 | 1. L1 弹幕礼物消息金色昵称+图标+"送给 xxx x N"<br>2. L2 麦位动画 2s 后自动结束<br>3. L3 全屏动画 5-8s 可跳过<br>4. 连击礼物动画仅播一次，数量徽章更新<br>5. 接收补偿消息不回放动画 | 8h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending | [T-30031.md](../design/android/T-30031.md) |
| **T-30032** | Android | Wallet | 余额不足引导弹窗 [TDS](../tds/android/T-30032.md) | T-30028 | `InsufficientBalanceDialog` AlertDialog：标题"钻石不足" + 当前余额 + 所需余额 + "去充值"按钮 → 跳 WalletScreen；"取消"按钮关闭 | 1. 显示当前/所需钻石<br>2. "去充值"跳钱包页<br>3. 点击外部不关闭<br>4. 关闭后礼物面板保留选中状态 | 2h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending | [T-30032.md](../design/android/T-30032.md) |
| **T-30033** | Android | Ranking | 魅力/财富榜页 [TDS](../tds/android/T-30033.md) | T-00021, T-30018 | 新建 `RankingScreen`：顶部双 Tab（魅力/财富）+ 子 Tab（日榜/周榜）；列表项：排名+头像(Top3带金银铜光圈+Top1王冠)+昵称+钻石数；底部固定"我的排名"；入口：大厅顶部 🏆 图标 + 房间页"榜单"菜单项 | 1. 四组 Tab 数据独立加载<br>2. Top3 头像光圈颜色不同<br>3. Top1 王冠图标<br>4. 未入榜显示"未上榜，继续加油"<br>5. 下拉刷新 | 7h | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending | [T-30033.md](../design/android/T-30033.md) |

---
