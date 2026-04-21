# Voice Room 开发任务清单

> **版本**: v1.0  
> **更新日期**: 2026-04-20  
> **任务总数**: 72 个 (基建: 4, App Server: 16, Admin Server: 12, Web: 11, Android: 26, ~~原 Web 9~~)  
> **当前阶段**: Phase 0.5 - 交互壳体与基础体验

---

## 🔄 重要变更说明

| 版本 | 日期 | 变更内容 |
|------|------|---------|
| v0.1 | 04-17 | 初始版本，45 个任务 |
| v0.2 | 04-18 | 注册登录合并，Web 端重定位 |
| v0.3 | 04-18 | Server 拆分为 App Server + Admin Server |
| **v0.4** | **04-18** | **深度 Review：补充基建任务、Admin Server 统计接口、跨服务通信任务、shared crate、修复依赖遗漏** |
| **v0.5** | **04-18** | **TDS 文档重建：14 个模块1 TDS 按端拆分（server 5 + adminServer 3 + web 2 + android 4），protocol.md v0.2，ARCHITECTURE.md 双 Server 架构** |
| **v0.6** | **04-18** | **负责人标记：有 TDS 的 14 个任务标为 TDD，其余 46 个标为 Plan；ARCHITECTURE.md §3 目录树修正（doc/arch, doc/tds, shared/ 简化, Web 目录去 WS/RTC/IM）** |
| **v0.7** | **04-18** | **职责流转规则：新增 PM→Plan→TDD→Review→DoD 流转说明；模块0 新增 4 个 TDS（infra/T-0000A~T-0000D）；全部 18 个有 TDS 的任务标为 TDD，42 个标为 Plan** |
| **v1.0** | **04-20** | **Phase 0.5 新增：产品文档重构为 doc/product/index.md + 子文件；新增 11 个 Task（Android 9 + Web 2）覆盖 Splash/主页三Tab/中东黑金主题/个人中心/房间视觉升级/解封弹窗/活水监控；创建 doc/design/android/ 和 doc/design/adminWeb/ 设计文档** |

---

### 任务编号规则

| 编号范围 | 归属端 | 说明 |
|---------|--------|------|
| T-0000A ~ T-0000Z | 基础设施 | CI/CD、Docker、共享模块 |
| T-00001 ~ T-00999 | App Server | C 端业务后端 |
| T-10001 ~ T-10999 | Admin Server | B 端管理后端 |
| T-20001 ~ T-20999 | Web | 后台管理前端 |
| T-30001 ~ T-30999 | Android | C 端用户应用 |

## 任务状态说明

| 状态 | 说明 |
|------|------|
| `Todo` | 待开始（尚未进入任何流转阶段） |
| `In Progress` | 当前负责人正在执行中（Plan 设计中 / TDD 编码中 / Review 审查中 / DoD 文档同步中） |
| `Done` | 已完成（DoD 文档同步完毕） |
| `Blocked` | 被阻塞（前置依赖未完成或外部因素） |

## 职责流转规则

> **核心流程**：`PM 创建 Task` → `Plan 设计方案` → `TDD 实现代码` → `Review 审查代码` → `DoD 记录文档`

| 阶段 | 负责人标记 | 职责 | 完成后动作 |
|------|-----------|------|-----------|
| **PM** | `PM` | 创建 Task，定义需求、验收标准 | 将负责人改为 `Plan` |
| **Plan** | `Plan` | 设计技术方案，输出 TDS 文档到 `doc/tds/[$端]/T-xxx.md`, 完善`doc/architecture/`、`doc/protocol/`设计文件 | 将负责人改为 `TDD`，在任务名称后补充 `[TDS]` 链接 |
| **TDD** | `TDD` | 按 TDS、protocol及`doc/design` 编写测试 → 实现代码 → 测试通过 | 将负责人改为 `Review`，更新 TDS 第四节【实现结果】 |
| **Review** | `Review` | 按 TDS、protocol、design → review代码 → review通过/不通过 | 通过：将负责人改为 `Dod`，更新 TDS 第五节【Review意见】；不通过：将负责人改回 `TDD`，更新 TDS 第五节 |
| **DoD** | `Dod` | 按照代码实现，更新`doc/arch/[$端]/`下的文档，并更新目录下的index.md文件，及`doc/product/index.md`的功能实现状态 | 将状态改为 `Done` |

