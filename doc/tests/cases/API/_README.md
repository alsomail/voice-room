# API/ 目录性质声明

> **🛡️ 此目录非"黑盒 E2E"** — 详见 [_README.md §0.1](../_README.md#01-目录性质重分类重要4-个目录性质不同不可混淆)。

## 目录性质

`doc/tests/cases/API/` 下的所有用例为**契约/集成测试套件**（contracts / integration tests），通过直接构造 HTTP 请求、WebSocket 帧、Redis Pub/Sub 消息验证后端协议字段、错误码、状态机、并发与幂等。

## 与黑盒 E2E 的边界

| 维度 | 本目录（API/） | 黑盒 E2E（AND/、WEB/、E2E/） |
|------|--------------|--------------------------|
| 触发方式 | 直接打 HTTP/WS/Redis | 真实 Android UI 点击 / Web 浏览器交互 |
| 验证对象 | 协议字段、错误码、并发、幂等 | 用户视角的业务闭环、UI 反馈、跨端联动 |
| 维护方 | server / adminServer 后端团队 | test-design / e2e-runner |
| 调度入口 | `cargo test` / 后端集成测试 | `npm run e2e:android` / `npm run e2e:local` |
| 是否计入 E2E 回归矩阵 | ❌ 否（不被 qa-coordinator 调度） | ✅ 是 |

## 维护规约

1. **冻结新增**：自 2026-05-07 铁律 8 落盘起，**禁止**在本目录新增文件；新增协议契约用例请直接落入对应端的集成测试目录：
   - `app/server/tests/`
   - `app/adminServer/tests/`
   - `app/web/src/**/__tests__/`
2. **存量保留**：现有 14 个文件保留作为协议契约的事实源与文档，不做物理迁移；存量用例若需修改，应同步迁出到上述集成测试目录。
3. **新协议黑盒覆盖**：任何新协议字段的"端到端业务可见性"验证必须在 `AND/`、`WEB/`、`E2E/` 中以**用户操作**为入口落锚。

## 现有文件清单

| 文件 | 当前性质 |
|------|---------|
| TC-AUTH.md | App Server / Admin Server 鉴权契约 |
| TC-ROOM.md | 房间 REST + WS 契约 |
| TC-WS.md | WS 握手 / 心跳 / 重连 / Pub-Sub 契约 |
| TC-MIC.md | 麦位 WS 契约（并发 / 权限） |
| TC-CHAT.md | 聊天 WS 契约 |
| TC-GIFT.md | 送礼事务原子性契约 |
| TC-WALLET.md | 钱包余额 / 流水 / Admin 调整契约 |
| TC-USER.md | Admin 用户管理契约 |
| TC-LOG.md | 操作日志查询契约 |
| TC-RANKING.md | 榜单接口契约 |
| TC-GOVERNANCE.md | 治理日志接口契约 |
| TC-ANALYTICS.md | 行为流接口契约 |
| TC-INFRA.md | 模块 0 工程基建契约 |
| TC-INFRA-E2E.md | 模块 9 E2E 测试基建契约 |
