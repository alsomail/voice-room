# 模块 3: 房间内核心功能 (In-Room Core)

> 返回 [任务总索引](./index.md)

## Phase 0: MVP 基础设施 (预计 6-8 周)


## 模块 3: 房间内核心功能 (In-Room Core)

#### App Server 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-00011** | App Server | WebSocket | WebSocket 连接管理 [TDS](../tds/server/T-00011.md) | T-00004 | 实现 WS 握手、心跳、断线检测 | 1. JWT 认证后建立连接<br>2. 30 秒无心跳断开<br>3. 支持断线重连（携带 last_msg_id）<br>4. 并发 1000 连接压测通过 | 6h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-00012** | App Server | Room | 进入房间逻辑 [TDS](../tds/server/T-00012.md) | T-00011 | 处理 `JoinRoom` 消息 | 1. 校验房间是否存在<br>2. 加入房间内存状态<br>3. 广播 `UserJoined` 事件<br>4. 返回房间状态快照 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-00013** | App Server | Room | 离开房间逻辑 [TDS](../tds/server/T-00013.md) | T-00012 | 处理 `LeaveRoom` 消息或连接断开 | 1. 从房间移除用户<br>2. 广播 `UserLeft` 事件<br>3. 若在麦上自动下麦 | 3h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-00014** | App Server | Mic | 麦位上麦接口 [TDS](../tds/server/T-00014.md) | T-00012 | 处理 `TakeMic` 消息，Redis 锁防并发 | 1. 检查麦位空闲<br>2. 检查是否被禁麦<br>3. 广播 `MicTaken` 事件<br>4. 并发抢麦只有一个成功 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-00015** | App Server | Mic | 麦位下麦接口 [TDS](../tds/server/T-00015.md) | T-00014 | 处理 `LeaveMic` 消息 | 1. 只能下自己的麦<br>2. 广播 `MicLeft` 事件 | 2h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-00016** | App Server | Chat | 文本消息广播 [TDS](../tds/server/T-00016.md) | T-00012 | 处理 `SendMessage` 消息 | 1. 消息长度限制 500 字符<br>2. 敏感词过滤<br>3. 基于 msg_id 去重<br>4. 禁言用户拒绝 | 4h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-00041** | App Server | WebSocket | WS 心跳 30s 超时主动断开（ping/pong） [TDS](../tds/server/T-00041.md) | T-00011 | 实现服务端心跳超时检测，30s 无心跳主动关闭连接 | 1. 客户端 15s ping 保活不断开<br>2. 35s 静默后服务端关闭（close code=1000）<br>3. 并发 1000 连接稳定性不回归 | 3h | Dod | ✅ Done | [✅ Passed](../review/模块3-6-8-架构阻塞修复.md) | - | ⏳ Pending |
| **T-00043** | App Server | Chat | Chat 消息持久化 + REST 历史查询接口 [TDS](../tds/server/T-00043.md) | T-00016 | 新建 `chat_messages` 表 + `GET /rooms/:id/messages` 分页接口 | 1. SendMessage 同步写 DB<br>2. 历史接口按时间倒序分页<br>3. 断线重连可拉取全量历史 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块3-6-8-架构阻塞修复.md) | - | ⏳ Pending |
| **T-00045** | App Server | Chat | REST `POST /chat-messages` 修复广播闭环（BUG-CHAT-WS-BROADCAST） [TDS](../tds/server/T-00045.md) | T-00043 | 新增 REST 端点：JWT 鉴权 → INSERT → `broadcast_to_room` 广播 RoomMessage（与 WS SendMessage 路径对齐） | 1. 房间内 WS 收到 RoomMessage<br>2. 其他房间不收<br>3. envelope `msg_id`+`payload.msg_id`+`timestamp` 齐全<br>4. 死连接容忍<br>5. content 长度/UUID 边界 | 3h | Dod | ✅ Done | [✅ Passed](../review/模块3-BUG-CHAT-WS修复链.md) | ✅ Passed ([Round 22](../../tests/report-20260505-124251/AND/TC-CHAT-00002/TC-CHAT-00002_Report.md)) · known-issue: BUG-CHAT-LONGPRESS | ⏳ Pending |
| **T-00046** | App Server | Chat | WS 广播可观测性增强（BUG-CHAT-WS-BROADCAST-SILENT） [TDS](../tds/server/T-00046.md) | T-00045 | `broadcast_to_room_inner` 发送失败时打 WARN 日志 + 清理 stale connection，广播前后打 INFO 统计日志 | 1. receiver drop 后广播打 WARN + registry 移除该连接<br>2. 正常广播打 DEBUG + INFO 计数<br>3. 单连接失败不阻断其他连接 | 2h | Dod | ✅ Done | [✅ Passed](../review/模块3-BUG-CHAT-WS修复链.md) | ✅ Passed ([Round 22](../../tests/report-20260505-124251/AND/TC-CHAT-00002/TC-CHAT-00002_Report.md)) · known-issue: BUG-CHAT-LONGPRESS | ⏳ Pending |
| **T-00047** ⭐ [TDS](../tds/server/T-00047.md) | App Server | Chat | **【试跑 Task】Chat 路径统一与协议路径绑定表回填**（协议治理铁律首发） | T-00045, T-00046, T-30054 | 协议治理铁律落地后**首个**走完整 Plan→TDD→Review→Dod 闭环的 Task。明确将 WS `SendMessage` 标记为客户端**主路径** ⭐，REST `POST /api/v1/chat-messages` 标记为运营/兜底备路径；TDS 必须填「协议路径绑定表」+ PROTO-1/PROTO-2 验收用例；DoD 反向写入 `doc/arch/server/index.md` & `doc/arch/server/room.md` 的「🔌 协议入口索引」 | 1. TDS 第二节「协议路径绑定表」完整列出双路径并加 ⭐<br>2. PROTO-1：grep-able 客户端断言（与 T-30054 联调）<br>3. PROTO-2：双路径广播 envelope 除 `envelope.msg_id` 外逐字段相等<br>4. DoD 反向写入 server arch 「🔌 协议入口索引」 | 4h | Dod | ✅ Done | [✅ Passed](../tds/server/T-00047.md) | - | ✅ Passed |
| **T-00048** | App Server | Chat | REST/WS Chat 双路径 envelope 等价回归集成测试 | T-00047 | 在 `app/server/tests/integration/` 下新增 `chat_dual_path_equivalence.rs`，启动两个 WS 客户端 + 一个 REST 客户端，验证两条写路径产生的 RoomMessage envelope 形态一致（除 `envelope.msg_id` UUIDv4 部分外），含 `payload.msg_id`/`content`/`timestamp`/`from_user` 等字段逐项断言 | 1. 同一 content 通过 WS 与 REST 各发一次，房间内观察到的 envelope 字段全等（除 envelope.msg_id）<br>2. 双路径 `payload.msg_id` 均为合法 UUID 且互不相同<br>3. 死连接清理逻辑在两条路径下行为一致 | 5h | Plan | Todo | - | - | ⏳ Pending |

