<!--
[AI 读写指令与维护规约 (Doc Management Skill)]
1. 本文件是 Server 架构的总路由，严禁在此文件内编写具体业务逻辑或冗长代码片段。
2. 架构拆分为独立的子 Markdown 文件存放于本目录下。
3. [索引规则]：当你在本目录新增了 `.md` 子文件，必须立即同步更新本文件的【二、子模块索引】。
4. [状态规则]：当某项能力完成开发，必须同步更新本文件的【三、当前能力全景与状态】。
5. 所有的相对路径链接必须真实有效，禁止生成无法点击的死链接。
-->

# Server 端架构总索引与状态盘点

## 一、 架构概述
Server 端基于 Rust + Axum 构建。启动骨架（配置、日志、健康检查）已完成；Auth 业务域（短信验证码、手机号登录、JWT 鉴权、用户信息）已全部落地并通过 Review；数据库（SQLx 0.8 + PostgreSQL）与 Redis 已接入运行链路；Room 业务域数据层（`rooms` 表 DDL + `RoomModel` struct，T-00006）已完成；**创建房间接口**（`POST /api/v1/rooms`，T-00007）已落地（含 JWT 鉴权、参数校验、bcrypt 密码哈希、唯一 active 房间约束，60 个单元测试全通过）；**房间列表接口**（`GET /api/v1/rooms`，T-00008）已落地（分页热度排序、无鉴权，78 个测试全通过）；WebSocket 网关与房间详情/关闭接口仍未展开。

## 二、 子模块索引 (Module Router)
> ⚠️ AI 寻路提示：请先通过以下子文档确认“当前已实现的骨架”和“尚未落地的业务边界”，再决定是否继续扩展。

### 实际目录：
- 🧱 [启动、配置与目录结构](./structure.md) - `main.rs`、`bootstrap`、`config`、`logging`、数据库 / Redis 初始化与测试入口现状。
- 📊 [能力状态与缺口盘点](./status.md) - 现有可用能力、未落地模块与下一步约束。
- 🔐 [Auth 模块架构](./auth.md) - 短信验证码（T-00002）、手机号登录（T-00003）、JWT 中间件（T-00004）、获取用户信息（T-00005）的路由、服务、Redis Key 设计与错误码映射。
- 🗄️ [数据库 Schema 设计](./database.md) - 各业务表 DDL 说明、字段约束、索引策略与 Rust 模型映射（含 `rooms` 表，T-00006）。

## 三、 当前能力全景与状态 (Capability Matrix)
> 状态枚举：🟢 已完成 | 🟡 开发/调试中 | 🔴 待开发

### 核心能力
- 🟢 Server 启动装配、优雅停机与 Axum 路由注册
- 🟢 `GET /ping` 健康检查、JSON 响应与 `x-request-id`
- 🟢 tracing 初始化、请求级 span 与访问日志字段注入
- 🟢 `app/shared` crate 集成（JWT encode/decode + iss 校验、bcrypt 密码工具、公共错误码）
- 🟢 配置分层读取（`.env` + `config/*.toml` + 环境变量覆盖）
- 🟢 数据库连接池（SQLx 0.8 + PostgreSQL）与自动 migration（`sqlx::migrate!`）
- 🟢 Redis 连接（`MultiplexedConnection` 缓存复用）
- 🟢 **Auth 模块**：`POST /api/v1/auth/verification-codes`（T-00002）、`POST /api/v1/auth/login`（T-00003）、JWT 鉴权中间件（T-00004）、`GET /api/v1/users/me`（T-00005）
- 🟢 SMS 防腐层（`SmsProvider` trait）：生产用 Twilio，开发/CI 用 Mock
- 🟢 统一错误响应结构（含 `request_id`、`safe_message` 防信息泄露）
- 🟢 **数据层 — rooms 表**（T-00006）：`002_create_rooms.sql` DDL（6 个 CHECK 约束、3 个索引含软删除偏滤）+ `RoomModel` struct（29 个单元测试全通过）
- 🟢 **房间创建接口**（T-00007）：`POST /api/v1/rooms`（JWT 鉴权、标题校验、唯一 active 房间约束、bcrypt 密码哈希、HTTP 201 响应）；`003_add_unique_active_room_per_owner.sql` 唯一偏滤索引 + 60 个单元测试全通过
- 🟢 **房间列表接口**（T-00008）：`GET /api/v1/rooms`（公开无鉴权、分页、按 `member_count DESC, created_at DESC` 热度排序、过滤已关闭房间、含房主信息 JOIN）；78 个单元测试全通过
- 🔴 WebSocket 网关与服务端广播
- 🔴 房间详情/关闭接口（T-00009 ～ T-00010）、支付业务域

### 遗留技术债 (Tech Debt)
- `is_in_cooldown` / `daily_count` 两个 `SmsCodeStore` 方法当前仅供测试辅助调用，生产代码路径未使用，后续迭代可酌情清理。
- `service.rs` 中 `revoke_code` 失败时静默丢弃（`.ok()`），建议后续改为 `tracing::warn!` 记录（TDS 第五轮 Review L-01）。
- `.env.example` 中 `JWT_SECRET`、`REDIS_URL`、Twilio 相关变量需在部署文档中补充说明。
