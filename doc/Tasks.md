# Voice Room 开发任务清单

> **版本**: v1.3  
> **更新日期**: 2026-04-21  
> **任务总数**: 111 个 (基建: 4, App Server: 30, Admin Server: 16, Web: 14, Android: 44, E-07 15 + E-07.5 6 + E-10 18)  
> **当前阶段**: Phase 1 - 核心营收闭环（E-07 + E-07.5 并行）→ Phase 1.5 E-10 房间治理

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
| **v1.1** | **04-21** | **Phase 1 启动：E-07 虚拟礼物与钱包闭环 MVP，新增 15 个 Task（App Server 5 / Admin Server 2 / Web 1 / Android 7）；产出 `doc/product/phase1_gift_economy.md` 方向总纲、`competitors.md` 附录 A、`business_flows.md §2.7`；Android 7 个新设计文档** |
| **v1.2** | **04-21** | **E-07.5 埋点与观测性基建（与 E-07 并行）：新增 6 个 Task（App Server 2 / Admin Server 1 / Web 1 / Android 2）；产出 `doc/product/phase1_observability.md` 方向总纲、`business_flows.md §2.9` 事件字典；Android 2 个新设计文档** |
| **v1.3** | **04-21** | **Phase 1.5 E-10 房间主权与管理员体系：新增 18 个 Task（App Server 7 / Admin Server 1 / Web 1 / Android 9）；产出 `doc/product/phase1_room_governance.md` 方向总纲、`competitors.md` 附录 B、`business_flows.md §2.8` 治理流程；Android 9 个新设计文档** |
| **v1.4** | **2026-04-29** | **T-30034 DoD 完成，E-07.5 进度 5/6：新建 `doc/arch/android/analytics.md`（AnalyticsPort 接口设计、SentryAnalytics/DefaultSentryHub Stub、SensitiveFilter 脱敏策略、ConsentMode 枚举、NoopAnalytics、BuildConfig.SENTRY_DSN 注入、CI 静态检查脚本、MVP 限制 HIGH-01/02、待修复项 MEDIUM-01/02）；doc/arch/android/index.md 新增 analytics.md 子模块索引与能力状态描述；Tasks.md T-30034 标记为 ✅ Done（负责人: Dod）；doc/product/index.md E-07.5 进度更新为 5/6** |
| **v1.5** | **2026-04-30** | **T-30035 DoD 完成，E-07.5 进度 6/6（全部完成）：doc/arch/android/analytics.md 新增第十二章 EventReportClient 主链路（EventReportClient 主入口 + 队列策略 + Throttler + Transport 选择 + SessionManager + CommonPropsProvider + ConsentRepository/DataStoreConsentStore + PrivacyConsentDialog + 26 个核心事件埋点）与第十三章 TDD 验收结果（42 个单元测试全部通过）；doc/arch/android/index.md 能力全景新增 T-30035 条目；Tasks.md T-30035 确认 ✅ Done（负责人: Dod）；doc/product/index.md E-07.5 进度更新为 6/6 全部完成** |
| **v1.9** | **2026-05-19** | **T-00030 DoD 完成，E-10 进度 7/18：doc/arch/server/room.md 新增三十二~三十九章（TransferAdmin assign/revoke C→S 信令格式、AdminChanged 广播含 previous_admin_id、ForceTakeMic/ForceLeaveMic 信令格式、权限矩阵 owner-only TransferAdmin/owner+admin ForceMic、管理员不能抱下房主约束、ForceTakeMic 检查 mic_muted、原子性 DB 失败不广播、遗留 LOW target 不在房间未显式校验、文件清单与 427 测试汇总）；Tasks.md T-00030 状态 → ✅ Done；doc/product/index.md E-10 进度 6/18 → 7/18** |
| **v2.0** | **2026-05-26** | **T-30040 DoD 完成，E-10 进度 14/18：doc/arch/android/features.md 新增用户操作菜单模块文档（UserActionBottomSheet testTag 清单 10 项、ActionMatrix.kt computeActions 9 角色组合权限矩阵、Role 枚举 OWNER/ADMIN/MEMBER、UserAction 枚举 INVITE_MIC/MUTE_MIC/MUTE_CHAT/KICK/ASSIGN_ADMIN/REVOKE_ADMIN/VIEW_PROFILE/REPORT 8 项、RevokeAdmin 两步确认流程 pendingRevokeTarget→event→confirmRevokeAdmin→WS TransferAdmin(revoke)→AdminChanged 广播、与 T-30041 联动 selectedKickTarget 字段解耦设计）；Tasks.md T-30040 确认 ✅ Done 负责人 Dod；doc/product/index.md E-10 进度 13/18 → 14/18** |
| **v2.1** | **2026-05-27** | **T-30041 DoD 完成，E-10 进度 15/18：doc/arch/android/features.md 新增踢人原因弹窗模块文档（KickReasonDialog AlertDialog dismissOnClickOutside=false、KickReason 枚举 HARASSMENT/SPAM/ABUSE/OTHER、KickDialogState canSubmit 逻辑（OTHER 必填 customText、isSubmitting 防重复提交）、reason 字段 JSON 安全转义（双引号→全角引号、反斜杠转义）、与 T-30040 selectedKickTarget 联动流程（ShowKickReasonDialog event→弹窗→kickUser→UserKicked 广播→dismiss+Toast）、testTag 清单 kick_reason_0~3/kick_reason_custom_input/btn_confirm_kick）；Tasks.md T-30041 确认 ✅ Done 负责人 Dod；doc/product/index.md E-10 进度 14/18 → 15/18** |
| **v1.8** | **2026-05-18** | **T-00029 DoD 完成，E-10 进度 6/18：doc/arch/server/room.md 新增二十四~三十一章（MuteUser/UnmuteUser C→S 信令格式、UserMuted 广播格式、Redis Key mic_muted/chat_muted TTL=duration_sec、处理流程 5 步、SendMessage→40305/TakeMic→40306 双重拦截、duration_sec [60,86400] 边界、送礼不受禁麦影响、文件清单与 365 测试汇总）；Tasks.md T-00029 状态 → ✅ Done 负责人 → Dod；doc/product/index.md E-10 进度 5/18 → 6/18** |
| **v1.7** | **2026-05-17** | **T-00028 DoD 完成，E-10 进度 5/18：doc/arch/server/room.md 新增十六~二十三章（KickUser C→S/S→C/广播信令格式、处理流程 7 步、权限校验矩阵 owner>admin>member 不可踢 owner、Redis 冷却 Key kicked:{room_id}:{user_id} TTL 600s、JoinRoom 42911 冷却拦截、并发保护 DashMap.remove() 原子性、遗留问题 MEDIUM MicLeft/UserLeft 广播顺序 + LOW TTL=-1 处理、文件清单与 366+ 测试汇总）；Tasks.md T-00028 状态 → ✅ Done 负责人 → Dod；doc/product/index.md E-10 进度 4/18 → 5/18** |
| **v1.6** | **2026-05-16** | **T-00027 DoD 完成，E-10 进度 4/18：doc/arch/server/room.md 新增十三~十五章（GET /api/v1/rooms/:id/members 接口契约、角色优先级 owner>admin>member、1 次批量 SQL WHERE id=ANY($1)、MemberSnapshot 单一数据源、muted_mic/muted_chat Redis Key、权限错误码、文件清单与 398 测试汇总）；Tasks.md T-00027 状态 → ✅ Done 负责人 → Dod；doc/product/index.md E-10 进度 3/18 → 4/18** |

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
| **T-30025** [TDS](./tds/android/T-30025.md) | Android | Room | 房间页视觉升级 | T-30018 | 将 RoomScreen 改造为黑金风格：主麦突出(80dp金色光圈) + 副麦4列 + 弹幕金色昵称 + 深色背景。WS/上下麦逻辑不变 | 1. 主麦80dp+金色光圈<br>2. 副麦60dp四列<br>3. 空麦位虚线+"+"<br>4. 系统消息金黄色居中<br>5. **WS/上下麦不回归** | ✅ Done | 6h | Done | [T-30025.md](./design/android/T-30025.md) |
| **T-30026** [TDS](./tds/android/T-30026.md) | Android | Room | 房间底部操作栏升级 | T-30018, T-30025 | 底部操作栏扩展：输入框 + 🎤麦克风开关 + 🎁礼物(灰禁) + ❤️表情(灰禁) + 🚪退出(二次确认) | 1. 4个功能按钮可见<br>2. 🎤不在麦上时禁用<br>3. 🎤在麦上时绿/红切换<br>4. 🎁❤️灰色禁用+Toast<br>5. 🚪二次确认退出 | ✅ Done | 5h | Done | [T-30026.md](./design/android/T-30026.md) |

