# 测试套件：CI 字段审计工具验证（Protocol Audit CI）

> **需求模糊点 (Ambiguity Notes)**：
> - `scripts/audit/validate-protocol-freeze.sh` 和 `scripts/audit/validate-tds-field-binding.sh` 的确切脚本路径及 P0/P1 输出格式未在协议文档中明确规范；本套件以 T-00106 实现结果中的 `scripts/audit/protocol-binding-audit.ts`（`npm run audit:fields`）和 T-0000T 的 `protocol-binding-audit.ts` 为基准，若脚本路径有变则同步更新。
> - AUDIT-01 和 AUDIT-03 中"P0=0"的输出格式假设为脚本 stdout 包含类似 `"P0 issues: 0"` 或 exit code=0；具体格式需对齐 T-00106 实现的报告格式。
> - AUDIT-02 中"注入 camelCase 字段"的操作需在受控沙箱/临时分支中执行，测试后必须恢复源文件，禁止污染主干代码。

---

## TC-AUDIT-00001：`validate-protocol-freeze.sh` 基准通过（PROTO-FREEZE P0=0）

**【元数据】**
- **归属模块**：`AUDIT`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. 工作目录为项目根目录 `/Users/yuanye/myWork/voice-room`。
2. Node.js 和 `npx`/`ts-node` 可用（`node --version` 输出版本号）。
3. `doc/protocol/schemas/ws/`、`doc/protocol/schemas/pubsub/`、`doc/protocol/schemas/http/` 目录下所有 `*.schema.json` 文件均为已冻结的合规状态（Phase 1.7-extended 全量冻结）。
4. `scripts/audit/protocol-binding-audit.ts` 脚本已存在，`package.json` 中 `audit:fields` script 已配置（T-00106 实现结果）。
5. 所有三端源代码（`app/server/`、`app/android/`、`app/web/`）的协议字段已与 schema 对齐。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                         | 预期结果 (Assertion)                                                                                                                                                        |
| :------: | :---------- | :------------------------------------------------------------------------------------------------------------------------ | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `AppServer` | 在项目根目录执行：`npm run audit:fields`                                                                                  | 命令开始执行，无立即报错                                                                                                                                                    |
|    2     | `AppServer` | 等待脚本执行完毕（超时 60s），检查 exit code                                                                              | exit code = **0**（无 P0 问题）                                                                                                                                             |
|    3     | `AppServer` | 检查脚本 stdout 输出内容                                                                                                  | stdout 中包含 `"P0 issues: 0"` 或等效文本（表示无字段级 P0 违规）；如有 P1 问题，数量 ≥ 0 均可接受（P1 不阻塞）                                                             |
|    4     | `AppServer` | 检查生成的审计报告文件（如 `scripts/audit/report.md` 或 `report.json`，具体路径以 T-00106 实现为准）                      | 报告文件存在；内容中无 `"P0"` 级别问题条目；若有 Markdown 报告，每个信令条目均标注 `✅` 通过状态                                                                             |
|    5     | `AppServer` | 执行 TypeScript 编译检查：`npx tsc --project tsconfig.cross-lang.json --noEmit`                                           | 零编译错误，零警告                                                                                                                                                          |

**【数据清理】**
- 无 DB 数据需清理。
- 删除审计过程中生成的临时报告文件（如不需要保留）。

---

## TC-AUDIT-00002：注入 camelCase 字段后，validate 脚本检出 P0

**【元数据】**
- **归属模块**：`AUDIT`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. 工作目录为项目根目录，脚本可正常执行（TC-AUDIT-00001 基准通过）。
2. 测试在 **临时副本** 或 **Git stash** 保护下执行，保证原始文件可恢复。
3. 目标注入文件：`app/server/src/room/handler/mic.rs`（或任一包含 `json!({...})` 宏的 server 源文件）。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                                                             | 预期结果 (Assertion)                                                                                                                               |
| :------: | :---------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `AppServer` | 执行 `git stash` 保存当前工作区状态                                                                                                                                                           | 命令输出 `Saved working directory`，工作区干净                                                                                                     |
|    2     | `AppServer` | 手动编辑 `app/server/src/room/handler/mic.rs`，将某处 `json!` 宏中的 `"mic_index"` 改为 `"micIndex"`（camelCase 注入）：将 `"mic_index": mic_index` 改为 `"micIndex": mic_index`             | 文件保存成功                                                                                                                                       |
|    3     | `AppServer` | 在项目根目录执行：`npm run audit:fields`                                                                                                                                                       | 命令执行完毕                                                                                                                                       |
|    4     | `AppServer` | 检查 exit code                                                                                                                                                                                 | exit code = **1**（检出 P0 问题，CI 阻塞）                                                                                                         |
|    5     | `AppServer` | 检查 stdout/stderr 输出内容                                                                                                                                                                    | 输出中包含 `"P0"` 级别告警；包含 `"micIndex"` 字段名；包含注入文件路径（`app/server/src/room/handler/mic.rs`）；包含行号（file:line 锚点）         |
|    6     | `AppServer` | 执行 `git stash pop` 恢复原始文件                                                                                                                                                              | 命令输出 `Applied stash`，`app/server/src/room/handler/mic.rs` 恢复为原始内容                                                                      |
|    7     | `AppServer` | 恢复后再次执行 `npm run audit:fields`                                                                                                                                                          | exit code = 0（P0=0，恢复验证成功）                                                                                                                |

