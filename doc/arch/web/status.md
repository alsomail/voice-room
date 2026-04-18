# Web Telemetry 与网络能力现状

## 一、 已实现能力

### 1. Telemetry Mock

`src/core/telemetry/` 已完成以下骨架：

- `IAnalyticsService`：定义埋点接口
- `ICrashReporter`：定义崩溃上报接口
- `MockAnalyticsService`：自动注入基础上下文后再输出事件
- `telemetryContext.ts`：生成 `device_id`、`os_version`、`network_type`、`locale`、`timezone`、`environment`、`analytics_endpoint`

其中 `device_id` 会优先复用 `localStorage` 中的缓存值，失败时退回内存态随机标识。

### 2. URL 安全拼接

`src/lib/url.ts` 的 `joinConfiguredUrl()` 已实现：

- 禁止传入绝对 URL 绕过配置基地址
- 禁止使用 `.` / `..` 路径段逃逸前缀
- 校验最终解析结果仍停留在配置的 base URL 范围内

### 3. HTTP / WS 最小 Helper

- `apiClient(path, init)`：通过 `fetch` 请求配置的 API base
- `createRoomSocket(path)`：通过 `WebSocket` 连接配置的 WS base

当前两者都只负责地址拼接，不处理鉴权、重试、错误归一化或协议编解码。

## 二、 已有测试

| 测试文件 | 覆盖范围 |
| --- | --- |
| `src/core/telemetry/MockAnalyticsService.test.ts` | 基础上下文注入、保留字段保护、设备 ID 复用 |
| `src/lib/url.test.ts` | API/WS URL 拼接、绝对地址绕过、路径逃逸防护 |

## 三、 尚未落地的部分

| 能力 | 当前状态 |
| --- | --- |
| 路由系统与页面编排 | 🔴 未开始 |
| 全局状态管理 / Provider | 🔴 未开始 |
| i18n / RTL | 🔴 未开始 |
| 真实 telemetry / crash provider | 🔴 未开始 |
| 真实业务 API、鉴权、房间协议 | 🔴 未开始 |

## 四、 文档维护提示

- 当 `services/` 下开始接入 RTC、IM、analytics、crash provider 时，应继续拆分独立子文档。
- 当页面从单页壳层扩展为多路由结构时，应在本目录新增路由与状态管理文档，而不是仅更新此文件。