---

### 模块 5: Web 管理端增强 (Admin Web Enhancements)

#### Web 端

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 | UI设计文档 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|------------|
| **T-20010** | Web | User | 解封用户确认弹窗 [TDS](./tds/web/T-20010.md) | T-20007, T-10009 | UnbanModal 组件：解封原因+备注+二次确认+API调用。与 BanModal 对称 | 1. 封禁用户 [解封] 弹出 UnbanModal<br>2. 原因必填<br>3. 成功后状态变"正常"<br>4. isConfirming 防重复 | ✅ Done | 3h | Done | [T-20010.md](./design/adminWeb/T-20010.md) |
| **T-20011** | Web | Room | 活水房间监控增强 [TDS](./tds/web/T-20011.md) | T-20004 | 房间列表增加"活跃状态"Tag(活跃/冷清/异常) + "持续时长"列 + 活跃度筛选条件 | 1. 新增活跃状态+持续时长两列<br>2. Tag颜色根据规则渲染<br>3. 活跃度筛选可过滤<br>4. 异常房间行高亮<br>5. **现有功能不回归** | ✅ Done | 4h | Done | [T-20011.md](./design/adminWeb/T-20011.md) |

---

## Phase 1: 核心营收闭环

> **说明**：Phase 1 聚焦营收打通。E-07 采用"封闭内循环"策略——充值通道为 Admin 手动调整（快速打通闭环），真实支付延后到 E-08。详见 [phase1_gift_economy.md](./product/phase1_gift_economy.md)。
> **产品流程规范**: [business_flows.md §2.7](./product/business_flows.md)

