# Voice Room 开发任务清单

> **版本**: v2.13  
> **更新日期**: 2026-05-31  
> **任务总数**: 123 个 (基建: 4 + 9, App Server: 30 + 1, Admin Server: 16 + 1, Web: 14 + 1, Android: 44 + 1, E-07 15 + E-07.5 6 + E-10 18)  
> **当前阶段**: Phase 1 - 核心营收闭环（E-07 + E-07.5 并行）→ Phase 1.5 E-10 房间治理 → Phase 1.6 E2E 测试基建（模块 9）

---

## 🔄 重要变更说明

| 版本 | 日期 | 变更内容 |
|------|------|---------|
| **v2.13** | **2026-05-31** | **T-0000G DoD 完成，模块 9 进度 3/12：doc/architecture/index.md 关联文档新增测试基建脚本三件套索引（T-0000G TDS + scripts/dev 路径 + sign-jwt CLI 位置）；doc/arch/infra/index.md §一目录结构补充 T-0000G 四脚本（seed-e2e.sh/sql、reset-e2e.sh、preflight.sh）+ sign-jwt.rs 二进制路径，新增 §四 E2E 测试基建脚本详解（Seed/Reset/Preflight 脚本表 + sign-jwt CLI 使用方式/参数/环保变量/退出码），能力矩阵补充四行 T-0000G 完成项；doc/tasks/模块9-E2E测试基建 (E2E QA Foundation).md T-0000G 行：研发状态 In Progress → ✅ Done，Review Gate → ✅ Passed；doc/product/index.md Phase 1.6 E2E 测试基建进度确认为 3/12（T-0000E/T-0000F/T-0000G）** |
| **v2.12** | **2026-05-31** | **T-0000G Review Round 1 通过（🟢）：Seed/Reset/Preflight 三件套 + sign-jwt CLI 静态/Profile-guard/cargo check 全部实测通过；安全（JWT_SECRET 不泄露）、Seed ON CONFLICT 幂等、Reset profile-guard 在 psql 之前生效、Preflight 2s 超时 + fail-fast + CI=1 关色，均符合验收。偏离决议：D1/D2/D3（按真实 schema 字段名/枚举值）接受、D4（init-db.sh / e2e_runner 角色推迟）接受为已知风险（profile-guard 兜底）、D5 接受。runtime 三项（真实 psql 写入幂等、reset 行计数、5 端 /health 200）由 T-0000H 联动验收，不阻塞本任务。模块 9 任务清单 T-0000G 行：负责人 Review → DoD** |
| **v2.11** | **2026-05-31** | **T-0000G TDD 完成（→ Review）：交付 Seed/Reset/Preflight 三件套（`scripts/dev/{seed-e2e.sql,seed-e2e.sh,reset-e2e.sh,preflight.sh}` 4 个新增 + sign-jwt CLI `app/shared/src/bin/sign_jwt.rs` 复用 `voice-room-shared::jwt::token` 算法 / workspace `uuid` 增 `v5` feature / `.gitignore` 收口 `.seed-output.env` 与 `.seed.lock`）；profile guard 三脚本均实测通过（seed staging→rc=21、缺 JWT_SECRET→rc=22、reset prod→rc=21 在 psql 之前生效）；preflight 5 端串行 + fail-fast + TTY 颜色 + CI=1 关色 + 机读行格式实测命中；sign-jwt 实测 exp-iat=86400 / op→operator iss=voiceroom-admin 映射正确；偏离 TDS：users 无 role 列、rooms 用 title/max_members、admins.role 取 super_admin/operator/cs/finance（均按真实 schema），init-db.sh / grant-permissions.sql 的 e2e_runner 角色推迟为扩展任务；DB 写入路径与真实 5 端 200 响应待 T-0000H runtime 联动；模块 9 任务清单 T-0000G 行：研发负责人 TDD → Review，状态保持 In Progress** |
| **v2.10** | **2026-05-31** | **T-0000G TDS 完成（Plan 阶段），负责人 Plan → TDD：doc/tds/infra/T-0000G.md 重写为完整结构（背景/目标/范围/数据流/文件清单/§2.4 Seed 数据契约 4 张子表（users/admins/rooms/JWT 策略）/§2.5 Reset 清理范围表（含 schema 不动 + Redis key 范围 + profile≠local 拒绝）/§2.6 Preflight 5 端 × 命令 × 期望 × 2s 超时 × 退出码 11~15 × 彩色 hint 矩阵/§2.7 与 T-0000H globalSetup 三脚本接口契约（命令/退出码/stdout 协议）/§3 TDD 验收用例（幂等 / 安全 / 故意打挂 / 契约 共 22 条）/§六 风险矩阵 7 项（含 G-R1 误删生产 / G-R3 JWT 密钥泄露 / G-R4 并发 seed）/§七 与 T-0000E §2.5/2.6/2.10/2.11/2.12 锚点映射）；模块 9 任务清单 T-0000G 行：研发负责人 Plan → TDD，研发状态 Todo → In Progress，准备进入 TDD 阶段** |
| v0.1 | 04-17 | 初始版本，45 个任务 |
| v0.2 | 04-18 | 注册登录合并，Web 端重定位 |
| v0.3 | 04-18 | Server 拆分为 App Server + Admin Server |
| **v0.4** | **04-18** | **深度 Review：补充基建任务、Admin Server 统计接口、跨服务通信任务、shared crate、修复依赖遗漏** |
| **v0.5** | **04-18** | **TDS 文档重建：14 个模块1 TDS 按端拆分（server 5 + adminServer 3 + web 2 + android 4），protocol.md v0.2，ARCHITECTURE.md 双 Server 架构** |
| **v0.6** | **04-18** | **负责人标记：有 TDS 的 14 个任务标为 TDD，其余 46 个标为 Plan；ARCHITECTURE.md §3 目录树修正（doc/arch, doc/tds, shared/ 简化, Web 目录去 WS/RTC/IM）** |
| **v0.7** | **04-18** | **职责流转规则：新增 PM→Plan→TDD→Review→DoD 流转说明；模块0 新增 4 个 TDS（infra/T-0000A~T-0000D）；全部 18 个有 TDS 的任务标为 TDD，42 个标为 Plan** |
| **v1.0** | **04-20** | **Phase 0.5 新增：产品文档重构为 doc/product/index.md + 子文件；新增 11 个 Task（Android 9 + Web 2）覆盖 Splash/主页三Tab/中东黑金主题/个人中心/房间视觉升级/解封弹窗/活水监控；创建 doc/design/android/ 和 doc/design/adminWeb/ 设计文档** |
| **v1.1** | **04-21** | **Phase 1 启动：E-07 虚拟礼物与钱包闭环 MVP，新增 15 个 Task（App Server 5 / Admin Server 2 / Web 1 / Android 7）；产出 `doc/product/phase1_gift_economy.md` 方向总纲、`competitors.md` 附录 A、`business_flows.md §2.7`；Android 7 个新设计文档** |
| **v1.2** | **04-21** | **E-07.5 埋点与观测性基建（与 E-07 并行）：新增 6 个 Task（App Server 2 / Admin Server 1 / Web 1 / Android 2）；产出 `doc/product/phase1_observability.md` 方向总纲、`business_flows.md §2.9` 事件字典；Android 2 个新设计文档** |
| **v1.3** | **04-21** | **Phase 1.5 E-10 房间主权与管理员体系：新增 18 个 Task（App Server 7 / Admin Server 1 / Web 1 / Android 9）；产出 `doc/product/phase1_room_governance.md` 方向总纲、`competitors.md` 附录 B、`business_flows.md §2.8` 治理流程；Android 9 个新设计文档** |
| **v1.4** | **2026-04-29** | **T-30034 DoD 完成，E-07.5 进度 5/6：新建 `doc/arch/android/analytics.md`（AnalyticsPort 接口设计、SentryAnalytics/DefaultSentryHub Stub、SensitiveFilter 脱敏策略、ConsentMode 枚举、NoopAnalytics、BuildConfig.SENTRY_DSN 注入、CI 静态检查脚本、MVP 限制 HIGH-01/02、待修复项 MEDIUM-01/02）；doc/arch/android/index.md 新增 analytics.md 子模块索引与能力状态描述；doc/tasks/index.md T-30034 标记为 ✅ Done（负责人: Dod）；doc/product/index.md E-07.5 进度更新为 5/6** |
| **v1.5** | **2026-04-30** | **T-30035 DoD 完成，E-07.5 进度 6/6（全部完成）：doc/arch/android/analytics.md 新增第十二章 EventReportClient 主链路（EventReportClient 主入口 + 队列策略 + Throttler + Transport 选择 + SessionManager + CommonPropsProvider + ConsentRepository/DataStoreConsentStore + PrivacyConsentDialog + 26 个核心事件埋点）与第十三章 TDD 验收结果（42 个单元测试全部通过）；doc/arch/android/index.md 能力全景新增 T-30035 条目；doc/tasks/index.md T-30035 确认 ✅ Done（负责人: Dod）；doc/product/index.md E-07.5 进度更新为 6/6 全部完成** |
| **v1.9** | **2026-05-19** | **T-00030 DoD 完成，E-10 进度 7/18：doc/arch/server/room.md 新增三十二~三十九章（TransferAdmin assign/revoke C→S 信令格式、AdminChanged 广播含 previous_admin_id、ForceTakeMic/ForceLeaveMic 信令格式、权限矩阵 owner-only TransferAdmin/owner+admin ForceMic、管理员不能抱下房主约束、ForceTakeMic 检查 mic_muted、原子性 DB 失败不广播、遗留 LOW target 不在房间未显式校验、文件清单与 427 测试汇总）；doc/tasks/index.md T-00030 状态 → ✅ Done；doc/product/index.md E-10 进度 6/18 → 7/18** |
| **v2.0** | **2026-05-26** | **T-30040 DoD 完成，E-10 进度 14/18：doc/arch/android/features.md 新增用户操作菜单模块文档（UserActionBottomSheet testTag 清单 10 项、ActionMatrix.kt computeActions 9 角色组合权限矩阵、Role 枚举 OWNER/ADMIN/MEMBER、UserAction 枚举 INVITE_MIC/MUTE_MIC/MUTE_CHAT/KICK/ASSIGN_ADMIN/REVOKE_ADMIN/VIEW_PROFILE/REPORT 8 项、RevokeAdmin 两步确认流程 pendingRevokeTarget→event→confirmRevokeAdmin→WS TransferAdmin(revoke)→AdminChanged 广播、与 T-30041 联动 selectedKickTarget 字段解耦设计）；doc/tasks/index.md T-30040 确认 ✅ Done 负责人 Dod；doc/product/index.md E-10 进度 13/18 → 14/18** |
| **v2.1** | **2026-05-27** | **T-30041 DoD 完成，E-10 进度 15/18：doc/arch/android/features.md 新增踢人原因弹窗模块文档（KickReasonDialog AlertDialog dismissOnClickOutside=false、KickReason 枚举 HARASSMENT/SPAM/ABUSE/OTHER、KickDialogState canSubmit 逻辑（OTHER 必填 customText、isSubmitting 防重复提交）、reason 字段 JSON 安全转义（双引号→全角引号、反斜杠转义）、与 T-30040 selectedKickTarget 联动流程（ShowKickReasonDialog event→弹窗→kickUser→UserKicked 广播→dismiss+Toast）、testTag 清单 kick_reason_0~3/kick_reason_custom_input/btn_confirm_kick）；doc/tasks/index.md T-30041 确认 ✅ Done 负责人 Dod；doc/product/index.md E-10 进度 14/18 → 15/18** |
| **v1.8** | **2026-05-18** | **T-00029 DoD 完成，E-10 进度 6/18：doc/arch/server/room.md 新增二十四~三十一章（MuteUser/UnmuteUser C→S 信令格式、UserMuted 广播格式、Redis Key mic_muted/chat_muted TTL=duration_sec、处理流程 5 步、SendMessage→40305/TakeMic→40306 双重拦截、duration_sec [60,86400] 边界、送礼不受禁麦影响、文件清单与 365 测试汇总）；doc/tasks/index.md T-00029 状态 → ✅ Done 负责人 → Dod；doc/product/index.md E-10 进度 5/18 → 6/18** |
| **v1.7** | **2026-05-17** | **T-00028 DoD 完成，E-10 进度 5/18：doc/arch/server/room.md 新增十六~二十三章（KickUser C→S/S→C/广播信令格式、处理流程 7 步、权限校验矩阵 owner>admin>member 不可踢 owner、Redis 冷却 Key kicked:{room_id}:{user_id} TTL 600s、JoinRoom 42911 冷却拦截、并发保护 DashMap.remove() 原子性、遗留问题 MEDIUM MicLeft/UserLeft 广播顺序 + LOW TTL=-1 处理、文件清单与 366+ 测试汇总）；doc/tasks/index.md T-00028 状态 → ✅ Done 负责人 → Dod；doc/product/index.md E-10 进度 4/18 → 5/18** |
| **v2.2** | **2026-04-27** | **模块 9 创建（E2E 测试基建 / E2E QA Foundation）：新增 12 个 Task 覆盖多环境（local / staging / prod）切换体系、Seed/Reset/Preflight 三件套、globalSetup/Teardown/envLoader、AppServer & AdminServer config 对称化、Web 多 profile env、Android productFlavors、npm scripts 一键命令、@prod-safe 标签体系、Midscene 接入文档、E2E_RUNBOOK；产出 [T-0000E 主 TDS](../tds/infra/T-0000E.md) 冻结 11 个子任务接口契约 + 11 个子 TDS（infra/T-0000F~T-0000L、server/T-00040、adminServer/T-10020、web/T-20020、android/T-30050）；任务总数 111 → 123；T-0000E 负责人 → TDD（已具备 TDS），其余 11 个负责人 → Plan（待 Plan Agent 细化时按依赖顺序激活）** |
| **v2.3** | **2026-04-27** | **T-0000E 进入 Review 阶段：TDS（doc/tds/infra/T-0000E.md）补全 §2.11 迁移路径（6 步 Migration Path + 4 条 Invariants）、§2.12 风险矩阵（R1~R10 含概率/影响/缓解/兜底/Owner）、§4.3 11 个下游子任务接口契约冻结索引表、§4.4 验收对照表、§4.5 残余风险 3 项；与 _template.md 偏离项显式声明（新增 6 个章节作为主 TDS 超集扩展）；模块 9 任务清单 T-0000E 行：研发负责人 TDD → Review，研发状态 Todo → In Progress** |
| **v2.4** | **2026-04-27** | **T-0000E Review 通过：抽样核对 11 个下游子 TDS（infra/T-0000F~T-0000L、server/T-00040、adminServer/T-10020、web/T-20020、android/T-30050）全部存在且首章引用主 TDS 对应章节锚点，无契约漂移；§2.11 迁移路径 6 步可执行（每步含验收锚点 + 回滚策略 + 4 条不变量保护 PR 不阻塞与 cargo test 0 回归）、§2.12 风险矩阵 R1~R10 字段齐全，R1（prod 误写）五道防线足够；流程特例（TDD 同步补 §2.11/§2.12）在主 TDS「产物即文档」场景下可接受并已在 §4.2 显式声明；§4.5 残余风险 R1 不追加约束、R4/R10 由 T-0000F/T-0000I 落地时再决议；模块 9 任务清单 T-0000E 行：研发负责人 Review → Dod** |
| **v2.6** | **2026-04-27** | **T-0000F TDS 完成（字段表/契约/风险矩阵补全），负责人 Plan → TDD：doc/tds/infra/T-0000F.md 重写为完整结构（背景/目标/方案设计/数据流/文件清单/24 字段三档 profile 表/envLoader MissingEnvError 错误契约/.gitignore 模式清单/TDD 验收命令清单/风险矩阵 5 项/迁移步骤 8 步/与 T-0000E §2.4.1 + §2.10 + §2.11 锚点映射）；模块 9 任务清单 T-0000F 行：研发负责人 Plan → TDD，研发状态 Todo → In Progress，准备进入 TDD 阶段** |
| **v2.8** | **2026-04-27** | **T-0000F Review 通过（Round 1），负责人 Review → Dod：实跑核验四档 `.example` 字段集合 1:1 对齐（24 keys/文件）、`prod.example E2E_ALLOW_WRITES=0`、`git check-ignore` 真实 env 全 IGNORED + `.example` 全 NOT-IGNORED、URL 合法性 8/8、`cargo check` 0 回归；偏离项 1（5 个 docker/server 字段改注释行 + 失去 JWT placeholder 默认值）按选项 A 接受——严格遵守 TDS §2.3 附表注脚 + server 启动 fail-fast 反而更安全，dev onboarding 风险跟踪到 T-0000L Runbook；偏离项 2（`.gitignore` 收窄暴露 tests/scripts/ 历史 spec）经实跑确认 `git status` 全 untracked、未入库、无真实凭据泄露事实，untracked 治理跟踪到 T-0000J；F-R4 typo `app_server_pwd` 残留确认归属 T-0000J；T-0000F TDS §八 Review Round 1 已落记；模块 9 任务清单 T-0000F 行：研发负责人 Review → Dod** |
| **v2.7** | **2026-04-27** | **T-0000F TDD 完成，负责人 TDD → Review：根 `.env.example` 重写为「docker/server 注释段 + 24 主字段段」结构，新增 `tests/scripts/env/.env.{local,staging,prod}.example` 三档模板（字段集合 1:1 对齐 T-0000F TDS §2.3）；prod 默认 `E2E_ALLOW_WRITES=0` 并加 ⚠️ 头部注释；`.gitignore` 删除整目录 `tests/scripts/` ignore，改为 3 行精确忽略真实 `.env.{local,staging,prod}`；TDD 验收 §3.1 字段 diff / §3.2 ignore 行为 / URL 合法性 / cargo check 全绿；遗留 typo `app_server_pwd` 命中点（tests/scripts/{API,E2E}/TC-AUTH.spec.ts）按 TDS 范围交由 T-0000J 处理；模块 9 任务清单 T-0000F 行：研发负责人 TDD → Review** |
| **v2.5** | **2026-05-31** | **T-0000E DoD 完成，模块 9 进度 1/12：doc/architecture/index.md 关联文档新增 E2E 测试基建多环境切换索引（指向 T-0000E TDS）；doc/tasks/模块9-E2E测试基建 (E2E QA Foundation).md T-0000E 行：研发状态 In Progress → ✅ Done，Review Gate 在 Review 通过事实基础上 → ✅ Passed；doc/tasks/index.md 版本更新 v2.4 → v2.5、更新日期 2026-04-27 → 2026-05-31** |
| **v2.9** | **2026-05-31** | **T-0000F DoD 完成，模块 9 进度 2/12：doc/architecture/index.md 关联文档补充三档 `.env.example` 索引说明（tests/scripts/env/ 中 `.env.{local,staging,prod}.example` 与 T-0000F TDS）；doc/tasks/模块9-E2E测试基建 (E2E QA Foundation).md T-0000F 行：研发状态 In Progress → ✅ Done，Review Gate → ✅ Passed；doc/product/index.md Phase 1.6 E2E 测试基建进度确认为 2/12（T-0000E/T-0000F）** |
| **v1.6** | **2026-05-16** | **T-00027 DoD 完成，E-10 进度 4/18：doc/arch/server/room.md 新增十三~十五章（GET /api/v1/rooms/:id/members 接口契约、角色优先级 owner>admin>member、1 次批量 SQL WHERE id=ANY($1)、MemberSnapshot 单一数据源、muted_mic/muted_chat Redis Key、权限错误码、文件清单与 398 测试汇总）；doc/tasks/index.md T-00027 状态 → ✅ Done 负责人 → Dod；doc/product/index.md E-10 进度 3/18 → 4/18** |

