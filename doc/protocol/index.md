# API 协议文档索引

> 🔒 **字段级冻结 (Field Freeze) — T-00100**
> 本目录自 T-00100 任务完成后进入**字段级冻结**状态。所有 WS 信令、HTTP REST DTO 和 Redis Pub/Sub 事件的字段定义以 `schemas/` 下的 JSON Schema 2020-12 文件为唯一机器可读契约。
> 任何字段变更（新增/改名/删除）须先修改对应 schema 文件并通过 CI 验证，再同步更新 markdown 文档和实现代码。

---

> **原始文件**: `doc/protocol.md`（已拆分为本目录下的子文件）
> **版本**: v0.9
> **拆分日期**: 2026-04-20
> **维护约束**: 新增/修改接口时必须同步更新对应子文件；前后端联调前必须以本目录文档为唯一契约源。

---

## 📐 机器可读 Schema 索引 (schemas/)

| 目录 | 内容 | 文件数 |
|------|------|--------|
| [schemas/ws/](schemas/ws/) | WebSocket 信令 JSON Schema 2020-12 | 34 个（28 核心 + 6 附加 Result 类型） |
| [schemas/http/](schemas/http/) | HTTP REST DTO JSON Schema 2020-12 | RoomDetail 等 |
| [schemas/pubsub/](schemas/pubsub/) | Redis Pub/Sub 事件 JSON Schema 2020-12 | 4 个 admin:events |

### WS Schema 速查 (ws/)
`Ping` · `Pong` · `JoinRoom` · `JoinRoomResult` · `LeaveRoom` · `LeaveRoomResult` · `TakeMic` · `TakeMicResult` · `LeaveMic` · `LeaveMicResult` · `SendMessage` · `SendMessageResult` · `SendGift` · `SendGiftResult` · `ReportEvent` · `EventReportAck` · `KickUser` · `MuteUser` · `UnmuteUser` · `TransferAdmin` · `ForceTakeMic` · `ForceLeaveMic` · `UserJoined` · `UserLeft` · `UserKicked` · `MicTaken` · `MicLeft` · `RoomMessage` · `UserMuted`

### Pub/Sub Schema 速查 (pubsub/)
`BanUser` · `UnbanUser` · `CloseRoom` · `BroadcastNotice`  — channel: `admin:events`

---

## 📑 子文件索引

| # | 文件 | 内容概要 | 原章节 |
|---|------|---------|--------|
| 0 | [conventions.md](conventions.md) | 基础地址、请求头、统一响应、错误码、分页、幂等策略 | §一 |
| 1 | [auth_api.md](auth_api.md) | 验证码发送、手机号登录、获取用户信息 | §二 |
| 2 | [room_api.md](room_api.md) | 创建房间、房间列表、房间详情、关闭房间 | §三 |
| 3 | [admin_api.md](admin_api.md) | Admin 登录、RBAC 权限矩阵、Admin 房间管理 | §四 |
| 4 | [rtc_api.md](rtc_api.md) | RTC Token 签发（预留） | §五 |
| 5 | [websocket_signals.md](websocket_signals.md) | WebSocket 信令格式（预留） | §六 |
| 6 | [data_models.md](data_models.md) | users 表、Redis 验证码存储、admins 表、admin_logs 表 | §七 |
| 7 | [providers.md](providers.md) | SMS Provider、RTC Provider 配置模型 | §八 |
| 8 | [ranking_api.md](ranking_api.md) | 魅力/财富榜单查询、Top N + 当前用户排名、奖牌字段、时区切换 | §九 |

---

## 🔗 关联文档

- **系统架构**: [doc/architecture/index.md](../architecture/index.md)
- **产品需求**: [doc/product/index.md](../product/index.md)
- **任务看板**: [doc/tasks/index.md](../tasks/index.md)
- **各端实现架构**: `doc/arch/{server,adminServer,android,web}/index.md`

---

## 🔧 协议治理工具

- [协议路径绑定审计脚本](../arch/infra/protocol-binding-audit.md) — 自动验证 TDS 绑定表 ↔ server 实现 ↔ client 调用三角对账

---

## 📝 文档变更历史

- 2026-04-17: 初始版本，定义模块1认证契约 + RTC/WS 预留
- 2026-04-17: v0.2 — 删除 register 端点改为一步登录；验证码存储从 PG 改 Redis；新增 Admin Server 认证契约（§四）；新增 admins/admin_logs 表；users 表增加 coin_balance/vip_level
- 2026-04-19: v0.4 — 新增 §三 3.2 `GET /api/v1/rooms` 接口定义
- 2026-04-20: v0.5 — 新增 §3.3 获取房间详情（T-00009）
- 2026-04-21: v0.6 — 新增 §3.4 关闭房间（T-00010），新增错误码 40301/40901
- 2026-04-22: v0.7 — 新增 §4.4 Admin 房间列表接口（T-10004）
- 2026-04-23: v0.8 — 新增 §4.5 Admin 房间详情接口（T-10005）
- 2026-04-24: v0.9 — 新增 §4.6 Admin 强制关闭房间（T-10006）
- 2026-04-20: 拆分为子文件结构
- 2026-04-22: v1.0 — 新增 §九 ranking_api.md 榜单查询接口（T-00021），包含 GET /api/v1/ranking 完整协议、time zone 切换说明、Redis key 格式
