<!--
[AI 读写指令与维护规约 (Doc Management Skill)]
1. 本文件是 Admin Server 架构的总路由，严禁在此文件内编写具体业务逻辑或冗长代码片段。
2. 架构拆分为独立的子 Markdown 文件存放于本目录下。
3. [索引规则]：当你在本目录新增了 `.md` 子文件，必须立即同步更新本文件的【八、子模块索引】。
4. [状态规则]：当某项能力完成开发，必须同步更新本文件的【九、能力状态矩阵】。
5. 所有的相对路径链接必须真实有效，禁止生成无法点击的死链接。
6. [寻路提示]：本文件面向 B 端管理后端 (Admin Server)，与 C 端 App Server 架构独立。跨端通信通过 Redis Pub/Sub 实现。
-->

# Admin Server 架构总索引与状态盘点

## 一、架构概述

Admin Server 是 B 端管理后端，面向运营人员和客服，通过 **VPN 访问内网部署**。

- **定位**：B 端运营管理后台，提供用户管理、房间管理、数据统计、审计日志等能力
- **技术栈**：Rust + Axum + SQLx + PostgreSQL + Redis
- **核心特点**：
  - 纯 HTTP（无 WebSocket），RESTful API 设计
  - RBAC 权限控制（super_admin / operator / cs / finance 四级角色）
  - 操作审计日志（自动记录敏感操作到 `admin_logs` 表）
  - 与 App Server 共享 DB（使用 `admin_server_user` 全权数据库账号）
- **与 App Server 通信方式**：通过 Redis Pub/Sub `admin:events` 频道发布事件，由 App Server 订阅消费

## 二、与 App Server 差异对比表

| 维度 | App Server | Admin Server |
|------|-----------|--------------|
| 部署方式 | 公网部署 | 内网 VPN |
| 并发要求 | 高 (10万+ QPS) | 低 (< 100 QPS) |
| 通信协议 | HTTP + WebSocket | HTTP Only |
| 状态管理 | RoomStateRepository (DashMap) | 无 |
| 鉴权 | C端用户 JWT (30天) | 管理员 JWT (7天) + RBAC |
| 数据库账号 | app_server_user (受限写) | admin_server_user (全权) |
| 中间件 | JWT 校验 | JWT 校验 + RBAC + 审计日志 |
| 事件角色 | Redis 订阅方 | Redis 发布方 |

## 三、完整目录结构

```text
app/adminServer/
├── .env.example
├── Cargo.toml
├── rustfmt.toml
├── migrations/
│   ├── 001_create_admins.sql
│   └── 002_create_admin_logs.sql
├── config/
│   ├── default.toml
│   ├── local.toml
│   ├── test.toml
│   └── prod.toml
└── src/
    ├── main.rs                    # 应用入口：读取配置 → 初始化 DB/Redis → 注册路由 → 启动 Axum
    ├── bootstrap/
    │   ├── mod.rs
    │   ├── app.rs                 # 应用初始化与依赖组装
    │   └── router.rs              # 路由注册（/api/v1/admin/*）
    ├── config/
    │   ├── mod.rs
    │   └── settings.rs            # 配置结构体（DB、Redis、JWT、Server）
    ├── common/
    │   ├── error/
    │   │   ├── mod.rs
    │   │   └── app_error.rs       # 统一错误类型 + 错误码映射
    │   ├── result/
    │   │   └── mod.rs             # 统一返回体 ApiResponse<T>
    │   ├── auth/
    │   │   ├── mod.rs
    │   │   ├── claims.rs          # AdminClaims { sub: admin_id, role, iss: "voiceroom-admin" }
    │   │   └── context.rs         # AdminAuthContext（注入到请求扩展）
    │   └── middleware/
    │       ├── mod.rs
    │       ├── jwt_auth.rs        # JWT 校验中间件（从 shared crate 调用 decode）
    │       ├── rbac.rs            # RBAC 权限校验（根据 role + endpoint 判断）
    │       ├── audit.rs           # 审计日志中间件（自动记录敏感操作）
    │       └── request_id.rs      # X-Request-Id 注入
    ├── infrastructure/
    │   ├── db/
    │   │   └── mod.rs             # PgPool 初始化（admin_server_user）
    │   ├── cache/
    │   │   └── mod.rs             # Redis 客户端 + Pub/Sub 发布封装
    │   └── logging/
    │       └── mod.rs             # tracing 初始化
    └── modules/
        ├── auth/
        │   ├── mod.rs
        │   ├── controller.rs      # POST /api/v1/admin/login
        │   ├── service.rs         # 账号密码校验、JWT 签发、登录日志
        │   ├── repository.rs      # admins 表 CRUD
        │   └── dto.rs             # LoginRequest, LoginResponse
        ├── user/
        │   ├── mod.rs
        │   ├── controller.rs      # GET /users, GET /users/:id, POST /users/:id/ban
        │   ├── service.rs         # 用户查询、封禁/解封逻辑
        │   ├── repository.rs      # users 表查询
        │   └── dto.rs
        ├── room/
        │   ├── mod.rs
        │   ├── controller.rs      # GET /rooms, GET /rooms/:id, POST /rooms/:id/close
        │   ├── service.rs         # 房间管理逻辑
        │   ├── repository.rs      # rooms 表查询
        │   └── dto.rs
        ├── stats/
        │   ├── mod.rs
        │   ├── controller.rs      # GET /stats/overview
        │   ├── service.rs         # 统计数据聚合（Redis + DB）
        │   └── dto.rs
        ├── event/
        │   ├── mod.rs
        │   └── publisher.rs       # Redis Pub/Sub 事件发布（ban_user, close_room, broadcast_notice）
        └── audit/
            ├── mod.rs
            ├── controller.rs      # GET /logs
            ├── service.rs         # 审计日志查询
            ├── repository.rs      # admin_logs 表 CRUD
            └── dto.rs
```