### 模块 6: 虚拟礼物与钱包闭环 MVP (E-07)

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

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-00017** | App Server | Wallet | ✅ 钱包 Schema 与迁移 [TDS](./tds/server/T-00017.md) | T-0000B | users 表增 `diamond_balance BIGINT DEFAULT 0`（CHECK >= 0）+ 新建 `wallet_transactions` 流水表（user_id/type/amount/balance_after/ref_id/reason/created_at） | 1. 迁移脚本可幂等执行<br>2. CHECK 约束防止余额为负<br>3. 流水表带 (user_id, created_at) 索引<br>4. 新注册用户默认 0 | ✅ Done | 3h | Dod |
| **T-00018** | App Server | Wallet | 余额查询 API + WS 推送 [TDS](./tds/server/T-00018.md) | T-00017 | GET `/api/v1/wallet/balance`、GET `/api/v1/wallet/transactions`（分页）；新增 WS 信令 `BalanceUpdated { msg_id, diamond_balance, delta, reason, ref_id, timestamp }`，在余额变化时推送给当前用户所有会话；支持 Redis PubSub 跨进程推送 | 1. 查询返回最新余额<br>2. 流水按时间倒序分页<br>3. 余额变化后 <500ms 内 WS 推送<br>4. 同一用户多连接全部收到推送，每条消息独立 msg_id<br>5. 离线用户重连后主动拉刷新 | ✅ Done | 5h | Dod |
| **T-00019** | App Server | Gift | 礼物配置表 + 列表 API [TDS](./tds/server/T-00019.md) | T-0000B | 新建 `gifts` 表（id/name_en/name_ar/icon_url/price/tier/effect_level/animation_url/is_active/sort_order）；GET `/api/v1/gifts/list` 返回上架礼物列表（按 tier+sort_order 排序） | 1. 迁移脚本创建表并插入 8 款 MVP 礼物种子数据<br>2. 列表只返回 is_active=true<br>3. 支持 Accept-Language 切换 name_en/name_ar<br>4. 响应时间 <50ms（加缓存） | ✅ Done | 5h | Dod |
| **T-00020** | App Server | Gift | SendGift 事务 + 广播 [TDS](./tds/server/T-00020.md) | T-00017, T-00019, T-00016 | 新增 WS 信令 `SendGift { gift_id, receiver_id, count, msg_id }`；SQLx 事务：查余额→扣发送者→加接收者魅力值→写流水→写 gift_records→Redis ZINCRBY 日/周榜；广播 `GiftReceived { sender, receiver, gift, count, effect_level, total }` 给房间所有人；发送者单独推送 BalanceUpdated | 1. 并发 20 QPS 无超扣/脏数据<br>2. 重复 msg_id 幂等返回首次结果<br>3. 余额不足返回 INSUFFICIENT_BALANCE 并回滚<br>4. 接收者离线返回 RECEIVER_UNAVAILABLE<br>5. 全链路落 4 张表 + 2 个 Redis key | ✅ Done | 10h | Dod |
| **T-00021** | App Server | Ranking | 魅力/财富榜单 API [TDS](./tds/server/T-00021.md) | T-00020 | GET `/api/v1/ranking?type=charm|wealth&period=day|week&limit=50`；读取 Redis ZSet 返回 Top N + 当前用户排名；定时任务：每日 00:00 Riyadh 切换日榜 key，每周六切换周榜 key | 1. Top 50 返回 <100ms<br>2. 返回值包含 Top3 金银铜标识字段<br>3. 当前用户未入榜时返回 rank=null<br>4. 时区切换任务可补偿执行<br>5. 旧榜归档到 ranking_archive | ✅ Done | 6h | Dod |

#### Admin Server

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-10013** | Admin Server | Wallet | 手动调整余额 API [TDS](./tds/adminServer/T-10013.md) | T-00017, T-10012 | POST `/api/v1/admin/users/:id/wallet/adjust { amount, reason }`；事务：改 users 余额 + 写 wallet_transactions (type='admin_adjust', operator_id) + 写 admin_logs；Redis PUBLISH admin:events {type:'balance_updated', user_id, new_balance} 通知 App Server 推 WS | 1. amount 正数=加，负数=扣<br>2. reason 必填且写入日志<br>3. 导致余额<0 返回 400<br>4. 事务原子性（任一步失败整体回滚）<br>5. Redis 事件已发布 | Done | 5h | Dod |
| **T-10014** | Admin Server | Gift | 礼物 CRUD 管理 API [TDS](./tds/adminServer/T-10014.md) | T-00019, T-10012 | `/api/v1/admin/gifts` CRUD（GET 列表含未上架 / POST 新增 / PUT 更新 / DELETE 软删）；图片/Lottie 上传走对象存储或本地静态目录；所有操作写 admin_logs | 1. 上架/下架通过 is_active 字段切换<br>2. 删除为软删（is_deleted=true）<br>3. 上传文件类型白名单校验<br>4. 价格必须 >=1<br>5. 所有写操作落 admin_logs | ✅ Done | 6h | Dod |

#### Web Admin

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 | UI设计文档 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|------------|
| **T-20012** | Web | User | 余额调整弹窗 + 礼物管理页 [TDS](./tds/web/T-20012.md) | T-10013, T-10014, T-20007 | 用户详情页新增"调整余额"按钮→`AdjustBalanceModal`（金额/原因/确认）；新增"礼物管理"菜单页：列表 + 新增弹窗 + 编辑 + 上下架开关 + 软删 | 1. 调整成功后用户余额实时刷新<br>2. 原因必填校验<br>3. 负数显示红色二次确认<br>4. 礼物列表可筛选 tier/状态<br>5. 上传图片预览 | ✅ Done | 8h | Dod | 完成 DoD 文档同步：web/user-management.md + web/gift-management.md 新增；product/index.md E-07 进度 8/15 |

