# 全局代码审查报告: 模块 5 - Web 管理端增强 (Admin Web Enhancements)
> **当前状态机**：负责人 [TDD] | 状态 [❌ Failed] | 修复轮次 [1/10]

---

## 0. 流转规则
- **状态枚举**：负责人 [-] 状态 [✅ Passed] | 负责人 [TDD] 状态 [❌ Failed] | 负责人 [GlobalReview] 状态 [⏳ In Review]
- 每轮 Review 追加一条记录，不要覆盖历史。
- 处于负责人 [GlobalReview] 状态 [⏳ In Review]，则由[GlobalReview]进行全局代码审查
- [GlobalReview]审查通过，则修改负责人 [-] 状态 [✅ Passed]
- [GlobalReview]审查未通过，则修改负责人 [TDD] 状态 [❌ Failed], 并将审查意见填入文档下方
- 处于负责人 [TDD] 状态 [❌ Failed]，则由[TDD]根据审查意见进行代码修复并自测
- [TDD]修复之后，将状态改为负责人 [GlobalReview] 状态 [⏳ In Review]

---

## 1. 审查上下文
- **包含任务**：[模块 5: Web 管理端增强](../tasks/模块5-Web%20管理端增强%20(Admin%20Web%20Enhancements).md)
  - Web：T-20010 (解封确认弹窗) / T-20011 (活水房间监控增强)
- **关联 TDS**：`doc/tds/web/T-20010.md`、`doc/tds/web/T-20011.md`
- **关联设计文档**：`doc/design/adminWeb/T-20010.md`、`doc/design/adminWeb/T-20011.md`
- **开始时间**：2026-04-25

---

## 2. 审查与修复日志

*(执行规则：GlobalReview 记录缺陷，TDD 在对应缺陷下方记录修复方案与 PR/Commit。严禁覆盖历史记录，只能向下追加)*

### 【第 1 轮审查】
**@GlobalReview 审查意见：**

本轮覆盖任务范围 T-20010（UnbanModal）与 T-20011（活水房间监控增强），共审计源文件：`app/web/src/pages/users/UnbanModal.tsx`、`useUnbanUser.ts`、`pages/users/index.tsx`、`core/network/apiClient.ts`、`pages/rooms/{roomUtils,RoomActivityTag,useRoomsPage,RoomsTable}.tsx?` 及对照 admin-server 路由 `app/adminServer/src/bootstrap/mod.rs`、`modules/user/{controller,service,dto}.rs`、`common/error.rs`、协议文档 `doc/protocol/admin_api.md`。

发现 **2 个 P0 致命问题**（均位于 T-20010 解封链路，破坏后端协议契约 & 用户实际使用），以及若干 P2 建议项（不阻塞）。

---

