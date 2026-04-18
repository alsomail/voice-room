# Web 目录结构与入口链路

## 一、 关键文件与职责

| 路径 | 职责 | 当前状态 |
| --- | --- | --- |
| `app/web/src/main.tsx` | 挂载 React 根组件 | 🟢 已落地 |
| `app/web/src/app/App.tsx` | 应用根组件，目前直接返回 `HomePage` | 🟢 已落地 |
| `app/web/src/pages/HomePage.tsx` | 页面壳层，目前直接返回 `TelemetryFeature` | 🟢 已落地 |
| `app/web/src/features/TelemetryFeature.tsx` | 首页占位内容，提示脚手架已准备就绪 | 🟡 仍为占位 |
| `app/web/src/core/config/env.ts` | 读取 `VITE_API_BASE_URL`、`VITE_WS_URL`、`VITE_ANALYTICS_ENDPOINT` | 🟢 已落地 |
| `app/web/src/api/client.ts` | 基于 `joinConfiguredUrl` 的最小 HTTP helper | 🟢 已落地 |
| `app/web/src/core/ws/index.ts` | 基于 `joinConfiguredUrl` 的最小 WS helper | 🟢 已落地 |
| `app/web/src/core/network/index.ts` | 重新导出 `apiClient` | 🟡 仍很薄 |
| `app/web/src/services/index.ts` | 第三方服务适配层入口 | 🔴 暂为空 |

## 二、 环境变量约束

当前 Web 端明确要求通过 `VITE_` 前缀注入配置：

| 变量 | 用途 |
| --- | --- |
| `VITE_API_BASE_URL` | HTTP API 基础地址 |
| `VITE_WS_URL` | WebSocket 基础地址 |
| `VITE_ANALYTICS_ENDPOINT` | telemetry mock 的环境上下文字段 |

`.env.example`、`.env.development`、`.env.production` 已存在，便于后续按环境切换。

## 三、 当前入口链路

1. `main.tsx` 渲染 `<App />`
2. `App` 渲染 `HomePage`
3. `HomePage` 渲染 `TelemetryFeature`
4. `TelemetryFeature` 只展示静态文本，尚未发起真实业务请求

## 四、 当前结论

Web 端已经具备“工程可以继续长出业务”的骨架，但还没有真正的多页面、多模块协作关系。