#### Android

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 | UI设计文档 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|------------|
| **T-30027** | Android | Wallet | 钱包页（余额 + 流水）[TDS](./tds/android/T-30027.md) | T-00018, T-30024 | 新建 `WalletScreen`：顶部大卡片显示钻石余额 + "充值"按钮（占位 Toast"即将上线"）；下方 LazyColumn 流水列表（收入绿色/支出红色 + 图标 + 时间）；WS `BalanceUpdated` 自动刷新；个人中心"钻石余额"项点击跳转进入 | 1. 余额大号金色显示<br>2. 下拉刷新拉最新余额<br>3. 流水分页加载<br>4. 收到 BalanceUpdated 事件即时更新<br>5. 空状态占位"暂无流水" | ✅ Done | 6h | Dod | [T-30027.md](./design/android/T-30027.md) |
| **T-30028** | Android | Gift | 礼物面板 Bottom Sheet [TDS](./tds/android/T-30028.md) | T-00019, T-30026 | `GiftPanelBottomSheet` Composable：顶部余额条 + 礼物网格（4列）+ 分类 Tab（热门/全部）+ 数量选择器（1/10/66/520/786/1314）+ 发送按钮；房间页 🎁 按钮点击弹出（替换 T-30026 的 Toast 占位） | 1. 面板占屏幕 55% 高度<br>2. 余额实时显示（WS 更新）<br>3. 选中礼物有金色边框<br>4. 数量按钮吉祥数档位<br>5. 余额不足时"送出"置灰 | ✅ Done | 7h | Dod | [T-30028.md](./design/android/T-30028.md) |
| **T-30029** | Android | Gift | 接收者选择器 [TDS](./tds/android/T-30029.md) | T-30028 | 礼物面板顶部横向滚动的麦位头像条：默认选中 1 号主麦；点击切换；选中项金色光圈；空麦位不显示 | 1. 显示所有在麦用户<br>2. 默认主麦<br>3. 选中高亮<br>4. 无人在麦时发送按钮禁用 + 提示<br>5. 麦位变化实时刷新 | ✅ Done | 4h | Dod | [T-30029.md](./design/android/T-30029.md) |
| **T-30030** | Android | Gift | SendGift 客户端 + 幂等 [TDS](./tds/android/T-30030.md) | T-30028, T-30029, T-00020 | 点"送出"生成 UUID msg_id → WS 发送 SendGift → 按钮 loading → 收到 GiftReceived 或错误后还原；同礼物 3s 内连击累加 count，最终只发一次；错误码对应处理（余额不足弹窗/接收者不可用 Toast） | 1. msg_id 每次唯一<br>2. 3s 连击聚合<br>3. 超时 5s 自动失败<br>4. 余额不足跳 T-30032 弹窗<br>5. 成功后面板不自动关 | ✅ Done | 5h | Dod | [T-30030.md](./design/android/T-30030.md) |
| **T-30031** | Android | Gift | 送礼特效播放器 + 弹幕样式 [TDS](./tds/android/T-30031.md) | T-30030 | 分层特效：L1 聊天区气泡（礼物图标+文字）；L2 接收者麦位金色光圈闪烁 2s；L3 全屏 Lottie 动画覆盖层（使用 airbnb/lottie-compose），动画期间可继续交互但不可点击覆盖层 | 1. L1 弹幕礼物消息金色昵称+图标+"送给 xxx x N"<br>2. L2 麦位动画 2s 后自动结束<br>3. L3 全屏动画 5-8s 可跳过<br>4. 连击礼物动画仅播一次，数量徽章更新<br>5. 接收补偿消息不回放动画 | ✅ Done | 8h | Dod | [T-30031.md](./design/android/T-30031.md) |
| **T-30032** | Android | Wallet | 余额不足引导弹窗 [TDS](./tds/android/T-30032.md) | T-30028 | `InsufficientBalanceDialog` AlertDialog：标题"钻石不足" + 当前余额 + 所需余额 + "去充值"按钮 → 跳 WalletScreen；"取消"按钮关闭 | 1. 显示当前/所需钻石<br>2. "去充值"跳钱包页<br>3. 点击外部不关闭<br>4. 关闭后礼物面板保留选中状态 | ✅ Done | 2h | Dod | [T-30032.md](./design/android/T-30032.md) |
| **T-30033** | Android | Ranking | 魅力/财富榜页 [TDS](./tds/android/T-30033.md) | T-00021, T-30018 | 新建 `RankingScreen`：顶部双 Tab（魅力/财富）+ 子 Tab（日榜/周榜）；列表项：排名+头像(Top3带金银铜光圈+Top1王冠)+昵称+钻石数；底部固定"我的排名"；入口：大厅顶部 🏆 图标 + 房间页"榜单"菜单项 | 1. 四组 Tab 数据独立加载<br>2. Top3 头像光圈颜色不同<br>3. Top1 王冠图标<br>4. 未入榜显示"未上榜，继续加油"<br>5. 下拉刷新 | ✅ Done | 7h | Dod | [T-30033.md](./design/android/T-30033.md) |

---

## Phase 1 并行 Epic：E-07.5 埋点与观测性基建

> **说明**：为 E-07 礼物闭环提供数据验证能力。需在 E-07 上线前完成。与 E-07 **零依赖冲突**，可完全并行。详见 [phase1_observability.md](./product/phase1_observability.md)。
> **产品流程规范**: [business_flows.md §2.9](./product/business_flows.md)

### 模块 7: 埋点与观测性基建 (E-07.5) ✅ 完成 (6/6)

> **依赖关系图**:
> ```
> T-00022 (events表+HTTP API) ──┬─► T-00023 (WS ReportEvent) ──────────┐
>                              └─► T-10015 (Admin 查询 API) ──► T-20013 (Web 行为流Tab)
> T-30034 (Analytics防腐层+Sentry) ─► T-30035 (EventReportClient+核心埋点+隐私弹窗)
>                                        ↑依赖 T-00022 + T-00023
> ```

#### App Server

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-00022** | App Server | Analytics | 事件表 Schema + 分区 + HTTP 接收 API [TDS](./tds/server/T-00022.md) | T-0000B | 新建 `events` 表（id / user_id? / device_id / event_name / properties JSONB / session_id / client_ts / server_ts / app_version / os_version / locale / network_type）。按日分区（PG `PARTITION BY RANGE (server_ts)`）；新增每日凌晨 Asia/Riyadh 自动建次日分区的定时任务；提供 HTTP `POST /api/v1/events/batch { events: [...] }`（兼容未登录 Splash 阶段），异步批量写入，取 `device_id` 作主键，登录后服务端可回填 `user_id` | 1. 迁移脚本幂等，含事件表 + 首日分区<br>2. 分区自动创建任务可补偿执行<br>3. 批量写入 ≥100 events/req 耗时 <200ms<br>4. 单个事件 properties JSON 限长 8KB，超出截断并记日志<br>5. 未登录请求 user_id 为 null则允许，但 device_id 必填<br>6. JWT 中 user_id 与上报 user_id 不一致时日志告警，以 JWT 为准 | Done | 6h | Dod |
| **T-00023** | App Server | Analytics | WS `ReportEvent` 信令 + 写入服务 [TDS](./tds/server/T-00023.md) | T-00022, T-00016 | WS 新增 `ReportEvent { events: [...] }` 收到后复用 T-00022 的写入服务；被动 ACK `EventReportAck { received, rejected_indices }`；服务端 user_id 以当前 WS 连接 JWT 为准覆盖客户端来的可选 user_id；创建 `handle_report_event` 函数供 WS 通道调用 | 1. 单次 WS 最多接受 100 events，超过返回 `BATCH_TOO_LARGE` 并仍写前 100 条<br>2. WS 上报时服务端时间戳一律用 server_ts 覆盖<br>3. ACK 返回 received 与 rejected_indices 列表<br>4. 客户端上报 user_id 与 JWT 不一致时服务端日志告警<br>5. 与 HTTP 通道共享写入层，无重复代码 | Done | 4h | Dod |

