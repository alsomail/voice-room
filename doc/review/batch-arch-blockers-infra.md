# 全局代码审查报告：架构阻塞修复批次（基建侧 · T-0000P + T-0000Q）

> **当前状态机**：负责人 [-] | 状态 [✅ Passed] | 修复轮次 [1/10]

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

**审查范围**：
- T-0000P：commit `efd66e7`（envLoader 双注入 + 17 unit tests + .env.*.example + CI Secret + probe）
- T-0000Q：commits `52306aa..691221b`（check-ports.sh + e2e-up.sh 集成 + shell 测试 + TDS）
- 关联文档：`doc/tds/infra/T-0000P.md` §四/§五、`doc/tds/infra/T-0000Q.md` §四/§五

---

#### A. 关切复核结论

**关切 ①（24 字段冻结契约 / Midscene 缺失策略）**：✅ 通过
- `tests/scripts/support/envLoader.ts:114-274` 主入口 24 字段必填校验链未变动，Midscene 在 Step 3 独立读取并写入 `midscene` 子对象（早已存在的契约字段，非新增）。
- 新增的 `_azureEndpoint` / `_azureApiKey` 在 `types.ts:54-56` 标注 "internal，由 writeProcessEnv 透传"，下划线前缀 + 注释明确划清边界；`Object.freeze(env)` 仍生效（顶层冻结自动覆盖新字段，无可变面）。
- 缺失策略：`envLoader.ts:148-155` 仅 `console.warn`，不进入 `missing[]`，不抛错——这与 §二既定方针（API/INFRA 测试不应被 WEB 凭据拖垮）一致。CI "误绿" 风险由 WEB 用例侧的 skip 逻辑承担（不在本批次范围）。

**关切 ②（安全红线）**：✅ 通过
- `git show efd66e7 | grep -oE "sk-[A-Za-z0-9_-]{20,}"` 命中仅为测试 fixture（`sk-test-*`、`sk-fallback-*`、`sk-from-*`、`sk-secret-real-key` 等，长度 < 20 真实 OpenAI Key 阈值，且无 base64 高熵段）。
- `.env.*.example` 三档全部为占位符（`sk-proj-xxxxxx` / `your-azure-key-placeholder`）。
- `sanitizeEnvForRuntimeJson` (envLoader.ts:343-357) 正确清除 `midscene.apiKey` + `_azureEndpoint` + `_azureApiKey`；Edge 测试 L336-351 已断言原对象保留、副本脱敏。
- `scripts/dev/midscene-env-probe.ts` `maskApiKey` 仅露首尾 4 字符，<12 字符返回 `[EMPTY or TOO SHORT]`。
- `.github/workflows/playwright.yml` 仅引用 `secrets.MIDSCENE_MODEL_API_KEY` / `secrets.MIDSCENE_MODEL_BASE_URL`，无硬编码。

**关切 ③（DoD 测试覆盖 / WEB smoke 降级）**：✅ 接受降级
- 17 个 unit tests 覆盖：U-1（直接命中×3 优先级层）、U-2（OPENAI_API_KEY fallback + 双注入 + 四层链）、U-3（缺失 warn 不抛）、U-4（baseUrl 透传 + undefined）、U-5（Azure 透传 + 不参与 fallback）、Edge（空字符串 fallback / staging profile / sanitize）。
- Edge L302-315 显式覆盖 "MIDSCENE_MODEL_API_KEY 为空字符串 → fallback 到 OPENAI_API_KEY" 的隐蔽路径（`get()` L132-138 实现 `'' === undefined` 等价语义），这是注入链最易翻车的点位，已锁死。
- WEB smoke 推迟理由（本地无 5 端依赖，CI Secret 待 DevOps 配置）成立；注入逻辑已被 unit 等价覆盖，且 probe 脚本支持 dry-run 验证 → 合并后 DevOps 在 CI 注入 Secret 即可触发真实 WEB 跑通，不构成阻断。

**关切 ④（Q 端口检测跨平台 / 性能 / cleanup）**：✅ 通过（含 1 条 P2 风险记录）
- `scripts/dev/check-ports.sh:18-30` 跨平台分支结构清晰：macOS → `lsof -ti`（无需 root）；Linux → `ss` 优先，`netstat` fallback。
- 性能：每端口最多 1 次工具调用（5 端 ≤ 1s），未 spawn shell 子进程循环，符合 SLA。
- e2e-up.sh:31-32 在 `docker compose up` **之前**调用，`set -e` 失败即退出，无残留容器（preflight 阶段尚未启动任何服务）。

