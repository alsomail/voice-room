# Voice Room 产品文档总索引

> **版本**: v1.0  
> **更新日期**: 2026-04-28  
> **负责人**: PM Agent  
> **目标市场**: MENA (Middle East & North Africa) 中东北非

---

## 产品定位

面向中东市场的实时语聊房 Monorepo，包含四端：Rust App Server (C端后端)、Rust Admin Server (B端后端)、Kotlin Android (C端App)、React Web (B端管理后台)。

---

## 模块化子文档索引

> ⚠️ **寻路规则**：所有详细内容按模块拆分到独立子文件中，本文件仅作为导航索引。严禁在此堆砌长篇细节。

| # | 子文档 | 内容概要 |
|---|--------|----------|
| 0 | [产品架构定位](./architecture_positioning.md) | 四端职责划分、App Server vs Admin Server 分离原因、数据库设计 |
| 1 | [竞品分析与营收拆解](./competitors.md) | Yalla/YoHo/Mico/Ahlan 竞品对比、Tier 1-4 营收模式、差异化机会 |
| 2 | [功能路线图](./roadmap.md) | Phase 0 (MVP) → Phase 1 (营收) → Phase 2 (社交) → Phase 3 (高级运营) 各阶段四端功能清单 |
| 3 | [业务流程与规则](./business_flows.md) | 一键登录、管理员登录、虚拟礼物打赏、麦位管理、贵族购买、跨服务通信等完整正向+异常流程 |
| 4 | [用户画像与场景](./user_personas.md) | 超级大R/中R/小R/白嫖用户分层、主播生态（头部/腰部/长尾） |
| 5 | [差异化竞争力](./differentiation.md) | 技术优势 (Rust/RTL/延迟)、产品差异化 (AI运营/Web3/极致本土化) |
| 6 | [后台管理系统设计](./admin_dashboard.md) | Web Admin 八大功能模块详细设计：数据看板、用户管理、房间管理、财务、素材、运营、分析、RBAC |
| 7 | [风险与应对](./risks.md) | 合规风险 (支付牌照/内容审核/数据主权)、竞争风险 |
| 8 | [成功指标与术语表](./kpi.md) | Phase 1 KPI 目标、术语表 |
| 9 | [Android App 界面设计规范](./android_app_design.md) | Splash/登录/首页三Tab/房间交互等全套 UI 流程与中东风格设计 |
| 10 | [Phase 1 虚拟礼物与钱包闭环 MVP](./phase1_gift_economy.md) | E-07 Epic 方向总纲：钱包/礼物/榜单 MVP 范围、分层特效、MENA 礼物清单 |
| 11 | [Phase 1 埋点与观测性基建](./phase1_observability.md) | E-07.5 Epic 方向总纲：Sentry 选型、WS 通道上报设计、核心事件字典 |
| 12 | [Phase 1.5 房间主权与管理员体系](./phase1_room_governance.md) | E-10 Epic 方向总纲：创建升级/观众席/房主与管理员权限/踢人禁麦禁言 |

---

## Epic 列表与完成状态