#### Admin Server

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-10015** | Admin Server | Analytics | 用户行为查询 API [TDS](./tds/adminServer/T-10015.md) | T-00022, T-10012 | `GET /api/v1/admin/users/:id/events?event_name=&from=&to=&page=&limit=` ；默认返回按 server_ts 倒序的事件流；只查最近 30 天（分区时窗命中）；返回格式与上报结构一致；操作入 admin_logs | 1. 时间窗超过 30 天返回 400<br>2. event_name 可多值（逗号分隔）<br>3. 分页 max limit=100<br>4. 超级管理员才能查询后台事件（`admin_*`）<br>5. 命中分区表时响应 <300ms | Done | 4h | Dod |

#### Web Admin

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 | UI设计文档 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|------------|
| **T-20013** | Web | User | 用户详情页"行为流"Tab [TDS](./tds/web/T-20013.md) | T-10015, T-20007 | 用户详情页新增 `EventStreamTab`：时间筛选（最近 1h/24h/7d/30d/自定义）+ event_name 多选下拉 + 时间线列表（事件名 + properties 折叠 JSON + 设备信息）；支持导出 CSV（当前筛选下前 1000 条） | 1. 默认加载最近 24h<br>2. event_name 下拉从后台枚举<br>3. 无数据空状态占位<br>4. CSV 下载文件名带 user_id 与时间戳<br>5. properties JSON 支持关键字高亮 | ✅ Done | 5h | Dod | 450 个测试全绿；EventTimelineItem覆盖率98.82%；R1 HIGH-1 XSS修复+HIGH-2 limit→100+MEDIUM-1 AbortController；R2 通过 |

#### Android

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 | UI设计文档 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|------------|
| **T-30034** | Android | Analytics | Analytics 防腐层 + Sentry 集成 [TDS](./tds/android/T-30034.md) | T-0000D | 新建 `core/analytics/`：`AnalyticsPort` 接口（`track(event, properties)` / `setUser(id)` / `captureException(e)`）；`impl/SentryAnalytics.kt` 包装 Sentry SDK（Self-Hosted 或 EU 区 DSN 从 BuildConfig 注入）；业务层一律通过 Hilt 注入 `AnalyticsPort`，严禁 import `io.sentry.*`；ANR/Native Crash 自动捕获；敏感字段 (手机号/JWT) 过滤器 | 1. 业务层代码中 `grep io.sentry` = 0<br>2. 模拟崩溃 Sentry Dashboard 能收到<br>3. DSN 由 `BuildConfig.SENTRY_DSN` 注入（dev/prod 区分）<br>4. 用户未同意时 Sentry 仍可用（合规豁免）<br>5. `captureException` 自动脱敏手机号 / token | ✅ Done | 6h | Dod | 无（纯基建） |
| **T-30035** | Android | Analytics | EventReportClient + 核心事件埋点 + 隐私弹窗 [TDS](./tds/android/T-30035.md) | T-30034, T-00022, T-00023, T-30002 | 新建 `core/analytics/EventReportClient`：本地 Room 队列（`event_queue`）+ 节流器（≥8 条 或 ≥2min flush）+ 优先 WS `ReportEvent`、离线时走 HTTP `POST /events/batch`；Splash 后弹出 `PrivacyConsentDialog`（同意/仅 Crash 二选一，仅 Crash 时非 Crash 事件不上报）；依照 [business_flows.md §2.9](../product/business_flows.md) 事件字典在现有 20+ 页面埋点；session_id/device_id/app_version/os_version/locale/network_type 公共字段自动注入 | 1. 队列 ≥1000 条时淘汰最旧事件<br>2. 断网 5min 后恢复，缓存事件全部上报成功<br>3. WS 在线时 WS 通道上报占比 >80%<br>4. 用户选"仅 Crash"后，Logcat 无 `track(` 实际上报日志<br>5. `Key('btn_privacy_agree')` / `Key('btn_privacy_crash_only')` 可点<br>6. 特殊字段 (手机号/JWT/精确地址) 绝不上报<br>7. 核心事件 (login_verify_success / gift_send_success / gift_send_fail / insufficient_balance_dialog_shown) 集成测试全覆盖 | ✅ Done | 10h | Dod | [T-30035.md](./design/android/T-30035.md) |

---

## Phase 1.5 Epic：E-10 房间主权与管理员体系

> **说明**：房主 + 管理员权限体系、观众席、创建房间升级（封面/分类/密码/公告）、踢人/禁麦/禁言/抱麦。**突破 MENA 合规下限**（女性房主保护）。详见 [phase1_room_governance.md](./product/phase1_room_governance.md)。
> **产品流程规范**: [business_flows.md §2.8](./product/business_flows.md)

### 模块 8: 房间主权与管理员体系 (E-10)

