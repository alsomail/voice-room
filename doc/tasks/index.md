# Voice Room 开发任务清单

> **版本**: v2.60  
> **更新日期**: 2026-04-29  
> **任务总数**: 130 个 (基建: 4 + 12, App Server: 33 + 1, Admin Server: 16 + 1, Web: 14 + 1, Android: 44 + 1, E-07 15 + E-07.5 6 + E-10 18)  
> **当前阶段**: Phase 1 - 核心营收闭环（E-07 + E-07.5 并行）→ Phase 1.5 E-10 房间治理 → Phase 1.6 E2E 测试基建（模块 9：14/15 ✅，T-0000P ✅ DoD，T-0000Q ✅ DoD） + QA 战报驱动架构补强（T-00041 ✅ DoD，T-00043 TDD→Review，T-00044 TDD→Review/DoD）

---

## 🔄 重要变更说明

| 版本 | 日期 | 变更内容 |
|------|------|---------|
| _规则_ | — | 本表只记录**版本级摘要**（一行 ≤ 200 字符），具体 Review/审查/实跑证据请落到对应 [TDS](../tds/) 第五节【Review 意见】或对应模块审查批次 `doc/review/模块N-XXX.md`，**严禁**在本表堆叠详细审查记录。 |
| **v2.60** | **2026-04-29** | T-00042 DoD 完成；TDD [2109c06](https://github.com/alsomail/voice-room/commit/2109c06) + R1 修复 [1f10ec3](https://github.com/alsomail/voice-room/commit/1f10ec3) 🟢 R2 通过；Admin 强制断连广播（user_banned/room_closed → connection_close 指令 → WS Close frame）；详见 [TDS](../tds/server/T-00042.md)。 |
| **v2.59** | **2026-04-29** | T-00043 Review Round 1 → TDD 修复 6 项 Should（CASCADE/排序/真DB并发/真DB性能/COUNT(*) OVER()/offset 软上限）→ Round 2 🟢 通过；commits [a191123](https://github.com/alsomail/voice-room/commit/a191123) 修复，[ec0c935](https://github.com/alsomail/voice-room/commit/ec0c935) 状态。详见 [TDS](../tds/server/T-00043.md) §4.5/§五。 |
| **v2.58** | **2026-04-29** | T-00043 TDD → Review，chat_messages 持久化 + REST 历史接口落地（migration 010 + 14 dedicated tests + 464 server suite 全绿）；commit [1beb68b](https://github.com/alsomail/voice-room/commit/1beb68b)。详见 [TDS](../tds/server/T-00043.md)。 |
| **v2.58** | **2026-04-29** | T-00041 DoD 完成；TDD [084f91e](https://github.com/alsomail/voice-room/commit/084f91e) + Review Round 1 🟢 [a8c0a64](https://github.com/alsomail/voice-room/commit/a8c0a64)；修复历史漏 spawn BUG，WS 心跳 30s 超时主动 Close(1000)。详见 [TDS](../tds/server/T-00041.md)。 |
| **v2.57** | **2026-04-29** | T-0000P DoD 完成（Midscene env 注入链 + 双注入 + 脱敏），模块 9 进度 14/15。详见 [TDS](../tds/infra/T-0000P.md)。 |
| **v2.56** | **2026-04-29** | T-0000Q DoD 完成（e2e-up.sh 端口冲突预检 5 端，跨平台 lsof/ss）。详见 [T-0000Q TDS](../tds/infra/T-0000Q.md)。 |
| **v2.56** | **2026-04-29** | T-0000P TDD → Review，Midscene env 注入链落地（envLoader + .env.example + CI workflow + 17 unit tests）。详见 [T-0000P TDS](../tds/infra/T-0000P.md)。 |
| **v2.56** | **2026-04-29** | T-00044 TDD → Review，HTTP 礼物端点复用 WS 事务，新增 7 个 HTTP 测试 + 12 WS 回归全绿。详见 [TDS](../tds/server/T-00044.md)。 |
| **v2.55** | **2026-04-29** | QA 战报反向拆出 6 个新 Task（T-00041~44 App Server + T-0000P/Q 基建），全部 Plan→TDD 流转；ARCH 阻塞（WS 心跳超时/Admin 强制断连/Chat 持久化/礼物 HTTP 端点/Midscene env 链/端口冲突检测）。详见各 TDS。 |
| **v2.54** | **2026-04-28** | T-0000O DoD 完成（ranking r08 perf flake known-issue 收口）；TDD [b793252](https://github.com/alsomail/voice-room/commit/b793252) + Review [ae20b9f](https://github.com/alsomail/voice-room/commit/ae20b9f) 🟢。详见 [TDS](../tds/infra/T-0000O.md)。 |
| **v2.53** | **2026-04-27** | T-0000N TDD → Review Round 1 🟢通过（[TDS](../tds/infra/T-0000N.md)）+ T-0000O 规划中；AppServer/AdminServer 暴露 `/health` 统一探活端点；doc/review/batch-e2e-foundation-01/02.md 合并为 [模块9-E2E测试基建.md](../review/模块9-E2E测试基建.md)。 |
| **v2.52** | **2026-04-27** | T-0000M Round 1 GlobalReview 暴露 DoD 失实，立 T-0000N（/health 端点）+ T-0000O（ranking r08 perf flake known-issue）作为 follow-up；T-0000M DoD #1 措辞修正为符合事实的承诺；P1.1 收敛 14 处遗留 sqlx::migrate! → common helper；P2 helper 透传 no_tx + 注释修正 + RAII guard + N-2 自动化。详见 [模块9-E2E测试基建](../review/模块9-E2E测试基建.md)。 |
| **v2.51** | **2026-04-27** | T-0000M DoD 完成 → 模块 9 全部闭环（13/13 ✅，M4 双服务共库迁移隔离）。详见 [T-0000M TDS](../tds/infra/T-0000M.md)、[ADR-0001](../adr/ADR-0001-migration-table-isolation.md)；doc/arch/* 更新引用；doc/tasks/模块9 状态行 ✅；doc/product/index.md v3.13。 |
| **v2.50** | **2026-04-27** | 模块 9 新增 T-0000M（双服务共库 Migration 表隔离），由 e2e:up 联调暴露架构级阻断；PM→Plan→TDD，TDS 完成。详见 [T-0000M TDS](../tds/infra/T-0000M.md)。 |
| **v2.49** | **2026-06-04** | T-0000L DoD 完成 → 模块 9 全部闭环（12/12 ✅，研发口径）。详见 [T-0000L TDS](../tds/infra/T-0000L.md) §五。 |
| **v2.48** | **2026-06-03** | T-0000L Review Round 1 通过（Review → DoD）。详见 T-0000L TDS。 |
| **v2.47** | **2026-06-03** | T-0000L TDD → Review，新增 `doc/tests/E2E_RUNBOOK.md`。详见 T-0000L TDS。 |
| **v2.46** | **2026-06-03** | T-0000L Plan → TDD，TDS 完成。详见 T-0000L TDS。 |
| **v2.45** | **2026-06-03** | T-0000K DoD 完成入档，M3 文档与一键命令闭环 3/3。详见 [T-0000K TDS](../tds/infra/T-0000K.md)。 |
| **v2.44** | **2026-06-02** | T-0000K Review Round 1 通过（Review → DoD）。详见 T-0000K TDS。 |
| **v2.43** | **2026-06-02** | T-0000K TDD → Review，Midscene LLM 配置文档落地。详见 T-0000K TDS。 |
| **v2.42** | **2026-06-02** | T-0000K Plan → TDD，TDS 完成。详见 T-0000K TDS。 |
| **v2.41** | **2026-06-01** | T-0000J DoD 完成，M1 本地 E2E 跑通闭环。详见 [T-0000J TDS](../tds/infra/T-0000J.md)。 |
| **v2.40** | **2026-04-27** | T-0000J Review Round 1 通过（→ DoD）。详见 T-0000J TDS。 |
| **v2.39** | **2026-05-31** | T-0000J TDD → Review（baseURL 修复 + typo 清理 + @prod-safe 标签）。详见 T-0000J TDS。 |
| **v2.38** | **2026-05-31** | T-0000J Plan → TDD，TDS 完成。详见 T-0000J TDS。 |
| **v2.37** | **2026-05-31** | T-0000I DoD 完成，npm scripts 一键命令落地。详见 [T-0000I TDS](../tds/infra/T-0000I.md)。 |
| **v2.36** | **2026-04-27** | T-0000I Review Round 1 通过（→ DoD）。详见 T-0000I TDS。 |
| **v2.35** | **2026-04-27** | T-0000I TDD → Review，6 个 npm scripts 落地。详见 T-0000I TDS。 |
| **v2.34** | **2026-05-31** | T-0000I Plan → TDD，TDS 完成。详见 T-0000I TDS。 |
| **v2.33** | **2026-05-31** | T-30050 DoD 完成，M2 多环境对称四端全部闭环。详见 [T-30050 TDS](../tds/android/T-30050.md)。 |
| **v2.32** | **2026-05-31** | T-30050 Review Round 1 通过（→ DoD）。详见 T-30050 TDS。 |
| **v2.31** | **2026-05-31** | T-30050 TDD → Review，Android productFlavors 与网络安全双锁落地。详见 T-30050 TDS。 |
| **v2.30** | **2026-05-31** | T-30050 Plan → TDD，TDS 完成。详见 T-30050 TDS。 |
| **v2.26** | **2026-05-31** | T-20020 DoD 完成 / TDS 完成（Plan → TDD）。详见 [T-20020 TDS](../tds/web/T-20020.md)。 |
| **v2.25** | **2026-05-31** | T-10020 DoD 完成。详见 [T-10020 TDS](../tds/adminServer/T-10020.md)。 |
| **v2.24** | **2026-04-27** | T-10020 Review Round 1 通过（→ DoD）。详见 T-10020 TDS。 |
| **v2.23** | **2026-04-27** | T-10020 TDD → Review，AdminServer config 多 profile 体系落地。详见 T-10020 TDS。 |
| **v2.22** | **2026-05-31** | T-10020 Plan → TDD，TDS 完成。详见 T-10020 TDS。 |
| **v2.21** | **2026-05-31** | T-00040 DoD 完成。详见 [T-00040 TDS](../tds/server/T-00040.md)。 |
| **v2.20** | **2026-05-31** | T-00040 Review Round 1 通过（→ DoD）。详见 T-00040 TDS。 |
| **v2.19** | **2026-05-31** | T-00040 TDD → Review，AppServer config 补全 + staging.toml。详见 T-00040 TDS。 |
| **v2.18** | **2026-05-31** | T-00040 Plan → TDD，TDS 完成。详见 T-00040 TDS。 |
| **v2.17** | **2026-05-31** | T-0000H DoD 完成，M1 本地 E2E 链路具备。详见 [T-0000H TDS](../tds/infra/T-0000H.md)。 |
| **v2.16** | **2026-05-31** | T-0000H Review Round 1 通过（→ DoD）。详见 T-0000H TDS。 |
| **v2.15** | **2026-05-31** | T-0000H TDD → Review，envLoader/globalSetup/globalTeardown/fixtures 三件套落地。详见 T-0000H TDS。 |
| **v2.14** | **2026-05-31** | T-0000H Plan → TDD，TDS 完成。详见 T-0000H TDS。 |
| **v2.13** | **2026-05-31** | T-0000G DoD 完成。详见 [T-0000G TDS](../tds/infra/T-0000G.md)。 |
| **v2.12** | **2026-05-31** | T-0000G Review Round 1 通过（→ DoD）。详见 T-0000G TDS。 |
| **v2.11** | **2026-05-31** | T-0000G TDD → Review，Seed/Reset/Preflight 三件套 + sign-jwt CLI 落地。详见 T-0000G TDS。 |
| **v2.10** | **2026-05-31** | T-0000G Plan → TDD，TDS 完成。详见 T-0000G TDS。 |
| **v2.9** | **2026-05-31** | T-0000F DoD 完成。详见 [T-0000F TDS](../tds/infra/T-0000F.md)。 |
| **v2.8** | **2026-04-27** | T-0000F Review Round 1 通过（→ DoD）。详见 T-0000F TDS。 |
| **v2.7** | **2026-04-27** | T-0000F TDD → Review，三档 .env.example 模板落地。详见 T-0000F TDS。 |
| **v2.6** | **2026-04-27** | T-0000F Plan → TDD，TDS 完成。详见 T-0000F TDS。 |
| **v2.5** | **2026-05-31** | T-0000E DoD 完成。详见 [T-0000E TDS](../tds/infra/T-0000E.md)。 |
| **v2.4** | **2026-04-27** | T-0000E Review 通过。详见 T-0000E TDS。 |
| **v2.3** | **2026-04-27** | T-0000E TDD → Review。详见 T-0000E TDS。 |
| **v2.2** | **2026-04-27** | 模块 9 创建（E2E QA Foundation），新增 12 个 Task。详见 [模块 9 文档](./模块9-E2E测试基建%20(E2E%20QA%20Foundation).md)。 |
| **v2.1** | **2026-05-27** | T-30041 DoD 完成（踢人原因弹窗），E-10 进度 15/18。 |
| **v2.0** | **2026-05-26** | T-30040 DoD 完成（用户操作菜单），E-10 进度 14/18。 |
| **v1.9** | **2026-05-19** | T-00030 DoD 完成（TransferAdmin/ForceMic 信令），E-10 进度 7/18。 |
| **v1.8** | **2026-05-18** | T-00029 DoD 完成（MuteUser/UnmuteUser 信令），E-10 进度 6/18。 |
| **v1.7** | **2026-05-17** | T-00028 DoD 完成（KickUser 信令 + 冷却），E-10 进度 5/18。 |
| **v1.6** | **2026-05-16** | T-00027 DoD 完成（房间成员列表接口），E-10 进度 4/18。 |
| **v1.5** | **2026-04-30** | T-30035 DoD 完成（埋点主链路 EventReportClient），E-07.5 进度 6/6。 |
| **v1.4** | **2026-04-29** | T-30034 DoD 完成（Sentry/AnalyticsPort 接口），E-07.5 进度 5/6。 |
| **v1.3** | **2026-04-21** | Phase 1.5 E-10 启动，新增 18 个 Task。 |
| **v1.2** | **2026-04-21** | E-07.5 埋点与观测性基建启动，新增 6 个 Task。 |
| **v1.1** | **2026-04-21** | Phase 1 启动 E-07，新增 15 个 Task。 |
| **v1.0** | **2026-04-20** | Phase 0.5 启动，产品文档重构 + 新增 11 个 Task。 |
| **v0.7** | **2025-04-18** | 职责流转规则确立（PM→Plan→TDD→Review→DoD），模块 0 补 4 个 TDS。 |
| **v0.6** | **2025-04-18** | 14 个有 TDS 的任务标为 TDD，其余标为 Plan。 |
| **v0.5** | **2025-04-18** | TDS 文档重建：14 个 TDS 按端拆分。 |
| **v0.4** | **2025-04-18** | 深度 Review，补充基建任务、跨服务通信任务、shared crate。 |
| **v0.3** | **2025-04-18** | Server 拆分为 App Server + Admin Server。 |
| **v0.2** | **2025-04-18** | 注册登录合并，Web 端重定位。 |
| **v0.1** | **2025-04-17** | 初始版本，45 个任务。 |
| 编号范围 | 归属端 | 说明 |
|---------|--------|------|
| T-0000A ~ T-0000Z | 基础设施 | CI/CD、Docker、共享模块 |
| T-00001 ~ T-00999 | App Server | C 端业务后端 |
| T-10001 ~ T-10999 | Admin Server | B 端管理后端 |
| T-20001 ~ T-20999 | Web | 后台管理前端 |
| T-30001 ~ T-30999 | Android | C 端用户应用 |

## 任务状态说明

| 状态 | 说明 |
|------|------|
| `Todo` | 待开始（尚未进入任何流转阶段） |
| `In Progress` | 当前负责人正在执行中（Plan 设计中 / TDD 编码中 / Review 审查中 / DoD 文档同步中） |
| `Done` | 已完成（DoD 文档同步完毕） |
| `Blocked` | 被阻塞（前置依赖未完成或外部因素） |


## 门禁状态说明

| 列名 | 含义 | 初始值 |
|------|------|--------|
| `Review Gate 审查门禁` | 全局架构级代码审查门禁，由 `review-coordinator` 调度 `global-code-reviewer` 在完成批次审查后填写。`✅ Passed` 表示通过，`❌ Failed` / `⏳ In Review` 表示进行中。**单 Task 流转中的 `Review` 角色不维护此列**。 | `-`（未评审） |
| `QA Gate 测试门禁` | 测试验收门禁，由 QA 在 E2E/手动测试通过后填写。`✅ Passed` / `❌ Failed` | `-`（未测试） |
| `Overall Gate 最终门禁` | 综合质量门禁，按下表规则自动推导 | `⏳ Pending` |

**Overall Gate 推导规则**：

| 条件 | Overall Gate |
|------|------|
| 研发状态非 `✅ Done`，或 `Review Gate` / `QA Gate` 任一为 `-` | `⏳ Pending` |
| 研发状态为 `✅ Done`，且 `Review Gate` / `QA Gate` 任一为 `❌ Failed` | `❌ Failed` |
| 研发状态为 `✅ Done`，且 `Review Gate` 与 `QA Gate` 均为 `✅ Passed` | `✅ Passed` |

## 职责流转规则

> **核心流程**：`PM 创建 Task` → `Plan 设计方案` → `TDD 实现代码` → `Review 审查代码` → `DoD 记录文档`

| 阶段 | 负责人标记 | 职责 | 完成后动作 |
|------|-----------|------|-----------|
| **PM** | `PM` | 创建 Task，定义需求、验收标准 | 将研发负责人改为 `Plan` |
| **Plan** | `Plan` | 设计技术方案，输出 TDS 文档到 `doc/tds/[$端]/T-xxx.md`, 完善`doc/architecture/`、`doc/protocol/`设计文件 | 将研发负责人改为 `TDD`，在任务名称后补充 `[TDS]` 链接 |
| **TDD** | `TDD` | 按 TDS、protocol及`doc/design` 编写测试 → 实现代码 → 测试通过 | 将研发负责人改为 `Review`，更新 TDS 第四节【实现结果】 |
| **Review** | `Review` | 按 TDS、protocol、design → review代码 → review通过/不通过 | 通过：将研发负责人改为 `Dod`，更新 TDS 第五节【Review意见】；不通过：将负责人改回 `TDD`，更新 TDS 第五节 |
| **DoD** | `Dod` | 按照代码实现，更新`doc/arch/[$端]/`下的文档，并更新目录下的index.md文件，及`doc/product/index.md`的功能实现状态 | 将状态改为 `Done` |

**规则**：
1. 每个阶段的负责人只能由**上一阶段的负责人**修改为下一阶段
2. `Plan` 未完成 TDS 前，不得将负责人改为 `TDD`
3. `TDD` 未通过全部验收用例前，不得将状态改为 `Review`
4. `Review` 未通过全部Review意见，不得将状态改为 `Dod`
5. `Dod` 未将实现更新到文档之前，不得将状态改为 `Done`
6. 当前所有 Task 已由 PM 创建完毕，初始负责人均为 `Plan`
7. **注意（命名消歧）**：本节定义的「研发负责人 = `Review`」是单 Task 内对当次 TDD 提交的轻量代码审查，由 `coordinator` 调度 `code-reviewer` 子代理执行；它**不等于**「Review Gate 审查门禁」列。Review Gate 是模块级架构审查，由独立流水线 `review-coordinator` + `global-code-reviewer` 维护（流程见 `.github/agents/review-coordinator.agent.md` 与 `doc/review/batch-*.md`）。本节的 `Plan/TDD/Review/Dod` 任一阶段均**不得**修改 Review Gate / QA Gate / Overall Gate 三列。

---

---

## 模块索引

### Phase 0: MVP 基础设施 (预计 6-8 周)

- [模块 0: 工程基建 (Infrastructure & Shared)](./模块0-工程基建%20(Infrastructure%20&%20Shared).md)
- [模块 1: 用户认证系统 (User Authentication)](./模块1-用户认证系统%20(User%20Authentication).md)
- [模块 2: 房间大厅与列表 (Room Hall)](./模块2-房间大厅与列表%20(Room%20Hall).md)
- [模块 3: 房间内核心功能 (In-Room Core)](./模块3-房间内核心功能%20(In-Room%20Core).md)

### Phase 0.5: 交互壳体与基础体验

- [模块 4: 中东黑金主题与 App 壳体 (MENA Theme & App Shell)](./模块4-中东黑金主题与%20App%20壳体%20(MENA%20Theme%20&%20App%20Shell).md)
- [模块 5: Web 管理端增强 (Admin Web Enhancements)](./模块5-Web%20管理端增强%20(Admin%20Web%20Enhancements).md)

### Phase 1: 核心营收闭环

- [模块 6: 虚拟礼物与钱包闭环 MVP (E-07)](./模块6-虚拟礼物与钱包闭环%20MVP%20(E-07).md)

### Phase 1 并行 Epic：E-07.5 埋点与观测性基建

- [模块 7: 埋点与观测性基建 (E-07.5)](./模块7-埋点与观测性基建%20(E-07.5).md)

### Phase 1.5 Epic：E-10 房间主权与管理员体系

- [模块 8: 房间主权与管理员体系 (E-10)](./模块8-房间主权与管理员体系%20(E-10).md)

### Phase 1.6 测试基建：E2E QA Foundation

- [模块 9: E2E 测试基建 (E2E QA Foundation)](./模块9-E2E测试基建%20(E2E%20QA%20Foundation).md)
