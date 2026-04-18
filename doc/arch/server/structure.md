# Server 启动、配置与目录结构

## 一、 目录与文件现状

| 路径 | 职责 | 当前状态 |
| --- | --- | --- |
| `app/shared/` | Workspace 共享 crate：JWT 工具、密码哈希、错误码、数据模型、新类型 | 🟢 已落地 |
| `app/server/src/main.rs` | 读取配置、初始化 tracing、构建应用并启动 Axum Server | 🟢 已落地 |
| `app/server/src/bootstrap/mod.rs` | 注册 `/ping` 路由并挂载请求上下文中间件 | 🟢 已落地 |
| `app/server/src/common/request_context.rs` | 定义 `RequestContext`，封装 `request_id` | 🟢 已落地 |
| `app/server/src/infrastructure/config.rs` | 负责 `.env`、`config/*.toml` 与环境变量覆盖的配置加载 | 🟢 已落地 |
| `app/server/src/infrastructure/logging.rs` | 负责 tracing 初始化、`x-request-id` 透传与请求日志 span | 🟢 已落地 |
| `app/server/src/modules/mod.rs` | 业务模块预留入口 | 🔴 仍为空壳 |
| `app/server/src/lib.rs` | 暴露模块并包含 `/ping` 集成测试 | 🟢 已落地 |

## 二、 当前启动流程

1. `ServerSettings::load()` 从 `.env`、`config/default.toml`、`config/{env}.toml` 与环境变量构建配置。
2. `init_tracing()` 按 `log.format` 选择 JSON 或普通文本日志。
3. `build_app()` 注册 `/ping` 并注入 `request_context_middleware`。
4. Server 监听 `settings.server.bind_addr()`，并支持 `Ctrl+C` / `SIGTERM` 优雅退出。

## 三、 配置来源

| 来源 | 说明 |
| --- | --- |
| `.env.example` | 提供 `APP_ENV`、`DATABASE_URL`、日志配置等本地模板 |
| `config/default.toml` | 默认配置基线 |
| `config/dev.toml` / `test.toml` / `prod.toml` | 分环境覆盖 |
| 环境变量 | 支持 `APP__SERVER__HOST`、`APP__SERVER__PORT`、`APP__LOG__LEVEL`、`APP__LOG__FORMAT` 等覆盖 |

## 四、 当前测试面

- `src/lib.rs` 中的 `#[tokio::test]` 会直接对 `build_app()` 发起请求。
- 测试覆盖点为：
  - `/ping` 返回 `200 OK`
  - 响应头包含 `x-request-id`
  - 响应体中的 `request_id` 与响应头一致

## 五、 结论
当前 Server 端更接近“可运行的生产骨架”，还不是“可支撑业务的领域服务”。后续应优先补上鉴权、模块边界、数据库访问与 WS 广播模型。