| Epic | 阶段 | 状态 | 关联 Tasks |
|------|------|------|-----------|
| E-01: 工程基建 | Phase 0 | ✅ 已完成 | T-0000A ~ T-0000D |
| E-02: 用户认证系统 | Phase 0 | ✅ 已完成 | T-00001~T-00005, T-10001~T-10003, T-20001~T-20002, T-30001~T-30004 |
| E-03: 房间大厅与列表 | Phase 0 | ✅ 已完成 | T-00006~T-00010, T-10004~T-10006, T-20003~T-20005, T-30005~T-30007 |
| E-04: 房间内核心功能 | Phase 0 | ✅ 已完成 | T-00011~T-00016, T-00011B~T-00011C, T-10007~T-10012, T-20006~T-20009, T-30008~T-30017 |
| **E-05: 中东黑金主题与 App 壳体** | **Phase 0.5** | ✅ 已完成 (9/9) | T-30018~T-30026 (Android 9 Tasks) |
| **E-06: Web 管理端增强** | **Phase 0.5** | ✅ 已完成 (2/2) | T-20010 ✅, T-20011 ✅ |
| **E-07: 虚拟礼物与钱包闭环 MVP** | **Phase 1** | ✅ **已完成 (15/15)** | T-00017 ✅（钱包 Schema）, T-00018 ✅（余额 API + WS 推送）, T-00019 ✅（礼物配置表+列表API）, T-00020 ✅（SendGift 事务+广播）, T-00021 ✅（魅力/财富榜单 API）, T-10013 ✅（Admin 手动调整余额）, T-10014 ✅（Admin 礼物 CRUD）, T-20012 ✅（Web 余额调整弹窗+礼物管理页），T-30027 ✅（Android 钱包页），T-30028 ✅（Android 礼物面板），T-30029 ✅（Android 接收者选择器），T-30030 ✅（Android SendGift 客户端+幂等），T-30031 ✅（Android 送礼特效+弹幕），T-30032 ✅（Android 余额不足引导弹窗），**T-30033 ✅（Android 魅力/财富榜页）** |
| **E-07.5: 埋点与观测性基建** | **Phase 1 并行** | 🟡 **设计中 (0/~6)** | T-00022~T-00023, T-10015, T-20013, T-30034~T-30035 （待拆解） |
| **E-10: 房间主权与管理员体系** | **Phase 1.5** | 🟡 **设计中 (0/~18)** | 待拆解，预计 Server 7 + AdminServer 1 + Web 1 + Android 9 |
| E-08: Google Play 真支付 | Phase 1 | 🔴 待开发 | 待拆解（依赖 E-07） |
| E-09: 贵族体系 | Phase 1 | 🔴 待开发 | 待拆解（依赖 E-07/E-08） |

---

## 设计文档索引

| 目录 | 内容 |
|------|------|
| [doc/design/android/](../design/android/) | Android 端 UI 设计文档（按 TaskId 命名） |
| [doc/design/adminWeb/](../design/adminWeb/) | Web Admin 端 UI 设计文档（按 TaskId 命名） |

---

