# 系统架构文档索引

> **原始文件**: `doc/ARCHITECTURE.md`（已拆分为本目录下的子文件）
> **拆分日期**: 2026-04-20

本目录包含实时语聊房项目的完整系统架构规范。按主题拆分为以下子文件，便于精准检索和增量更新。

---

## 📑 子文件索引

| # | 文件 | 内容概要 | 原章节 |
|---|------|---------|--------|
| 0 | [goals_and_overview.md](goals_and_overview.md) | 文档目标、四端技术栈、总体架构 Mermaid 图、分层原则 | §1-§2 |
| 1 | [directory_and_ddd.md](directory_and_ddd.md) | Monorepo 目录结构、目录总原则 | §3 |
| 2 | [domain_design.md](domain_design.md) | 业务域拆分、bounded context、模块结构、Rust 分层规范 | §4 |
| 3 | [android_architecture.md](android_architecture.md) | Android Clean Architecture + MVVM、关键接口 | §5 |
| 4 | [web_architecture.md](web_architecture.md) | Web Admin 后台架构定位与约束 | §6 |
| 5 | [api_and_auth.md](api_and_auth.md) | HTTP 统一返回体、JWT 鉴权、WS 鉴权与 Session 绑定 | §7 |
| 6 | [websocket_and_state.md](websocket_and_state.md) | WS 信令格式、房间状态同步、RoomStateRepository、幂等防重 | §8 |
| 7 | [transaction_and_gift.md](transaction_and_gift.md) | 送礼事务强一致性、事务边界、表结构、广播时机 | §9 |
| 8 | [anticorruption_layer.md](anticorruption_layer.md) | 客户端 & 服务端防腐层接口定义 | §10 |
| 9 | [resilience.md](resilience.md) | 弱网高可用：心跳、重连、乐观 UI、优雅降级、状态回补 | §11 |
| 10 | [observability.md](observability.md) | 结构化日志、客户端埋点防腐层、MENA 弱网上报、崩溃捕获、合规 | §12 |
| 11 | [mena_localization.md](mena_localization.md) | 中东本土化：i18n、RTL、时间与数字格式 | §13 |
| 12 | [code_standards.md](code_standards.md) | 各端 Lint/格式化规范、Git Hooks 与 CI | §14 |
| 13 | [environments_cicd.md](environments_cicd.md) | 多环境配置、CI/CD、Gateway、实施红线、落地优先级 | §15-§17 |

---

## 🔗 关联文档

- **协议契约**: [doc/protocol/index.md](../protocol/index.md)
- **产品需求**: [doc/product/index.md](../product/index.md)
- **任务看板**: [doc/tasks/index.md](../tasks/index.md)
- **各端实现架构**: `doc/arch/{server,adminServer,android,web}/index.md`