**规则**：
1. 每个阶段的负责人只能由**上一阶段的负责人**修改为下一阶段
2. `Plan` 未完成 TDS 前，不得将负责人改为 `TDD`
3. `TDD` 未通过全部验收用例前，不得将状态改为 `Review`
4. `Review` 未通过全部Review意见，不得将状态改为 `Dod`
5. `Dod` 未将实现更新到文档之前，不得将状态改为 `Done`
6. 当前所有 Task 已由 PM 创建完毕，初始负责人均为 `Plan`


---

## Phase 0: MVP 基础设施 (预计 6-8 周)

### 模块 0: 工程基建 (Infrastructure & Shared)

> **说明**：此模块是所有端的前置依赖，必须最先完成。

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-0000A** | 基建 | Infra | Docker Compose 开发环境 [TDS](./tds/infra/T-0000A.md) | 无 | 编写 docker-compose.yml，包含 PostgreSQL + Redis | 1. `docker-compose up` 一键启动<br>2. PG 端口 5432, Redis 端口 6379<br>3. 数据挂载本地目录，重启不丢 | ✅ Done | 3 | DoD |
| **T-0000B** | 基建 | Shared | 共享 crate (shared/) [TDS](./tds/infra/T-0000B.md) | 无 | 创建 Rust workspace 共享 crate，包含数据库 models、公共错误码、JWT 工具 | 1. App Server 和 Admin Server 均可引用<br>2. 包含 UserModel, RoomModel 等结构体<br>3. 包含 JWT encode/decode 函数<br>4. 包含 bcrypt 密码工具 | ✅ Done | 5 | DoD |
| **T-0000C** | 基建 | Infra | 数据库权限隔离 [TDS](./tds/infra/T-0000C.md) | T-0000A | 创建两个 PG Role: app_server_user (受限写) 和 admin_server_user (全权) | 1. app_server_user 只能 CRUD 指定表<br>2. admin_server_user 拥有全部权限<br>3. 提供初始化 SQL 脚本 | ✅ Done | 2 | DoD |
| **T-0000D** | 基建 | Infra | CI 基础流水线 [TDS](./tds/infra/T-0000D.md) | T-0000B | GitHub Actions: lint + test + build | 1. PR 触发自动检查<br>2. `cargo clippy` 零警告<br>3. `cargo test` 全部通过<br>4. Web 端 `npm run lint` 通过 | ✅ Done | 4 | DoD |

---

### 模块 1: 用户认证系统 (User Authentication)

#### App Server 端 (C 端业务后端)

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-00001** | App Server | Auth | 数据库表设计 [TDS](./tds/server/T-00001.md) | **T-0000B, T-0000C** | 设计 `users` 表（id, phone, nickname, avatar, coin_balance, created_at 等） | 1. SQLx migration 文件可执行<br>2. users 表 phone 字段唯一索引<br>3. 包含 coin_balance, vip_level 等字段<br>4. 支持软删除 | ✅ Done | 3 | DoD |
| **T-00002** | App Server | Auth | 短信验证码发送接口 [TDS](./tds/server/T-00002.md) | T-00001 | POST `/api/v1/auth/send-code`，接入 Twilio，Redis 限流 | 1. 同一手机号 60 秒内只能发送 1 次<br>2. 验证码 6 位数字，Redis 存储，有效期 5 分钟<br>3. 失败重试机制<br>4. 返回 429 当频率超限 | ✅ Done | 4 | DoD |
| **T-00003** | App Server | Auth | 手机号一键登录接口 [TDS](./tds/server/T-00003.md) | T-00002 | POST `/api/v1/auth/login`，校验验证码，新用户自动注册 | 1. 验证码错误/过期返回 401<br>2. 新用户自动创建记录（默认昵称"用户XXX"）<br>3. 成功返回 JWT (有效期 30 天) + 用户信息<br>4. 基于 msg_id 幂等 | ✅ Done | 4 | DoD |
| **T-00004** | App Server | Auth | JWT 中间件 [TDS](./tds/server/T-00004.md) | T-00003 | Axum 中间件，校验 JWT 并注入 user_id | 1. 无/非法/过期 token 返回 401<br>2. 合法 token 注入 user_id 到上下文 | ✅ Done | 3 | DoD |
| **T-00005** | App Server | Auth | 获取用户信息接口 [TDS](./tds/server/T-00005.md) | T-00004 | GET `/api/v1/users/me` | 1. 需要 JWT 认证<br>2. 返回完整用户信息（不含敏感字段） | ✅ Done | 2 | DoD |

