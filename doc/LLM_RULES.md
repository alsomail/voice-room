# LLM 编码行为准则

### 1. 编码前先思考
**不要猜测。不要掩盖困惑。暴露权衡取舍。**
实现前：
- 明确陈述你的假设。不确定就问。
- 如果存在多种解读，呈现出来——不要默默选一个。
- 如果有更简单的方案，说出来。必要时提出异议。
- 如果有不清楚的地方，停下来。说出困惑点。询问。

### 2. 简单优先
**用最少的代码解决问题。不做投机性扩展。**
- 不实现需求之外的功能。
- 不为一次性代码创建抽象。
- 不做没有被要求的"灵活性"或"可配置性"。
- 不处理不可能发生的场景的错误。
- 如果写了 200 行但 50 行就够了，重写。

自问："高级工程师会说这太复杂了吗？"如果是，简化。

### 3. 精准修改
**只动必须改的地方。只清理你自己制造的混乱。**
编辑现有代码时：
- 不"优化"相邻代码、注释或格式。
- 不重构没有问题的东西。
- 沿用现有风格，即使你会选择不同的方式。
- 发现无关的死代码，提及它——不要删除它。

当你的改动产生孤儿代码时：
- 移除**你的改动**导致的未使用的 import / 变量 / 函数。
- 不移除预先存在的死代码，除非被要求。

检验标准：每一行改动都应直接溯源到用户的请求。

### 4. 目标驱动执行
**定义成功标准。循环直到验证通过。**
将任务转化为可验证的目标：
- "添加校验" → "为非法输入写测试，然后让它们通过"
- "修复 bug" → "写一个复现它的测试，然后让它通过"
- "重构 X" → "确保测试在重构前后都通过"

对于多步骤任务，陈述简短计划：
```
1. [步骤] → 验证：[检查点]
2. [步骤] → 验证：[检查点]
3. [步骤] → 验证：[检查点]
```

### 5. 可视化表达优先

**用图表和表格传达信息，而不是长段文字。**

- 多个选项/方案对比 → 用**表格**
- 流程/调用链/依赖关系 → 用 **Mermaid 流程图**
- 架构层级/目录结构 → 用**树形图**或 ASCII 图
- 步骤序列 → 用**编号列表**而非散文
- 数据结构关系 → 用 **ER 图**或 UML 类图

能用图说清楚的，不用段落堆砌。

---

## 🚨 产品验收三红线（v3.38 引入，Plan / TDD / Review / DoD 全阶段必须遵守）

随 product 边界三件套（`doc/product/state_machines.md` / `user_journeys.md` / `business_constraints.md`）与 `doc/specs/` 功能簇规约（14 份）一同上架，附加以下三条红线：

### 红线 1：Plan 阶段必须有对应 Spec

- E-08 及之后的所有 Task，TDS 顶部 §0 必须显式关联 `doc/specs/<feature>.md`。
- 历史已 Done Task 不回填；E-08/E-09 的 37 份 TDS 已批量补齐 §0。
- 无 Spec 锚点的 TDS 视为不完备，**禁止流转 TDD**。

### 红线 2：TDS §0 必须列 GWT 锚点清单，**禁止本地改写**

每份 TDS 顶部 §0「产品验收（Given-When-Then）」段必须包含：

1. **4 个事实源锚点**：Spec / 状态机 / 用户旅程 / 业务约束。
2. **本 Task 必须满足的 GWT 编号清单**（例：`GWT-O1 / GWT-O2 / GWT-O5`）。
3. 明确声明：**GWT 全文以 spec §5 为唯一事实源，禁止本地改写**。

TDS §0 中出现任何 GWT 文本重述/改写/裁剪 → **直接判 P0 缺陷**，Review 打回。

### 红线 3：test-design Agent 输入必须四源齐全

测试用例设计必须以下列四源齐全为前提：

| 事实源 | 用途 |
|---------|------|
| `doc/specs/<feature>.md` §5 GWT | 验收契约（正向主路径 + 边界 + 异常分支） |
| `doc/product/user_journeys.md` | 端到端跨端流 |
| `doc/product/business_constraints.md` | 边界常量（TTL / 上限 / 阈值） |
| `doc/product/state_machines.md` | 状态转换矩阵 |

任何缺源的测试用例视为不完备，**QA Gate 直接打回**。

### 关联事实源（唯一源表）

- 状态机：[`doc/product/state_machines.md`](./product/state_machines.md)
- 用户旅程：[`doc/product/user_journeys.md`](./product/user_journeys.md)
- 业务约束：[`doc/product/business_constraints.md`](./product/business_constraints.md)
- 功能簇规约：[`doc/specs/`](./specs/)（14 份：auth_login / room_lifecycle / room_chat / mic_seat / rtc_voice / gift_economy / ranking_leaderboard / analytics_funnel / room_governance / admin_dashboard / recharge_order / google_play_billing / nobility_purchase / nobility_privileges）

---