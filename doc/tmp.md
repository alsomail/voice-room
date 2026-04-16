# 今日改动简记

## server 检查与补全

- 补全 `app/server` 的启动骨架，统一由 `bootstrap` 装配 Axum 路由。
- 实现 `GET /ping`，返回 JSON，并在日志中输出带 `request_id` 的访问记录。
- 接通 tracing 初始化与请求上下文中间件，服务默认监听 `3000` 端口。
- 新增 `#[tokio::test]`，验证 `/ping` 的响应体和 `x-request-id`。
- 执行并通过 `cargo check`、`cargo test`，同时完成一次本地 `/ping` 活体验证。

## web 基础框架搭建

- 初始化 `app/web` 的 React + Vite + TypeScript 基础工程。
- 创建 `.env.development`、`.env.production`、`.env.example`，统一使用 `VITE_` 前缀。
- 建立 `src/app`、`src/api`、`src/core`、`src/features`、`src/pages`、`src/services` 等核心目录。
- 在 `src/core/telemetry/` 下新增 `IAnalyticsService`、`ICrashReporter`、`MockAnalyticsService`。
- Mock 埋点防腐层支持自动注入 `device_id`、`os_version`、`network_type`、`locale`、`timezone`、`environment`、`analytics_endpoint`。
- 增加 URL 拼接与 telemetry 相关测试，补齐基础的路径约束与保留字段保护。
- 执行并通过 `npm test`、`npm run typecheck`、`npm run lint -- --fix`、`npm run build`。
