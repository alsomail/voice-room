# Voice Room 产品文档总索引

> **版本**: v0.4  
> **更新日期**: 2026-04-20  
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

---

## Epic 列表与完成状态

| Epic | 阶段 | 状态 | 关联 Tasks |
|------|------|------|-----------|
| E-01: 工程基建 | Phase 0 | ✅ 已完成 | T-0000A ~ T-0000D |
| E-02: 用户认证系统 | Phase 0 | ✅ 已完成 | T-00001~T-00005, T-10001~T-10003, T-20001~T-20002, T-30001~T-30004 |
| E-03: 房间大厅与列表 | Phase 0 | ✅ 已完成 | T-00006~T-00010, T-10004~T-10006, T-20003~T-20005, T-30005~T-30007 |
| E-04: 房间内核心功能 | Phase 0 | ✅ 已完成 | T-00011~T-00016, T-00011B~T-00011C, T-10007~T-10012, T-20006~T-20009, T-30008~T-30017 |
| **E-05: 中东黑金主题与 App 壳体** | **Phase 0.5** | 🟡 进行中 (8/9) | T-30018~T-30026 (Android 9 Tasks) |
| **E-06: Web 管理端增强** | **Phase 0.5** | 🔴 待开发 | T-20010~T-20011 (Web 2 Tasks) |
| E-07: 虚拟礼物打赏 | Phase 1 | 🔴 待开发 | 待拆解 |
| E-08: 虚拟货币充值 | Phase 1 | 🔴 待开发 | 待拆解 |
| E-09: 贵族体系 | Phase 1 | 🔴 待开发 | 待拆解 |

---

## 设计文档索引

| 目录 | 内容 |
|------|------|
| [doc/design/android/](../design/android/) | Android 端 UI 设计文档（按 TaskId 命名） |
| [doc/design/adminWeb/](../design/adminWeb/) | Web Admin 端 UI 设计文档（按 TaskId 命名） |

---

**文档变更历史**:
- 2026-04-20: v0.4，文档架构重构，将原 `product.md` 拆分为索引 + 9 个模块化子文件
- 2026-04-18: v0.3，章节编号修正，新增跨服务通信、Admin Server 技术栈说明
- 2026-04-18: v0.2，明确四端定位，Web 端重定位为后台管理系统
- 2026-04-17: v0.1，初始版本
