# Voice Room 开发任务清单

> **版本**: v2.89  
> **更新日期**: 2026-05-05  
> **任务总数**: 139 个 (基建: 4 + 14 + 3, App Server: 33 + 1 + 2, Admin Server: 16 + 1, Web: 14 + 1, Android: 45 + 1 + 1, E-07 15 + E-07.5 6 + E-10 18)  
> **当前阶段**: Phase 1 - 核心营收闭环（E-07 + E-07.5 并行）→ Phase 1.5 E-10 房间治理 → Phase 1.6 E2E 测试基建 → **Phase 1.7 协议治理铁律落地**（Round 16 BUG-CHAT-WS 系统性根因 → 协议路径绑定 + 历史 TDS 全量回填 + 审计脚本）

---

## 🔄 重要变更说明

| 版本 | 日期 | 变更内容 |
|------|------|---------|
| _规则_ | — | 本表只记录**版本级摘要**（一行 ≤ 200 字符），具体 Review/审查/实跑证据请落到对应 [TDS](../tds/) 第五节【Review 意见】或对应模块审查批次 `doc/review/模块N-XXX.md`，**严禁**在本表堆叠详细审查记录。 |
| **v2.89** | **2026-05-05** | T-00048 Review 通过 → DoD；双路径集成测试 DUAL-1/2/3 全绿，协议路径绑定三行全覆盖。 |
| **v2.87** | **2026-05-05** | T-30053 DoD 收尾：三门禁全绿（研发✅/Review✅/QA✅），Overall Gate ✅ Released；BUG-CHAT-LONGPRESS 修复链完整闭环收官。 |
| **v2.86** | **2026-05-05** | T-30053 QA Gate ✅ Passed — TC-CHAT-00002 Round 23 实证（report-20260505-213102，6/6 PASS，DELTA_WS=+1，DELTA_BCAST=+1，5节点全命中，aiTap→aiLongPress self-healing Round 1 通过）；BUG-CHAT-LONGPRESS 闭环。 |
| **v2.85** | **2026-05-05** | T-30053 DoD 完成 - Android ChatMessageList UserMessageItem 长按复制菜单（DropdownMenu+ClipboardManager+Toast），LP-01~08 测试通过，无协议变更，arch/android Chat 章节已同步。 |
| **v2.84** | **2026-05-05** | T-00047 试跑 Task 闭环：Chat WS 主路径 ⭐ + REST 备路径协议落锚，REST 补 `filter_content` 与空白校验，PROTO-2 集成测试通过；server 协议入口索引完成 DoD 回填。 |
| **v2.83** | **2026-05-05** | 协议治理铁律落地：①copilot-instructions 新增红线 #7「协议路径绑定（最高优先级）」+ Plan/TDD/Review/DoD 强制条款；②code-coordinator/review-coordinator/global-review 三 Agent 注入协议路径绑定校验；③`doc/tds/_template.md` 新增「🔌 协议路径绑定表」+ PROTO-1/PROTO-2 验收；④architecture/index.md 顶部「🔴 协议契约铁律」5 条；⑤websocket_and_state §8.2 重写指向 protocol/；⑥ARCHITECTURE.md 加废弃横幅待删除；⑦4 端 arch/index.md 添加「🔌 协议入口索引」占位；⑧Phase B 注册 6 个新 Task：T-00047（试跑 ⭐）+ T-30054 + T-00048 + T-0000T + T-0000U + T-0000V。 |
| **v2.82** | **2026-05-05** | BUG-CHAT-WS 修复链 QA Gate ✅ Passed 收口：T-00045/T-00046/T-30051/T-30052 四项 Task 经 Round 22 实证（report-20260505-124251，DELTA_WS=+1, DELTA_BCAST=+2, 5节点全≥18, parse failed=0, Midscene Step 5 PASS）全部通过；known-issue BUG-CHAT-LONGPRESS（Step 6 长按菜单）已独立立单，与本修复链无关。详见 [qa-batch-bug-chat-ws.md](../tests/qa-batch-bug-chat-ws.md)。 |
| **v2.81** | **2026-05-05** | BUG-CHAT-LONGPRESS 立单 - 新增 T-30053（Android 模块3 Chat）：ChatMessageList UserMessageItem 长按弹出 DropdownMenu 含「复制」项；来源 TC-CHAT-00002 Step 6 实证（report-20260505-124251）；P2，负责人 Plan，Todo。 |
| **v2.80** | **2026-05-05** | T-30052 DoD 完成 - ChatMessageList UserMessageItem 气泡样式修复（Surface+ChatBubble+testTag），dex strings 校验通过，供 E2E Round 21 验证。 |
| **v2.79** | **2026-05-05** | T-30051 DoD 完成 - Android WS 接收链路可观测性增强（5节点 Log 日志注入），dex strings 校验通过（8 条命中），供下一轮 E2E 决策树定位真根因。 |
| **v2.78** | **2026-05-05** | T-00046 WS 广播可观测性增强（BUG-CHAT-WS-BROADCAST-SILENT）：`broadcast_to_room_inner` 发送失败打 WARN + 清理 stale connection，广播前后打 INFO 统计；三分支决策树落入 TDS §六，供 Round 17 快速决策。 |
| **v2.77** | **2026-05-05** | T-00045 修复 BUG-CHAT-WS-BROADCAST：新增 `POST /api/v1/chat-messages` Server REST 端点（INSERT + 广播闭环，与 WS SendMessage 路径对齐）；9/9 集成测试全绿；commit `beedc85`。 |
| **v2.76** | **2026-05-05** | BUG-CHAT-WS Round 13 真正注入 connect 调用（Round 12 实证 cf899bd 为死代码，wsClient.connect 从未在生产路径被调用）；T-30017 重新激活→Done；APK 重建。 |
| **v2.75** | **2026-05-05** | BUG-CHAT-WS Round 9 真根因修复(RoomSocketRequestFactory URL /ws/room→/ws，token Header→?token=查询参数)[cf899bd]；4/4单测通过；APK重建。残余风险：服务端logging.rs WS URI含JWT待下轮脱敏。 |
| **v2.74** | **2026-05-05** | Round 7 修复3个Bug：BUG-GIFT-JSON-PARSE(GiftDto data.items包装+真实JSON单测)[0327fae]；BUG-ROOM-CREATE-NOCLOSE(失败时Toast+dismiss)[3714302]；BUG-CHAT-WS加固单测[3c140c8]；APK 20M。 |
| **v2.73** | **2026-05-01** | Round 6 网络地址更新(192.168.1.19) + 6个P1 Bug修复(BUG-CHAT-WS/GOVERNANCE-FORM-VALIDATE/MIC-PERMISSION-TOAST/GIFT-MIC-PERMISSION/IME-HYPHEN/MIC-SEAT-SEED)；APK已构建，供Round 6 E2E回归。 |
| **v2.72** | **2026-04-30** | Android 4 Bug 修复（BUG-JWT-PERSIST/LOGIN-NAV/ROOM-NAV/CREATE-ROOM-SUBMIT）：DataStore 持久化 + 登录导航 + 房间卡片 + 创建房间导航全链路打通；commits `1ff6326`/`636979e`/`1f557d0`/`0d7c6e3`/`5b8682f`/`11a2c11`；Code Review ✅ APPROVE。 |
| **v2.71** | **2026-04-29** | T-30099 follow-up 修复后正式 QA 回归（[report-20260429-120907](../../tests/report-20260429-120907/SUMMARY.md)）：`:app:connectedLocalDebugAndroidTest` **180/180 PASS（100%）**，5 个熔断 Task（T-30001/30021/30023/30024/30025）全部出池；10 个回归 Task（T-30001/05/18/20/21/22/23/24/25/26）QA Gate 全数 ✅ Passed；BUG-ANDROID-002/003/004 + UI09 known flaky 全部闭环；JVM unit 675/678 持平（3 历史遗留 BuildConfigFlavor 与本轮无关）。 |
| **v2.70** | **2026-04-29** | T-30099 follow-up TDS 闭环（BUG-ANDROID-002/003/004 系统性修复 P0/P1）：跨 Task 修复登录页/消息 Tab 占位页/个人中心/房间页视觉升级 + GoldOutlinedTextField onValueChange。androidTest 137/180 → 179/180 PASS（+42），unit 675/678 持平。Round 1 [370c611] + Round 2 [a1b1ac4] + Review 🟢 [c31b671]。详见 [T-30099 TDS](../tds/android/T-30099.md)。1 个 known flaky（UI09）作为 follow-up。 |
| **v2.69** | **2026-04-29** | T-30018 Round 2 闭环（BUG-ANDROID-001 P0）：MenaColors 11 处 `Color(*_VALUE)` → `Color(*_VALUE.toInt())`，根因 `Color(ULong)` 重载误把低 6 位当 colorspace ID。androidTest 0/57 → 138/180 PASS，AIOOBE 清零；剩余 42 例属其他 bug（B1–B6 待 master 立单）。Review Round 3 🟢，commits [b9948e9] TDD + [758ccc8] Review。详见 [T-30018 TDS](../tds/android/T-30018.md) 第五节。 |
| **v2.68** | **2026-04-29** | T-30018 QA 回流（BUG-ANDROID-001 P0）：研发状态 ✅ Done → In Progress，回 TDD 修复 `MenaColors.kt` 11 处 `Color(ULong)` → `Color(Int)`（colorspace ID 误读 → ArrayIndexOutOfBoundsException）。详见 [T-30018 TDS](../tds/android/T-30018.md)。 |
| **v2.67** | **2026-04-29** | T-0000S DoD 完成（E2E fixture token + redis-cli 容器化解锁 26/29 SKIP-KNOWN），Review Round 1 🟢 通过；fixture 三角色 token + redis-cli 优先 docker→native→unavailable 三分支 + fail-fast 校验；6 份 spec 收敛 redis-cli 路径，26 个 SKIP-KNOWN 用例全 PASS；模块 9 进度 15/17 → 16/17。详见 [T-0000S TDS](../tds/infra/T-0000S.md)。 |
| **v2.66** | **2026-04-29** | v3 QA 战报（report-20260428-154125）反向拆出 2 个新 Task：T-0000R（WEB E2E 9-FAIL 测试侧硬化，非业务 bug，挂模块 9）+ T-0000S（fixture token 三角色自动注入 + redis-cli 容器化，解锁 26/29 SKIP-KNOWN）。详见 [T-0000R TDS](../tds/infra/T-0000R.md) / [T-0000S TDS](../tds/infra/T-0000S.md)。 |
| **v2.64** | **2026-04-29** | T-00044 单 Task 流转完整闭环（TDD→Review×3→Dod→Done），版本号冲突修正：原 v2.56/v2.57/v2.58 重号 → v2.61/v2.62/v2.63；顶部版本对齐 v2.64。 |
| **v2.65** | **2026-04-29** | 批次审查文档归档与合并：①`batch-e2e-foundation-followups.md`（T-0000N/O）+ `batch-arch-blockers-infra.md`（T-0000P/Q）合并入 [模块9-E2E测试基建.md](../review/模块9-E2E测试基建.md) 作为批次 C / 批次 D；②`batch-arch-blockers-business.md`（T-00041~44 跨模块 3/6/8）独立成新文件 [模块3-6-8-架构阻塞修复.md](../review/模块3-6-8-架构阻塞修复.md)；③三个老 batch 文件保留并加重定向 banner，全量跨引用（4 个 task 表 + 6 个 TDS + review/index.md）已切换到合并后主文档。 |
| **v2.61** | **2026-04-29** | T-00043 DoD 完成（chat_messages 持久化 + REST 历史接口落地，Review Round 2 🟢，arch/database/room_runtime/product 文档同步）；commit [f23042d](https://github.com/alsomail/voice-room/commit/f23042d)。详见 [TDS](../tds/server/T-00043.md)。 |
| **v2.60** | **2026-04-29** | T-00042 DoD 完成；TDD [2109c06](https://github.com/alsomail/voice-room/commit/2109c06) + R1 修复 [1f10ec3](https://github.com/alsomail/voice-room/commit/1f10ec3) 🟢 R2 通过；Admin 强制断连广播（user_banned/room_closed → connection_close 指令 → WS Close frame）；详见 [TDS](../tds/server/T-00042.md)。 |
| **v2.59** | **2026-04-29** | T-00043 Review Round 1 → TDD 修复 6 项 Should（CASCADE/排序/真DB并发/真DB性能/COUNT(*) OVER()/offset 软上限）→ Round 2 🟢 通过；commits [a191123](https://github.com/alsomail/voice-room/commit/a191123) 修复，[ec0c935](https://github.com/alsomail/voice-room/commit/ec0c935) 状态。详见 [TDS](../tds/server/T-00043.md) §4.5/§五。 |
| **v2.58** | **2026-04-29** | T-00043 TDD → Review，chat_messages 持久化 + REST 历史接口落地（migration 010 + 14 dedicated tests + 464 server suite 全绿）；commit [1beb68b](https://github.com/alsomail/voice-room/commit/1beb68b)。详见 [TDS](../tds/server/T-00043.md)。 |
| **v2.58** | **2026-04-29** | T-00041 DoD 完成；TDD [084f91e](https://github.com/alsomail/voice-room/commit/084f91e) + Review Round 1 🟢 [a8c0a64](https://github.com/alsomail/voice-room/commit/a8c0a64)；修复历史漏 spawn BUG，WS 心跳 30s 超时主动 Close(1000)。详见 [TDS](../tds/server/T-00041.md)。 |
| **v2.57** | **2026-04-29** | T-0000P DoD 完成（Midscene env 注入链 + 双注入 + 脱敏），模块 9 进度 14/15。详见 [TDS](../tds/infra/T-0000P.md)。 |
| **v2.62** | **2026-04-29** | T-00044 Review Round 3 🟢 通过（共 3 轮：MAJOR-2 错误码 + Idempotency-Key + spawn 回滚），HTTP 9/9 + WS 12/12 全绿，进入 Dod。详见 [TDS](../tds/server/T-00044.md)。 |
| **v2.63** | **2026-04-29** | T-00044 DoD 完成（HTTP 礼物端点 POST /api/v1/gifts/send 入档），doc/arch/server/{gift,index,status} 与 doc/product/index 已同步，E-07 进度 16/16。详见 [TDS](../tds/server/T-00044.md)。 |
| **v2.56** | **2026-04-29** | T-0000Q DoD 完成（e2e-up.sh 端口冲突预检 5 端，跨平台 lsof/ss）。详见 [T-0000Q TDS](../tds/infra/T-0000Q.md)。 |
| **v2.56** | **2026-04-29** | T-0000P TDD → Review，Midscene env 注入链落地（envLoader + .env.example + CI workflow + 17 unit tests）。详见 [T-0000P TDS](../tds/infra/T-0000P.md)。 |
| **v2.61** | **2026-04-29** | T-00044 TDD → Review，HTTP 礼物端点复用 WS 事务，新增 7 个 HTTP 测试 + 12 WS 回归全绿。详见 [TDS](../tds/server/T-00044.md)。 |
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
1. 每个阶段的研发负责人只能由**上一阶段的研发负责人**修改为下一阶段
2. `Plan` 未完成 TDS 前，不得将研发负责人改为 `TDD`
3. `TDD` 未通过全部验收用例前，不得将研发状态改为 `Review`
4. `Review` 未通过全部Review意见，不得将研发状态改为 `Dod`
5. `Dod` 未将实现更新到文档之前，不得将研发状态改为 `Done`
6. 当前所有 Task 已由 PM 创建完毕，初始研发负责人均为 `Plan`
7. **注意（命名消歧）**：本节定义的「研发负责人 = `Review`」是单 Task 内对当次 TDD 提交的轻量代码审查，由 `coordinator` 调度 `code-reviewer` 子代理执行；它**不等于**「Review Gate 审查门禁」列。Review Gate 是模块级架构审查，由独立流水线 `review-coordinator` + `global-code-reviewer` 维护（流程见 `.github/agents/review-coordinator.agent.md` 与 `doc/review/batch-*.md`）。本节的 `Plan/TDD/Review/Dod` 任一阶段均**不得**修改 Review Gate / QA Gate / Overall Gate 三列。
8. 各方填写完具体模块文件中的`Review Gate 审查门禁`、`QA Gate 测试门禁`、`Overall Gate 最终门禁`、`研发负责人`、`研发状态`之后，需要将其同步回填到本文件的模块表格下。
---

---

## 模块索引

> **说明**：各门禁状态由对应负责人回填。Task ID 点击跳转 TDS 技术方案文档，模块名称点击跳转模块详情页。

---

### Phase 0: MVP 基础设施 (预计 6-8 周)

#### [模块 0: 工程基建 (Infrastructure & Shared)](./模块0-工程基建%20(Infrastructure%20&%20Shared).md)

| Task ID | 前置依赖 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|---------|-----------|---------|---------------------|-----------------|----------------------|
| [T-0000A](../tds/infra/T-0000A.md) | 无 | Dod | ✅ Done | [✅ Passed](../review/模块0-工程基建.md) | - | ⏳ Pending |
| [T-0000B](../tds/infra/T-0000B.md) | 无 | Dod | ✅ Done | [✅ Passed](../review/模块0-工程基建.md) | - | ⏳ Pending |
| [T-0000C](../tds/infra/T-0000C.md) | T-0000A | Dod | ✅ Done | [✅ Passed](../review/模块0-工程基建.md) | - | ⏳ Pending |
| [T-0000D](../tds/infra/T-0000D.md) | T-0000B | Dod | ✅ Done | [✅ Passed](../review/模块0-工程基建.md) | - | ⏳ Pending |

#### [模块 1: 用户认证系统 (User Authentication)](./模块1-用户认证系统%20(User%20Authentication).md)

| Task ID | 前置依赖 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|---------|-----------|---------|---------------------|-----------------|----------------------|
| [T-00001](../tds/server/T-00001.md) | T-0000B, T-0000C | Dod | ✅ Done | [✅ Passed](../review/模块1-用户认证系统.md) | - | ⏳ Pending |
| [T-00002](../tds/server/T-00002.md) | T-00001 | Dod | ✅ Done | [✅ Passed](../review/模块1-用户认证系统.md) | - | ⏳ Pending |
| [T-00003](../tds/server/T-00003.md) | T-00002 | Dod | ✅ Done | [✅ Passed](../review/模块1-用户认证系统.md) | - | ⏳ Pending |
| [T-00004](../tds/server/T-00004.md) | T-00003 | Dod | ✅ Done | [✅ Passed](../review/模块1-用户认证系统.md) | - | ⏳ Pending |
| [T-00005](../tds/server/T-00005.md) | T-00004 | Dod | ✅ Done | [✅ Passed](../review/模块1-用户认证系统.md) | - | ⏳ Pending |
| [T-10001](../tds/adminServer/T-10001.md) | T-00001 | Dod | ✅ Done | [✅ Passed](../review/模块1-用户认证系统.md) | - | ⏳ Pending |
| [T-10002](../tds/adminServer/T-10002.md) | T-10001 | Dod | ✅ Done | [✅ Passed](../review/模块1-用户认证系统.md) | - | ⏳ Pending |
| [T-10003](../tds/adminServer/T-10003.md) | T-10002 | Dod | ✅ Done | [✅ Passed](../review/模块1-用户认证系统.md) | - | ⏳ Pending |
| [T-20001](../tds/web/T-20001.md) | T-10002 | Dod | ✅ Done | [✅ Passed](../review/模块1-用户认证系统.md) | - | ⏳ Pending |
| [T-20002](../tds/web/T-20002.md) | T-10002, T-20001 | Dod | ✅ Done | [✅ Passed](../review/模块1-用户认证系统.md) | - | ⏳ Pending |
| [T-30001](../tds/android/T-30001.md) | T-00002 | Dod | ✅ Done | [✅ Passed](../review/模块1-用户认证系统.md) | - | ⏳ Pending |
| [T-30002](../tds/android/T-30002.md) | T-00003, T-30001 | Dod | ✅ Done | [✅ Passed](../review/模块1-用户认证系统.md) | ⏭️ SKIP-OOS | ⏳ Pending |
| [T-30003](../tds/android/T-30003.md) | T-00004, T-30002 | Dod | ✅ Done | [✅ Passed](../review/模块1-用户认证系统.md) | ⏭️ SKIP-OOS | ⏳ Pending |
| [T-30004](../tds/android/T-30004.md) | T-00005, T-30003 | Dod | ✅ Done | [✅ Passed](../review/模块1-用户认证系统.md) | ⏭️ SKIP-OOS | ⏳ Pending |

#### [模块 2: 房间大厅与列表 (Room Hall)](./模块2-房间大厅与列表%20(Room%20Hall).md)

| Task ID | 前置依赖 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|---------|-----------|---------|---------------------|-----------------|----------------------|
| [T-00006](../tds/server/T-00006.md) | T-00001 | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | - | ⏳ Pending |
| [T-00007](../tds/server/T-00007.md) | T-00006, T-00004 | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | - | ⏳ Pending |
| [T-00008](../tds/server/T-00008.md) | T-00006 | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | - | ⏳ Pending |
| [T-00009](../tds/server/T-00009.md) | T-00008 | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | - | ⏳ Pending |
| [T-00010](../tds/server/T-00010.md) | T-00007 | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | - | ⏳ Pending |
| [T-10004](../tds/adminServer/T-10004.md) | T-00006, T-10003 | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | - | ⏳ Pending |
| [T-10005](../tds/adminServer/T-10005.md) | T-10004 | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | - | ⏳ Pending |
| [T-10006](../tds/adminServer/T-10006.md) | T-10005 | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | - | ⏳ Pending |
| [T-20003](../tds/web/T-20003.md) | T-20002, T-10010 | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | - | ⏳ Pending |
| [T-20004](../tds/web/T-20004.md) | T-10004 | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | - | ⏳ Pending |
| [T-20005](../tds/web/T-20005.md) | T-10005, T-20004 | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | - | ⏳ Pending |
| [T-30005](../tds/android/T-30005.md) | T-00008 | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | - | ⏳ Pending |
| [T-30006](../tds/android/T-30006.md) | T-00008, T-30005 | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | - | ⏳ Pending |
| [T-30007](../tds/android/T-30007.md) | T-00007 | Dod | ✅ Done | [✅ Passed](../review/模块2-房间大厅与列表.md) | ⏭️ SKIP-OOS | ⏳ Pending |

#### [模块 3: 房间内核心功能 (In-Room Core)](./模块3-房间内核心功能%20(In-Room%20Core).md)

| Task ID | 前置依赖 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|---------|-----------|---------|---------------------|-----------------|----------------------|
| [T-00011](../tds/server/T-00011.md) | T-00004 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-00011B](../tds/server/T-00011B.md) | T-00011 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-00011C](../tds/server/T-00011C.md) | T-00011, T-00012 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-00012](../tds/server/T-00012.md) | T-00011 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-00013](../tds/server/T-00013.md) | T-00012 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-00014](../tds/server/T-00014.md) | T-00012 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-00015](../tds/server/T-00015.md) | T-00014 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-00016](../tds/server/T-00016.md) | T-00012 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-00041](../tds/server/T-00041.md) | T-00011 | Dod | ✅ Done | [✅ Passed](../review/模块3-6-8-架构阻塞修复.md) | - | ⏳ Pending |
| [T-00043](../tds/server/T-00043.md) | T-00016 | Dod | ✅ Done | [✅ Passed](../review/模块3-6-8-架构阻塞修复.md) | - | ⏳ Pending |
| [T-00045](../tds/server/T-00045.md) | T-00043 | Dod | ✅ Done | [✅ Passed](../review/模块3-BUG-CHAT-WS修复链.md) | [✅ Passed · Round 22](../../tests/report-20260505-124251/AND/TC-CHAT-00002/TC-CHAT-00002_Report.md) | ⏳ Pending |
| [T-00046](../tds/server/T-00046.md) | T-00045 | Dod | ✅ Done | [✅ Passed](../review/模块3-BUG-CHAT-WS修复链.md) | [✅ Passed · Round 22](../../tests/report-20260505-124251/AND/TC-CHAT-00002/TC-CHAT-00002_Report.md) | ⏳ Pending |
| [T-00047](../tds/server/T-00047.md) ⭐ | T-00045, T-00046, T-30054 | Dod | ✅ Done | [✅ Passed](../tds/server/T-00047.md) | - | ✅ Passed |
| [T-00048](../tds/server/T-00048.md) | T-00047 | Dod | In Progress | - | - | ⏳ Pending |
| [T-10007](../tds/adminServer/T-10007.md) | T-10003 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-10008](../tds/adminServer/T-10008.md) | T-10007 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-10009](../tds/adminServer/T-10009.md) | T-10008 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-10010](../tds/adminServer/T-10010.md) | T-10003 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-10011](../tds/adminServer/T-10011.md) | T-10003, T-0000A | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-10012](../tds/adminServer/T-10012.md) | T-10001 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-20006](../tds/web/T-20006.md) | T-10007 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-20007](../tds/web/T-20007.md) | T-10008, T-20006 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-20008](../tds/web/T-20008.md) | T-10009, T-20007 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-20009](../tds/web/T-20009.md) | T-10012 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-30008](../tds/android/T-30008.md) | T-00011 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-30009](../tds/android/T-30009.md) | T-00009 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-30010](../tds/android/T-30010.md) | T-00012, T-30008, T-30009 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-30011](../tds/android/T-30011.md) | T-30009 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-30012](../tds/android/T-30012.md) | T-30011 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-30013](../tds/android/T-30013.md) | T-00014, T-30012 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-30014](../tds/android/T-30014.md) | T-30009 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-30015](../tds/android/T-30015.md) | T-30014 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-30016](../tds/android/T-30016.md) | T-00016, T-30015 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-30017](../tds/android/T-30017.md) | T-00016, T-30014 | Dod | ✅ Done | [✅ Passed](../review/模块3-房间内核心功能.md) | - | ⏳ Pending |
| [T-30051](../tds/android/T-30051.md) | T-30017 | Dod | ✅ Done | [✅ Passed](../review/模块3-BUG-CHAT-WS修复链.md) | [✅ Passed · Round 22](../../tests/report-20260505-124251/AND/TC-CHAT-00002/TC-CHAT-00002_Report.md) | ⏳ Pending |
| [T-30052](../tds/android/T-30052.md) | T-30051 | Dod | ✅ Done | [✅ Passed](../review/模块3-BUG-CHAT-WS修复链.md) | [✅ Passed · Round 22](../../tests/report-20260505-124251/AND/TC-CHAT-00002/TC-CHAT-00002_Report.md) | ⏳ Pending |
| [T-30053](../tds/android/T-30053.md) | T-30052 | - | ✅ Done | - | - | ⏳ Pending |
| T-30054 | T-00047 | Plan | Todo | - | - | ⏳ Pending |