> **依赖关系图**:
> ```
> T-00024 (rooms扩字段+治理表+迁移) ─┬─► T-00025 (创建房间升级)
>                                   ├─► T-00026 (密码房校验)
>                                   ├─► T-00027 (观众席列表)
>                                   ├─► T-00028 (KickUser 信令+冷却)
>                                   ├─► T-00029 (MuteUser 信令+双重拦截)
>                                   └─► T-00030 (TransferAdmin/强制抱麦)
> T-00028 + T-00029 ─► T-10016 (Admin 治理日志查询) ─► T-20014 (Web 治理日志页)
> T-00025 ─► T-30036/T-30037 (创建房间/封面选择器)
> T-00026 ─► T-30038 (密码房进房弹窗)
> T-00027 ─► T-30039 (观众席) ─► T-30040 (用户操作菜单) ─► T-30041 (踢人原因弹窗)
> T-00028 + T-00029 ─► T-30042 (被踢/被禁弹窗)
> T-00025 + T-00030 ─► T-30043 (公告栏 + 管理员徽章)
> T-00029 + T-00030 ─► T-30044 (禁麦/禁言 UI 反馈 + 抱麦集成)
> ```

#### App Server

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-00024** | App Server | Room | rooms 表扩字段 + 治理审计表迁移 [TDS](./tds/server/T-00024.md) | T-0000B | 迁移脚本：rooms 增 `cover_url TEXT DEFAULT ''`, `category VARCHAR(32) DEFAULT 'chat'`, `password_hash VARCHAR(60) NULL`, `announcement TEXT NULL`, `admin_user_id UUID NULL REFERENCES users(id)`；新建 `room_kick_records(id, room_id, target_user_id, operator_user_id, reason, created_at)` + `room_mute_records(id, room_id, target_user_id, operator_user_id, type, duration_sec, reason, created_at)`；平译迁移保证线上存量房间自动默认值 | 1. 迁移幂等，允许回滚<br>2. category 种子数据含 6 类 (chat/emotion/music/game/matchmaking/other)<br>3. 两张治理表均有 (room_id, created_at) 索引<br>4. 存量房间 cover_url 缺省为空不影响旧逻辑 | Done | 4h | Dod |
| **T-00025** | App Server | Room | 创建房间 API 升级（封面/分类/密码/公告）[TDS](./tds/server/T-00025.md) | T-00024 | POST `/api/v1/rooms` 请求体新增 `cover_url, category, announcement, password`（可选）；密码 6 位数字 bcrypt hash 写入 `password_hash`；封面 URL 白名单校验（MVP 仅允许 8 张预设）；公告 ≤200 字；分类枚举校验；复用现有 unique 约束（用户限一个活跃房间）；新建 PATCH `/api/v1/rooms/:id` 仅房主可改 name/announcement/category；变更后广播 `RoomInfoUpdated` | 1. 密码非 6 位数字返回 400<br>2. 非白名单封面返回 400<br>3. 公告 >200 字返回 400<br>4. 已有活跃房返回 409<br>5. PATCH 非房主返回 403<br>6. RoomInfoUpdated 广播成功 | ✅ Done | 6h | Dod |
| **T-00026** | App Server | Room | 密码房进房校验 + 锁定机制 [TDS](./tds/server/T-00026.md) | T-00025 | POST `/api/v1/rooms/:id/verify-password { password }`：bcrypt 验证，正确签发 short-live token（JWT，TTL 60s，claim `room_access`）；错误计数写 Redis `pwd_fail:{user_id}:{room_id}` INCR，=5 时 SET 锁定 Key `pwd_lock:{user_id}:{room_id}` TTL 1800；WS JoinRoom 增读 password_token 逻辑：密码房必传 token，无 token 返回 `PASSWORD_REQUIRED` | 1. 正确密码返回 token 且能通过 WS JoinRoom<br>2. 错误 5 次后任何结果直接 401 + 锁定剩余秒数<br>3. 60s 后 token 失效返回 `TOKEN_EXPIRED`<br>4. 非密码房返回 400<br>5. 并发 5 次错误尝试仅触发一次锁定 | ✅ Done | 5h | Dod |
| **T-00027** | App Server | Room | 观众席列表 API（含角色标签）[TDS](./tds/server/T-00027.md) | T-00024, T-00016 | GET `/api/v1/rooms/:id/members?page=1&limit=20`：从 `RoomManager` 内存读成员 → 批量查 users 补头像/昵称 → 复合 `role` 字段（owner/admin/member）+ `mic_slot`（如在麦）+ `joined_at`；麦上用户置顶，观众按 joined_at 倒序 | 1. 100 人房间耗时 <150ms<br>2. 分页边界（page=0/超界）返回空数组<br>3. 角色优先级 owner > admin > member<br>4. 麦上用户始终置顶无论进房时间<br>5. role='admin' 仅在 admin_user_id 匹配时返回 | ✅ Done | 5h | Dod |
| **T-00028** | App Server | Governance | WS `KickUser` 信令 + 10min 冷却 [TDS](./tds/server/T-00028.md) | T-00024, T-00027 | WS 新增 `KickUser { room_id, target_user_id, reason }` 信令：强事务（1) 权限校验（操作者=owner或admin 且 target≠owner） 2) Redis SETEX `kicked:{room_id}:{user_id}` 600 reason 3) INSERT room_kick_records 4) RoomManager 移除 + 若在麦自动下麦 5) 广播 UserKicked仅给目标 + UserLeft 给房间 6) 广播 MicLeft 如在麦则额外）；JoinRoom 处先查冷却 Key返回 `KICKED_COOLDOWN { remaining_sec }` | 1. 非权限返回 `PERMISSION_DENIED`<br>2. 管理员踢房主返回 `CANNOT_KICK_OWNER`<br>3. 10min 内重进返回 `KICKED_COOLDOWN`<br>4. 踢麦上用户同步广播 MicLeft<br>5. room_kick_records 每次踢人均有一行<br>6. 并发 3 个管理员同时踢同一人仅成功一次 | Done | 7h | Dod |
| **T-00029** | App Server | Governance | WS `MuteUser`/`UnmuteUser` 信令 + 双重拦截 [TDS](./tds/server/T-00029.md) | T-00024, T-00027 | `MuteUser { room_id, target_user_id, type: 'mic'\|'chat', duration_sec }`：权限校验 + SETEX `{mic|chat}_muted:{room_id}:{user_id}` + INSERT room_mute_records + 若 type=mic 且在麦强制下麦；广播 UserMuted；`UnmuteUser` 对应删除键 + 广播 `UserMuted { duration_sec: 0 }`；**SendMessage 前置校验** chat_muted:*—命中则返回 CHAT_MUTED；**TakeMic 前置校验** mic_muted:* | 1. type=mic 时在麦用户自动下麦 + MicLeft 广播<br>2. 禁言用户 SendMessage 返回 CHAT_MUTED<br>3. 禁麦用户 TakeMic 返回 MIC_MUTED<br>4. duration_sec 到期后 Redis Key 自动过期（TTL 验证）<br>5. UnmuteUser 仅 owner/admin 可发<br>6. 送礼物不受禁麦/禁言影响（见权限矩阵） | ✅ Done | 6h | Dod |
| **T-00030** | App Server | Governance | WS `TransferAdmin` + `ForceTakeMic`/`ForceLeaveMic` [TDS](./tds/server/T-00030.md) | T-00024, T-00027 | `TransferAdmin { room_id, target_user_id, action: 'assign'\|'revoke' }`：仅房主可发，assign 时若已有管理员先隐式 revoke 旧管理员；UPDATE rooms.admin_user_id；广播 AdminChanged。`ForceTakeMic { room_id, target_user_id, slot_index }`：owner/admin 可发，校验目标非禁麦 + 麦位空闲 + 广播 MicTaken { forced_by }。`ForceLeaveMic { room_id, target_user_id }`：owner/admin 可发，广播 MicLeft { forced_by } | 1. 管理员试图 TransferAdmin 返回 PERMISSION_DENIED<br>2. TransferAdmin assign 新管理员时旧管理员自动卸任<br>3. ForceTakeMic 目标拒绝麦元权限后自动发 MicLeave（客户端负责）<br>4. ForceLeaveMic 非麦上返回 MIC_NOT_FOUND<br>5. AdminChanged 广播所有房间成员<br>6. 管理员不能卸任房主 | ✅ Done | 7h | Dod |

