# ADR-0003 — 协议字段机器可读 Schema（JSON Schema 2020-12）

| 属性 | 值 |
|------|-----|
| **编号** | ADR-0003 |
| **状态** | Accepted |
| **决策日期** | 2026-05-15 |
| **关联任务** | T-00100 (协议字段全量冻结) |
| **前置 ADR** | ADR-0002 (协议单一来源与绑定表) |

---

## 背景 (Context)

ADR-0002 确立了协议文档作为前后端契约单一来源的原则，并引入了 `protocol-binding-audit.ts` 工具验证 TDS 绑定表 ↔ 服务端实现 ↔ 客户端调用的三角对账。

然而，现有的 markdown 文档是**描述性**的——字段名、类型、约束等信息以自然语言表达，无法被工具直接解析验证。这导致：

1. 字段命名风格漂移（`camelCase` 混入 `snake_case`）
2. 新增信令时缺乏结构约束模板
3. 跨端（Android / Web / Admin）联调时各自理解不一致
4. 无法在 CI 中自动验证消息格式合规性

---

## 决策 (Decision)

在 `doc/protocol/schemas/` 下为所有协议消息创建 **JSON Schema 2020-12** 格式的机器可读 schema 文件，并配套以下三条铁律（写入 `conventions.md`）：

### 铁律一：字段命名 — snake_case 强制（§4）
所有协议字段（payload 字段、HTTP body 字段、Redis 事件字段）必须使用 `snake_case`。`type` 枚举值使用 PascalCase（WS信令）或 snake_case（Redis事件），遵循服务端 serde 标签。

### 铁律二：WS payload 嵌套（§5）
所有 WebSocket 信令的业务字段必须嵌套在顶层 `payload` 对象内，不得直接暴露在 envelope 根层级。

### 铁律三：envelope 双 ID（§6）
所有 C→S 信令必须携带客户端生成的 `msg_id`（UUID v4）；S→C 应答必须回显 `msg_id` 并附带服务端 `timestamp`（Unix ms）。

### Schema 目录结构

```
doc/protocol/schemas/
├── ws/          # 28 个 WS 信令 schema
├── http/        # HTTP REST DTO schema（RoomDetail 等）
└── pubsub/      # Redis Pub/Sub 事件 schema（admin:events 频道，4 个）
```

所有 schema 文件使用 `"additionalProperties": false` 强制字段封闭（禁止未声明字段）。

---

## 影响 (Consequences)

### 正面影响
- **字段冻结**：新字段必须先修改 schema 才能在实现中引入，强制文档先行。
- **工具可验证**：可用 `ajv`、`jsonschema` 等工具在 CI 中验证服务端输出和客户端输入。
- **跨端一致**：Android/Web/Admin 以同一份 schema 为参考，消除理解歧义。
- **代码生成**：可基于 schema 自动生成 TypeScript 类型、Kotlin data class、Rust struct 等。

### 负面影响/权衡
- **维护成本**：每新增信令需同时维护 markdown 文档和 schema 文件。
  → 缓解：`validate-protocol-freeze.sh` 自动检测缺失 schema，防止漏更新。
- **历史债务**：现有实现并非 100% 严格遵循（如 `ping`/`pong` 小写兼容期）。
  → 缓解：在 TDS T-00100 §四 记录兼容期策略，逐步迁移。

---

## 变更历史

| 日期 | 变更 | 作者 |
|------|------|------|
| 2026-05-15 | 初始版本 | T-00100 自动生成 |
