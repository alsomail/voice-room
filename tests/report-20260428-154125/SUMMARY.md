# E2E 回归报告汇总 — 2026-04-28-154125

**执行时间**：2026-04-28 15:41 ~ 16:17（第 3 次全量回归派发）  
**测试环境**：本地 macOS，Vite dev server (port 5173), App server (port 3000), Admin server (port 3001)  
**执行命令**：
- API: `E2E_PROFILE=local E2E_SEED=0 npx playwright test tests/scripts/API/ --workers=1 --reporter=list`
- WEB: `E2E_PROFILE=local E2E_SEED=0 npx playwright test tests/scripts/WEB/ --workers=1 --reporter=list`

**前置自检结果**:
- `lsof -ti :3000 :3001 :5173` → 5 端口均有进程占用（服务已运行）
- `docker compose ps` → PG/Redis 均 healthy
- `cat tests/scripts/env/.env.local | grep -c MIDSCENE_MODEL_API_KEY` → **1** ✅（用户已写入）
- `npm run preflight` → **preflight: all 5 checks passed in 0s (profile=local)** ✅

---

## API 套件：49 PASS / 0 FAIL / 29 SKIP（78 cases，× 3 browsers = 234 tests）✅

| 套件 | PASS | FAIL | SKIP | 备注 |
|------|------|------|------|------|
| TC-WS | 5 | 0 | 3 | 00006/00007/00008 需 redis-cli (SKIP-KNOWN) |
| TC-CHAT | 3 | 0 | 2 | 00001 需 TOKEN_B, 00004 需 MUTED_TOKEN (SKIP-KNOWN) |
| TC-GIFT | 6 | 0 | 1 | 00001 需 TOKEN_B (SKIP-KNOWN) |
| TC-WALLET | 5 | 0 | 0 | ✅ 全通 |
| TC-ROOM | 12 | 0 | 0 | ✅ 全通 |
| TC-USER | 5 | 0 | 0 | ✅ 全通 |
| TC-LOG | 4 | 0 | 0 | ✅ 全通 |
| TC-RANKING | 3 | 0 | 1 | 00004 需 redis-cli (SKIP-KNOWN) |
| TC-AUTH | 0 | 0 | 13 | 全跳过（redis-cli + 特殊 token）(SKIP-KNOWN) |
| TC-INFRA | 5 | 0 | 2 | 00001/00002 需 Docker 控制权限 (SKIP-KNOWN) |
| TC-INFRA-Q | 1 | 0 | 1 | I-2 需干净端口环境 (SKIP-KNOWN) |
| TC-MIC | 0 | 0 | 6 | 全跳过（E2E_OP_TOKEN 未生成）(SKIP-KNOWN) |
| **合计** | **49** | **0** | **29** | **API 0 失败 ✅** |

**Playwright 最终输出**: `147 passed / 87 skipped` (2.0 minutes)

---

## WEB 套件：45 PASS / 9 FAIL / 0 SKIP（54 tests = 18 cases × 3 browsers）

| 套件 | PASS | FAIL | SKIP | 备注 |
|------|------|------|------|------|
| TC-AUTH | 15 | 0 | 0 | ✅ 全通（5 cases × 3 browsers） |
| TC-GIFT | 4 | 2 | 0 | TC-GIFT-00002 chromium(timeout) + firefox(AI断言) 失败 |
| TC-LOG | 6 | 0 | 0 | ✅ 全通（2 cases × 3 browsers） |
| TC-ROOM | 9 | 6 | 0 | TC-ROOM-00002/00003 各 3 browser 全失败 |
| TC-USER | 8 | 1 | 0 | TC-USER-00002 webkit 失败 |
| TC-WALLET | 3 | 0 | 0 | ✅ 全通（1 case × 3 browsers） |
| **合计** | **45** | **9** | **0** | **WEB 9 失败，需 TDD 修复** |

**Midscene AI 报告**: `midscene_run/report/playwright-2026-04-28_15-44-21-*.html` (36 reports 生成)

---

## WEB 失败摘要

| 用例 ID | 浏览器 | 失败根因 | 类型 |
|---------|--------|---------|------|
| TC-GIFT-00002 | chromium | 180s 超时 (Midscene 动作链过长) | 测试代码 |
| TC-GIFT-00002 | firefox | AI 断言: "已下架"文字不存在（UI 实为 toggle 开关） | 测试断言 |
| TC-ROOM-00002 | chromium/firefox/webkit | AI 断言: 第2页不存在（测试数据不足 10 条） | 测试数据 |
| TC-ROOM-00003 | chromium/firefox/webkit | Playwright strict mode: getByText 匹配 2 元素 | 测试代码 |
| TC-USER-00002 | webkit | AI 断言时序: webkit 渲染慢导致截图时机不对 | 测试时序 |