#### Admin Server

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|
| **T-10016** | Admin Server | Governance | 房间治理日志查询 API [TDS](./tds/adminServer/T-10016.md) | T-00028, T-00029, T-10012 | `GET /api/v1/admin/governance/logs?room_id=&target_user_id=&operator_user_id=&type=kick\|mute&from=&to=&page=&limit=`：UNION 查询 room_kick_records + room_mute_records，补齐用户昵称 / 房间名 / 类型标签返回；支持申诉导出 CSV | 1. 多条件索引命中耗时 <300ms<br>2. type=kick 返回不包含 mute 记录<br>3. operator 登记 admin_logs<br>4. max limit=100<br>5. CSV 导出正确编码 UTF-8 BOM | Done | 4h | DoD |

#### Web Admin

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 | UI设计文档 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|------------|
| **T-20014** | Web | Governance | 房间治理日志查询页 [TDS](./tds/web/T-20014.md) | T-10016, T-20007 | 新菜单 `/governance/logs`：筛选栏（房间/操作者/目标/类型/时间区间）+ 表格（时间/类型/房间/操作者/目标/原因/时长）+ CSV 导出 + 分页；用户详情页可跳转到此页预填目标 user_id | 1. 默认查最近 7 天<br>2. type 下拉 [全部/踢出/禁麦/禁言]<br>3. 操作者点击可跳转该管理员详情页<br>4. CSV 导出当前筛选结果<br>5. 空数据空状态占位 | Done | 5h | DoD | 32 tests passed |