---

### 任务编号规则

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
| `Review Gate 审查门禁` | 代码审查门禁，由 Reviewer 在完成代码审查后填写。`✅ Passed` 表示通过，`❌ Failed` 表示不通过 | `-`（未评审） |
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
| **PM** | `PM` | 创建 Task，定义需求、验收标准 | 将负责人改为 `Plan` |
| **Plan** | `Plan` | 设计技术方案，输出 TDS 文档到 `doc/tds/[$端]/T-xxx.md`, 完善`doc/architecture/`、`doc/protocol/`设计文件 | 将负责人改为 `TDD`，在任务名称后补充 `[TDS]` 链接 |
| **TDD** | `TDD` | 按 TDS、protocol及`doc/design` 编写测试 → 实现代码 → 测试通过 | 将负责人改为 `Review`，更新 TDS 第四节【实现结果】 |
| **Review** | `Review` | 按 TDS、protocol、design → review代码 → review通过/不通过 | 通过：将负责人改为 `Dod`，更新 TDS 第五节【Review意见】；不通过：将负责人改回 `TDD`，更新 TDS 第五节 |
| **DoD** | `Dod` | 按照代码实现，更新`doc/arch/[$端]/`下的文档，并更新目录下的index.md文件，及`doc/product/index.md`的功能实现状态 | 将状态改为 `Done` |

**规则**：
1. 每个阶段的负责人只能由**上一阶段的负责人**修改为下一阶段
2. `Plan` 未完成 TDS 前，不得将负责人改为 `TDD`
3. `TDD` 未通过全部验收用例前，不得将状态改为 `Review`
4. `Review` 未通过全部Review意见，不得将状态改为 `Dod`
5. `Dod` 未将实现更新到文档之前，不得将状态改为 `Done`
6. 当前所有 Task 已由 PM 创建完毕，初始负责人均为 `Plan`


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