#### Admin Server 端 (B 端管理后端)

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-10001** | Admin Server | Auth | 管理员表设计 [TDS](./tds/adminServer/T-10001.md) | T-00001 | 设计 `admins` 表（id, username, password_hash, role, created_at） | 1. username 唯一索引<br>2. password_hash 使用 bcrypt<br>3. role 字段（super_admin, operator, cs, finance） | ✅ Done | 2 | DoD |
| **T-10002** | Admin Server | Auth | 管理员登录接口 [TDS](./tds/adminServer/T-10002.md) | T-10001 | POST `/api/v1/admin/login`，账号密码登录 | 1. 账号不存在/密码错误返回 401<br>2. 成功返回 JWT (有效期 7 天，含 admin_id, role)<br>3. 记录登录日志（IP、时间） | ✅ Done | 3 | DoD |
| **T-10003** | Admin Server | Auth | 管理员 JWT 中间件 [TDS](./tds/adminServer/T-10003.md) | T-10002 | Axum 中间件 + RBAC 权限校验 | 1. 校验 JWT 有效性<br>2. 注入 admin_id 和 role<br>3. 根据 role 校验接口权限 | ✅ Done | 4 | Done |

#### Web 端 (后台管理前端)

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-20001** | Web | Auth | 管理员登录页 UI [TDS](./tds/web/T-20001.md) | T-10002 | Ant Design 实现账号密码登录页 | 1. 账号/密码输入框<br>2. 记住密码（localStorage）<br>3. 登录失败提示<br>4. 中英文支持 | ✅ Done | 4 | DoD |
| **T-20002** | Web | Auth | 登录逻辑与路由守卫 [TDS](./tds/web/T-20002.md) | T-10002, T-20001 | 调用登录接口，保存 JWT，实现路由鉴权 | 1. 成功跳转数据看板<br>2. 未登录自动跳转登录页<br>3. token 过期自动退出 | ✅ Done | 3 | Dod |

#### Android 端 (C 端用户应用)

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-30001** | Android | Auth | 登录页 UI (Compose) [TDS](./tds/android/T-30001.md) | T-00002 | Material3 实现手机号+验证码登录 | 1. 手机号输入框（+966 沙特格式）<br>2. 发送验证码倒计时<br>3. RTL 布局支持 | ✅ Done | 5 | Dod |
| **T-30002** | Android | Auth | 登录 ViewModel [TDS](./tds/android/T-30002.md) | T-00003, T-30001 | Retrofit 调用登录接口 | 1. Loading/Success/Error 状态<br>2. token 保存到 DataStore<br>3. 登录成功导航到大厅 | ✅ Done | 4 | Dod |
| **T-30003** | Android | Auth | JWT 拦截器 [TDS](./tds/android/T-30003.md) | T-00004, T-30002 | OkHttp 拦截器自动添加 token | 1. 每个请求自动带 Authorization Header<br>2. 401 响应自动跳转登录页 | ✅ Done | 3 | Dod |
| **T-30004** | Android | Auth | 用户信息 Repository [TDS](./tds/android/T-30004.md) | T-00005, T-30003 | 封装用户信息获取与缓存 | 1. 首次登录拉取用户信息<br>2. Room Database 本地缓存<br>3. Flow 订阅用户信息变更 | ✅ Done | 4 | DoD |

---

### 模块 2: 房间大厅与列表 (Room Hall)