**文档变更历史**:
- 2026-04-28: v1.0，T-30033 DoD 完成，E-07 Epic 进度更新为 15/15（新增 T-30033 ✅ Android 魅力/财富榜页，Review R2 通过，18 个单元测试全部通过）；doc/arch/android/ranking.md 新增完整的 RankingScreen 架构设计（四组 Tab 独立加载、防腐层 IRankingRepository + RetrofitRankingRepository、竞态取消机制 loadingJob、Top3 金银铜光圈+Top1 王冠、MyRankFooter 粘性底部、下拉刷新与错误重试、大厅+房间菜单双入口），doc/arch/android/index.md 新增 ranking.md 子模块索引，doc/product/index.md E-07 Epic 状态更新为 ✅ 已完成 (15/15)
- 2026-04-27: v0.9，T-30032 DoD 完成，E-07 Epic 进度更新为 14/15（新增 T-30032 ✅ Android 余额不足引导弹窗，Review R2 通过，10 个 TDD 验收用例全部通过）；doc/arch/android/gift.md 新增第六章完整的 InsufficientBalanceDialog 架构（触发机制、弹窗设计、状态与事件、集成方式、回调拆分修复），Tasks.md T-30032 标记为 ✅ Done（负责人: Dod）
- 2026-04-26: v1.8，T-30031 DoD 完成，E-07 Epic 进度更新为 13/15（新增 T-30031 ✅ Android 送礼特效播放器+弹幕，Review R2 通过，全部 15 个 TDD 验收用例通过）；doc/arch/android/gift.md 新增第十章完整的送礼特效架构（GiftEffectController L1/L2/L3 三级特效、ILottiePlayer 防腐层、GiftDanmakuMessage 弹幕组件、GiftReceivedEvent 字段约定），Tasks.md T-30031 标记为 ✅ Done（负责人: Dod）
- 2026-04-25: v1.7，T-30030 DoD 完成，E-07 Epic 进度更新为 12/15（新增 T-30030 ✅ Android SendGift 客户端+幂等，Review R2 通过，366+ tests 全部通过）；doc/arch/android/gift.md 补充 SendGiftJob、ComboAggregator、GiftEvents 新增类，sendGift() 幂等流程，Gson JsonObject 安全构造模式说明；Tasks.md T-30030 标记为 ✅ Done（负责人: Dod）
- 2026-04-24: v1.6，T-30028 DoD 完成，E-07 Epic 进度更新为 10/15（新增 T-30028 ✅ Android 礼物面板，Review R2 通过，336+ tests 全部通过）；Tasks.md T-30028 标记为 ✅ Done（负责人: Dod）
- 2026-04-23: v1.5，T-30027 DoD 完成，E-07 Epic 进度更新为 9/15（新增 T-30027 ✅ Android 钱包页，Review R2 通过）；Android 架构文档新增 wallet.md 子模块，Tasks.md T-30027 标记为 ✅ Done（负责人: Dod）
- 2025-07-17: v1.4，T-10014 DoD 完成，E-07 Epic 进度更新为 7/15（新增 T-10014 ✅ Admin 礼物 CRUD 管理 API）；Admin Server 架构文档新增 gift.md 子模块，Tasks.md T-10014 标记为 Done（负责人: Dod）
- 2025-07-16: v1.0，T-10013 DoD 完成，E-07 Epic 进度更新为 6/15（新增 T-10013 ✅ Admin 手动调整余额）；Admin Server 架构文档新增 wallet.md 子模块
- 2026-04-22: v0.9，T-00021 DoD 完成，E-07 Epic 进度更新为 5/15（T-00017 ✅ + T-00018 ✅ + T-00019 ✅ + T-00020 ✅ + T-00021 ✅）；Protocol 协议文档新增 ranking_api.md §九 魅力/财富榜单 API 接口定义
- 2025-06-27: v0.8，T-00020 DoD 完成，E-07 Epic 进度更新为 4/15（T-00017 ✅ + T-00018 ✅ + T-00019 ✅ + T-00020 ✅）；Server 架构文档新增 gift.md 子模块（T-00019 + T-00020），Protocol 协议文档 websocket_signals.md §6.4.2 SendGift 错误码与 §6.4.3 GiftReceived payload 已同步
- 2025-07-15: v0.7，T-00018 DoD 完成，E-07 Epic 进度更新为 2/15（T-00017 ✅ + T-00018 ✅）；Server 架构文档新增 wallet.md 子模块，Protocol 协议文档 websocket_signals.md §6.4.1 BalanceUpdated 已完整同步 msg_id 字段
- 2026-04-21: v0.6，启动 E-07 虚拟礼物与钱包闭环 MVP；新增 `phase1_gift_economy.md` 方向总纲；`competitors.md` 追加附录 A（礼物UX/榜单/MENA文化）；`business_flows.md` 追加 §2.7 钱包礼物闭环细化流；Tasks.md 新增 15 个 Task（T-00017~T-00021 / T-10013~T-10014 / T-20012 / T-30027~T-30033）
- 2026-05-16: v0.5，T-20011 活水房间监控增强完成，E-06 Epic 状态更新为 ✅ 已完成 (2/2)
- 2026-04-20: v0.4，文档架构重构，将原 `product.md` 拆分为索引 + 9 个模块化子文件
- 2026-04-18: v0.3，章节编号修正，新增跨服务通信、Admin Server 技术栈说明
- 2026-04-18: v0.2，明确四端定位，Web 端重定位为后台管理系统
- 2026-04-17: v0.1，初始版本
