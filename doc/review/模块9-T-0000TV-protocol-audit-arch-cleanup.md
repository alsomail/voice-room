# 全局代码审查报告: 模块9 — T-0000T/V 协议审计脚本 + 架构文档清理
> **当前状态机**：负责人 [GlobalReview] | 状态 [⏳ In Review] | 修复轮次 [1/10]

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
- **包含任务**：
  - [模块 9: E2E 测试基建](../tasks/模块9-E2E测试基建%20(E2E%20QA%20Foundation).md), T-0000T, T-0000V
- **关联 TDS**：
  - [T-0000T](../tds/infra/T-0000T.md)（协议路径绑定审计脚本）
  - [T-0000V](../tds/infra/T-0000V.md)（ARCHITECTURE.md 物理删除 + ADR-0002 + §8.2 重写）
- **开始时间**：2026-05-06

---

## 🔌 协议路径绑定汇总

> **说明**：两个 Task 均为纯基础设施/文档层任务，不涉及任何跨端 API/WS 路径新增或变更。

### HTTP REST

| Task | 协议类型 | 入口/信令名 | 客户端调用方 | 服务端处理函数 | protocol/ 锚点 |
|------|---------|------------|-------------|---------------|---------------|
| T-0000T | N/A | — | — | — | — |
| T-0000V | N/A | — | — | — | — |

### WebSocket

| Task | 协议类型 | 入口/信令名 | 客户端调用方 | 服务端处理函数 | protocol/ 锚点 |
|------|---------|------------|-------------|---------------|---------------|
| T-0000T | N/A | — | — | — | — |
| T-0000V | N/A | — | — | — | — |

### Redis Pub-Sub

| Task | Channel | Publisher | Subscriber | 说明 |
|------|---------|-----------|-----------|------|
| T-0000T | N/A | — | — | — |
| T-0000V | N/A | — | — | — |

> **P0 必查声明**：
> - T-0000T 绑定表声明：`N/A — 本 Task 为纯基础设施工具，无跨端协议路径；脚本本身读取 doc/protocol/index.md 作为协议锚点参考，不新增任何 HTTP REST / WebSocket 通信入口。`
> - T-0000V 绑定表声明：`N/A — 纯文档清理任务，不涉及跨端 API/WS 路径变更`

---

## 2. 审查与修复日志

*(执行规则：GlobalReview 记录缺陷，TDD 在对应缺陷下方记录修复方案与 PR/Commit。严禁覆盖历史记录，只能向下追加)*

### 【第 1 轮审查】

**@GlobalReview 审查意见：**

> 审查范围：
> - `scripts/audit/protocol-binding-audit.ts` 脚本逻辑正确性
> - ERE grep 模式准确性与覆盖率
> - P0/P1 分级规则合理性
> - CI 集成规范性（`npm run audit:protocol`）
> - `doc/adr/ADR-0002-protocol-single-source-and-binding-table.md` 内容准确性
> - `doc/architecture/websocket_and_state.md §8.2` 重写质量
> - 悬空引用清理完整性
> - TDS §六 Review 意见的自评结论需要独立复核

*(等待 GlobalReview Agent 填写...)*

---

### 【第 1 轮审查】

**@GlobalReview 审查意见：**

> 审查执行时间：2026-05-06  
> 实际运行验证命令：`npm run audit:protocol`、`npx jest scripts/audit/`、文件存在性核查、grep 模式独立验证  
> 53/53 单测通过；审计脚本实际运行报告 2 P0 + 1 P1 问题

---

**T-0000T 审查结果：**

