# 3. Monorepo 目录结构与 DDD 设计

下述为目标目录结构，即使当前仓库尚未完全创建，也必须按该结构落地。  
**Server 端严格采用 Package by Feature 的模块化设计，避免全局服务和领域层混乱；四端都必须为基建目录预留明确位置。**

```text
/
├── doc/                                  # 架构、协议、排障与运维文档
│   ├── architecture/                     # 系统总架构（本目录）
│   ├── protocol/                         # HTTP/WS API 契约、错误码、数据模型（唯一 API 契约源）
│   ├── DEBUG_SOP.md                      # AI/开发者通用调试与故障定位方法论
│   ├── arch/                             # 各端详细架构文档
│   │   ├── server/index.md               # App Server 架构入口
│   │   ├── adminServer/index.md          # Admin Server 架构入口
│   │   ├── web/index.md                  # Web Admin 架构入口
│   │   └── android/index.md              # Android 架构入口
│   └── tds/                              # 技术设计方案（按端分目录）
│       ├── _template.md                  # TDS 模板
│       ├── server/                       # App Server TDS
│       ├── adminServer/                  # Admin Server TDS
│       ├── web/                          # Web Admin TDS
│       └── android/                      # Android TDS
│
├── shared/                               # 跨端共享资源（非代码）
│   ├── localization/                     # 多语言 key 与文案规范
│   │   ├── ar/                           # 阿拉伯语文案
│   │   ├── en/                           # 英语文案
│   │   └── key-conventions.md            # i18n key 命名约定
│   └── assets/                           # 跨端可复用静态资源说明或规范
│
├── app/
│   ├── server/
│   │   ├── .env.example                  # 服务端环境变量模板，禁止提交真实密钥
│   │   ├── .env                          # 本地开发环境变量，仅本地存在
│   │   ├── Cargo.toml                    # Rust 依赖声明
│   │   ├── rustfmt.toml                  # Rust 格式化规则
│   │   ├── .sqlx/                        # SQLx 离线编译缓存目录，供 CI 使用
│   │   ├── migrations/                   # 数据库迁移脚本
│   │   ├── config/                       # 分环境配置文件目录
│   │   │   ├── default.toml              # 默认配置
│   │   │   ├── local.toml                # 本地开发配置
│   │   │   ├── test.toml                 # 测试环境配置
│   │   │   └── prod.toml                 # 生产环境配置
│   │   └── src/
│   │       ├── main.rs                   # 应用入口
│   │       ├── bootstrap/                # 启动装配、依赖注入、路由注册
│   │       ├── config/                   # 配置读取、环境识别、密钥装载
│   │       ├── common/                   # 全局通用代码
│   │       │   ├── error/                # 全局错误定义与错误码映射
│   │       │   ├── result/               # 统一返回体封装
│   │       │   ├── auth/                 # AuthContext、Claims、鉴权基础类型
│   │       │   ├── middleware/           # HTTP/WS 中间件
│   │       │   ├── tracing/              # tracing 初始化与字段注入
│   │       │   ├── telemetry/            # 日志字段、trace_id、request_id 基础工具
│   │       │   ├── types/                # 跨模块基础类型
│   │       │   └── utils/                # 通用工具函数
│   │       ├── infrastructure/           # 全局基建与防腐层落地目录
│   │       │   ├── db/                   # PostgreSQL 连接池、事务工厂、仓储基础设施
│   │       │   ├── cache/                # DashMap / Redis 客户端与状态缓存封装
│   │       │   ├── logging/              # 日志落地、输出格式、采样策略
│   │       │   ├── telemetry/            # 观测基建、链路追踪、指标打点
│   │       │   ├── messaging/            # 站内消息、推送、广播总线抽象
│   │       │   ├── gateway/              # 对外网关或内部服务网关适配
│   │       │   ├── storage/              # 对象存储、文件上传、媒体资源元数据
│   │       │   ├── security/             # 签名、加密、限流、风控基础组件
│   │       │   ├── config_provider/      # 远程配置/配置中心适配层（预留）
│   │       │   └── third_party/          # 第三方服务适配层
│   │       │       ├── rtc/              # Agora 等 RTC Provider 适配
│   │       │       ├── im/               # IM / 消息服务适配
│   │       │       ├── analytics/        # 埋点/日志上报服务端转发或聚合适配
│   │       │       ├── crash/            # 崩溃/告警平台服务端适配（预留）
│   │       │       ├── moderation/       # 审核/风控/内容安全适配
│   │       │       ├── sms/              # 短信服务适配
│   │       │       ├── payment/          # 支付服务适配
│   │       │       └── cdn/              # CDN / 边缘加速配置适配
│   │       └── modules/                  # 业务模块目录，严格 Package by Feature
│   │           ├── auth/                 # 登录、刷新、会话、设备绑定
│   │           ├── user/                 # 用户资料、等级、在线状态
│   │           ├── room/                 # 房间生命周期与成员管理
│   │           ├── seat/                 # 麦位状态与上/下麦逻辑
│   │           ├── wallet/               # 钱包、余额、冻结、流水
│   │           ├── gift/                 # 礼物定义、送礼、广播
│   │           ├── billing/              # 收益、分成、结算、账单
│   │           ├── rtc/                  # RTC Token、频道映射、媒体会话
│   │           ├── moderation/           # 风控、封禁、踢人、敏感词
│   │           ├── notification/         # 站内通知与系统消息
│   │           ├── family/               # 家族系统（未来扩展）
│   │           ├── cp/                   # CP 关系（未来扩展）
│   │           ├── vip/                  # 贵族/VIP 特权（未来扩展）
│   │           ├── backpack/             # 背包、道具、资产（未来扩展）
│   │           ├── game/                 # 小游戏接入（未来扩展）
│   │           └── _reserved/            # 预留扩展模块目录
│   │
│   ├── adminServer/
│   │   ├── .env.example                  # Admin Server 环境变量模板
│   │   ├── Cargo.toml                    # Rust 依赖声明（引用 shared crate）
│   │   ├── rustfmt.toml                  # Rust 格式化规则
│   │   ├── migrations/                   # Admin 专属 migration（如 admins 表、admin_logs 表）
│   │   ├── config/                       # 分环境配置
│   │   │   ├── default.toml
│   │   │   ├── local.toml
│   │   │   ├── test.toml
│   │   │   └── prod.toml
│   │   └── src/
│   │       ├── main.rs                   # 应用入口
│   │       ├── bootstrap/                # 启动装配、路由注册
│   │       ├── config/                   # 配置读取
│   │       ├── common/                   # 通用代码
│   │       │   ├── error/                # 错误定义与错误码
│   │       │   ├── result/               # 统一返回体
│   │       │   ├── auth/                 # Admin AuthContext、Claims
│   │       │   └── middleware/           # RBAC 中间件、审计日志中间件
│   │       ├── infrastructure/           # 基建层
│   │       │   ├── db/                   # PostgreSQL 连接池（admin_server_user 全权）
│   │       │   ├── cache/                # Redis 客户端（Pub/Sub 发布端）
│   │       │   └── logging/              # 日志
│   │       └── modules/                  # 业务模块
│   │           ├── auth/                 # 管理员登录、JWT 签发
│   │           ├── user/                 # 用户查询、封禁/解封
│   │           ├── room/                 # 房间查询、强制关闭
│   │           ├── stats/                # 数据统计
│   │           ├── event/                # Redis Pub/Sub 事件发布
│   │           └── audit/                # 操作审计日志
│   │
│   ├── shared/                           # Rust workspace 共享 crate
│   │   ├── Cargo.toml                    # 共享依赖声明
│   │   └── src/
│   │       ├── lib.rs                    # crate 根
│   │       ├── models/                   # 共享数据模型（UserModel, RoomModel 等）
│   │       ├── jwt/                      # JWT encode/decode 工具
│   │       ├── error/                    # 公共错误码定义
│   │       ├── crypto/                   # bcrypt 密码工具
│   │       └── types/                    # 公共类型（UserId, RoomId 等）
│   │
│   ├── android/
│   │   ├── build.gradle.kts              # Android 顶层构建配置
│   │   ├── gradle.properties             # Gradle 构建参数
│   │   ├── local.properties              # 本地私密配置，不可提交
│   │   ├── .editorconfig                 # Kotlin/XML 风格规范
│   │   └── app/
│   │       ├── build.gradle.kts          # App 模块构建；定义 productFlavors
│   │       └── src/main/
│   │           ├── java/com/example/.../
│   │           │   ├── core/             # 全局基建与平台能力
│   │           │   │   ├── network/      # Retrofit、拦截器、Token 刷新、环境切换
│   │           │   │   ├── ws/           # WS 客户端、心跳、重连、消息分发
│   │           │   │   ├── telemetry/    # 埋点、日志缓冲、崩溃上报、防腐层
│   │           │   │   ├── media/        # RTC 防腐层（IMediaService）
│   │           │   │   ├── im/           # IM 防腐层（IIMService）
│   │           │   │   ├── config/       # BuildConfig、环境识别、远程配置
│   │           │   │   ├── i18n/         # 多语言与 RTL 支持
│   │           │   │   ├── storage/      # 本地缓存、DataStore、文件存储
│   │           │   │   ├── security/     # 签名、加密、设备标识、安全存储
│   │           │   │   └── logging/      # 本地日志、采样、调试辅助
│   │           │   ├── common/           # 公共 UI、Result、State、Base 类
│   │           │   ├── data/             # DTO、RemoteDataSource、LocalDataSource、RepositoryImpl
│   │           │   ├── domain/           # 领域模型、仓储接口、UseCase
│   │           │   ├── presentation/     # BaseActivity/BaseFragment、导航、通用状态管理
│   │           │   └── feature/          # 业务模块页面
│   │           │       ├── auth/         # 登录注册
│   │           │       ├── room/         # 房间页
│   │           │       ├── seat/         # 麦位与申请上麦
│   │           │       ├── gift/         # 礼物与送礼
│   │           │       ├── wallet/       # 钱包
│   │           │       ├── profile/      # 用户资料
│   │           │       ├── family/       # 家族（预留）
│   │           │       ├── cp/           # CP（预留）
│   │           │       ├── vip/          # 贵族（预留）
│   │           │       ├── backpack/     # 背包（预留）
│   │           │       └── game/         # 小游戏（预留）
│   │           └── res/
│   │               ├── layout/           # Compose 布局（兼容传统 XML）
│   │               ├── values/           # 默认文案、主题、尺寸
│   │               ├── values-ar/        # 阿拉伯语文案
│   │               ├── drawable/         # 图片与形状资源
│   │               └── xml/              # networkSecurityConfig 等 XML 配置
│   │
│   └── web/                              # Admin Web 后台管理系统
│       ├── package.json                  # 前端依赖与脚本
│       ├── vite.config.ts                # Vite 配置与本地代理
│       ├── tsconfig.json                 # TypeScript 配置
│       ├── .env.example                  # 前端环境变量模板
│       ├── .env.development              # 开发环境变量
│       ├── .env.production               # 生产环境变量
│       ├── .eslintrc.cjs                 # ESLint 规则
│       ├── .prettierrc                   # Prettier 规则
│       └── src/
│           ├── app/                      # App 根组件、Provider、Router、Store
│           ├── core/                     # 全局基建层
│           │   ├── network/              # Axios/fetch 封装、拦截器、环境切换
│           │   ├── i18n/                 # 多语言（中/英）
│           │   ├── config/               # 环境变量读取
│           │   ├── security/             # Token 管理、Admin JWT 存取
│           │   └── constants/            # 常量定义
│           ├── api/                      # HTTP API 定义层（仅对接 Admin Server）
│           ├── hooks/                    # 复用 Hook
│           ├── components/               # 通用 UI 组件（基于 Ant Design）
│           ├── features/                 # 业务功能组件
│           ├── pages/                    # 路由页面（登录、仪表盘、用户管理等）
│           ├── styles/                   # 全局样式与主题
│           ├── types/                    # 类型声明
│           ├── lib/                      # 辅助库与纯函数
│           └── assets/                   # 静态资源
│
├── scripts/                              # 工程化脚本目录
│   ├── dev/                              # 本地开发脚本，如启动依赖服务
│   ├── ci/                               # CI 校验脚本
│   ├── release/                          # 发布脚本
│   └── ops/                              # 运维与诊断脚本
│
├── .github/
│   └── workflows/                        # GitHub Actions 流水线定义
│
├── .gitignore                            # 忽略敏感配置与构建产物
├── .editorconfig                         # 跨项目基础缩进与编码规范
└── README.md                             # 仓库导航与本地启动说明
```

