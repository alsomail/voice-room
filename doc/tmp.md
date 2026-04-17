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

## android 基础框架搭建

- 初始化 `app/android` 的 Gradle/Kotlin Android 工程，补齐 `settings.gradle.kts`、顶层 `build.gradle.kts`、wrapper、`gradle.properties`、`.editorconfig` 与 `.gitignore`。
- 按架构约束建立 `core / common / data / domain / presentation / feature` 分层目录，并预留 `auth`、`room`、`gift`、`wallet`、`profile` 等业务模块入口。
- 在 `core` 层补齐网络、WebSocket、遥测、媒体、IM、配置等基础能力接口，第三方能力暂以 `NoOp*` / `Debug*` 适配器占位，避免业务层直接依赖外部 SDK。
- 新增 `AppEnvironment`、`AppHttpClientFactory`、`RoomSocketRequestFactory`、`AppContainer`，先把环境配置、HTTP 客户端、WS 请求构造和依赖装配链路打通。
- 搭建 `MainActivity`、`MainViewModel`、`MainUiState` 与基础 XML 资源，先提供可启动的壳层页面，并补齐 `values-ar/strings.xml` 以覆盖阿拉伯语文案入口。
- 当前房间、登录、钱包、礼物等数据链路仍是骨架态，Repository / Service 侧以调试实现为主，后续需要继续接真实 API、WS 信令与 RTC/IM 防腐层。
- 已补充单元测试与 smoke test 骨架，覆盖 `AppContainer`、环境识别、HTTP 工厂、WS 请求工厂、`MainViewModel` 等基础模块。
