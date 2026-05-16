# 🤖 核心行为规范与 Agent 指南

在执行任何代码修改或架构设计之前，请务必先读取并严格遵循以下文件中的规则：

- **全局行为规范与代码风格**：请参阅 [`.github/copilot-instructions.md`](./.github/copilot-instructions.md)
- **LLM 编码行为准则**：请参阅 [`doc/LLM_RULES.md`](./doc/LLM_RULES.md)

---

## 🚨 产品验收三红线（v3.38 引入 · Plan / TDD / Review / DoD 全阶段必须遵守）

### 红线 1：Plan 阶段必须有对应 Spec

E-08 及之后所有 Task，TDS §0 必须显式关联 [`doc/specs/<feature>.md`](./doc/specs/)。无 Spec 锚点的 TDS 不完备 → 禁止流转 TDD。

### 红线 2：TDS §0 GWT 编号清单·禁止本地改写

每份 TDS 顶部 §0「产品验收（Given-When-Then）」必须：

1. 4 个事实源锚点：Spec / [状态机](./doc/product/state_machines.md) / [用户旅程](./doc/product/user_journeys.md) / [业务约束](./doc/product/business_constraints.md)。
2. 本 Task 必须满足的 GWT 编号清单（例：`GWT-O1 / GWT-O5`）。
3. 明示声明「GWT 全文以 spec §5 为唯一事实源，禁止本地改写」。TDS §0 中出现任何 GWT 文本重述→**判 P0**。

### 红线 3：test-design Agent 输入四源齐全

测试用例必须以 **Spec §5 GWT + user_journeys + business_constraints + state_machines** 四源齐全为前提。缺源用例 → QA Gate 打回。

### 事实源唯一表

- 状态机：[`doc/product/state_machines.md`](./doc/product/state_machines.md)
- 用户旅程：[`doc/product/user_journeys.md`](./doc/product/user_journeys.md)
- 业务约束：[`doc/product/business_constraints.md`](./doc/product/business_constraints.md)
- 功能簇规约：[`doc/specs/`](./doc/specs/)（14 份）

- **全局行为规范与代码风格**：请参阅 `@.github/copilot-instructions.md`