> **App Server 补充任务: 跨服务事件消费**

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-00011B** | App Server | Event | Redis 事件订阅 [TDS](../tds/server/T-00011B.md) | T-00011 | 订阅 `admin:events` 频道，执行管理事件 | 1. 收到 `ban_user` → 找到该用户 WS 连接 → 发送封禁通知 → 断开连接<br>2. 收到 `close_room` → 广播房间关闭 → 断开所有成员连接<br>3. 收到 `broadcast_notice` → 向所有在线用户推送公告<br>4. 事件处理失败不影响主服务 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-00011C** | App Server | Stats | 在線統計上報 [TDS](../tds/server/T-00011C.md) | T-00011, T-00012 | 實時維護 Redis 在線統計數據 | 1. 用戶上線/下線時更新 `stats:online_users` (HyperLogLog)<br>2. 用戶進入/離開房間時更新 `stats:active_rooms` (Set)<br>3. 每分鐘快照一次統計數據到 `stats:snapshot:{date}` | 3h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |

#### Admin Server 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-10007** | Admin Server | User | 用户列表接口 [TDS](../tds/adminServer/T-10007.md) | T-10003 | GET `/api/v1/admin/users` | 1. 支持手机号/ID/昵称搜索<br>2. 分页返回<br>3. 包含资产信息（coin_balance） | 3h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-10008** | Admin Server | User | 用户详情接口 [TDS](../tds/adminServer/T-10008.md) | T-10007 | GET `/api/v1/admin/users/:id` | 1. 完整用户信息<br>2. 充值/消费记录<br>3. 登录设备信息 | 4h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-10009** | Admin Server | User | 封禁/解封接口 [TDS](../tds/adminServer/T-10009.md) | T-10008 | POST `/api/v1/admin/users/:id/ban` | 1. 支持永久/临时封禁<br>2. 记录封禁原因<br>3. 推送封禁事件到 Redis (→ App Server)<br>4. 记录操作日志 | 4h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-10010** | Admin Server | Stats | 数据统计接口 [TDS](../tds/adminServer/T-10010.md) | T-10003 | GET `/api/v1/admin/stats/overview` | 1. 返回 DAU、新增用户、活跃房间数、在线人数<br>2. 支持按日期范围查询<br>3. 在线人数从 Redis 获取（App Server 维护）<br>4. 响应时间 < 500ms | 5h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-10011** | Admin Server | Event | 跨服务事件发布 [TDS](../tds/adminServer/T-10011.md) | T-10003, T-0000A | Redis Pub/Sub 发布管理事件 | 1. 封禁用户时发布 `ban_user` 事件<br>2. 关闭房间时发布 `close_room` 事件<br>3. 消息格式: `{type, payload, admin_id, ts}`<br>4. 集成到 T-10009 和 T-10006 中 | 4h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-10012** | Admin Server | Log | 操作审计日志 [TDS](../tds/adminServer/T-10012.md) | T-10001 | 设计 `admin_logs` 表 + 写入中间件 | 1. 记录所有敏感操作（封禁/解封/关闭房间/充值）<br>2. 字段: admin_id, action, target_id, ip, detail, created_at<br>3. Axum 中间件自动拦截记录<br>4. GET `/api/v1/admin/logs` 查询接口 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |

#### Web 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-20006** | Web | User | 用户管理页面 [TDS](../tds/web/T-20006.md) | T-10007 | Ant Design Table 展示用户列表 | 1. 搜索框（手机号/ID/昵称）<br>2. 分页加载<br>3. 状态筛选（全部/正常/封禁） | 5h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-20007** | Web | User | 用户详情抽屉 [TDS](../tds/web/T-20007.md) | T-10008, T-20006 | Drawer 展示用户详细信息 | 1. 基础信息卡片<br>2. 资产信息<br>3. 行为数据<br>4. [封禁] [解封] 按钮 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-20008** | Web | User | 封禁对话框 [TDS](../tds/web/T-20008.md) | T-10009, T-20007 | Modal 实现封禁操作 | 1. 选择封禁时长<br>2. 选择封禁原因<br>3. 填写备注<br>4. 二次确认 | 3h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-20009** | Web | Log | 操作日志页面 [TDS](../tds/web/T-20009.md) | T-10012 | Ant Design Table 展示审计日志 | 1. 按时间倒序<br>2. 支持按操作人/类型/时间筛选<br>3. 展示操作详情 | 4h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |

#### Android 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-30008** | Android | WebSocket | WebSocket 连接封装 [TDS](../tds/android/T-30008.md) | T-00011 | OkHttp WebSocket + Flow | 1. 自动重连（指数退避）<br>2. Kotlin Flow 发射连接状态<br>3. 心跳包发送 | 6h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-30009** | Android | Room | 房间页 UI (Compose) [TDS](../tds/android/T-30009.md) | T-00009 | 实现房间完整布局 | 1. 顶部房间信息<br>2. 麦位 Grid<br>3. 聊天列表<br>4. 底部输入栏 | 8h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-30010** | Android | Room | 房间 ViewModel [TDS](../tds/android/T-30010.md) | T-00012, T-30008, T-30009 | 管理房间状态，处理 WS 消息 | 1. 进入房间发送 JoinRoom<br>2. 监听服务端事件更新 State<br>3. 离开清理资源 | 6h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-30011** | Android | Mic | 麦位组件 (Compose) [TDS](../tds/android/T-30011.md) | T-30009 | 可复用麦位卡片 | 1. 三种状态渲染<br>2. Lottie 音浪动画<br>3. RTL 布局 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-30012** | Android | Mic | 麦克风权限请求 [TDS](../tds/android/T-30012.md) | T-30011 | Accompanist Permissions | 1. 运行时权限请求<br>2. 权限拒绝对话框<br>3. 跳转系统设置 | 3h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-30013** | Android | Mic | 上麦/下麦逻辑 [TDS](../tds/android/T-30013.md) | T-00014, T-30012 | 发送上麦请求 + RTC 推流 | 1. 权限通过后上麦<br>2. 集成 RTC SDK<br>3. 成功后开启推流 | 7h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-30014** | Android | Chat | 聊天列表 (Compose) [TDS](../tds/android/T-30014.md) | T-30009 | LazyColumn 聊天消息 | 1. 自动滚动到最新<br>2. 不同类型消息样式<br>3. 系统消息居中 | 5h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-30015** | Android | Chat | 输入框组件 [TDS](../tds/android/T-30015.md) | T-30014 | TextField + 发送按钮 | 1. 软键盘弹出布局调整<br>2. 回车发送<br>3. 空消息禁用发送 | 3h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-30016** | Android | Chat | 发送消息逻辑 [TDS](../tds/android/T-30016.md) | T-00016, T-30015 | 发送 SendMessage | 1. 发送中禁用<br>2. 成功清空输入<br>3. 失败重试 | 3h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-30017** | Android | Chat | 接收消息逻辑 [TDS](../tds/android/T-30017.md) | T-00016, T-30014 | 监听服务端消息 | 1. 实时追加到列表<br>2. 去重（msg_id）<br>3. 自动滚动 | 3h | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| **T-30051** | Android | Chat | Android WS 接收链路可观测性增强（BUG-CHAT-WS-ANDROID-SILENT）[TDS](../tds/android/T-30051.md) | T-30017 | 在 WS 接收链路 5 个关键节点注入 Log 日志（onMessage/parse/dispatch/rvm/ui），不修改业务逻辑 | 1. dex strings 含至少 3 条日志字符串<br>2. FakeWS 推消息后 5 节点日志均触发<br>3. 日志不含消息正文 PII | 2h | Dod | ✅ Done | [✅ Passed](../review/模块3-BUG-CHAT-WS修复链.md) | ✅ Passed ([Round 22](../../tests/report-20260505-124251/AND/TC-CHAT-00002/TC-CHAT-00002_Report.md)) · known-issue: BUG-CHAT-LONGPRESS | ⏳ Pending |
| **T-30052** | Android | Chat | Android ChatMessageList 气泡样式修复（BUG-CHAT-WS-BUBBLE）[TDS](../tds/android/T-30052.md) | T-30051 | 为 UserMessageItem 包裹 Surface 容器（圆角+气泡背景色+padding），新增 MenaColors.ChatBubble 令牌，保留 5 节点日志 | 1. CB-01~03 测试通过<br>2. APK dex strings 含 "chat_bubble"<br>3. Round 19 日志字符串仍在 | 1h | Dod | ✅ Done | [✅ Passed](../review/模块3-BUG-CHAT-WS修复链.md) | ✅ Passed ([Round 22](../../tests/report-20260505-124251/AND/TC-CHAT-00002/TC-CHAT-00002_Report.md)) · known-issue: BUG-CHAT-LONGPRESS | ⏳ Pending |
| **T-30053** | Android | Chat | Android ChatMessageList 长按复制菜单（BUG-CHAT-LONGPRESS） [TDS](../tds/android/T-30053.md) | T-30052 | 为 `UserMessageItem` 接入 `combinedClickable(onLongClick)` + `DropdownMenu`/`ModalBottomSheet`，菜单含「复制」项，点击后写入剪贴板并 Toast；不破坏 Round 21 气泡 Surface/MenaColors.ChatBubble 与 Round 19 注入的 5 节点可观测性日志 | 1. 单测：长按触发 onLongClick 回调；点击「复制」后 ClipboardManager 内容 == 消息原文<br>2. 单测：Round 19 5 节点日志 tag/key 仍存在<br>3. APK dex strings 含菜单标签字符串（如 "chat_msg_copy"）<br>4. E2E：TC-CHAT-00002 Step 6 `aiLongPress` + `aiWaitFor("弹出操作菜单")` PASS<br>5. RTL 布局下菜单锚点正确，testTag/Key 暴露用于自动化（如 `Key('chat_msg_long_press_menu')`/`Key('chat_msg_copy')`） | 3h | - | ✅ Done | - | - | ⏳ Pending |
| **T-30054** | Android | Chat | RoomViewModel.sendMessage 主路径=WS 显式契约 + 集成测试（协议路径绑定锁定） | T-00047 | 在 `RoomViewModel.sendMessage`（或 ChatRepository）注入 grep-able 调用入口断言点：必须经 `wsClient.send(...)` 发送 type=`SendMessage` 的 envelope；新增/补充集成测试 `chat_send_message_protocol_binding_test.kt` 断言：(a) 实际写入 `wsClient.send` 的 JSON 字符串包含 `"type":"SendMessage"`；(b) **不**走 Retrofit `POST /chat-messages`；(c) 与 T-00047 服务端实现的字段命名 1:1 对齐 | 1. 集成测试断言客户端发送 JSON 包含 `"type":"SendMessage"` 关键字段<br>2. 测试断言**不**触发 Retrofit POST 路径（mockWebServer 期望 0 次调用）<br>3. 与 server 端 `room/handler/chat.rs::handle_send_message` 接收的字段一一对齐<br>4. 单测覆盖空内容/超长内容/网络断开三种边界 | 3h | Plan | Todo | - | - | ⏳ Pending |

---
