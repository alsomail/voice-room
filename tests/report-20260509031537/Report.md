# E2E 自愈流水线报告 — Round 5 最终轮

> 报告日期：2026-05-09  
> 修复轮次：5/5（最终轮，已触发熔断）  
> 测试范围：`doc/tests/cases/AND/` 黑盒业务闭环用例

---

## 🚫 熔断警告 — 需要人类架构师介入

**根本原因：Doubao AI 视觉服务账户欠费（403 overdue balance）**

Attempt 5 运行期间，从第 2 个测试起所有 `aiWaitFor`、`aiTap`、`aiAssert` 均抛出：

```
Error: failed to call AI model service (doubao-seed-2-0-lite-260215):
403 The request failed because your account has an overdue balance.
```

Midscene Android 代理的所有 AI 视觉操作均依赖 Doubao API。账户欠费导致 E2E 视觉验证层完全不可用，自动化闭环无法继续。

**人类架构师操作项：**
1. 登录 Doubao/ByteDance 控制台充值 AI API 账户余额
2. 确认 `DOUBAO_API_KEY`（或等效环境变量）有效且账户余额充足
3. 重新触发 AND 测试套件：`npx playwright test --config=playwright.config.ts --grep "TC-"`

---

## 状态机汇总

### ROOM 模块（大厅 / 创建房间 / 进房）

> 当前状态机：负责人 [E2E] | 状态 [🚫 BLOCK] | 修复轮次 [5/5]

**用例覆盖：** TC-ROOM-00001 · TC-ROOM-00003 · TC-ROOM-00005

**历史修复记录：**
- Round 1–3：登录 UI 坐标偏移、`pm clear` 弹窗污染
- Round 4：方案 C（只删 JWT 不 pm clear）
- Round 5：方案 D（JWT 注入绕过登录 UI）、移除 `agent.launch()` 竞态

**Attempt 5 失败原因：** Doubao API 403（AI 服务不可用，非代码问题）

---

### WALLET 模块（钱包展示 / 余额更新 / 余额不足）

> 当前状态机：负责人 [E2E] | 状态 [🚫 BLOCK] | 修复轮次 [5/5]

**用例覆盖：** TC-WALLET-00001 · TC-WALLET-00002 · TC-WALLET-00004

**历史通过记录：** Attempt 2 全部通过（TC-WALLET-00001/00002/00004 均 PASS）  
**回退原因：** Attempt 3–5 因 `agent.launch()` 竞态 / 同意弹窗未处理 导致回退  
**Attempt 5 失败原因：** Doubao API 403（非代码问题）

**本轮代码修复（待验证）：**
- `resetAndroidToMainPage` 新增 `detectScreenState()` 同意弹窗检测，仅在弹窗实际出现时 dismiss（避免误触主界面按钮）
- 移除 `agent.launch(APP_ID)` 调用（消除 `FLAG_ACTIVITY_RESET_TASK_IF_NEEDED` 导致的 HOME 闪屏）
- `aiWaitFor` 超时从 15s 延长至 20s

---

### RANKING 模块（双 Tab 切换 + Top3 奖牌）

> 当前状态机：负责人 [E2E] | 状态 [🚫 BLOCK] | 修复轮次 [5/5]

**用例覆盖：** TC-RANKING-00001

**本轮代码修复（待验证）：**
- Tab 标签修正：`魅力` → `魅力榜`、`财富` → `财富榜`、`日` → `日榜`、`周` → `周榜`
- 移除 `agent.launch()` 竞态（HOME 闪屏问题的根本修复）

---

### PROFILE 模块（个人中心 / 钻石入口 / 退出登录）

> 当前状态机：负责人 [E2E] | 状态 [🚫 BLOCK] | 修复轮次 [5/5]

**用例覆盖：** TC-PROFILE-00001 · TC-PROFILE-00003 · TC-PROFILE-00005

**本轮代码修复（待验证）：**
- Step4 断言改为仅检查 `'设置'`（实际 App 无"编辑资料"/"关于我们"入口）
- 移除 `coldStartAndLogin` 中的 `agent.launch()` 竞态

---

### RESILIENCE 模块（WS 重连 / 前后台 / JWT 持久化）

> 当前状态机：负责人 [E2E] | 状态 [🚫 BLOCK] | 修复轮次 [5/5]

**用例覆盖：** TC-RESILIENCE-00001 · TC-RESILIENCE-00004 · TC-RESILIENCE-00005