## 3.1 目录总原则

1. **基建代码必须有固定归属目录**，禁止埋在业务模块内部。
2. **第三方 SDK/Provider 只能出现在 infrastructure、core 或 services 中**，禁止直接进入 UI 与 Domain。
3. **shared 只放稳定契约，不放端内实现代码**。
4. **scripts 必须按用途拆分**，避免一个脚本做所有事情。
5. **环境配置必须模板化**，真实密钥不得提交仓库。

## 3.2 数据库共享策略（双服务共库迁移隔离规约）

> 决议：**共享 DB（`voiceroom`）+ 表/Schema 权限隔离**，物理双库延迟到 SaaS / 多租户阶段。
> 决策细节见 [ADR-0001](../adr/ADR-0001-migration-table-isolation.md)。

### 3.2.1 共享与隔离的边界

| 资产 | 策略 | 说明 |
|------|------|------|
| 业务表（users / rooms / wallets / events / ...） | **共享**，AppServer 主权 | AdminServer 通过受限读写访问，禁止 DDL |
| 迁移源（`*/migrations/*.sql`） | **物理分离** | 各服务在自身 crate 下管理，互不引用 |
| 迁移登记表 | **逻辑分离**（关键约束） | AppServer = `_sqlx_app_migrations`；AdminServer = `_sqlx_admin_migrations` |
| DB 角色 | 双账号 | `app_server_user`（受限） / `admin_server_user`（schema 全权） |