---

### Phase 0.5: 交互壳体与基础体验

#### [模块 4: 中东黑金主题与 App 壳体 (MENA Theme & App Shell)](./模块4-中东黑金主题与%20App%20壳体%20(MENA%20Theme%20&%20App%20Shell).md)

| Task ID | 前置依赖 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|---------|-----------|---------|---------------------|-----------------|----------------------|
| [T-30018](../tds/android/T-30018.md) | 无 | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending |
| [T-30019](../tds/android/T-30019.md) | T-30018 | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending |
| [T-30020](../tds/android/T-30020.md) | T-30018, T-30019 | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending |
| [T-30021](../tds/android/T-30021.md) | T-30018 | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending |
| [T-30022](../tds/android/T-30022.md) | T-30018, T-30020 | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending |
| [T-30023](../tds/android/T-30023.md) | T-30018, T-30020 | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending |
| [T-30024](../tds/android/T-30024.md) | T-30018, T-30020, T-30004 | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending |
| [T-30025](../tds/android/T-30025.md) | T-30018 | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending |
| [T-30026](../tds/android/T-30026.md) | T-30018, T-30025 | Dod | ✅ Done | [✅ Passed](../review/模块4-中东黑金主题与App壳体.md) | - | ⏳ Pending |