- [ ] **缺陷 1**：[级别 P0] **ERE grep 模式中 `(` 未转义，导致客户端 grep 静默全部失败，CI 门禁功能性失效**
  - **文件与行号**：
    - `scripts/audit/protocol-binding-audit.ts:699` — Android WS 模式末尾 `\.sendEnvelope(`
    - `scripts/audit/protocol-binding-audit.ts:707` — Retrofit 模式末尾 `@(GET|POST|PUT|DELETE|PATCH)(`
    - `scripts/audit/protocol-binding-audit.ts:719` — Web 模式末尾 `apiClient\.(get|post|put|delete|patch|request)(`
  - **问题说明**：上述三处 grep 模式传入 `runGrep()` 时，都以未转义的 `(` 结尾。在 ERE（`-E`）模式下，`(` 是开括号分组符号，必须配对或转义为 `\(`，否则 grep 以 `parentheses not balanced` 退出码 2 终止。`runGrep()` 内使用 `2>/dev/null || true`，错误被完全吞没，返回空数组。
  - **实验证据（grep 命令 + 文件行号）**：
    ```bash
    # 验证1：脚本实际输出 0 客户端 grep 结果
    # npm run audit:protocol 输出：
    #   Found 0 client grep results
    #   [P0] MISSING_CLIENT_CALL ... SendMessage (expected: RoomViewModel.kt::sendMessage)

    # 验证2：模拟脚本所用命令 → 括号报错
    echo "test" | grep -E '\.sendEnvelope(' 2>&1
    # → grep: parentheses not balanced

    # 验证3：正确转义后立即返回 10 条命中
    grep -rEn 'wsClient\.send|\.sendEnvelope\(' \
      app/android/app/src/main --include="*.kt" 2>/dev/null | wc -l
    # → 10
    ```
  - **后果**：所有携带 `clientFile` 字段的绑定行都产生假阳性 P0 `MISSING_CLIENT_CALL`，`RoomViewModel.kt::sendMessage` 实际存在（行 466）但被误报缺失。CI 门禁永远阻断，"三角对账"核心功能名存实亡。
  - **修复建议**：
    ```typescript
    // 行 699（Android WS）：
    // 错误：'\\.sendEnvelope('
    // 修复：'\\.sendEnvelope\\('

    // 行 707（Retrofit）：
    // 错误：'@(GET|POST|PUT|DELETE|PATCH)('
    // 修复：'@(GET|POST|PUT|DELETE|PATCH)\\('

    // 行 719（Web apiClient）：
    // 错误：'apiClient\\.(get|post|put|delete|patch|request)('
    // 修复：'apiClient\\.(get|post|put|delete|patch|request)\\('
    ```
    同时补充单测：对每个 ERE 模式调用 `grep -E "<pattern>" /dev/null` 验证不产生 "parentheses not balanced" 错误。
  - **TDD 修复记录**：将三处 grep 模式末尾未转义 `(` 修正为 `\(`（TypeScript 字符串层 `\\(`）：行 699 Android WS `\.sendEnvelope\(`，行 707 Retrofit `@(GET|POST|PUT|DELETE|PATCH)\(`，行 719 Web `apiClient\.(get|post|put|delete|patch|request)\(`。修复后 `npm run audit:protocol` P0 Errors 由 2 降至 0，53/53 单测继续通过。Commit: 94e12a5

- [ ] **缺陷 2**：[级别 P1] **NA_PATTERNS 不完整，「纯文档清理」类声明无法被识别，T-0000V.md 被误报 MISSING_BINDING_TABLE**
  - **文件与行号**：`scripts/audit/protocol-binding-audit.ts:70-75`（`NA_PATTERNS` 常量）
  - **问题说明**：现有 4 条模式仅覆盖「无跨端协议」「仅内部不动协议」「基础设施」「纯测试」，而 T-0000V.md 的声明 `N/A — 纯文档清理任务，不涉及跨端 API/WS 路径变更` 未被任何模式匹配，被分入 `missingTableFiles`，触发虚假 P1 MISSING_BINDING_TABLE。
  - **实验证据**：`tests/protocol-audit/report.json` → `p1Warnings[0].tdsFile = "doc/tds/infra/T-0000V.md"`, `type = "MISSING_BINDING_TABLE"`
  - **修复建议**：在 `NA_PATTERNS` 中补充：
    ```typescript
    /N\/A.*纯文档/,
    /N\/A.*不涉及跨端/,
    ```
  - **TDD 修复记录**：在 `NA_PATTERNS` 数组末尾新增 `/N\/A.*纯.*文档清理/` 和 `/N\/A.*不涉及.*跨端/` 两条模式（`protocol-binding-audit.ts` 第 74-75 行）。修复后 `Missing tables: 0`，T-0000V 不再触发 MISSING_BINDING_TABLE。Commit: 94e12a5

- [ ] **缺陷 3**：[级别 P1] **`tests/protocol-audit/report.json` 和 `report.md` 未写入 `.gitignore`，已被 git 追踪**
  - **文件与行号**：`.gitignore`（整文件缺失条目）；对应 TDS 交付物要求：`doc/tds/infra/T-0000T.md:98`
  - **问题说明**：TDS 明确将 `.gitignore` 追加 `tests/protocol-audit/report.*` 列为交付物，但该条目实际未添加。`git status` 确认两文件已被追踪并在运行脚本后显示为 modified，每次 CI 运行后都产生脏工作区。
  - **实验证据**：
    ```bash
    git status tests/protocol-audit/
    # 修改：tests/protocol-audit/report.json
    # 修改：tests/protocol-audit/report.md
    grep "protocol-audit" .gitignore  # → 无输出
    ```
  - **修复建议**：`.gitignore` 追加：
    ```
    # 协议审计运行时报告（.gitkeep 保留目录，report.* 忽略）
    tests/protocol-audit/report.json
    tests/protocol-audit/report.md
    ```
    并执行 `git rm --cached tests/protocol-audit/report.json tests/protocol-audit/report.md`。
  - **TDD 修复记录**：`.gitignore` 追加两条忽略规则（`tests/protocol-audit/report.json` 和 `report.md`）；执行 `git rm --cached` 将已追踪文件移出版本控制；`git status tests/protocol-audit/` 确认不再显示 modified。Commit: 94e12a5

