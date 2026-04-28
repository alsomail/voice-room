# 全局代码审查报告：QA-Coord-Regression-v3 未提交改动收尾批次

> **当前状态机**：负责人 [GlobalReview] | 状态 [⏳ In Review] | 修复轮次 [0/10]

> **批次定位**：`qa-coord-regression-v3`（report-20260428-154125）跑完后留在 working tree 的 23 个未提交改动的合并审查批次。由 master-coordinator D-1 + D-2 + 部分 D-3 派发，目的是在 commit 之前补上缺失的全局架构审查门禁。

---

## 0. 流转规则

- **状态枚举**：负责人 [-] 状态 [✅ Passed] | 负责人 [TDD] 状态 [❌ Failed] | 负责人 [GlobalReview] 状态 [⏳ In Review]
- 每轮 Review 追加一条记录，不要覆盖历史。
- 处于负责人 [GlobalReview] 状态 [⏳ In Review]，则由 [GlobalReview] 进行全局代码审查。
- [GlobalReview] 审查通过 → 修改负责人 [-] 状态 [✅ Passed]。
- [GlobalReview] 审查未通过 → 修改负责人 [TDD] 状态 [❌ Failed]，并将审查意见追加到文档下方。
- [TDD] 修复并自测后 → 状态改为负责人 [GlobalReview] 状态 [⏳ In Review]，触发下一轮复审。

---

## 1. 审查上下文

- **关联任务（门禁连带）**：
  - **T-00042**（Admin 强制断连 / 模块 8）：本批次内 `app/server/src/main.rs` 是其 admin 事件订阅 task 的真集成点 —— 之前 `批次-架构阻塞修复-业务侧.md` Round 1 ✅ 通过的范围只包含 service / events 模块，未覆盖 main.rs spawn 入口；本批次补做 **T-00042 Round 2 增量审查**。审查通过前，T-00042 主表 Overall Gate 已临时撤回为 `⏳ Pending`。
  - **T-0000P**（Midscene env 注入 / 模块 9）：本批次内 i18n 默认值切换 + 9 个 WEB 测试自愈属于 P 之后的 web e2e 缺陷面，**本批次不修复 9 个 WEB FAIL**（由 product-manager 拆新 Task 处理）。T-0000P Review/QA Gate 不受本批次影响，但 Overall Gate 从越权 `✅ Released` 修正为 `⚠️ Conditional`（带 9 WEB FAIL pending 备注）。
- **关联 TDS**：
  - [T-00042](../tds/server/T-00042.md)（追加 Round 2 main.rs 集成审查记录）
- **范围分组**（23 个改动 = 1 个 (b) + 22 个 (a)+(c)）：

### A1 组 — (b) 业务代码（Round 2 增量审查 · T-00042）

| 文件 | 增删 | 说明 |
|------|------|------|
| `app/server/src/main.rs` | +18 | 在 main 启动序列中 spawn `start_admin_event_subscriber` task，订阅 Redis `admin:events`，复用 `snapshot_shutdown_tx` 优雅停机 watch channel |

### A2 组 — (a) 测试自愈 + (c) i18n / 轻量 UI（22 个）

#### Web 业务代码（10 个）

| 文件 | 类型 | 说明 |
|------|------|------|
| `app/web/src/i18n/index.ts` | (c) | 默认 lng/fallbackLng `en` → `zh` |
| `app/web/src/app/AppLayout.tsx` | (a) | 加退出登录按钮 + `data-testid="logout-btn"` |
| `app/web/src/core/network/apiClient.ts` | (a) | Room list `id` → `room_id` 字段映射兼容；Stats `new_users_today` → `new_users` 对齐后端，trend 字段标记 optional |
| `app/web/src/stores/useAuthStore.ts` | (a) | 新增 `ADMIN_INFO_KEY` localStorage 持久化 + logout 清理 |
| `app/web/src/pages/login/LoginForm.tsx` | (a) | 错误信息 "Invalid credentials" 映射到 i18n key `login.error.invalidCredentials` |
| `app/web/src/features/gift/GiftEditModal.tsx` | (a) | `onSuccess(isCreate: boolean)` 签名 |
| `app/web/src/features/gift/GiftManagementPage.tsx` | (a/c) | 编辑/创建分别 toast `gift.mgmt.createSuccess` / `updateSuccess` |
| `app/web/src/features/user/AdjustBalanceModal.tsx` | (a) | 调整余额成功后 `wallet.adjust.successMsg` toast |
| `app/web/src/pages/dashboard/TrendChart.tsx` | (a) | `trend` undefined guard（`safeTrend`）防 ECharts 崩溃 |
| `app/web/src/pages/dashboard/useDashboardStats.ts` | (a) | 字段映射对齐 + trend `?? []` |