- [ ] **缺陷 1**：[级别 P0] **解封 API 端点不存在 — 前端调用 `PUT /api/v1/admin/users/:id/unban` 在 admin-server 无任何对应路由，生产环境必 404，解封功能完全失效**
  - **文件与行号**：
    - `app/web/src/core/network/apiClient.ts:460-471`（`adminUnbanUser` 实现 `PUT /users/:id/unban`）
    - `app/web/src/pages/users/UnbanModal.tsx:65-68`（调用点）
    - 对照：`app/adminServer/src/bootstrap/mod.rs:179-181` 仅注册了 `POST /api/v1/admin/users/{id}/ban`，**未注册任何 `/unban` 路由**
    - 对照：`app/adminServer/src/modules/user/controller.rs:78-127` `ban_user_handler` 通过请求体字段 `action: "ban" | "unban"` 区分语义；`dto.rs:161-174` `BanUserRequest { action, ban_type, duration_hours, reason }`
  - **问题说明**：
    1. **协议根本错位**：admin-server 端解封语义复用 `POST /:id/ban` + `{"action":"unban"}`（无独立 unban 端点；可由 `app/adminServer/src/bootstrap/mod.rs:2566` 集成测试 `post_ban(... r#"{"action":"unban"}"#)` 与 `controller.rs:112` `if req.action != "ban" && req.action != "unban"` 双重佐证）。前端实现的 `PUT /users/:id/unban` 在路由表中不存在，运行时会得到 404 / 405 Method Not Allowed。
    2. **TDS 来源错误**：`doc/tds/web/T-20010.md §2.3` 凭空规定了 `PUT /api/v1/admin/users/:id/unban` 与 `{reason, remark}` 请求体，TDD Agent 忠实实现了一个**虚构端点**。Reviewer 与 Plan 均未与 admin-server 实现对齐——这是模块 3 P0-2 已经修过一次的同类问题（封禁字段对齐 `ban_type/duration_hours`），解封侧再次出现协议错位。
    3. **payload 字段偏差**：当前发送 `{reason: string, remark?: string}`；服务端 DTO 仅识别 `{action, ban_type, duration_hours, reason}`，`remark` 字段会被 serde 忽略（且 BanModal 在 P0-2 修复中已将备注合并入 `reason`，UnbanModal 反向倒退，未保持对称）。
    4. **单元测试无法暴露**：`apiClient.test.ts` 中无 `adminUnbanUser` 的契约测试；`UnbanModal.test.tsx` mock 了 `useUnbanUser`，从未真正发起请求；`UsersPage.test.tsx` 也是 mock 网络层。所有 317/371 条用例通过 ≠ 该接口能跑通。
  - **修复建议**：
    - 删除 `adminUnbanUser` 独立函数，改为复用 `adminBanUser(userId, { action: 'unban', reason })`；或保留命名但内部实现改为 `POST /users/:id/ban` + `{action: 'unban', reason}`。
    - `UnbanModal.handleSubmit.onOk` 内构造请求体时，与 BanModal P0-2 修复保持对称：`reason = remark ? \`${selectedReason}: ${remark}\` : selectedReason`，且不传 `ban_type`/`duration_hours`（unban 时这些字段服务端忽略，参考 `dto.rs` `Option<...>`）。
    - 同步修改 `doc/tds/web/T-20010.md §2.3` 与 §2.4 的接口规格；并在 `apiClient.test.ts` 补充 `adminUnbanUser` 契约测试（fetch mock 校验 method=POST、url=`/users/.../ban`、body 包含 `action: 'unban'`），防止再次回归。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

- [ ] **缺陷 2**：[级别 P0] **错误码 `40901` 与服务端实际返回的 `40900` 不匹配，"用户当前未被封禁" 友好文案永远不会触发**
  - **文件与行号**：
    - `app/web/src/pages/users/UnbanModal.tsx:72` `err.message.includes('40901') ? t('users.unban.alreadyNormal') : ...`
    - `doc/tds/web/T-20010.md §2.3` 规定错误码 `40901`
    - 对照：`app/adminServer/src/common/error.rs:51,85,100,269-274` 明确 `AppError::UserAlreadyNormal` → HTTP 409 / **错误码 40900**（单元测试 `e02_t10009_user_already_normal_maps_to_409_40900` 锁定）
  - **问题说明**：服务端"用户已是正常状态"的业务错误码为 **40900**，但 TDS 与实现均写成 `40901`（`40901` 实际是另一个不相关错误 `RoomAlreadyClosed`，见 `error.rs:255`）。结果：当运营对一个已正常用户重复点击解封时，UI 不会显示 i18n 化的 `users.unban.alreadyNormal`（"该用户当前未被封禁"），而是落入 fallback 分支显示原始英文 `User already in normal status` 或 `common.requestError`，违反 TDS §2.5 错误处理与 i18n 要求。
  - **修复建议**：
    1. `UnbanModal.tsx:72` 将 `'40901'` 改为 `'40900'`；
    2. 进一步建议改为基于 `code` 字段而非 `message` 子串匹配（`adminFetch` 抛出的 Error 应带结构化 code，避免 i18n message 翻译变化导致匹配失败——后续迭代）；
    3. 同步修正 `doc/tds/web/T-20010.md §2.3` 错误码列表。
    4. 备注：BanModal 中相同位置（`BanModal.tsx:92`）也用了 `40901` 检查 `users.ban.alreadyBanned`，是模块 3 已通过审查的旧代码；按"未修改代码不报"原则不计入本批次缺陷，但建议 TDD 在修复 UnbanModal 时一并修正以保持对称（自愿，非阻塞）。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