#### Android

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 状态 | 预估工时 | 负责人 | UI设计文档 |
|---------|--------|------|----------|----------|----------|-------------|------|----------|--------|------------|
| **T-30036** | Android | Room | 创建房间表单升级（分类/公告/密码）[TDS](./tds/android/T-30036.md) | T-00025, T-30007 | `CreateRoomScreen` 重构：房名 + 封面选择器（嵌入 T-30037） + 分类下拉（6 项） + 公告 TextField（200 字计数） + 密码 Switch + 6 位数字密码输入框（Switch 开启时显示）；提交前本地校验 | 1. 未填房名按钮置灰<br>2. 密码 开关关闭时密码框隐藏且不提交 password<br>3. 密码非 6 位数字时提交按钮置灰<br>4. 公告计数 ≥200 置红<br>5. `Key('btn_submit_create_room')` 可测<br>6. 创建成功后自动 JoinRoom 进入 | Done | 7h | DoD | [T-30036.md](./design/android/T-30036.md) |
| **T-30037** | Android | Room | 房间封面选择器（8 张预设）[TDS](./tds/android/T-30037.md) | T-30036 | `CoverPickerBottomSheet`：3列网格展示 8 张中东风预设封面（沙漠/清真寺/烛灯/鹰/玫瑰/游艇/太阳/书法）；选中金色边框；首次进入默认第一张 | 1. 8 张预设归在内置 drawable<br>2. 选中状态有金色 2dp 边框<br>3. `Key('cover_option_0')`~`cover_option_7'` 可测<br>4. 确认后返回 cover_url 给父页 | Done | 3h | DoD | [T-30037.md](./design/android/T-30037.md) |
| **T-30038** | Android | Room | 密码房进房弹窗 [TDS](./tds/android/T-30038.md) | T-00026, T-30007 | 大厅房间卡片点击时如果 `has_password=true` 则弹 `PasswordInputDialog`（标题 + 6 位分格输入框 + 错误提示剧本）；验证成功拿 token 发 JoinRoom | 1. 6 位输完自动提交<br>2. 错误显示剩余次数红字剧本<br>3. 锁定时显示"30 分钟后重试"<br>4. `Key('password_input')` + `Key('btn_submit_password')`<br>5. 返回键关闭弹窗不进房 | Done | 3h | DoD | [T-30038.md](./design/android/T-30038.md) |
| **T-30039** | Android | Room | 观众席 Bottom Sheet [TDS](./tds/android/T-30039.md) | T-00027, T-30018 | `AudienceBottomSheet`：下拉展开占屏 70%；上方显示总人数；列表：头像 + 昵称 + 角色徽章 + 在线时长；麦上用户置顶分组标头"麦上"，观众组标头"观众 (N)"；LazyColumn 分页加载（每次 20）；点击用户弹出 T-30040 | 1. 空记录双空状态文案<br>2. 100 人房间滚动不卡顿（目测/帧率）<br>3. WS `UserJoined`/`UserLeft` 实时刷新列表<br>4. `Key('audience_sheet')` + `Key('audience_item_$userId')`<br>5. 麦上用户始终置顶 | Done | 8h | DoD | [T-30039.md](./design/android/T-30039.md) |
| **T-30040** | Android | Room | 用户操作菜单 BottomSheet（动态权限）[TDS](./tds/android/T-30040.md) | T-30039 | `UserActionBottomSheet`：根据（我角色, 目标角色）组合添减菜单项【抱上麦 / 禁麦 / 禁言 / 踢出 / 任命管理员 / 卸任 / 查看资料(占位) / 举报】；不可用项不渲染；权限矩阵严格对齐 [phase1_room_governance.md §2.3](../product/phase1_room_governance.md) | 1. 普通用户看到的仅 [查看资料 举报]<br>2. 房主看另一普通用户时有 5 项操作<br>3. 管理员看房主时仅显示 [查看资料 举报]<br>4. 什任管理员项点击后弹确认 Dialog<br>5. `Key('user_action_$actionType')` 可测 | Done | 6h | DoD | [T-30040.md](./design/android/T-30040.md) |
| **T-30041** | Android | Governance | 踢人原因选择弹窗 [TDS](./tds/android/T-30041.md) | T-30040, T-00028 | `KickReasonDialog`：4 预设原因单选按钮 [骚扰/刷屏/辱骂/其他] + 自定义 TextField（其他为必填）；确认后 WS KickUser；不可外部点击关闭 | 1. 默认选中"骚扰"<br>2. 选"其他"未填自定义确认按钮置灰<br>3. 成功后关闭 + Toast "已踢出"<br>4. 失败弹错误原因<br>5. `Key('kick_reason_$index')` + `Key('btn_confirm_kick')` | Done | 3h | DoD | [T-30041.md](./design/android/T-30041.md) |
| **T-30042** | Android | Governance | 被踢/被禁提示弹窗 [TDS](./tds/android/T-30042.md) | T-00028, T-00029 | 顶层 `UserKickedDialog`：收到 `UserKicked` 时全屏弹窗显示"你已被移出房间，原因：XXX，10 分钟后可再次进入" + [知道了] → 自动回大厅。`UserMutedDialog`：收到 `UserMuted` 时 Toast "你已被禁{麦\|言} N 分钟" + 底部 Chip 倒计时显示直到解除 | 1. UserKicked 弹窗 `Key('dialog_kicked')` 可见<br>2. 确认后大厅进房按钮灰色 10min（本地 countdown）<br>3. UserMuted Chip `Key('mute_countdown')` 倒计时准确<br>4. 解除广播后 Chip 自动消失<br>5. 多条 UserMuted 按最新一条覆盖倒计时 | Review | 4h | Review | [T-30042.md](./design/android/T-30042.md) |
| **T-30043** | Android | Room | 公告栏 + 管理员徽章 + RoomInfoUpdated [TDS](./tds/android/T-30043.md) | T-00025, T-00030, T-30018 | 进房后首次弹 `AnnouncementPopup`（有公告时）；房间顶部其它位置持续显示 📄 图标点击再弹；麦位/观众席/弹幕昵称旁渲染角色徽章 👑房主 / 🛡️管理员；WS `AdminChanged` / `RoomInfoUpdated` 实时刷新 | 1. 首次弹后 24h 内同一用户再进房不再自动弹<br>2. 公告为空时顶部无图标<br>3. 房主徽章金色王冠<br>4. 管理员徽章金色盾牌<br>5. AdminChanged 到达 500ms 内全局徽章刷新<br>6. `Key('announcement_popup')` + `Key('btn_show_announcement')` | In Progress | 5h | TDD | [T-30043.md](./design/android/T-30043.md) |
| **T-30044** | Android | Governance | 禁麦/禁言 UI 反馈 + 抱麦集成 [TDS](./tds/android/T-30044.md) | T-00029, T-00030, T-30042 | 自身被禁麦时麦位"+"按钮置灰且点击 Toast；自身被禁言时聊天输入框 enabled=false + 占位文本 "你已被禁言 N 分钟"；被 `ForceTakeMic` 时自动请求麦克风权限，成功则进麦，拒绝则自动发 MicLeave；被 `ForceLeaveMic` 时自动停止推流显示 Toast "你已被抱下麦" | 1. 禁麦用户点麦位"+" 无网络请求<br>2. 禁言用户输入框 disabled 且提交按钮置灰<br>3. 被抱上麦后如未授权麦克风，自动完成 MicLeave<br>4. 抱下麦后本地麦元状态同步为离线<br>5. `Key('chat_input')` disabled 可测 | In Progress | 6h | TDD | [T-30044.md](./design/android/T-30044.md) |

---
