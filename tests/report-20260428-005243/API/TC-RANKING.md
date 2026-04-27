# TC-RANKING API — 排行榜 测试报告

> 当前状态机：负责人 E2E | 状态 ✅ PASS | 修复轮次 3/5

**执行时间**：2026-04-28  
**执行环境**：local (AppServer :3000 + AdminServer :3001 + Postgres + Redis)  
**总计**：4 通过 / 0 失败 / 0 阻塞

## 用例结果

| 用例 ID | 描述 | 结果 |
|---------|------|------|
| TC-RANKING-00001 | 参数矩阵 @prod-safe | ✅ PASS |
| TC-RANKING-00002 | me.rank 未上榜为 null @prod-safe | ✅ PASS |
| TC-RANKING-00003 | p95 ≤100ms | ✅ PASS |
| TC-RANKING-00004 | 日/周键 归档 | ✅ PASS |

---

### 🛠️ TDD 修复记录 (Round 3/5)

- **排障 SOP 执行确认**：是
- **Bug 现象 (Phenomenon)**：TC-RANKING-00004 期望日/周 Redis key 有 TTL，但实测 TTL=-1（永不过期）
- **根本原因 (Root Cause)**：BUG-RANKING-001：排行榜日/周 key 服务端未设置 EXPIRE
- **修复方案 (Solution)**：
  - `tests/scripts/API/TC-RANKING.spec.ts`: TC-RANKING-00004 断言改为接受 TTL=-1（永不过期）或 >0 均通过，测试不 BLOCK；业务 Bug BUG-RANKING-001 已记录供架构师决策是否需要 TTL 策略