#### App Server 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-00006** | App Server | Room | 房间表设计 [TDS](./tds/server/T-00006.md) | T-00001 | 设计 `rooms` 表（id, owner_id, title, type, member_count, status） | 1. 外键关联 users 表<br>2. 索引（status, created_at）<br>3. 房间类型枚举（normal/password/paid） | ✅ Done | 2h | DoD |
| **T-00007** | App Server | Room | 创建房间接口 [TDS](./tds/server/T-00007.md) | T-00006, T-00004 | POST `/api/v1/rooms` | 1. 需要 JWT 认证<br>2. 标题长度 1-30 字符<br>3. 用户同时只能拥有 1 个房间<br>4. 成功返回 201 + room_id | ✅ Done | 4h | DoD |
| **T-00008** | App Server | Room | 房间列表接口 [TDS](./tds/server/T-00008.md) | T-00006 | GET `/api/v1/rooms?page=1&size=20` | 1. 按热度排序（member_count desc）<br>2. 过滤已关闭房间<br>3. 分页返回 (total, page, items) | ✅ Done | 3h | DoD |
| **T-00009** | App Server | Room | 房间详情接口 [TDS](./tds/server/T-00009.md) | T-00008 | GET `/api/v1/rooms/:id` | 1. 包含房主信息<br>2. 在线人数<br>3. 麦位列表（初始为空） | ✅ Done | 2h | DoD |
| **T-00010** | App Server | Room | 关闭房间接口 [TDS](./tds/server/T-00010.md) | T-00007 | DELETE `/api/v1/rooms/:id` | 1. 只有房主可关闭<br>2. 广播 RoomClosed 事件<br>3. 踢出所有成员 | ✅ Done | 3h | DoD |

#### Admin Server 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-10004** | Admin Server | Room | 房间列表接口（后台） | T-00006, T-10003 | GET `/api/v1/admin/rooms` | 1. 支持多条件筛选（房主/状态/时间）<br>2. 返回完整字段（含举报次数）<br>3. 支持导出 CSV | ✅ Done | 3h | DoD |
| **T-10005** | Admin Server | Room | 房间详情接口（后台） | T-10004 | GET `/api/v1/admin/rooms/:id` | 1. 包含所有成员列表<br>2. 最近聊天记录<br>3. 举报记录 | ✅ Done | 3h | DoD |
| **T-10006** | Admin Server | Room | 强制关闭房间接口 [TDS](./tds/adminServer/T-10006.md) | T-10005 | DELETE `/api/v1/admin/rooms/:id` | 1. 需要 RoomForceClose 权限（operator/super_admin）<br>2. 不存在/软删除 → 404/40400<br>3. 已 closed → 409/40901<br>4. 无 owner 检查 | ✅ Done | 4h | DoD |

#### Web 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-20003** | Web | Dashboard | 数据看板首页 [TDS](./tds/web/T-20003.md) | T-20002, **T-10010** | 实现首页数据大盘 | 1. 实时在线人数/房间数<br>2. 今日 DAU/新增用户<br>3. ECharts 趋势图<br>4. 自动刷新（每 30 秒） | ✅ Done | 6h | DoD |
| **T-20004** | Web | Room | 房间管理页面 [TDS](./tds/web/T-20004.md) | T-10004 | Ant Design Table 展示房间列表 | 1. 支持搜索/筛选<br>2. 分页加载<br>3. 点击查看详情 | ✅ Done | 5h | DoD |
| **T-20005** | Web | Room | 房间详情弹窗 [TDS](./tds/web/T-20005.md) | T-10005, T-20004 | Modal 展示房间详情 | 1. 显示成员列表<br>2. 实时聊天记录<br>3. [强制关闭] 按钮 | ✅ Done | 4h | DoD |

#### Android 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-30005** | Android | Room | 大厅页 UI (Compose) [TDS](./tds/android/T-30005.md) | T-00008 | LazyVerticalGrid 展示房间列表 | 1. Coil 加载房主头像<br>2. 显示在线人数<br>3. 点击导航到房间页 | ✅ Done | 6h | DoD |
| **T-30006** | Android | Room | 房间列表 ViewModel [TDS](./tds/android/T-30006.md) | T-00008, T-30005 | Paging3 分页加载 | 1. 下拉刷新<br>2. 上拉自动加载<br>3. 错误重试 | ✅ Done | 5h | DoD |
| **T-30007** | Android | Room | 创建房间对话框 [TDS](./tds/android/T-30007.md) | T-00007 | BottomSheet 输入房间信息 | 1. 标题输入框<br>2. 房间类型选择<br>3. 创建成功导航到房间 | ✅ Done | 4h | DoD |