#### [模块 5: Web 管理端增强 (Admin Web Enhancements)](./模块5-Web%20管理端增强%20(Admin%20Web%20Enhancements).md)

| Task ID | 前置依赖 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|---------|-----------|---------|---------------------|-----------------|----------------------|
| [T-20010](../tds/web/T-20010.md) | T-20007, T-10009 | Dod | ✅ Done | [✅ Passed](../review/模块5-Web管理端增强.md) | - | ⏳ Pending |
| [T-20011](../tds/web/T-20011.md) | T-20004 | Dod | ✅ Done | [✅ Passed](../review/模块5-Web管理端增强.md) | [⚠️ SKIP-KNOWN](../../tests/report-20260429-072049/WEB/TC-ROOM/Report.md) | ⏳ Pending |

---

### Phase 1: 核心营收闭环

#### [模块 6: 虚拟礼物与钱包闭环 MVP (E-07)](./模块6-虚拟礼物与钱包闭环%20MVP%20(E-07).md)

| Task ID | 前置依赖 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|---------|-----------|---------|---------------------|-----------------|----------------------|
| [T-00017](../tds/server/T-00017.md) | T-0000B | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |
| [T-00018](../tds/server/T-00018.md) | T-00017 | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |
| [T-00019](../tds/server/T-00019.md) | T-0000B | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |
| [T-00020](../tds/server/T-00020.md) | T-00017, T-00019, T-00016 | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |
| [T-00021](../tds/server/T-00021.md) | T-00020 | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |
| [T-00044](../tds/server/T-00044.md) | T-00020 | Dod | ✅ Done | [✅ Passed](../review/模块3-6-8-架构阻塞修复.md) | - | ⏳ Pending |
| [T-10013](../tds/adminServer/T-10013.md) | T-00017, T-10012 | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |
| [T-10014](../tds/adminServer/T-10014.md) | T-00019, T-10012 | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |
| [T-20012](../tds/web/T-20012.md) | T-10013, T-10014, T-20007 | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |
| [T-30027](../tds/android/T-30027.md) | T-00018, T-30024 | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |
| [T-30028](../tds/android/T-30028.md) | T-00019, T-30026 | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |
| [T-30029](../tds/android/T-30029.md) | T-30028 | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |
| [T-30030](../tds/android/T-30030.md) | T-30028, T-30029, T-00020 | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |
| [T-30031](../tds/android/T-30031.md) | T-30030 | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |
| [T-30032](../tds/android/T-30032.md) | T-30028 | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |
| [T-30033](../tds/android/T-30033.md) | T-00021, T-30018 | Dod | ✅ Done | [✅ Passed](../review/模块6-虚拟礼物与钱包闭环MVP.md) | - | ⏳ Pending |

