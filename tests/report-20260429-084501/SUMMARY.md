# QA Gate Regression Report — 20260429-084501

> **任务关联**: C方案 B阶段 — 模块 1/2/4/7 Android 切片 QA Gate（BUG-ANDROID-001 修复回归）  
> **执行人**: E2E-Runner (Android QA Agent)  
> **执行时间**: 2026-04-29 08:45:01  
> **报告目录**: `tests/report-20260429-084501/`  
> **对比基线**: `tests/report-20260429-081255/` (0/57 PASS，全量 AIOOBE)  
> **修复 Commit**: `b9948e9` — fix(android/theme): MenaColors 用 Color(Int) 替代 Color(ULong) 修 BUG-ANDROID-001

---

## 🚫 最终战报（已熔断 — 等待 master 处置）

| 套件 | 设备 | PASS | FAIL | SKIP | 全局失败率 | 结论 |
|------|------|------|------|------|------------|------|
| Android instrumentation | Pixel 4 / Android 13 | 137 | 43 | 0 | **23.9%** | 🚫 **BLOCK（熔断）** |
| **合计** | — | **137** | **43** | **0** | **23.9%** | 🔴 **部分熔断** |

> 全局失败率 23.9% < 50%（未触发全局熔断），但 **T-30001 / T-30021 / T-30023 / T-30024 / T-30025** 各自 ≥ 3 failures 且根因相同，触发单 Task 熔断规则。

---

## ✅ BUG-ANDROID-001 修复验证（本轮主要目标）

| 项目 | 上轮 (081255) | 本轮 (084501) | 变化 |
|------|--------------|--------------|------|
| 总测试数 | 57 | **180** | +123 |
| PASS 数 | **0** | **137** | **+137** 🎉 |
| FAIL 数 | 57 | 43 | -14 |
| ArrayIndexOutOfBoundsException | **57/57 (全量崩溃)** | **0** | ✅ **完全修复** |
| MenaThemeTest (核心验证) | 0/8 FAIL | **8/8 PASS** | ✅ **全绿** |

**结论：BUG-ANDROID-001 已由 commit `b9948e9` 彻底修复，MenaColors.kt `Color(ULong)` → `Color(Int)` 修复正确，无任何 AIOOBE 残留。**

---

## 新发现 Bug

### 🐛 BUG-ANDROID-002 — Compose 语义树组件不可见（系统性）
- **影响范围**: LoginScreenVisualTest / ProfileScreenTest / HostMicSlotTest / MicSlotsGridBlackGoldTest / HallScreenTest / HallScreenVisualUpgradeTest / MainScreenTest / RoomBottomBarTest（共 38 tests，11 个测试类）
- **症状**: `Assert failed: The component is not displayed!` / `Can't retrieve node at index '1' of 'Text + EditableText contains 'تسجيل الدخول''`
- **关联 Task**: T-30001 / T-30021 / T-30022 / T-30024 / T-30025 / T-30026（熔断: T-30001, T-30021, T-30024, T-30025）
- **推测根因**: 
  - GoldButton / GoldOutlinedTextField 视觉升级后，组件包裹层级发生变化，测试通过 `onNodeWithText(arabicText).get(index=1)` 等方式查找节点失败
  - 部分组件未向 Compose 语义树暴露正确的 `mergeDescendants=true` 节点，导致 `assertIsDisplayed()` 失败
  - RTL/Arabic 文本节点位置与测试预期不符
- **logcat 关键片段**:
  ```
  androidx.compose.ui.test.SemanticsNodeInteractionKt: 
    Can't retrieve node at index '1' of 'Text + EditableText contains 'تسجيل الدخول' (ignoreCase: false)'
    There are no existing nodes for that selector.
    Nodes in subtree for '...': 
      Node #1 ... (only root, no matching children)
  ```
- **建议**: TDD 检查 GoldButton/GoldOutlinedTextField 是否正确暴露语义树节点（`testTag`/`contentDescription`/`mergeDescendants`）；测试需更新为基于 `testTag` 而非文本内容查找。

### 🐛 BUG-ANDROID-003 — PlaceholderScreenTest.PH09 setContent 重复调用
- **影响范围**: PlaceholderScreenTest.PH09_rtlLayout_doesNotCrash_andDisplaysContent（1 test）
- **症状**: `java.lang.IllegalStateException: Cannot call setContent twice per test!`
- **关联 Task**: T-30023（熔断触发）
- **推测根因**: PH09 测试方法内部（或其 @Before 设置）重复调用了 `rule.setContent {}`，可能由 RTL 环境切换导致重复初始化
- **建议**: TDD 检查 PlaceholderScreenTest 的 `@Before`/`@Rule` 配置，避免 `setContent` 被调用两次。

### 🐛 BUG-ANDROID-004 — 输入回调返回空值（3 tests）
- **影响范围**: GoldOutlinedTextFieldTest.GT02 / ChatInputBarTest.CI / RoomScreenTest.UI06（3 tests，OOS: 后2项为 T-30009/T-30015）
- **症状**: `AssertionError: Last changed value should contain 'Hello', got: (empty string)`
- **关联 Task**: T-30018（GT02，1 test in scope）
- **推测根因**: `performTextInput()` 触发了 onValueChange 但值为空；GoldOutlinedTextField 的 onValueChange 回调封装可能丢失了文本变更事件
- **建议**: TDD 检查 GoldOutlinedTextField 的 `onValueChange` lambda 传递链路。

