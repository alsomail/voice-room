# Server 能力状态与缺口盘点

## 一、 已实现能力

### 1. 健康检查
- 已提供 `GET /ping`。
- 返回体结构为 `{ "status": "ok", "request_id": "..." }`。
- 可以复用作存活探针与链路排障的最小入口。

### 2. 请求上下文
- 中间件会优先透传请求头中的 `x-request-id`。
- 若上游未提供，则自动生成 UUID。
- `request_id` 会同时进入响应头与 tracing span。

### 3. 配置与日志骨架
- 已支持 `.env` + `config/*.toml` + 环境变量覆盖的组合加载。
- 日志格式支持 `json` 与普通文本两种模式。
- 启动日志会携带 `service_name`、`environment`、`host`、`port` 等字段。

## 二、 未实现能力

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| 鉴权与 Claims | 🔴 未开始 | `common/auth` 尚不存在 |
| 业务域模块 | 🔴 未开始 | `modules/` 仅有 `mod.rs` |
| 数据库连接池与 SQLx 事务 | 🟡 依赖已就绪 | Workspace `Cargo.toml` 已固定 `sqlx = "0.8"`，`app/server/Cargo.toml` 已引用，但运行链路（连接池初始化、Repository 层）尚未接入 |
| WebSocket 网关 | 🔴 未开始 | 尚无 WS 路由、状态机、广播或回补 |
| 第三方防腐层 | 🔴 未开始 | `infrastructure/third_party/*` 尚未落地 |

## 三、 文档维护约束

- 当新增 HTTP/WS 契约时，应同步补齐 `doc/protocol.md` 或对应协议文档。
- 当 `modules/` 下开始出现业务域时，需要为每个重要域继续拆分子文档，而不是把细节堆回 `index.md`。
- 当数据库与事务接入后，需要在本目录同步补充“事务边界”和“幂等策略”说明。
