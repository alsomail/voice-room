# ADR-0002: 协议单一事实源与 TDS 绑定表铁律

- **状态**: Accepted
- **日期**: 2026-05-06
- **提案人**: 架构团队
- **关联 Task**: [T-0000V](../tds/infra/T-0000V.md)

---

## 背景

### 根因事件：BUG-CHAT-WS Round 16 协议漂移

2026-05-05，BUG-CHAT-WS 修复链追踪至 Round 16 时，发现真正的系统性根因：

**双重事实源导致客户端误用副路径。**

具体表现：
1. `doc/ARCHITECTURE.md`（遗留文件，含旧版信令格式示例）与 `doc/architecture/websocket_and_state.md`（当前维护版）同时存在
2. `websocket_and_state.md §8.2` 曾包含 `APPLY_SEAT`/`SEAT_UPDATED` 等 JSON 格式示例，但这些示例未与 `doc/protocol/` 中的权威定义保持同步
3. Android 端 `RoomViewModel.sendMessage` 在 Round 12 实证中被确认走的是**死代码路径**（`cf899bd` 提交），`wsClient.connect` 从未在生产路径被调用——根因正是开发者查阅了不一致的旧文档

这一问题在之前 13 轮修复尝试中均未被识别为文档治理问题，直到 Round 16 全链路 diff 对账才暴露。

### 现有痛点

| 痛点 | 影响 |
|------|------|
| 两个信令格式定义来源（`ARCHITECTURE.md` + `websocket_and_state.md §8.2`） | 开发者不知该以哪个为准 |
| TDS 无强制要求跨端路径绑定声明 | Plan/TDD/Review 阶段无法发现协议漂移 |
| 无自动化审计工具 | 协议路径偏差只能在 E2E 实跑时才被发现 |
| `doc/ARCHITECTURE.md` 废弃但未物理删除 | 仍可被搜索到，持续误导开发者 |

---

## 决策

### D-1: 协议单一事实源

**所有跨端 API/WebSocket 路径的唯一权威定义位于 `doc/protocol/index.md`。**

- `doc/architecture/` 中的任何文档只能**引用** `doc/protocol/index.md`，不得**重复定义**具体信令格式
- 如两处描述存在冲突，以 `doc/protocol/index.md` 为准
- `doc/ARCHITECTURE.md` 已于 2026-05-06 由 T-0000V 物理删除，不得复活

### D-2: TDS 强制协议路径绑定表

**所有涉及跨端通信的 TDS 第二节必须包含完整的「协议路径绑定表」。**

绑定表最低要求：
- 客户端实际调用入口（文件路径 + 函数名 + 信令 `type` 值）
- 服务端处理函数（文件路径 + 函数名）
- `doc/protocol/` 锚点
- 客户端选用路径标注 ⭐

判定规则：
- 纯单端内部逻辑 → 显式写 `N/A — 仅 X 端内部`
- 有跨端通信但绑定表为空或缺锚点 → **退回 Plan 重做，禁止流转 TDD**

### D-3: 自动化审计作为 CI 门禁

**`npm run audit:protocol` 作为 CI 强制门禁（T-0000T 落地）：**

- 解析所有 `doc/tds/**/T-*.md` 第二节绑定表
- grep server `Router::route` / WS `match envelope.r#type` 实现入口
- grep android `wsClient.send` / Retrofit、web `apiClient.*` 真实调用
- 三方比对，P0 级别不一致时以非 0 退出码阻断 PR 合并

---

## 结果与影响

### 正面影响

1. **消除双重事实源**：物理删除 `ARCHITECTURE.md` + §8.2 明确指向单一源，开发者不再迷失于多个文档
2. **强制 TDS 对账**：Plan 阶段即确认协议路径，比 E2E 实跑提前 N 轮发现漂移
3. **自动化防退化**：`audit:protocol` CI 门禁持续守护，新功能 PR 无法跳过协议对账

### 负面影响 / 迁移成本

1. **历史 TDS 回填**：约 140 份历史 TDS 需补充绑定表声明（T-0000U 批量完成）
2. **研发流程变更**：Plan 阶段新增绑定表填写负担，预估每 Task 增加 15-30 分钟

### 影响范围

- 全部 4 端（App Server / Admin Server / Web / Android）
- 所有 TDS 文件（`doc/tds/**/*.md`）
- 所有协调器 Agent（`.github/agents/` 中的 Plan/TDD/Review 红线注入）
- CI pipeline（`.github/workflows/` 中的 PR 检查）

---

## 反例参考

### BUG-CHAT-WS Round 12 → Round 16 演化

```
Round 12: 发现 wsClient.connect 未在生产路径调用（commit cf899bd 为死代码）
Round 13: 注入真正的 connect 调用
...
Round 16: 追根溯源 → ARCHITECTURE.md 旧文档误导 + §8.2 信令示例过时
根因: 开发者查阅了 doc/ARCHITECTURE.md（废弃文件）中的 APPLY_SEAT 示例
      未意识到 doc/protocol/index.md 中该信令已更新
```

本次修复总计耗费 **16+ 轮 E2E 迭代**，若 ADR-0002 在 Round 1 即落地，预计 **Round 3 即可闭环**。

---

## 相关文档

- [协议索引](../protocol/index.md) — 单一事实源
- [WebSocket 架构](../architecture/websocket_and_state.md) — 架构层设计（非协议定义）
- [协议绑定审计脚本](../arch/infra/protocol-binding-audit.md) — T-0000T 落地
- [T-0000V TDS](../tds/infra/T-0000V.md) — 本次文档清理任务
- [T-0000U TDS](../tds/infra/T-0000U.md) — 历史 TDS 绑定表回填
- [T-0000T TDS](../tds/infra/T-0000T.md) — 审计脚本落地