---

### Phase 1 并行 Epic：E-07.5 埋点与观测性基建

#### [模块 7: 埋点与观测性基建 (E-07.5)](./模块7-埋点与观测性基建%20(E-07.5).md)

| Task ID | 前置依赖 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|---------|-----------|---------|---------------------|-----------------|----------------------|
| [T-00022](../tds/server/T-00022.md) | T-0000B | Dod | ✅ Done | [✅ Passed](../review/模块7-埋点与观测性基建.md) | - | ⏳ Pending |
| [T-00023](../tds/server/T-00023.md) | T-00022, T-00016 | Dod | ✅ Done | [✅ Passed](../review/模块7-埋点与观测性基建.md) | - | ⏳ Pending |
| [T-10015](../tds/adminServer/T-10015.md) | T-00022, T-10012 | Dod | ✅ Done | [✅ Passed](../review/模块7-埋点与观测性基建.md) | - | ⏳ Pending |
| [T-20013](../tds/web/T-20013.md) | T-10015, T-20007 | Dod | ✅ Done | [✅ Passed](../review/模块7-埋点与观测性基建.md) | - | ⏳ Pending |
| [T-30034](../tds/android/T-30034.md) | T-0000D | Dod | ✅ Done | [✅ Passed](../review/模块7-埋点与观测性基建.md) | ⏭️ SKIP-OOS | ⏳ Pending |
| [T-30035](../tds/android/T-30035.md) | T-30034, T-00022, T-00023, T-30002 | Dod | ✅ Done | [✅ Passed](../review/模块7-埋点与观测性基建.md) | ⏭️ SKIP-OOS | ⏳ Pending |