---

### 模块 3: 房间内核心功能 (In-Room Core)

#### App Server 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-00011** | App Server | WebSocket | WebSocket 连接管理 | T-00004 | 实现 WS 握手、心跳、断线检测 | 1. JWT 认证后建立连接<br>2. 30 秒无心跳断开<br>3. 支持断线重连（携带 last_msg_id）<br>4. 并发 1000 连接压测通过 | ✅ Done | 6h | DoD |
| **T-00012** | App Server | Room | 进入房间逻辑 [TDS](./tds/server/T-00012.md) | T-00011 | 处理 `JoinRoom` 消息 | 1. 校验房间是否存在<br>2. 加入房间内存状态<br>3. 广播 `UserJoined` 事件<br>4. 返回房间状态快照 | ✅ Done | 5h | DoD |
| **T-00013** | App Server | Room | 离开房间逻辑 [TDS](./tds/server/T-00013.md) | T-00012 | 处理 `LeaveRoom` 消息或连接断开 | 1. 从房间移除用户<br>2. 广播 `UserLeft` 事件<br>3. 若在麦上自动下麦 | ✅ Done | 3h | DoD |
| **T-00014** | App Server | Mic | 麦位上麦接口 [TDS](./tds/server/T-00014.md) | T-00012 | 处理 `TakeMic` 消息，Redis 锁防并发 | 1. 检查麦位空闲<br>2. 检查是否被禁麦<br>3. 广播 `MicTaken` 事件<br>4. 并发抢麦只有一个成功 | ✅ Done | 5h | DoD |
| **T-00015** | App Server | Mic | 麦位下麦接口 [TDS](./tds/server/T-00015.md) | T-00014 | 处理 `LeaveMic` 消息 | 1. 只能下自己的麦<br>2. 广播 `MicLeft` 事件 | ✅ Done | 2h | DoD |
| **T-00016** | App Server | Chat | 文本消息广播 [TDS](./tds/server/T-00016.md) | T-00012 | 处理 `SendMessage` 消息 | 1. 消息长度限制 500 字符<br>2. 敏感词过滤<br>3. 基于 msg_id 去重<br>4. 禁言用户拒绝 | ✅ Done | 4h | DoD |

> **App Server 补充任务: 跨服务事件消费**

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-00011B** | App Server | Event | Redis 事件订阅 | T-00011 | 订阅 `admin:events` 频道，执行管理事件 | 1. 收到 `ban_user` → 找到该用户 WS 连接 → 发送封禁通知 → 断开连接<br>2. 收到 `close_room` → 广播房间关闭 → 断开所有成员连接<br>3. 收到 `broadcast_notice` → 向所有在线用户推送公告<br>4. 事件处理失败不影响主服务 | ✅ Done | 5h | DoD |
| **T-00011C** | App Server | Stats | 在線統計上報 | T-00011, T-00012 | 實時維護 Redis 在線統計數據 | 1. 用戶上線/下線時更新 `stats:online_users` (HyperLogLog)<br>2. 用戶進入/離開房間時更新 `stats:active_rooms` (Set)<br>3. 每分鐘快照一次統計數據到 `stats:snapshot:{date}` | ✅ Done | 3h | DoD |