#### 测试脚本自愈（9 个）

| 文件 | 类型 | 说明 |
|------|------|------|
| `tests/scripts/API/TC-AUTH.spec.ts` | (a) | qa-coord-regression-v3 自愈 |
| `tests/scripts/API/TC-CHAT.spec.ts` | (a) | qa-coord-regression-v3 自愈 |
| `tests/scripts/API/TC-GIFT.spec.ts` | (a) | qa-coord-regression-v3 自愈 |
| `tests/scripts/API/TC-INFRA.spec.ts` | (a) | qa-coord-regression-v3 自愈 |
| `tests/scripts/API/TC-RANKING.spec.ts` | (a) | qa-coord-regression-v3 自愈 |
| `tests/scripts/API/TC-WS.spec.ts` | (a) | qa-coord-regression-v3 自愈 |
| `tests/scripts/WEB/TC-AUTH.spec.ts` | (a) | qa-coord-regression-v3 自愈 |
| `tests/scripts/WEB/TC-LOG.spec.ts` | (a) | qa-coord-regression-v3 自愈 |
| `tests/scripts/WEB/TC-WALLET.spec.ts` | (a) | qa-coord-regression-v3 自愈 |

- **开始时间**：2026-04-29

---

## 2. 审查关切

### A1 组关切（T-00042 Round 2 main.rs 集成）

1. **是不是 T-00042 admin 事件订阅的真集成点？** —— 之前 Round 1 给出 ✅ 是基于 service/events 单测，但 main.rs 的 spawn 入口当时未含此 task；如果 spawn 缺失，T-00042 在生产上根本不工作（典型"半截子"BUG）。本轮必须确认：
   - 调用的是 `voice_room_server::events::start_admin_event_subscriber`（Round 1 已确认导出的对外 API）
   - 注入参数齐全：Redis URL、`ws_registry`（与正常 WS 上行链路同源）、shutdown receiver
2. **生命周期与停机风险**：
   - shutdown channel 是否复用全局 `snapshot_shutdown_tx`（避免泄漏一条孤儿 task）
   - `tokio::spawn` 抛出 panic / Err 是否打 `tracing::warn` 而不是吞没（main 进程不能因为 admin 事件订阅失败而退出）
   - 与 T-00041 心跳 task 的 spawn 顺序：心跳 task 是否已先 spawn（确认 main.rs:175-176 的 heartbeat_task 不会被本次新增打乱）
3. **错误隔离**：admin 事件订阅 task 失败时是否不阻塞 `build_app` / `bind` / `serve` 主路径

### A2 组关切（22 个 (a)+(c) 改动）

1. **测试脚本自愈**：是否削弱了断言强度（例如把严格相等改成仅状态码 200）？是否引入条件 skip 掩盖真实失败？
2. **i18n 默认 `en`→`zh` 决策**：参考 `doc/product/index.md`，**目标市场是 MENA（中东）**，但本端是 **Web 管理后台（B 端，运营/产品内部使用）**，运营团队为中文。需明确：
   - 本端 i18n 默认 `zh` 是否符合"内部运营操作语言"的事实标准
   - 是否需要在 README / arch/web 文档中登记此决策（避免未来误以为是产品面向用户的语言默认）
3. **API 字段映射兼容（apiClient.ts）**：`room_id` ?? `id` 与 `new_users_today` → `new_users` 是 frontend 适配层，是否会掩盖真正的后端 contract drift？是否应上抛 issue 给 backend 收口而非 frontend 兼容？
4. **TrendChart undefined guard**：是否表示后端 `/admin/stats/overview` 不再返回 trend 字段？需对照 T-20011 / dashboard 文档判断是否回归。
5. **localStorage adminInfo 持久化**：是否引入 XSS 风险（admin 对象中是否含敏感字段如手机号、role 元数据）？
6. **AppLayout logout 按钮 absolute bottom 定位**：在 collapsed 状态下是否布局崩溃？

### 跨切关切

- **9 个 WEB FAIL（TC-ROOM 6 + TC-GIFT 2 + TC-USER 1）不在本批次 scope**，后续由 product-manager 拆新 Task；本批次只确认本 23 改动**没有引入新的 web 回归**。

---

## 3. 审查与修复日志

*(执行规则：GlobalReview 记录缺陷，TDD 在对应缺陷下方记录修复方案与 PR/Commit。严禁覆盖历史记录，只能向下追加)*

### 【第 1 轮审查】

**@GlobalReview 审查意见**：[等待 global-code-reviewer 子代理填写]

---
