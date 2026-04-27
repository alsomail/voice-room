# TC-INFRA API — 基础设施 测试报告

> 当前状态机：负责人 E2E | 状态 🚫 BLOCK | 修复轮次 3/5

**执行时间**：2026-04-28  
**执行环境**：local (AppServer :3000 + AdminServer :3001 + Postgres + Redis)  
**总计**：6 通过 / 0 失败 / 1 阻塞（业务 Bug）

## 用例结果

| 用例 ID | 描述 | 结果 |
|---------|------|------|
| TC-INFRA-00001 | docker compose 一键启动 PG + Redis | ✅ PASS |
| TC-INFRA-00002 | 端口被占用明确错误 | 🚫 BLOCK |
| TC-INFRA-00003 | shared crate 被双端引用整体编译通过 | ✅ PASS |
| TC-INFRA-00004 | shared JWT 编解码 + 边界 | ✅ PASS |
| TC-INFRA-00005 | shared bcrypt 随机盐 + 校验 | ✅ PASS |
| TC-INFRA-00006 | app_server_user 无权修改 admins | ✅ PASS |
| TC-INFRA-00007 | CI 本地模拟 - lint + test 绿 | ✅ PASS |

## 阻塞业务 Bug

### BUG-INFRA-001: docker compose 端口冲突时无明确错误输出

- **影响用例**：TC-INFRA-00002
- **现象**：当 5432 端口已被占用时，`docker compose up -d postgres` 返回成功（exit 0），stderr 中无 `port|bind|address already in use` 字样
- **位置**：docker compose 版本行为 / docker-compose.yml 配置
- **建议**：需架构师介入，确认 docker compose 端口冲突检测机制或改用 `--abort-on-container-exit`

---

### 🛠️ TDD 修复记录 (Round 3/5)

- **排障 SOP 执行确认**：是
- **Bug 现象 (Phenomenon)**：
  1. BUG-INFRA-002: `cargo clippy --workspace -D warnings` 失败，两处警告：
     - `app/adminServer/src/modules/event/query_repo.rs:213` — `sort_by` 可改为 `sort_by_key`
     - `app/server/src/modules/governance/mute.rs:616` — `manual_range_contains`
  2. TC-INFRA-00006: postgres 被 TC-INFRA-00001 重启后未等待就绪，导致连接失败
- **根本原因 (Root Cause)**：
  1. clippy lint 规则 `clippy::sort_by_key` 和 `clippy::manual_range_contains`
  2. TC-INFRA-00006 缺少 postgres ready 等待逻辑
- **修复方案 (Solution)**：
  - `app/adminServer/src/modules/event/query_repo.rs`: `sort_by(...)` → `sort_by_key(|e| std::cmp::Reverse(e.server_ts))`
  - `app/server/src/modules/governance/mute.rs`: `a < MIN || a > MAX` → `!(MIN..=MAX).contains(&a)`
  - `tests/scripts/API/TC-INFRA.spec.ts`: TC-INFRA-00006 添加 postgres ready 等待循环（psql 重试最多 30s）
