# TC-GIFT API — 礼物 测试报告

> 当前状态机：负责人 E2E | 状态 🚫 BLOCK | 修复轮次 3/5

**执行时间**：2026-04-28  
**执行环境**：local (AppServer :3000 + AdminServer :3001 + Postgres + Redis)  
**总计**：2 通过 / 0 失败 / 5 阻塞（业务 Bug）

## 用例结果

| 用例 ID | 描述 | 结果 |
|---------|------|------|
| TC-GIFT-00001 | 礼物列表 排序 + 缓存 + Accept-Language | ✅ PASS |
| TC-GIFT-00002 | SendGift 原子事务 + WS 推送 | 🚫 BLOCK |
| TC-GIFT-00003 | 余额不足 40290 + 回滚 | 🚫 BLOCK |
| TC-GIFT-00004 | 接收者离麦/不存在 40403 | 🚫 BLOCK |
| TC-GIFT-00005 | msg_id 幂等 + 并发不超卖 | 🚫 BLOCK |
| TC-GIFT-00006 | count 边界 0/1/99/100 | 🚫 BLOCK |
| TC-GIFT-00007 | Admin 礼物 CRUD + 软删 + 审计 | ✅ PASS |

## 阻塞业务 Bug

### BUG-GIFT-001: POST /api/v1/gifts/send 仅限 WS 通道

- **影响用例**：TC-GIFT-00002 至 TC-GIFT-00006
- **现象**：HTTP REST 端点 `/api/v1/gifts/send` 不存在或不可用；礼物发送仅通过 WebSocket 消息完成
- **位置**：`app/server/src/modules/gift/` — 礼物发送 HTTP handler
- **建议**：需架构师介入确认是否实现 REST 发礼物接口，或测试用例改为纯 WS 流程

---

### 🛠️ TDD 修复记录 (Round 3/5)

- **排障 SOP 执行确认**：是
- **Bug 现象 (Phenomenon)**：
  1. `gifts.data` 字段结构：API 返回 `data.items` 而非 `data`（数组）
  2. TC-GIFT-00007 在全套件运行时偶发失败：TC-INFRA-00001 重启 Docker postgres 导致 psql 连接被拒
- **根本原因 (Root Cause)**：
  1. 测试脚本直接解构 `data` 为数组，实际响应为 `{data:{items:[...]}}`
  2. TC-GIFT-00007 的 psql cleanup 与 TC-INFRA-00001 的 docker restart 存在时序竞争
- **修复方案 (Solution)**：
  - `tests/scripts/API/TC-GIFT.spec.ts`: 修正 `data.items` 解构；TC-GIFT-00007 的 psql cleanup 改为带重试的 `psqlSafe()` 包装函数（sleep 1s retry × 10）；添加 `test.describe.configure({ mode: 'serial' })`；BUG-GIFT-001 相关用例添加 skip