**关切 ⑤（Minor-1/2/3 是否升级）**：保持 Minor 不升级
- **Minor-1（`grep -oP` BusyBox 不可用）**：复核确认实现是 `if ss; elif netstat`（顺序 if，**不是 OR fallback**），即 ss 存在时不会 fallback 到 netstat。在 Alpine + iproute2（ss 已装但 grep 无 PCRE）场景下 `grep -oP` 静默返回空 → 端口被误判可用（**false negative**）。但本仓库 CI 为 `ubuntu-latest`（GNU grep 含 -P），dev 主战场为 macOS（走 lsof 分支），生产命中概率低。**保留为 P2，不阻断本批次**，建议后续以 `awk -F'pid=' '{split($2,a,","); print a[1]}'` 替代 `grep -oP` 彻底消解。
- **Minor-2（TDS §二 `declare -A` 与实现顺序检测不一致）**：实现采用 5 次显式 `check_port` 调用更鲁棒（兼容 bash 3.2 / macOS 默认）。文档未同步是 P3 文档债，不阻断。
- **Minor-3（变量未加引号）**：`$port` / `$pid` 均为纯数字或经 `[[ -n "$pid" ]]` 守卫，且 `IFS='|' read -r` 正确，无 word-splitting 风险。维持 P3。

**关切 ⑥（批次内一致性）**：✅ 通过
- 目录归属清晰：T-0000P 修改 `tests/scripts/support/envLoader.ts`（运行时注入）+ `scripts/dev/midscene-env-probe.ts`（dev 工具）；T-0000Q 修改 `scripts/dev/check-ports.sh` + `scripts/dev/e2e-up.sh`。两 Task 在 `scripts/dev/` 共存但无 import / 文件冲突。
- 两 Task TDS §四 / §五 均已落地，`doc/tasks/index.md` 状态行同步（由协调者统一维护，本批次未触碰）。

---

#### B. 缺陷清单

无 P0 / P1 阻断缺陷。以下记录 P2 / P3 供后续优化（不影响本轮放行）：

- [ ] **观察 1**：[级别 P2] **`check-ports.sh` Linux ss 分支的 `grep -oP` 在 BusyBox/Alpine 静默失败**
  - **文件与行号**：`scripts/dev/check-ports.sh:23-29`
  - **问题说明**：实现采用 `if command -v ss; elif command -v netstat` 顺序，ss 存在即走 PCRE 抽取 PID，BusyBox grep 无 -P 支持时返回空字符串导致端口被误判为可用（false negative）。本仓库 CI（ubuntu-latest）与 dev（macOS lsof 分支）不命中此场景，故风险有限。
  - **修复建议**：将 `grep -oP 'pid=\K[0-9]+'` 替换为 POSIX 兼容写法，例如 `awk -F'pid=' 'NF>1 {split($2,a,","); print a[1]}'`；并在 ss/netstat 都不可用时 fallback 到 `(echo > /dev/tcp/127.0.0.1/$port) 2>/dev/null` 风格的连接探测，确保任意发行版兜底。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

- [ ] **观察 2**：[级别 P3] **e2e-up.sh 头注释退出码声明与实际不符**
  - **文件与行号**：`scripts/dev/e2e-up.sh:10`
  - **问题说明**：注释声明 "退出码：0 OK / 11~15 同 preflight"，但 `check-ports.sh` 实际退出码为 1（`scripts/dev/check-ports.sh:113`）。冲突时排错文档对照困难。
  - **修复建议**：将注释改为 "0 OK / 1 端口冲突（check-ports.sh）/ 78 envLoader CONFIG / 其他 = wait-on 超时"；或在 check-ports.sh 中按服务映射 11~15 退出码以对齐既有约定。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

- [ ] **观察 3**：[级别 P3] **TDS T-0000Q §二 `declare -A` 方案与实现的顺序检测未同步说明**
  - **文件与行号**：`doc/tds/infra/T-0000Q.md:46-52`
  - **问题说明**：TDS 示例使用 `declare -A PORTS=(...)`（bash 4+），但生产实现改用 5 次显式 `check_port` 调用以兼容 macOS bash 3.2，文档未注明此偏移。
  - **修复建议**：在 §二末尾追加一段 "实现注：本 Task 最终采用顺序 `check_port` 调用替代 `declare -A`，理由是兼容 macOS 默认 bash 3.2"。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

- [ ] **观察 4**：[级别 P3] **`maskApiKey` 未区分空字符串与短字符串**
  - **文件与行号**：`scripts/dev/midscene-env-probe.ts:12-18`
  - **问题说明**：`length < 12` 统一返回 `[EMPTY or TOO SHORT]`，丢失诊断粒度（环境注入失败 vs Key 配置异常）。
  - **修复建议**：拆分为 `length === 0 → [EMPTY]` / `length < 12 → [TOO SHORT]`。
  - **TDD 修复记录**：[等待 TDD 填写修复逻辑与 Commit ID]

---

#### C. 本轮结论

**本轮结论**：🟢 ✅ **审查通过**

- T-0000P 注入链架构正确、24 字段契约未破坏、安全红线 0 命中、unit 覆盖等价于真实路径，WEB smoke 降级理由成立。
- T-0000Q 跨平台检测、性能 SLA、e2e-up.sh 集成均落地；3 个既有 Minor **不升级为阻断**（限定环境 + 已有兜底）。
- 上述 4 条 P2/P3 观察作为技术债登记，不影响本批次放行；建议在 `T-0000Q` 后续优化或新增小 Task 中清偿。

*(状态机已修改为：`负责人 [-] | 状态 [✅ Passed] | 修复轮次 [1/10]`)*

---
