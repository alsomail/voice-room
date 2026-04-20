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

## 三、 业务能力现状

> T-20001 ~ T-20009 已全部落地（详见 [index.md](./index.md) 能力矩阵）。

| 能力 | 当前状态 | 说明 |
| --- | --- | --- |
| 路由系统与页面编排 | 🟢 已完成 | BrowserRouter + AuthGuard，Login / Dashboard / Rooms / Users / Logs 五大页面 |
| 全局状态管理 | 🟢 已完成 | Zustand `useAuthStore`（JWT 持久化、登录/登出） |
| i18n | 🟢 已完成 | `src/i18n/`（en / zh），RTL 由 Ant Design ConfigProvider 支持 |
| Admin 登录 | 🟢 已完成 | T-20001：LoginForm + useAuthStore + JWT 持久化 |
| Dashboard | 🟢 已完成 | T-20002：StatCards + TrendChart + useDashboardStats |
| 房间管理 | 🟢 已完成 | T-20003（列表）+ T-20004（详情 Modal）+ T-20005（强制关闭） |
| 用户管理 | 🟢 已完成 | T-20006（列表）+ T-20007（详情 Drawer）+ T-20008（封禁/解封 Modal） |
| 审计日志 | 🟢 已完成 | T-20009：LogsTable + LogSearchForm + useLogsPage |
| 真实 telemetry / crash provider | 🔴 未开始 | 当前仅 Mock 实现 |

## 四、 文档维护提示

- Admin Web 不涉及 RTC / IM / 实时音视频，此类能力无需在本文档追踪。
- 当新增页面或功能模块时，应同步更新 [structure.md](./structure.md) 和本文件。