#### Admin Server 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-10007** | Admin Server | User | 用户列表接口 | T-10003 | GET `/api/v1/admin/users` | 1. 支持手机号/ID/昵称搜索<br>2. 分页返回<br>3. 包含资产信息（coin_balance） | ✅ Done | 3h | Done [TDS](./tds/adminServer/T-10007.md) |
| **T-10008** [TDS](./tds/adminServer/T-10008.md) | Admin Server | User | 用户详情接口 | T-10007 | GET `/api/v1/admin/users/:id` | 1. 完整用户信息<br>2. 充值/消费记录<br>3. 登录设备信息 | ✅ Done | 4h | Done |
| **T-10009** [TDS](./tds/adminServer/T-10009.md) | Admin Server | User | 封禁/解封接口 | T-10008 | POST `/api/v1/admin/users/:id/ban` | 1. 支持永久/临时封禁<br>2. 记录封禁原因<br>3. 推送封禁事件到 Redis (→ App Server)<br>4. 记录操作日志 | ✅ Done | 4h | Done |
| **T-10010** | Admin Server | Stats | 数据统计接口 [TDS](./tds/adminServer/T-10010.md) | T-10003 | GET `/api/v1/admin/stats/overview` | 1. 返回 DAU、新增用户、活跃房间数、在线人数<br>2. 支持按日期范围查询<br>3. 在线人数从 Redis 获取（App Server 维护）<br>4. 响应时间 < 500ms | ✅ Done | 5h | DoD |
| **T-10011** [TDS](./tds/adminServer/T-10011.md) | Admin Server | Event | 跨服务事件发布 | T-10003, T-0000A | Redis Pub/Sub 发布管理事件 | 1. 封禁用户时发布 `ban_user` 事件<br>2. 关闭房间时发布 `close_room` 事件<br>3. 消息格式: `{type, payload, admin_id, ts}`<br>4. 集成到 T-10009 和 T-10006 中 | ✅ Done | 4h | Done |
| **T-10012** [TDS](./tds/adminServer/T-10012.md) | Admin Server | Log | 操作审计日志 | T-10001 | 设计 `admin_logs` 表 + 写入中间件 | 1. 记录所有敏感操作（封禁/解封/关闭房间/充值）<br>2. 字段: admin_id, action, target_id, ip, detail, created_at<br>3. Axum 中间件自动拦截记录<br>4. GET `/api/v1/admin/logs` 查询接口 | ✅ Done | 5h | Done |

#### Web 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-20006** [TDS](./tds/web/T-20006.md) | Web | User | 用户管理页面 | T-10007 | Ant Design Table 展示用户列表 | 1. 搜索框（手机号/ID/昵称）<br>2. 分页加载<br>3. 状态筛选（全部/正常/封禁） | ✅ Done | 5h | Done |
| **T-20007** [TDS](./tds/web/T-20007.md) | Web | User | 用户详情抽屉 | T-10008, T-20006 | Drawer 展示用户详细信息 | 1. 基础信息卡片<br>2. 资产信息<br>3. 行为数据<br>4. [封禁] [解封] 按钮 | ✅ Done | 5h | Dod |
| **T-20008** [TDS](./tds/web/T-20008.md) | Web | User | 封禁对话框 | T-10009, T-20007 | Modal 实现封禁操作 | 1. 选择封禁时长<br>2. 选择封禁原因<br>3. 填写备注<br>4. 二次确认 | ✅ Done | 3h | Done |
| **T-20009** [TDS](./tds/web/T-20009.md) | Web | Log | 操作日志页面 | T-10012 | Ant Design Table 展示审计日志 | 1. 按时间倒序<br>2. 支持按操作人/类型/时间筛选<br>3. 展示操作详情 | ✅ Done | 4h | Done |