- [ ] **缺陷 4**：[级别 P1] **`audit:protocol` 未接入 CI Pipeline，自动化阻断目标未落地**
  - **文件与行号**：`.github/workflows/ci.yml`（整文件）
  - **问题说明**：TDS 核心目标为「接入 CI 门禁，将协议路径一致性校验从人工目测升级为自动化阻断」；契约文档 `doc/arch/infra/protocol-binding-audit.md` 注释「PR 触发 audit:protocol，P0 时阻断合并」。但 `ci.yml` 中无任何 `audit:protocol` step，该 npm script 只能手动运行，不具备门禁效果。
  - **实验证据**：
    ```bash
    grep -n "audit" .github/workflows/ci.yml  # → 无输出
    ```
  - **修复建议**：在 `ci.yml` 中新增 step（建议作为独立 job 或追加到现有 web-check 后）：
    ```yaml
    - name: Protocol Binding Audit
      run: npm run audit:protocol
    ```
  - **TDD 修复记录**：在 `.github/workflows/ci.yml` 新增独立 job `protocol-audit`，包含 `actions/checkout`、`actions/setup-node`（node 20）、`npm ci`、`npm run audit:protocol` 四步，PR 触发时自动运行，P0 时阻断合并。`grep -n "audit" .github/workflows/ci.yml` 确认存在。Commit: 94e12a5

- [ ] **缺陷 5**：[级别 P2] **契约文档报告命名（带日期戳）与实际脚本输出（无日期）不一致**
  - **文件与行号**：
    - `doc/arch/infra/protocol-binding-audit.md`（输出路径表格）：`report-YYYY-MM-DD.json`
    - `scripts/audit/protocol-binding-audit.ts:795,799`：实际写入 `report.json` / `report.md`
  - **问题说明**：契约文档与实现不一致，按文档寻找带日期报告文件将失败。
  - **修复建议**：更新 `protocol-binding-audit.md` 输出路径表格为 `report.json` / `report.md`（与脚本实现对齐）。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

---

**T-0000V 审查结果：**

经独立执行验证命令，T-0000V 所有交付产物满足验收标准，**无缺陷**：

| 验收标准 | 验证命令 | 结果 |
|---------|---------|------|
| AC-01: `doc/ARCHITECTURE.md` 不存在 | `[ ! -f doc/ARCHITECTURE.md ]` | ✅ DELETED - PASS |
| AC-02: ADR-0002 存在且完整 | `ls -la doc/adr/ADR-0002*.md` | ✅ 4849 字节，D-1/D-2/D-3 三条决策齐全 |
| AC-03: §8.2 无过时 JSON，明确指向 protocol/index.md | `grep -n "APPLY_SEAT\|protocol/index.md" doc/architecture/websocket_and_state.md` | ✅ APPLY_SEAT 仅出现于"禁止"语境；protocol/index.md 直接引用存在（行 18） |
| AC-04: 无悬空引用 | `grep -rn "doc/ARCHITECTURE\.md" . --include="*.md" \| grep -v ADR-0002 \| grep -v T-0000V` | ✅ 无命中 |

ADR-0002 独立复核（TDS §六自评结论验证）：D-1/D-2/D-3 内容基于真实项目历史（cf899bd / BUG-CHAT-WS Round 16），每条决策均有可操作标准，结论客观。注：D-3 依赖 T-0000T 正确落地，后者当前有 P0/P1 缺陷，ADR 自身无问题，属执行层问题。

---

**综合结论**: ❌ T-0000T 存在 **1 个 P0 + 3 个 P1 + 1 个 P2** 问题，T-0000V 全部通过。

P0 根本问题：ERE 括号未转义导致客户端 grep 全部静默失败，"三角对账"核心功能失效，CI 门禁产生假阳性永远阻断，架构治理目标无法实现。必须修复全部 P0/P1 缺陷方可放行。

*(文档头部状态机已更新为：`负责人 [TDD] | 状态 [❌ Failed] | 修复轮次 [1/10]`)*

---