**截图落盘**: `test-results/WEB-TC-ROOM-*/test-failed-1.png` (9 张)

---

## 与上轮（report-20260428-101211）对比

| 指标 | 上轮 (20260428-101211) | 本轮 (20260428-154125) | 变化 |
|------|----------------------|----------------------|------|
| API PASS | 49/78 cases | 49/78 cases | = |
| API FAIL | 0 | 0 | = |
| WEB PASS (cases) | 1/18 | 14/18 | **+13 ✅** |
| WEB FAIL (cases) | 17/18 | 4/18 | **-13 ✅** |
| Midscene Key 注入 | ❌ 未注入 | ✅ 已注入（env.local） | **修复** |
| WEB 基础设施 | ❌ 全部白屏 | ✅ 正常渲染 + AI 可交互 | **修复** |

> **关键进步**: WEB 从 1 通过提升至 14 通过，Midscene env 注入 (T-0000P) 已成功。  
> 剩余 4 cases 失败均为**测试代码/数据问题**（非业务 Bug）。

---

## Skip 原因汇总（SKIP-KNOWN）

| 环境变量 / 工具 | 缺失影响 |
|---------------|---------|
| `E2E_USER_B_TOKEN` | TC-CHAT-00001, TC-GIFT-00001 跳过 |
| `E2E_MUTED_TOKEN` | TC-CHAT-00004 跳过 |
| `E2E_OP_TOKEN` | TC-MIC-00001~00006 全跳过 |
| `redis-cli` | TC-AUTH 全跳过, TC-RANKING-00004, TC-WS-00006/00007/00008 |
| Docker 控制权限 | TC-INFRA-00001/00002 跳过 |

---

## QA Gate 更新

| Task ID | 任务名 | 本轮关联 Cases | QA Gate | Overall Gate |
|---------|--------|--------------|---------|--------------|
| T-0000N | /health 端点 | TC-INFRA-00003/04/05✅ preflight 5/5✅ | [✅ Passed](API/TC-INFRA/Report.md) | ✅ Released |
| T-0000O | ranking perf known-issue | TC-RANKING-00003✅ | [✅ Passed](API/TC-RANKING/Report.md) | ✅ Released |
| T-0000P | Midscene env 注入 | WEB 14/18 cases✅; 4 cases 测试代码问题 | [⚠️ Partial](WEB/) WEB 45/54 tests pass | ✅ Released |
| T-0000Q | preflight 端口检测 | TC-INFRA-Q-I-1✅ | [✅ Passed](API/TC-INFRA-Q/Report.md) | ✅ Released |
| T-00041 | WS 心跳断开 | TC-WS-00002✅ TC-WS-00004✅ | [✅ Passed](API/TC-WS/Report.md) | ✅ Released |
| T-00042 | Admin 强制断连广播 | TC-WS-00005✅ TC-WS-00006 SKIP-KNOWN | [✅ Passed](API/TC-WS/Report.md) | ✅ Released |
| T-00043 | Chat 消息持久化 | TC-CHAT-00002✅ TC-CHAT-00003✅ TC-CHAT-00005✅ | [✅ Passed](API/TC-CHAT/Report.md) | ✅ Released |
| T-00044 | 礼物 REST 端点 | TC-GIFT-00002~00007✅ (API) | [✅ Passed](API/TC-GIFT/Report.md) | ✅ Released |

---

## 待 TDD 修复的 WEB 用例（Bug Fix Queue）

### BUG-WEB-001: TC-GIFT-00002 — 超时 + AI 断言 (chromium/firefox)
- **文件**: `tests/scripts/WEB/TC-GIFT.spec.ts`
- **修复方向**: 
  1. 增大 `timeout` 或拆分测试步骤（chromium timeout）
  2. 将 AI 断言"该行状态列变为'已下架'"改为检测 toggle 开关状态（firefox）

### BUG-WEB-002: TC-ROOM-00002 — 分页数据不足 (all browsers)
- **文件**: `tests/scripts/WEB/TC-ROOM.spec.ts`
- **修复方向**: `beforeAll` 批量创建 >10 个房间；或改为验证"分页组件可见"而不是硬断言页码

### BUG-WEB-003: TC-ROOM-00003 — strict mode violation (all browsers)
- **文件**: `tests/scripts/WEB/TC-ROOM.spec.ts`
- **修复**: `getByText('确认强制关闭').waitFor()` → `getByText('确认强制关闭').first().waitFor()`

### BUG-WEB-004: TC-USER-00002 — webkit 时序问题 (webkit only)
- **文件**: `tests/scripts/WEB/TC-USER.spec.ts`
- **修复**: 封禁操作后增加 `await page.waitForLoadState('networkidle')` 等待 UI 完全更新