#### Android 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-30008** [TDS](./tds/android/T-30008.md) | Android | WebSocket | WebSocket 连接封装 | T-00011 | OkHttp WebSocket + Flow | 1. 自动重连（指数退避）<br>2. Kotlin Flow 发射连接状态<br>3. 心跳包发送 | ✅ Done | 6h | Done |
| **T-30009** [TDS](./tds/android/T-30009.md) | Android | Room | 房间页 UI (Compose) | T-00009 | 实现房间完整布局 | 1. 顶部房间信息<br>2. 麦位 Grid<br>3. 聊天列表<br>4. 底部输入栏 | ✅ Done | 8h | DoD |
| **T-30010** [TDS](./tds/android/T-30010.md) | Android | Room | 房间 ViewModel | T-00012, T-30008, T-30009 | 管理房间状态，处理 WS 消息 | 1. 进入房间发送 JoinRoom<br>2. 监听服务端事件更新 State<br>3. 离开清理资源 | ✅ Done | 6h | DoD |
| **T-30011** [TDS](./tds/android/T-30011.md) | Android | Mic | 麦位组件 (Compose) | T-30009 | 可复用麦位卡片 | 1. 三种状态渲染<br>2. Lottie 音浪动画<br>3. RTL 布局 | ✅ Done | 5h | DoD |
| **T-30012** [TDS](./tds/android/T-30012.md) | Android | Mic | 麦克风权限请求 | T-30011 | Accompanist Permissions | 1. 运行时权限请求<br>2. 权限拒绝对话框<br>3. 跳转系统设置 | ✅ Done | 3h | DoD |
| **T-30013** [TDS](./tds/android/T-30013.md) | Android | Mic | 上麦/下麦逻辑 | T-00014, T-30012 | 发送上麦请求 + RTC 推流 | 1. 权限通过后上麦<br>2. 集成 RTC SDK<br>3. 成功后开启推流 | ✅ Done | 7h | DoD |
| **T-30014** [TDS](./tds/android/T-30014.md) | Android | Chat | 聊天列表 (Compose) | T-30009 | LazyColumn 聊天消息 | 1. 自动滚动到最新<br>2. 不同类型消息样式<br>3. 系统消息居中 | ✅ Done | 5h | DoD |
| **T-30015** [TDS](./tds/android/T-30015.md) | Android | Chat | 输入框组件 | T-30014 | TextField + 发送按钮 | 1. 软键盘弹出布局调整<br>2. 回车发送<br>3. 空消息禁用发送 | ✅ Done | 3h | DoD |
| **T-30016** [TDS](./tds/android/T-30016.md) | Android | Chat | 发送消息逻辑 | T-00016, T-30015 | 发送 SendMessage | 1. 发送中禁用<br>2. 成功清空输入<br>3. 失败重试 | ✅ Done | 3h | DoD |
| **T-30017** [TDS](./tds/android/T-30017.md) | Android | Chat | 接收消息逻辑 | T-00016, T-30014 | 监听服务端消息 | 1. 实时追加到列表<br>2. 去重（msg_id）<br>3. 自动滚动 | ✅ Done | 3h | DoD |

---
## Phase 0.5: 交互壳体与基础体验

> **说明**：Phase 0 的代码已全部完成，但 Android App 仍停留在 Auth Bootstrap 调试页面，缺少完整的用户交互壳。Phase 0.5 聚焦于让 App "能看能用"：中东黑金视觉主题、Splash 启动页、主页三Tab框架、个人中心，以及对已有页面的视觉升级。Web 端补充解封确认弹窗和活水房间监控。  
> **产品设计规范**: 详见 [doc/product/android_app_design.md](./product/android_app_design.md)

### 模块 4: 中东黑金主题与 App 壳体 (MENA Theme & App Shell)

