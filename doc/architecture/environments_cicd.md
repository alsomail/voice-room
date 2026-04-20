# 15. 多环境配置与 CI/CD 基建规范 (Environments & DevOps)

本 Monorepo 必须支持从本地开发到测试再到生产的平滑切换，严禁在代码中硬编码域名、数据库地址、第三方 AppID。

## 15.1 环境划分

1. **Dev (Local)**：本地开发环境
2. **Test (QA)**：测试/联调环境
3. **Prod (Production)**：正式生产环境

## 15.2 各端配置文件规范

### Server
- 加载顺序：`.env` -> `config/default.toml` -> `config/{env}.toml`
- 敏感信息：
  - `DATABASE_URL`
  - `JWT_SECRET`
  - `AGORA_APP_CERT`
- 只能来自环境变量或密钥管理系统

### Admin Server
- 加载顺序：`.env` -> `config/default.toml` -> `config/{env}.toml`
- 与 App Server 共享 `JWT_SECRET`（通过 shared crate 统一签发/校验逻辑）
- 独有敏感信息：
  - `ADMIN_DATABASE_URL`（使用 admin_server_user 全权账号）
  - `ADMIN_JWT_EXPIRY`（默认 7 天）
  - `REDIS_URL`（用于 Pub/Sub 事件发布）
- 只能来自环境变量或密钥管理系统

### Web
- 使用 `.env.development` / `.env.production`
- 所有变量必须以 `VITE_` 开头
- 例如：
  - `VITE_API_BASE_URL`
  - `VITE_WS_URL`
  - `VITE_ANALYTICS_ENDPOINT`

### Android
- 私密配置放 `local.properties`
- 使用 Gradle `productFlavors` 区分 `dev`、`test`、`prod`
- 通过 `BuildConfig` 注入：
  - `API_URL`
  - `WS_URL`
  - `ANALYTICS_HOST`
  - `RTC_APP_ID`

## 15.3 CI/CD 流水线

### CI
- PR 触发：
  - Rust 编译与 clippy
  - Web lint + typecheck
  - Android lint / detekt
- Rust 必须使用 `.sqlx` 离线缓存校验

### CD
- 合并 `main` 后：
  - Web 构建并发布静态资源
  - Android 打包测试版并分发
  - Server 构建 Docker 镜像并推送仓库

## 15.4 Gateway / Proxy 规范

### 本地开发
- Web 通过 Vite Proxy 解决跨域
- Android 直连本地 Rust 服务或开发机局域网 IP

### 测试/生产
- 使用 Nginx / API Gateway：
  - 终结 HTTPS
  - 转发 `/api/`
  - 转发 `/ws/` 并保留 Upgrade 头

---

# 16. 实施红线与下一步

## 16.1 实施红线

1. 任何核心状态不得由客户端本地拍板。
2. 任何资金变更必须落数据库强事务。
3. 任何第三方 SDK 不得直接进入 UI 和 Domain 核心层。
4. 任何跨模块调用不得绕过 Service / Facade。
5. 任何状态变更消息必须带 `msg_id` 并可幂等。
6. 任何 WS 连接必须绑定用户、设备、房间和会话。
7. 任何错误必须纳入统一错误码体系。
8. 任何新业务模块必须按 bounded context 独立建模。
9. 任何客户端界面必须从一开始支持 i18n 与 RTL。
10. 任何日志与崩溃上报必须通过防腐层。
11. 任何环境地址、AppID、密钥不得写死在代码中。
12. SQLx 必须支持离线宏编译。
13. 高频日志必须限流或降级为 DEBUG。
14. 埋点与崩溃上报必须支持后续更换服务商。

## 16.2 首批落地优先级

### P0
- Auth
- User
- Room
- Seat
- RTC ACL
- Wallet
- Gift
- Billing
- WS Gateway
- RoomStateRepository
- Logging / Tracing
- Analytics / Crash 基础设施
- I18n / RTL 基础设施
- Dev/Test/Prod 环境切换基础设施
- Admin Server 基础（Auth + RBAC + 审计日志）
- Admin Web 登录与路由守卫
- shared crate（JWT、Error Codes、DB Models）
- Redis（验证码存储 + Pub/Sub 跨服务通信）

### P1
- Moderation
- Notification
- Transaction Outbox
- Redis RoomStateRepository
- Remote Config
- Edge Gateway / CDN 策略完善

### P2
- Family
- CP
- VIP
- Backpack
- Mini Game

---

# 17. 结论

本架构文档从整体分层、DDD 域拆分、四端目录设计、接口规范、WebSocket 信令、商业化事务、防腐层、弱网恢复、可观测性、中东本土化等维度给出了完整的技术指导。所有后续开发必须以此为准绳，任何偏离需走 ADR 变更流程。