**【数据清理】**
- 确保 `git stash pop` 已执行，源文件已恢复。
- 执行 `git status` 确认工作区干净（无 modified 文件残留）。

---

## TC-AUDIT-00003：TDS 绑定表格式校验（validate-tds-field-binding.sh）P0=0

**【元数据】**
- **归属模块**：`AUDIT`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. 工作目录为项目根目录。
2. `scripts/audit/protocol-binding-audit.ts`（T-0000T 原始路径级审计脚本）可执行，`package.json` 中 `audit:protocol` 或 `audit` script 已配置。
3. `doc/tds/` 目录下所有 TDS 文件的协议路径绑定表（`### 🔌 协议路径绑定表`）已与实现对齐（T-00100 至 T-00108 均标记 Done）。
4. 所有 TDS 绑定表中引用的 Schema 文件（`doc/protocol/schemas/ws/*.schema.json`）均存在。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                          | 预期结果 (Assertion)                                                                                                                                                                    |
| :------: | :---------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `AppServer` | 执行原始协议绑定审计脚本：`npm run audit:protocol`（或等效命令，以实际 `package.json` script 名为准）                                                     | 命令开始执行                                                                                                                                                                            |
|    2     | `AppServer` | 等待执行完毕（超时 60s），检查 exit code                                                                                                                   | exit code = **0**                                                                                                                                                                       |
|    3     | `AppServer` | 检查 stdout 中的测试计数                                                                                                                                   | 输出包含类似 `"53 tests passed"` 或测试通过数量 ≥ 53（T-0000T 原有测试基线，T-00106 §三验收 REGRESSION 项）；**零回归**（0 failed）                                                     |
|    4     | `AppServer` | 检查 TDS 绑定表中所有 schema 文件引用是否存在：`find doc/protocol/schemas/ws -name "*.schema.json" | wc -l`                                               | 返回数量 ≥ 35（对应 `doc/protocol/schemas/ws/` 目录下全部 schema 文件）                                                                                                                 |
|    5     | `AppServer` | 检查字段级审计脚本：`npm run audit:fields`                                                                                                                  | exit code = 0；stdout 输出 `P0 issues: 0`；字段级 9 个单元测试全绿（`scripts/audit/__tests__/field-level-audit.test.ts`）                                                               |

**【数据清理】**
- 无 DB 数据需清理。

---

## TC-AUDIT-00004：故意移除 TDS 绑定表行，脚本检出缺失

**【元数据】**
- **归属模块**：`AUDIT`
- **测试类型**：`Functional`
- **回归级别**：`P0`

**【前置条件】**
1. 工作目录为项目根目录，TC-AUDIT-00003 基准已通过。
2. 测试在 **临时副本** 或 **Git stash** 保护下执行。
3. 目标 TDS 文件：`doc/tds/infra/T-00104.md`（T-00104 协议路径绑定表有 19 行）。

**【执行步骤与断言】**
| 步骤序号 | 目标端      | 操作动作 (Action)                                                                                                                                                                                                               | 预期结果 (Assertion)                                                                                                                                   |
| :------: | :---------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | :----------------------------------------------------------------------------------------------------------------------------------------------------- |
|    1     | `AppServer` | 执行 `git stash` 保存工作区状态                                                                                                                                                                                                 | `git stash` 成功                                                                                                                                       |
|    2     | `AppServer` | 手动编辑 `doc/tds/infra/T-00104.md`，在 `### 🔌 协议路径绑定表` 中删除第 3 行（JoinRoom 信令绑定行，即包含 `JoinRoom` 和 `schemas/ws/JoinRoom.schema.json` 的那一行）                                                         | 文件保存成功                                                                                                                                           |
|    3     | `AppServer` | 执行：`npm run audit:protocol`（或协议绑定审计脚本）                                                                                                                                                                            | 命令执行完毕                                                                                                                                           |
|    4     | `AppServer` | 检查 exit code                                                                                                                                                                                                                  | exit code = **1**（检出绑定表缺失行）                                                                                                                  |
|    5     | `AppServer` | 检查 stdout/stderr 输出内容                                                                                                                                                                                                     | 输出中包含缺失信令名 `"JoinRoom"` 或被删行对应的文件路径；包含 `"missing"` 或 `"not found"` 或 `"P0"` 关键词；包含 TDS 文件路径 `T-00104.md`           |
|    6     | `AppServer` | 执行 `git stash pop` 恢复 TDS 文件                                                                                                                                                                                              | `doc/tds/infra/T-00104.md` 恢复原始内容                                                                                                               |
|    7     | `AppServer` | 执行 `npm run audit:protocol` 验证恢复效果                                                                                                                                                                                      | exit code = 0（P0=0，恢复后回归通过）                                                                                                                  |

**【数据清理】**
- 确保 `git stash pop` 已执行，TDS 文件已恢复。
- 执行 `git status` 确认工作区无 modified 文件残留。