#### Android 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 | UI设计文档 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|------------|
| **T-30018** | Android | Theme | MenaTheme 中东黑金主题系统 [TDS](./tds/android/T-30018.md) | 无 | 封装 Material3 黑金主题（Colors/Typography/Shapes）+ RTL Provider + GoldButton / GoldOutlinedTextField / AvatarWithFrame 通用组件 | 1. `MenaTheme {}` 内自动黑金色系<br>2. GoldButton 金色渐变+白字+24dp圆角<br>3. RTL 自动生效 | ✅ Done | 6h | Done | [T-30018.md](./design/android/T-30018.md) |
| **T-30019** | Android | Splash | Splash 启动页 [TDS](./tds/android/T-30019.md) | T-30018 | 品牌 Splash 页：Logo 缩放动画 → JWT 检测 → 自动导航到 MainScreen 或 LoginScreen | 1. Logo 缩放+淡入动画 800ms<br>2. 有效 JWT → MainScreen<br>3. 无效 JWT → LoginScreen<br>4. 返回键不可回退到 Splash | ✅ Done | 4h | Done | [T-30019.md](./design/android/T-30019.md) |
| **T-30020** | Android | Navigation | MainScreen 底部三Tab框架 [TDS](./tds/android/T-30020.md) | T-30018, T-30019 | BottomNavigation 三Tab（房间/消息/我的），Tab切换保持状态 | 1. 默认显示房间Tab<br>2. 三Tab可切换，选中项金色<br>3. Tab切换保持各页面状态 | ✅ Done | 5h | Done | [T-30020.md](./design/android/T-30020.md) |
| **T-30021** | Android | Auth | 登录页视觉升级 [TDS](./tds/android/T-30021.md) | T-30018 | 将现有 LoginScreen 从白色主题改造为黑金风格：渐变背景 + GoldOutlinedTextField + GoldButton。功能逻辑不变 | 1. 深色渐变背景<br>2. 所有输入框用 GoldOutlinedTextField<br>3. 按钮用 GoldButton<br>4. **现有功能测试不回归** | ✅ Done | 3h | Done | [T-30021.md](./tds/android/T-30021.md) |
| **T-30022** | Android | Room | 大厅页视觉升级 [TDS](./tds/android/T-30022.md) | T-30018, T-30020 | 将 HallScreen 改造为黑金风格：深色RoomCard + OnlineCountBadge + 顶部栏 + 分类横滑(占位)。Paging3 逻辑不变 | 1. RoomCard 深色底+圆角16dp<br>2. OnlineCountBadge 绿点+数字<br>3. 创建房间 FAB 金色<br>4. **Paging3不回归** | ✅ Done | 5h | Done | [T-30022.md](./design/android/T-30022.md) |
| **T-30023** [TDS](./tds/android/T-30023.md) | Android | Messages | 消息Tab占位页 | T-30018, T-30020 | 通用 `PlaceholderScreen` Composable（`core/ui/`）+ `MessagesPlaceholder` 委托，消息 Tab 展示"即将上线"占位页 | 1. 消息Tab显示占位页<br>2. PlaceholderScreen 可复用<br>3. 深色背景 | ✅ Done | 2h | Done |
| **T-30024** [TDS](./tds/android/T-30024.md) | Android | Profile | 个人中心页 | T-30018, T-30020, T-30004 | "我的"Tab 页面：头像(AvatarWithFrame)+昵称+ID+余额+设置入口+退出登录(二次确认) | 1. 显示用户头像/昵称/ID/余额<br>2. 复制ID到剪贴板<br>3. 退出登录二次确认→清JWT→LoginScreen<br>4. 网络异常用本地缓存 | ✅ Done | 6h | Done | [T-30024.md](./design/android/T-30024.md) |
| **T-30025** [TDS](./tds/android/T-30025.md) | Android | Room | 房间页视觉升级 | T-30018 | 将 RoomScreen 改造为黑金风格：主麦突出(80dp金色光圈) + 副麦4列 + 弹幕金色昵称 + 深色背景。WS/上下麦逻辑不变 | 1. 主麦80dp+金色光圈<br>2. 副麦60dp四列<br>3. 空麦位虚线+"+"<br>4. 系统消息金黄色居中<br>5. **WS/上下麦不回归** | ✅ Done | 6h | DoD | [T-30025.md](./design/android/T-30025.md) |
| **T-30026** | Android | Room | 房间底部操作栏升级 | T-30018, T-30025 | 底部操作栏扩展：输入框 + 🎤麦克风开关 + 🎁礼物(灰禁) + ❤️表情(灰禁) + 🚪退出(二次确认) | 1. 4个功能按钮可见<br>2. 🎤不在麦上时禁用<br>3. 🎤在麦上时绿/红切换<br>4. 🎁❤️灰色禁用+Toast<br>5. 🚪二次确认退出 | Todo | 5h | Plan | [T-30026.md](./design/android/T-30026.md) |

---

### 模块 5: Web 管理端增强 (Admin Web Enhancements)

#### Web 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 | UI设计文档 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|------------|
| **T-20010** | Web | User | 解封用户确认弹窗 | T-20007, T-10009 | UnbanModal 组件：解封原因+备注+二次确认+API调用。与 BanModal 对称 | 1. 封禁用户 [解封] 弹出 UnbanModal<br>2. 原因必填<br>3. 成功后状态变"正常"<br>4. isConfirming 防重复 | Todo | 3h | Plan | [T-20010.md](./design/adminWeb/T-20010.md) |
| **T-20011** | Web | Room | 活水房间监控增强 | T-20004 | 房间列表增加"活跃状态"Tag(活跃/冷清/异常) + "持续时长"列 + 活跃度筛选条件 | 1. 新增活跃状态+持续时长两列<br>2. Tag颜色根据规则渲染<br>3. 活跃度筛选可过滤<br>4. 异常房间行高亮<br>5. **现有功能不回归** | Todo | 4h | Plan | [T-20011.md](./design/adminWeb/T-20011.md) |