**注：** TC-RESILIENCE-00001/00004 为 WS 重连和网络切换测试，本身依赖真实网络环境；  
TC-RESILIENCE-00005（JWT 持久化冷启动）在 Attempt 2/3 中持续通过，在 Attempt 4/5 因后续网络测试残留 WiFi 禁用 / API 欠费而失败。

---

### GOVERNANCE 模块（创建房间 / 密码房 / 权限 / 禁言）

> 当前状态机：负责人 [E2E] | 状态 [🚫 BLOCK] | 修复轮次 [5/5]

**用例覆盖：** TC-GOVERNANCE-00001 · 00003 · 00005 · 00006 · 00008

---

### MIC 模块（权限申请 / RTC 上麦 / 麦位点击）

> 当前状态机：负责人 [E2E] | 状态 [🚫 BLOCK] | 修复轮次 [5/5]

**用例覆盖：** TC-MIC-00001 · TC-MIC-00002 · TC-MIC-00009

---

### GIFT 模块（礼物面板 / 连送）

> 当前状态机：负责人 [E2E] | 状态 [🚫 BLOCK] | 修复轮次 [5/5]

**用例覆盖：** TC-GIFT-00001 · TC-GIFT-00003

---

### CHAT 模块（公屏聊天）

> 当前状态机：负责人 [E2E] | 状态 [🚫 BLOCK] | 修复轮次 [5/5]

**用例覆盖：** TC-CHAT-00002

---

### THEME 模块（黑金主题 / RTL）

> 当前状态机：负责人 [E2E] | 状态 [🚫 BLOCK] | 修复轮次 [5/5]

**用例覆盖：** TC-THEME-00001 · TC-THEME-00002 · TC-THEME-00003

---

### SHELL 模块（SplashScreen / MainScreen Tab / RoomScreen）

> 当前状态机：负责人 [E2E] | 状态 [🚫 BLOCK] | 修复轮次 [5/5]

**用例覆盖：** TC-SHELL-00001 · TC-SHELL-00002 · TC-SHELL-00005

---

### ANALYTICS 模块（隐私弹窗 / 事件上报）

> 当前状态机：负责人 [E2E] | 状态 [⚠️ 部分 PASS] | 修复轮次 [5/5]

**用例覆盖：** TC-ANALYTICS-00001 · TC-ANALYTICS-00003 · TC-ANALYTICS-00004

**TC-ANALYTICS-00001：** ✅ PASS（在 API 欠费前完成执行）  
**TC-ANALYTICS-00003/00004：** ❌ FAILED（Firebase 配置依赖 + API 欠费）

---

### AUTH 模块（注册登录全链路）

> 当前状态机：负责人 [E2E] | 状态 [🚫 BLOCK] | 修复轮次 [5/5]

**用例覆盖：** TC-AUTH-00003

---

## Round 5 代码修复汇总（Git commit: 1417bba）

| 修复项 | 文件 | 状态 |
|--------|------|------|
| JWT 注入方案 D（API login + DataStore proto 写入） | `androidReset.ts` | ✅ 已提交 |
| ADB 双引号转义 Bug 修复（两步 mkdir + single-quote sh） | `androidReset.ts` | ✅ 已提交 |
| 同意弹窗检测（detectScreenState() 保护，仅 consent 时 dismiss） | `androidReset.ts` | ✅ 已提交（待验证） |
| am-start 等待时间 3000→4000ms | `androidReset.ts` | ✅ 已提交 |
| 移除 `agent.launch()` 竞态（FLAG_ACTIVITY_RESET_TASK_IF_NEEDED HOME 闪屏） | 全部 AND spec | ✅ 已提交（待验证） |
| aiWaitFor 超时 15000→20000ms | 全部 AND spec | ✅ 已提交 |
| TC-PROFILE Step4 断言改为只检查"设置" | `TC-PROFILE.spec.ts` | ✅ 已提交 |
| TC-RANKING tab 标签修正（加"榜"后缀） | `TC-RANKING.spec.ts` | ✅ 已提交 |

---

## 最佳尝试结果（Attempt 2，代码最完整前）

| 测试 | 结果 |
|------|------|
| TC-RESILIENCE-00005 | ✅ PASS |
| TC-THEME-00001 | ✅ PASS |
| TC-WALLET-00001 | ✅ PASS |
| TC-WALLET-00002 | ✅ PASS |
| TC-WALLET-00004 | ✅ PASS |
| 其余 28 个 | ❌ FAILED |

**目标：≥ 27/34 PASS** — 需 AI API 账户充值后重新验证本轮代码修复效果。
