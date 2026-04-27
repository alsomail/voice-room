# 全局代码审查报告：架构阻塞修复批次（基建侧 · T-0000P + T-0000Q）

> **当前状态机**：负责人 [GlobalReview] | 状态 [⏳ In Review] | 修复轮次 [0/10]

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

- **批次定位**：QA 战报反向拆出的 6 个架构阻塞 Task 中，**基建侧 2 个**的合并审查批次。AppServer 侧 4 个（T-00041~44）由其它批次承接，本批次不重复审。
- **批次范围**：模块 9 E2E 测试基建在 v2.55 之后增量补的两个基建项。模块 9 主体 13+1 个任务在 [模块9-E2E测试基建.md](./模块9-E2E测试基建.md) 已闭环；本批次仅审 T-0000P / T-0000Q 两条增量。
- **包含任务**：
  - **T-0000P**：Midscene env 注入链补齐（envLoader 双注入 `MIDSCENE_MODEL_API_KEY` + fallback `OPENAI_API_KEY`，+ `.env.{local,staging,prod}.example` 占位 + `.github/workflows/playwright.yml` Secret 注入 + 17 unit tests）
  - **T-0000Q**：`scripts/dev/e2e-up.sh` 端口冲突 preflight（5432/6379/3000/3001/5173 五端，跨平台 lsof/ss/netstat，彩色错误 + PID/进程名 + `kill -9` 提示）
- **关联 TDS**：
  - [T-0000P](../tds/infra/T-0000P.md)（§5 已记录单 Task Round 1 GlobalReview 🟢，commit `efd66e7`）
  - [T-0000Q](../tds/infra/T-0000Q.md)（§5 已记录单 Task Round 1 GlobalReview ✅，commits `52306aa..691221b`）
- **代码 diff 范围**：
  - T-0000P 主提交：`efd66e7`（envLoader 双注入 + .env.example 三档 + playwright.yml + envLoader.midscene.test.ts）
  - T-0000Q 提交链：`52306aa`(RED) → `fadd37b`(GREEN) → `82238d6`(集成 e2e-up.sh) → `0cde7fb`(集成测试) → `691221b`(文档)
  - 后续 DoD 同步：`91c47da` / `ad45f5a` / `f280a8a` / `ae62725`
- **开始时间**：2026-04-29

---

## 2. 审查关切（架构级）

本批次核心问题是「QA 战报反向暴露的两个架构阻塞 follow-up，是否在补回时引入新风险或破坏既有契约」。具体关切：

### 关切 ①：T-0000P envLoader Midscene 双注入是否破坏 24 字段契约？
- envLoader 在 Step 6 组装阶段写入 `MIDSCENE_MODEL_API_KEY` + `OPENAI_API_KEY` 双键，是否影响既有的 24 主字段冻结（freeze）契约 / sanitize 流程 / Playwright 进程环境？
- Midscene 缺失时仅 warn 不抛错的策略，是否会让 WEB 用例在 CI 上「误绿」（既不 skip 也不 fail）？

### 关切 ②：T-0000P 安全红线是否守住？
- `.env.*.example` / 单元测试 fixture / probe 脚本是否存在真实 Key 泄露？
- `sanitizeEnvForRuntimeJson` 是否对 `apiKey` / `_azureApiKey` 全部脱敏写盘？

### 关切 ③：T-0000P 测试覆盖是否对得起「DoD」？
- 17 个 unit tests 覆盖 fallback 优先级 / Azure 透传 / 缺失 warn / 双注入幂等是否充分？
- 注意：本 Task 没有跑真实 WEB 用例 smoke（仅 unit + dry-run），单 Task Reviewer 已知会并接受降级理由（"本地无 docker 服务依赖"）。批次评审需复核此降级是否仍然合理。

### 关切 ④：T-0000Q `e2e-up.sh` 端口检测的真生效性
- `lsof` 不可用时 `ss` / `netstat` fallback 链是否真覆盖 macOS / Linux / Alpine / BusyBox？
- 端口检测开销是否真 < 1s（不破坏 e2e:up 启动 SLA）？
- 集成进 e2e-up.sh 的失败行为：冲突时退出码 1 + 不留半启动容器的 cleanup 是否健壮？

### 关切 ⑤：T-0000Q 单 Task Reviewer 已列 3 个 Minor，批次复核是否升级阻断？
- Minor-1：Linux `grep -P` 在 BusyBox 不可用 → fallback 路径是否真生效。
- Minor-2：TDS §二方案描述 `declare -A`（bash 4+），实现改用顺序检测（更鲁棒）→ 文档与实现是否同步。
- Minor-3：变量未加引号 → 端口为纯数字，是否确认无 word-splitting 风险。

### 关切 ⑥：批次内一致性
- 两个 Task 都修改 `tests/scripts/`（envLoader）/ `scripts/dev/`（e2e-up.sh）等基建脚本；是否有目录归属冲突或重叠 import？
- 两个 Task 的 DoD 文档（doc/arch/infra、doc/arch/web、doc/product）是否都同步更新？

---

## 2. 审查与修复日志

*(执行规则：GlobalReview 记录缺陷，TDD 在对应缺陷下方记录修复方案与 PR/Commit。严禁覆盖历史记录，只能向下追加)*

### 【第 1 轮审查】

**@GlobalReview 审查意见**：

(待 global-code-reviewer 智能体填写)

---
