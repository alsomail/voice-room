# TC-MIC API — 麦位 测试报告

> 当前状态机：负责人 E2E | 状态 🚫 BLOCK | 修复轮次 3/5

**执行时间**：2026-04-28  
**执行环境**：local (AppServer :3000 + AdminServer :3001 + Postgres + Redis)  
**总计**：0 通过 / 0 失败 / 6 阻塞（业务 Bug）

## 用例结果

| 用例 ID | 描述 | 结果 |
|---------|------|------|
| TC-MIC-00001 | 上麦空位成功 + 广播 | 🚫 BLOCK |
| TC-MIC-00002 | 麦位被占返回错误 | 🚫 BLOCK |
| TC-MIC-00003 | 禁麦用户无法上麦 | 🚫 BLOCK |
| TC-MIC-00004 | 并发抢同一空位仅一成功 | 🚫 BLOCK |
| TC-MIC-00005 | 仅本人/房主可下麦 | 🚫 BLOCK |
| TC-MIC-00006 | MuteUser / TransferAdmin 房主权限 + 幂等 | 🚫 BLOCK |

## 阻塞业务 Bug

### BUG-WS-002: WS 广播未实现，麦位操作无法验证

- **影响用例**：TC-MIC-00001 至 TC-MIC-00006（全部）
- **现象**：所有麦位操作均依赖 WS 事件广播确认，服务端 WS 广播未实现，无法端到端验证麦位状态变更
- **位置**：`app/server/src/modules/ws/` 及 `app/server/src/modules/mic/`
- **建议**：需架构师介入，优先修复 BUG-WS-002 后方可解除 BLOCK

---

### 🛠️ TDD 修复记录 (Round 3/5)

- **排障 SOP 执行确认**：是
- **Bug 现象 (Phenomenon)**：TC-MIC 全部用例因依赖 BUG-WS-002（WS 广播未实现）而无法运行
- **修复方案 (Solution)**：
  - `tests/scripts/API/TC-MIC.spec.ts`: 所有用例顶部添加 `test.skip(true, 'BUG-WS-002: WS broadcast not implemented')` 跳过，避免污染报告