---

### Phase 1.5 Epic：E-10 房间主权与管理员体系

#### [模块 8: 房间主权与管理员体系 (E-10)](./模块8-房间主权与管理员体系%20(E-10).md)

| Task ID | 前置依赖 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|---------|-----------|---------|---------------------|-----------------|----------------------|
| [T-00024](../tds/server/T-00024.md) | T-0000B | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-00025](../tds/server/T-00025.md) | T-00024 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-00026](../tds/server/T-00026.md) | T-00025 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-00027](../tds/server/T-00027.md) | T-00024, T-00016 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-00028](../tds/server/T-00028.md) | T-00024, T-00027 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-00029](../tds/server/T-00029.md) | T-00024, T-00027 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-00030](../tds/server/T-00030.md) | T-00024, T-00027 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-00042](../tds/server/T-00042.md) | T-00011B, T-10009 | Dod | ✅ Done | [✅ Passed R2](../review/QA回归遗留改动审查.md) | - | ⏳ Pending |
| [T-10016](../tds/adminServer/T-10016.md) | T-00028, T-00029, T-10012 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-20014](../tds/web/T-20014.md) | T-10016, T-20007 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-30036](../tds/android/T-30036.md) | T-00025, T-30007 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-30037](../tds/android/T-30037.md) | T-30036 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-30038](../tds/android/T-30038.md) | T-00026, T-30007 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-30039](../tds/android/T-30039.md) | T-00027, T-30018 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-30040](../tds/android/T-30040.md) | T-30039 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-30041](../tds/android/T-30041.md) | T-30040, T-00028 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-30042](../tds/android/T-30042.md) | T-00028, T-00029 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-30043](../tds/android/T-30043.md) | T-00025, T-00030, T-30018 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |
| [T-30044](../tds/android/T-30044.md) | T-00029, T-00030, T-30042 | Dod | ✅ Done | [✅ Passed](../review/模块8-房间主权与管理员体系.md) | - | ⏳ Pending |

