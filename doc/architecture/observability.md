# 12. 可观测性、埋点与中东运维基建 (Observability & MENA Telemetry)

针对中东语聊房的强数据驱动特性及复杂的跨国网络环境，系统必须建立"低侵入、可替换、抗弱网"的统一观测基建。严禁任何"直连服务器查日志"的原始运维操作。

## 12.1 服务端结构化日志与追踪 (Server Logging & Tracing)
Rust 服务端统一使用 `tracing` 库输出结构化 JSON 日志。
- **强制上下文**：核心业务日志必须附带 `request_id`, `trace_id`, `user_id`, `room_id`, `msg_id`，以串联跨系统的分布式调用（如送礼事务、第三方 API 调用）。
- **精准日志与防刷屏**：
  - `ERROR`：仅用于需人类介入的故障（如资金不一致、数据库宕机）。
  - `WARN`：可恢复异常（风控拦截、重试）。
  - `INFO`：关键里程碑（进房、支付成功）。
  - `DEBUG`：高频事件（WS心跳、音量回调）必须降级为 DEBUG，并配合采样/限流逻辑，严禁在生产环境导致日志雪崩。
- **旁路采集架构**：Rust 进程只负责将日志输出到标准输出（stdout）或本地轮转文件，由独立的守护进程（如 FluentBit / Filebeat）异步采集并上报至集中式日志中心，**绝对禁止在 Rust 业务代码中同步发送网络请求上报日志**。

## 12.2 客户端日志与埋点防腐层 (Client Telemetry Anti-Corruption Layer)
为应对服务商在中东节点的不稳定性及未来的合规替换需求，Android 与 Web 端必须建立隔离层，严禁在业务 UI 中硬编码 `Firebase`, `AppsFlyer` 或 `SensorsData` 的 API。

- **统一门面接口 (Facade)**：
  必须定义全局接口 `IAnalyticsService` 和 `ICrashReporter`。
  ```typescript
  // 统一接口定义，底层按需注入 FirebaseAdapter 或 自研 Adapter
  export interface IAnalyticsService {
    trackEvent(eventName: string, payload: Record<string, any>): void;
    setUserProperties(props: Record<string, any>): void;
  }
  export interface ICrashReporter {
    logBreadcrumb(message: string): void;
    reportError(error: Error, context?: Record<string, any>): void;
  }
  ```
- **公共参数自动注入**：Adapter 底层必须自动拼装环境参数（`device_id`, `os_version`, `network_type`, `locale`, `timezone`）与业务参数（`user_id`, `room_id`），禁止业务层重复传递。
- **无侵入采集**：Web 端利用 HOC 或 `IntersectionObserver` 捕获曝光；Android 利用 `LifecycleObserver` 捕获页面停留。

## 12.3 面向中东弱网的上报策略 (MENA-Optimized Reporting)
中东部分地区网络波动大，且跨国 TCP 握手成本高，客户端的上报底层（Adapter 内部实现）必须遵循以下机制：
- **批量与节流 (Batching)**：除核心支付事件外，常规点击、曝光和 Info 级本地日志必须在内存中缓冲（Buffer），满 N 条或满 M 秒后合并为一次 HTTP 请求上报。
- **极致压缩 (Compression)**：上报的 Payload 必须强制启用 Gzip 或 Zstd 压缩，最大限度节省中东用户的流量带宽。
- **断网持久化与重传**：断网时，必须将埋点和崩溃日志落盘到本地（Android SQLite / Web IndexedDB）。网络恢复后，采用指数退避算法（Exponential Backoff）按先进先出（FIFO）顺序重传。
- **边缘加速 (Edge Acceleration)**：上报网关的域名必须配置全球 CDN 或 Anycast IP，确保中东用户能就近接入边缘节点，避免直接跨大洋上报导致的极高丢包率。

## 12.4 崩溃捕获与故障现场保留 (Crash Handling & Breadcrumbs)
- **全局捕获**：Android 必须配置全局 `UncaughtExceptionHandler`；Web 必须配置 `ErrorBoundary` 与全局 `unhandledrejection` 监听。
- **面包屑导航 (Breadcrumbs)**：发生致命异常时，上报的载荷中必须包含用户最近的 10 步操作轨迹（如"点击上麦 -> 收到 WS 确认 -> 渲染动画 -> 崩溃"）及最近的网络状态，供开发者极速复现。

## 12.5 在线分析、检索与合规基建 (Analytics & Data Compliance)
统一日志与埋点的在线查看规范，提升排障与数据运营效率：

- **服务端日志检索 (ELK / Cloud Native)**：
  - 日志汇聚至云厂商的集中日志服务（如 AWS CloudWatch, 阿里云 CLS 或自建 Kibana）。
  - 支持按 `request_id` 或 `user_id` 全文检索和 Live Tail（实时滚动查看）。
  - 大规模排查时，支持将查询结果导出/下载为 CSV/JSON。
- **客户端行为分析 (BI Dashboards)**：
  - 产品与运营通过埋点后台（如 神策分析、Firebase Analytics）查看漏斗（Funnel）、留存（Retention）和事件分布面板。
  - 开发通过 Crashlytics 或 Bugly 后台查看带符号表（Deobfuscated）解析的崩溃堆栈。
- **中东数据合规 (Data Residency)**：
  为了应对沙特 (KSA) 和阿联酋 (UAE) 等国家的数据本地化隐私法规，日志接收网关和数据仓库的物理节点应优先部署在中东本地云机房（ME Regions），隔离敏感用户隐私数据出境。
