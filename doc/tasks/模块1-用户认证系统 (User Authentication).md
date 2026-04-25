# 模块 1: 用户认证系统 (User Authentication)

> 返回 [任务总索引](./index.md)

## Phase 0: MVP 基础设施 (预计 6-8 周)


## 模块 1: 用户认证系统 (User Authentication)

#### App Server 端 (C 端业务后端)

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-00001** | App Server | Auth | 数据库表设计 [TDS](../tds/server/T-00001.md) | **T-0000B, T-0000C** | 设计 `users` 表（id, phone, nickname, avatar, coin_balance, created_at 等） | 1. SQLx migration 文件可执行<br>2. users 表 phone 字段唯一索引<br>3. 包含 coin_balance, vip_level 等字段<br>4. 支持软删除 | 3 | Dod | ✅ Done | - | - | ⏳ Pending |
| **T-00002** | App Server | Auth | 短信验证码发送接口 [TDS](../tds/server/T-00002.md) | T-00001 | POST `/api/v1/auth/send-code`，接入 Twilio，Redis 限流 | 1. 同一手机号 60 秒内只能发送 1 次<br>2. 验证码 6 位数字，Redis 存储，有效期 5 分钟<br>3. 失败重试机制<br>4. 返回 429 当频率超限 | 4 | Dod | ✅ Done | - | - | ⏳ Pending |
| **T-00003** | App Server | Auth | 手机号一键登录接口 [TDS](../tds/server/T-00003.md) | T-00002 | POST `/api/v1/auth/login`，校验验证码，新用户自动注册 | 1. 验证码错误/过期返回 401<br>2. 新用户自动创建记录（默认昵称"用户XXX"）<br>3. 成功返回 JWT (有效期 30 天) + 用户信息<br>4. 基于 msg_id 幂等 | 4 | Dod | ✅ Done | - | - | ⏳ Pending |
| **T-00004** | App Server | Auth | JWT 中间件 [TDS](../tds/server/T-00004.md) | T-00003 | Axum 中间件，校验 JWT 并注入 user_id | 1. 无/非法/过期 token 返回 401<br>2. 合法 token 注入 user_id 到上下文 | 3 | Dod | ✅ Done | - | - | ⏳ Pending |
| **T-00005** | App Server | Auth | 获取用户信息接口 [TDS](../tds/server/T-00005.md) | T-00004 | GET `/api/v1/users/me` | 1. 需要 JWT 认证<br>2. 返回完整用户信息（不含敏感字段） | 2 | Dod | ✅ Done | - | - | ⏳ Pending |

#### Admin Server 端 (B 端管理后端)

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-10001** | Admin Server | Auth | 管理员表设计 [TDS](../tds/adminServer/T-10001.md) | T-00001 | 设计 `admins` 表（id, username, password_hash, role, created_at） | 1. username 唯一索引<br>2. password_hash 使用 bcrypt<br>3. role 字段（super_admin, operator, cs, finance） | 2 | Dod | ✅ Done | - | - | ⏳ Pending |
| **T-10002** | Admin Server | Auth | 管理员登录接口 [TDS](../tds/adminServer/T-10002.md) | T-10001 | POST `/api/v1/admin/login`，账号密码登录 | 1. 账号不存在/密码错误返回 401<br>2. 成功返回 JWT (有效期 7 天，含 admin_id, role)<br>3. 记录登录日志（IP、时间） | 3 | Dod | ✅ Done | - | - | ⏳ Pending |
| **T-10003** | Admin Server | Auth | 管理员 JWT 中间件 [TDS](../tds/adminServer/T-10003.md) | T-10002 | Axum 中间件 + RBAC 权限校验 | 1. 校验 JWT 有效性<br>2. 注入 admin_id 和 role<br>3. 根据 role 校验接口权限 | 4 | Dod | ✅ Done | - | - | ⏳ Pending |

#### Web 端 (后台管理前端)

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-20001** | Web | Auth | 管理员登录页 UI [TDS](../tds/web/T-20001.md) | T-10002 | Ant Design 实现账号密码登录页 | 1. 账号/密码输入框<br>2. 记住密码（localStorage）<br>3. 登录失败提示<br>4. 中英文支持 | 4 | Dod | ✅ Done | - | - | ⏳ Pending |
| **T-20002** | Web | Auth | 登录逻辑与路由守卫 [TDS](../tds/web/T-20002.md) | T-10002, T-20001 | 调用登录接口，保存 JWT，实现路由鉴权 | 1. 成功跳转数据看板<br>2. 未登录自动跳转登录页<br>3. token 过期自动退出 | 3 | Dod | ✅ Done | - | - | ⏳ Pending |

#### Android 端 (C 端用户应用)

| Task ID | 归属端 | 模块 | 任务名称 | 前置依赖 | 核心描述 | TDD 验收标准 | 预估工时 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|--------|------|----------|----------|----------|-------------|----------|------------|----------|---------------------|------------------|----------------------|
| **T-30001** | Android | Auth | 登录页 UI (Compose) [TDS](../tds/android/T-30001.md) | T-00002 | Material3 实现手机号+验证码登录 | 1. 手机号输入框（+966 沙特格式）<br>2. 发送验证码倒计时<br>3. RTL 布局支持 | 5 | Dod | ✅ Done | - | - | ⏳ Pending |
| **T-30002** | Android | Auth | 登录 ViewModel [TDS](../tds/android/T-30002.md) | T-00003, T-30001 | Retrofit 调用登录接口 | 1. Loading/Success/Error 状态<br>2. token 保存到 DataStore<br>3. 登录成功导航到大厅 | 4 | Dod | ✅ Done | - | - | ⏳ Pending |
| **T-30003** | Android | Auth | JWT 拦截器 [TDS](../tds/android/T-30003.md) | T-00004, T-30002 | OkHttp 拦截器自动添加 token | 1. 每个请求自动带 Authorization Header<br>2. 401 响应自动跳转登录页 | 3 | Dod | ✅ Done | - | - | ⏳ Pending |
| **T-30004** | Android | Auth | 用户信息 Repository [TDS](../tds/android/T-30004.md) | T-00005, T-30003 | 封装用户信息获取与缓存 | 1. 首次登录拉取用户信息<br>2. Room Database 本地缓存<br>3. Flow 订阅用户信息变更 | 4 | Dod | ✅ Done | - | - | ⏳ Pending |

---