### 3.2.2 为何必须分迁移登记表

sqlx 默认所有迁移都登记到同一张 `_sqlx_migrations`。一旦双服务共库：

1. version 重号：双方 `001..N` 互相冲突，hash 校验必失败。
2. 「missing migration」误报：sqlx 把对方的版本视为本端的「已应用但缺失文件」，启动直接拒绝。
3. 表所有权抢占：先启动方建表并独占 owner，对方 `INSERT` 触发 `permission denied`。

### 3.2.3 实现规约（强制）

- **每个 server 进程必须使用专属登记表名**，通过 `voice_room_shared::migrate::run_migrations_with_table` 注入：
  - `app/server/src/main.rs` → `_sqlx_app_migrations`
  - `app/adminServer/src/main.rs` → `_sqlx_admin_migrations`
  - 集成测试统一走 `app/server/tests/common/mod.rs::run_migrations()`
- **新增进程接入共享库时**，必须分配新的 `_sqlx_<service>_migrations` 表名并在本节登记。
- **不得**在 main.rs 中直接调用 `sqlx::migrate!(...).run(&pool)`（会回退默认 `_sqlx_migrations`，破坏隔离）。
- **dev/docker-compose** 起的 PG 必须由 `scripts/dev/init-db.sh` 给 `app_server_user` 授予 `CREATE ON SCHEMA public`，否则它无法建立自有登记表（详见 N-2）。
- **staging/prod** 切「migrate-on-deploy」（ADR-0001 阶段 C）后，运行账号无需 `CREATE` 权限。

### 3.2.4 物理双库的触发条件（保留）

任一信号出现即评估切换至物理双库 + RPC：
1. AdminServer 跨域查询 < 3 处（耦合度可控的反向）。
2. C / B 端数据安全审计要求物理隔离。
3. 多租户 SaaS 化。