---

#### 已通过项（积极评价）

- **T-20010 BanModal 对称性**：UnbanModal 组件结构、`isConfirming` 三处重置（onOk、afterClose、handleClose）、`destroyOnHidden`、Form `validateFields` → `Modal.confirm` → `unban` 流程完全对称，并发防护到位。
- **T-20010 useUnbanUser**：与 useBanUser 对称，re-throw 让 UI 决定错误展示，loading 在 finally 中重置；类型 `UseUnbanUserReturn` 显式声明，无 `any`。
- **T-20010 UsersPage 集成**：`unbanUserId` state + 三个 `useCallback` 回调按 TDS §2.7 实现，原内联 `Modal.confirm` 已彻底移除，drawer/list 状态联动顺序正确（`setUnbanUserId(null) → setSelectedUserId(null) → refresh()`），不会触发 UserDetailDrawer 内的脏 fetch。
- **T-20011 roomUtils 纯函数**：`getActivityStatus` / `formatDuration` / `filterByActivity` 拆分干净，`now` 注入支持时间确定性测试，规则优先级与 `doc/design/adminWeb/T-20011.md` 表格完全一致；类型 `ActivityFilter` 已在 Round 1 修复中精确收敛为 `'all' | 'active' | 'quiet' | 'abnormal'`，移除越界 `'normal'` 选项。
- **T-20011 useRoomsPage 集成**：`filteredItems` 使用 `useMemo([items, activityFilter])` 纯前端过滤，不触发新 API 请求（H17 测试覆盖）；既有 `AbortController` 竞态保护（fetch effect cleanup `controller.abort()`，line 103-105）未被破坏；`debouncedKeyword` 链路保留。
- **T-20011 RoomsTable**：活跃状态/持续时长两列在合理位置插入；异常行高亮通过 `onRow.style` 实现；`<Select<ActivityFilter>>` 泛型化，消除了不安全 `as` 断言；`columns` / `activityOptions` 均 `useMemo`；`statusOptions` 仍裸定义（Round 1 LOW-1 残留，本轮不再追责）。
- **测试覆盖**：371/371 通过，rooms 目录覆盖率行 100% / 分支 95%+，UnbanModal 13 条 + useUnbanUser 4 条 + UsersPage 集成 3 条覆盖 TDS §3 全部用例。

#### P2 建议项（不阻塞，可后续迭代）

1. **可访问性**：异常房间行高亮仅靠 `background: rgba(231,76,60,0.1)`，未对 `<tr>` 添加 `aria-label` / `aria-describedby`。RoomActivityTag 已提供文本"异常"，对屏幕阅读器具备语义；但若考虑高对比度无障碍模式，可在 `onRow` 返回时附加 `aria-label={t('rooms.activityLevelAbnormal')}`。
2. **`adminUnbanUser` signal 参数预留但未使用**：`useUnbanUser.ts:23` 未透传 AbortSignal。考虑到 unban 是单次用户主动 mutation（非 useEffect 中的 fetch），无 race condition；`isConfirming` 已防重复，OK，仅作记录。
3. **i18n 死键 `users.unban.description`**：zh/en 中定义但 UnbanModal 组件未使用（TDS §2.8 列出，§2.5 表单设计也未引用）。建议 DoD 阶段决定保留或删除。
4. **`UsersPage.handleUnbanSuccess` 中的 `void userId`**（`index.tsx:83`）：建议改为 `_userId` 形参（前次 Round 1 内部 review 已建议，仍未采纳）。

---

**本轮结论**: ❌ 存在 P0 级别问题（共 2 条，均集中在 T-20010 解封 API 协议错位 + 错误码错位）。T-20011 实现质量良好，无阻塞问题。
*(请在文档头部将状态机修改为：`负责人 [TDD] | 状态 [❌ Failed] | 修复轮次 [1/10]`)*

**缺陷分布汇总**：
| 级别 | 数量 | 任务分布 |
|------|------|---------|
| P0 (致命) | 2 | T-20010 ×2 |
| P1 (高危) | 0 | — |
| P2 (一般) | 4（建议） | T-20010 ×3 / T-20011 ×1 |
| **合计阻塞项** | **2** | — |