### ⚠️ BUG-ANDROID-005 — MainActivitySmokeTest Espresso 节点未找到（1 test，OOS）
- **影响范围**: MainActivitySmokeTest.launch_shows_auth_bootstrap_title（1 test，OOS）
- **症状**: `NoMatchingViewException: No views in hierarchy found matching: view.getId() is <id/screenTitle>`
- **说明**: 不在本轮 18 Task 范围内，可能因 T-30021 登录页视觉升级后 screenTitle View ID 变化。仅供参考。

---

## 18 Task QA Gate 汇总

| Task ID | 模块 | 任务名称 | 测试类 | PASS | FAIL | QA Gate | 熔断 |
|---------|------|----------|--------|------|------|---------|------|
| **T-30001** | 模块1 | 登录页 UI | LoginScreenVisualTest | 7 | 12 | ❌ Fail · BUG-ANDROID-002 | 🚫 熔断 |
| **T-30002** | 模块1 | 登录 ViewModel | JVM-only | — | — | ⏭️ SKIP-OOS | — |
| **T-30003** | 模块1 | JWT 拦截器 | JVM-only | — | — | ⏭️ SKIP-OOS | — |
| **T-30004** | 模块1 | 用户信息 Repository | JVM-only | — | — | ⏭️ SKIP-OOS | — |
| **T-30005** | 模块2 | 大厅页 UI | HallScreenTest | 5 | 1 | ⚠️ Partial | — |
| **T-30006** | 模块2 | 房间列表 Paging3 | HallScreenPagingTest | 6 | 0 | ✅ Pass | — |
| **T-30007** | 模块2 | 创建房间对话框 | 无androidTest | — | — | ⏭️ SKIP-OOS | — |
| **T-30018** | 模块4 | MenaTheme 黑金主题 | Mena+Gold+Avatar Tests | 25 | 1 | ⚠️ Partial · BUG-ANDROID-004 | — |
| **T-30019** | 模块4 | Splash 启动页 | SplashScreenTest | 4 | 0 | ✅ Pass | — |
| **T-30020** | 模块4 | MainScreen 三Tab | MainScreenTest | 7 | 1 | ⚠️ Partial · BUG-ANDROID-002 | — |
| **T-30021** | 模块4 | 登录页视觉升级 | LoginScreenVisualTest | 7 | 12 | ❌ Fail · BUG-ANDROID-002 | 🚫 熔断 |
| **T-30022** | 模块4 | 大厅页视觉升级 | HallScreenVisualUpgradeTest | 10 | 2 | ⚠️ Partial · BUG-ANDROID-002 | — |
| **T-30023** | 模块4 | 消息Tab占位页 | PlaceholderScreenTest | 8 | 3 | ❌ Fail · BUG-ANDROID-003 | 🚫 熔断 |
| **T-30024** | 模块4 | 个人中心页 | ProfileScreenTest | 5 | 4 | ❌ Fail · BUG-ANDROID-002 | 🚫 熔断 |
| **T-30025** | 模块4 | 房间页视觉升级 | HostMicSlot+Grid+ChatBG | 14 | 4 | ❌ Fail · BUG-ANDROID-002 | 🚫 熔断 |
| **T-30026** | 模块4 | 房间底部操作栏 | RoomBottomBarTest | 8 | 2 | ⚠️ Partial · BUG-ANDROID-002 | — |
| **T-30034** | 模块7 | Analytics 防腐层 | JVM-only | — | — | ⏭️ SKIP-OOS | — |
| **T-30035** | 模块7 | EventReportClient | JVM-only | — | — | ⏭️ SKIP-OOS | — |

> **说明**: SKIP-OOS = 超出 androidTest 适用范畴（JVM-only 或无 androidTest 文件）。T-30002/T-30003/T-30004/T-30034/T-30035 等 JVM 测试已在 TDD 阶段通过。

---

## 熔断详情

| 触发顺序 | Task | 失败数 | 熔断规则 | Bug ID |
|---------|------|--------|---------|--------|
| 1st | T-30001 | 12 FAIL | ≥ 3 且根因相同（组件不可见） | BUG-ANDROID-002 |
| 2nd | T-30021 | 12 FAIL | ≥ 3 且根因相同（同T-30001，同一测试文件） | BUG-ANDROID-002 |
| 3rd | T-30023 | 3 FAIL | ≥ 3 且根因相同（含setContent twice独立问题） | BUG-ANDROID-003 |
| 4th | T-30024 | 4 FAIL | ≥ 3 且根因相同（Profile组件不可见） | BUG-ANDROID-002 |
| 5th | T-30025 | 4 FAIL | ≥ 3 且根因相同（房间视觉升级组件不可见） | BUG-ANDROID-002 |