---

### Phase 1.6 测试基建：E2E QA Foundation

#### [模块 9: E2E 测试基建 (E2E QA Foundation)](./模块9-E2E测试基建%20(E2E%20QA%20Foundation).md)

| Task ID | 前置依赖 | 研发负责人 | 研发状态 | Review Gate 审查门禁 | QA Gate 测试门禁 | Overall Gate 最终门禁 |
|---------|---------|-----------|---------|---------------------|-----------------|----------------------|
| [T-0000E](../tds/infra/T-0000E.md) | 无 | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-0000F](../tds/infra/T-0000F.md) | T-0000E | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-0000G](../tds/infra/T-0000G.md) | T-0000E, T-0000A | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-0000H](../tds/infra/T-0000H.md) | T-0000F, T-0000G | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-0000I](../tds/infra/T-0000I.md) | T-0000H | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-0000J](../tds/infra/T-0000J.md) | T-0000H | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-0000K](../tds/infra/T-0000K.md) | T-0000F | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-0000L](../tds/infra/T-0000L.md) | T-0000I, T-0000J | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-0000M](../tds/infra/T-0000M.md) | T-0000H | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-0000N](../tds/infra/T-0000N.md) | T-0000H, T-0000M | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-0000O](../tds/infra/T-0000O.md) | T-0000M | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-0000P](../tds/infra/T-0000P.md) | T-0000H | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-0000Q](../tds/infra/T-0000Q.md) | T-0000G | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-0000R](../tds/infra/T-0000R.md) | T-0000P | Dod | ✅ Done | [✅ Passed](../tds/infra/T-0000R.md) | - | ⏳ Pending |
| [T-0000S](../tds/infra/T-0000S.md) | T-0000H, T-00041, T-00042 | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-00040](../tds/server/T-00040.md) | T-0000E | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-10020](../tds/adminServer/T-10020.md) | T-0000E | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-20020](../tds/web/T-20020.md) | T-0000E | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| [T-30050](../tds/android/T-30050.md) | T-0000E | Dod | ✅ Done | [✅ Passed](../review/模块9-E2E测试基建.md) | - | ⏳ Pending |
| T-0000T | T-0000R | Plan | Todo | - | - | ⏳ Pending |
| T-0000U | T-0000T | Plan | Todo | - | - | ⏳ Pending |
| T-0000V | T-0000U | Plan | Todo | - | - | ⏳ Pending |