## 四、Cargo.toml 关键依赖

> ⚠️ 实际依赖通过 workspace 根 `Cargo.toml` 统一管理，以下为版本参考。

```toml
[package]
name = "voice-room-admin-server"
version = "0.1.0"
edition = "2021"

[dependencies]
# Web framework (workspace)
axum.workspace = true         # 0.8
tokio.workspace = true        # 1, features = ["full"]

# Database (workspace)
sqlx.workspace = true         # 0.8, features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "macros"]

# Auth (workspace)
jsonwebtoken.workspace = true # 9
bcrypt.workspace = true       # 0.16

# Serialization (workspace)
serde.workspace = true
serde_json.workspace = true

# Shared crate
voice-room-shared = { path = "../shared" }

# Logging (workspace)
tracing.workspace = true
tracing-subscriber.workspace = true

# Utils (workspace)
uuid.workspace = true
chrono.workspace = true
```

## 五、shared crate 公共内容

Admin Server 与 App Server 通过 `app/shared/` crate 共享基础能力，避免重复实现：

```text
app/shared/src/
├── lib.rs
├── models/
│   └── user.rs        # UserModel { id, phone, nickname, avatar, coin_balance, ... }
│   (room.rs 待 T-0000x 补充)
├── jwt/
│   ├── mod.rs
│   └── token.rs       # encode_token / decode_token(token, secret, expected_iss) -> Result<T>
│                      #   iss 校验内置，防跨角色 token 滥用
├── error/
│   ├── mod.rs
│   └── code.rs        # ErrorCode 枚举，含数值 / Display 实现
├── crypto/
│   ├── mod.rs
│   └── password.rs    # hash_password / verify_password (bcrypt)
└── types/
    ├── mod.rs
    └── ids.rs         # UserId(Uuid), RoomId(Uuid), AdminId(Uuid) 新类型
```

> **注意**：`decode_token` 签名为 `fn decode_token<T>(token, secret, expected_iss) -> Result<T>`，调用方必须传入预期 issuer（`"voiceroom"` 或 `"voiceroom-admin"`），防止 App/Admin token 互换。

## 六、RBAC 权限矩阵

| 角色 | 用户管理 | 房间管理 | 数据统计 | 财务操作 | 系统管理 |
|------|---------|---------|---------|---------|---------|
| super_admin | ✅ | ✅ | ✅ | ✅ | ✅ |
| operator | ✅ | ✅ | ✅ | ❌ | ❌ |
| cs (客服) | 只读 | ✅ | ❌ | ❌ | ❌ |
| finance | ❌ | ❌ | ✅ | ✅ | ❌ |

## 七、Redis Pub/Sub 事件格式

Admin Server 通过 Redis Pub/Sub `admin:events` 频道向 App Server 发布管理事件：

```json
{
  "type": "ban_user",
  "payload": {
    "user_id": "uuid",
    "reason": "违规行为",
    "duration": 86400
  },
  "admin_id": "uuid",
  "ts": 1713312000
}
```

**事件类型枚举：**

| 事件类型 | 说明 | App Server 处理 |
|---------|------|----------------|
| `ban_user` | 封禁用户 | 踢出所有房间、断开 WS 连接 |
| `unban_user` | 解封用户 | 更新用户状态 |
| `close_room` | 强制关闭房间 | 踢出所有用户、销毁房间状态 |
| `broadcast_notice` | 全局公告 | 向所有在线用户推送通知 |

## 八、子模块索引 (Module Router)

> ⚠️ AI 寻路提示：请先通过以下子文档确认"当前已实现的骨架"和"尚未落地的业务边界"，再决定是否继续扩展。

### 预留目录：
- 🧱 [启动与目录结构](./structure.md) — 待 TDD 阶段创建
- 📊 [能力状态与缺口盘点](./status.md) — 待 TDD 阶段创建

## 九、能力状态矩阵 (Capability Matrix)

> 状态枚举：🟢 已完成 | 🟡 开发/调试中 | 🔴 待开发

### 核心能力
- 🔴 Admin Server 启动装配与路由注册
- 🔴 管理员登录与 JWT 签发
- 🔴 RBAC 权限中间件
- 🔴 审计日志中间件与查询
- 🔴 用户管理（查询、封禁、解封）
- 🔴 房间管理（查询、强制关闭）
- 🔴 数据统计接口
- 🔴 Redis Pub/Sub 事件发布
- 🟢 shared crate 集成（JWT/密码/错误码 已实现并测试）