> **注**: 因所有 180 个测试在单次 Gradle 批量执行中完成，无法在熔断触发时中止剩余用例，已将全量结果纳入本报告。  
> **熔断实际触发点**: T-30001（模块1，第一个达到 ≥3 同根因失败的任务）。

---

## 与上轮对比 (081255 → 084501)

| 项目 | 081255 (熔断前) | 084501 (本轮) | 备注 |
|------|----------------|--------------|------|
| 总测试 | 57 | 180 | +123（修复后更多测试类可加载） |
| **PASS** | **0** | **137** | **+137 🎉** |
| FAIL | 57 | 43 | -14 |
| AIOOBE 崩溃 | 57 | **0** | ✅ 彻底修复 |
| 新 Bug | — | BUG-ANDROID-002/003/004 | 视觉升级语义树问题 |
| 熔断原因 | BUG-ANDROID-001 (全量崩溃) | BUG-ANDROID-002 (部分组件不可见) | 由崩溃 → 功能性缺陷 |

---

## 模块覆盖汇总

| 模块 | 端 | 关联 Task | 结果 | QA 结论 |
|------|----|----------|------|---------|
| 模块1 — 用户认证 | Android | T-30001 | 7P/12F | ❌ FAIL · BUG-ANDROID-002 · 🚫 BLOCK |
| 模块1 — 用户认证 | Android | T-30002/T-30003/T-30004 | — | ⏭️ SKIP-OOS |
| 模块2 — 房间大厅 | Android | T-30005 | 5P/1F | ⚠️ Partial |
| 模块2 — 房间大厅 | Android | T-30006 | 6P/0F | ✅ PASS |
| 模块2 — 房间大厅 | Android | T-30007 | — | ⏭️ SKIP-OOS |
| 模块4 — 黑金主题 | Android | T-30018 | 25P/1F | ⚠️ Partial |
| 模块4 — 黑金主题 | Android | T-30019 | 4P/0F | ✅ PASS |
| 模块4 — 黑金主题 | Android | T-30020 | 7P/1F | ⚠️ Partial |
| 模块4 — 黑金主题 | Android | T-30021 | 7P/12F | ❌ FAIL · BUG-ANDROID-002 · 🚫 BLOCK |
| 模块4 — 黑金主题 | Android | T-30022 | 10P/2F | ⚠️ Partial |
| 模块4 — 黑金主题 | Android | T-30023 | 8P/3F | ❌ FAIL · BUG-ANDROID-003 · 🚫 BLOCK |
| 模块4 — 黑金主题 | Android | T-30024 | 5P/4F | ❌ FAIL · BUG-ANDROID-002 · 🚫 BLOCK |
| 模块4 — 黑金主题 | Android | T-30025 | 14P/4F | ❌ FAIL · BUG-ANDROID-002 · 🚫 BLOCK |
| 模块4 — 黑金主题 | Android | T-30026 | 8P/2F | ⚠️ Partial |
| 模块7 — 埋点基建 | Android | T-30034/T-30035 | — | ⏭️ SKIP-OOS |

---

## OOS 附录（范围外但有失败的测试类）

> 以下测试类不在 B 阶段 18 Task 范围内，但在本次批量执行中发现失败，供 master 参考：

| 测试类 | Task | PASS | FAIL | 失败摘要 |
|--------|------|------|------|---------|
| RoomScreenTest | T-30009 (OOS) | 5 | 5 | 组件不可见/输入回调异常 |
| MicSlotCardTest | T-30011 (OOS) | 4 | 6 | 组件不可见/语义节点未找到 |
| ChatInputBarTest | T-30015 (OOS) | 8 | 1 | onValueChange 回调返回空 |
| MainActivitySmokeTest | (无Task标注) | 0 | 1 | Espresso NoMatchingViewException |

---

## master 决策建议

1. **BUG-ANDROID-002 (系统性，高优先)**：  
   - 请派 TDD 修复 GoldButton/GoldOutlinedTextField/AvatarWithFrame/ProfileScreen 等组件的 Compose 语义树暴露问题。  
   - 建议核查：`semantics { mergeDescendants = true }` 的使用；`testTag` 是否在视觉升级时遗失；`onNodeWithText` 查找是否因组件层级变化失效（改用 `onNodeWithTag` 更稳健）。  
   - 预计修复后可消除约 38 个失败测试。

2. **BUG-ANDROID-003 (独立，中优先)**：  
   - PlaceholderScreenTest.PH09 的 `setContent` 重复调用需修复测试代码（允许修改测试代码，不涉及业务逻辑）。

3. **BUG-ANDROID-004 (中优先)**：  
   - GoldOutlinedTextField.onValueChange 回调丢失问题，影响 T-30018 的一个 case。

4. **SKIP-OOS 说明**：T-30007/T-30034/T-30035 无 androidTest，需 master 确认是否需要补充。

---

## 参考链接

- 个人任务详情: [T-30001](./T-30001/result.json) / [T-30021](./T-30021/result.json) / [T-30025](./T-30025/result.json)
- 上轮熔断报告: [tests/report-20260429-081255/SUMMARY.md](../report-20260429-081255/SUMMARY.md)
- 修复 TDS: [doc/tds/android/T-30018.md](../../doc/tds/android/T-30018.md)
