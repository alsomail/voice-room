# Web 目录结构与入口链路

## 一、 关键文件与职责

### 入口与路由

| 路径 | 职责 | 当前状态 |
| --- | --- | --- |
| `app/web/src/main.tsx` | 挂载 React 根组件（`BrowserRouter` 包裹） | 🟢 已落地 |
| `app/web/src/app/App.tsx` | 应用根组件，渲染 `<AppRouter />` | 🟢 已落地 |
| `app/web/src/router/index.tsx` | `BrowserRouter` 路由表（`/login`、`/`、`/dashboard`、`/rooms`、`/users`、`/logs`） | 🟢 已落地 |
| `app/web/src/components/AuthGuard.tsx` | 路由守卫，未登录重定向到 `/login` | 🟢 已落地 |

### 状态管理

| 路径 | 职责 | 当前状态 |
| --- | --- | --- |
| `app/web/src/stores/useAuthStore.ts` | Zustand store：JWT 持久化、登录/登出、token getter | 🟢 已落地 |

### 页面模块

| 路径 | 职责 | Task | 当前状态 |
| --- | --- | --- | --- |
| `app/web/src/pages/login/` | Admin 登录页：`LoginForm.tsx` + `index.tsx` | T-20001 | 🟢 已落地 |
| `app/web/src/pages/dashboard/` | Dashboard：`StatCards.tsx` + `TrendChart.tsx` + `useDashboardStats.ts` | T-20002 | 🟢 已落地 |
| `app/web/src/pages/rooms/` | 房间管理：`RoomsTable.tsx` + `RoomDetailModal.tsx` + `RoomStatusTag.tsx` + `useRoomsPage.ts` + `useRoomDetail.ts` | T-20003~T-20005 | 🟢 已落地 |
| `app/web/src/pages/users/` | 用户管理：`UsersTable.tsx` + `UserDetailDrawer.tsx` + `BanModal.tsx` + `UserSearchForm.tsx` + `UserStatusTag.tsx` + hooks | T-20006~T-20008 | 🟢 已落地 |
| `app/web/src/pages/logs/` | 审计日志：`LogsTable.tsx` + `LogSearchForm.tsx` + `useLogsPage.ts` | T-20009 | 🟢 已落地 |

### 基础设施

| 路径 | 职责 | 当前状态 |
| --- | --- | --- |
| `app/web/src/core/config/env.ts` | 读取 `VITE_API_BASE_URL`、`VITE_WS_URL`、`VITE_ANALYTICS_ENDPOINT` | 🟢 已落地 |
| `app/web/src/core/network/apiClient.ts` | 基于 `fetch` 的 HTTP 客户端（自动注入 JWT） | 🟢 已落地 |
| `app/web/src/core/ws/index.ts` | 基于 `joinConfiguredUrl` 的最小 WS helper | 🟢 已落地 |
| `app/web/src/core/telemetry/` | Mock 埋点与崩溃上报 | 🟡 仅 Mock |
| `app/web/src/lib/url.ts` | URL 安全拼接（防路径逃逸） | 🟢 已落地 |
| `app/web/src/i18n/` | i18n 国际化（en / zh） | 🟢 已落地 |
| `app/web/src/services/index.ts` | 第三方服务适配层入口 | 🔴 暂为空 |

## 二、 环境变量约束

当前 Web 端明确要求通过 `VITE_` 前缀注入配置：

| 变量 | 用途 |
| --- | --- |
| `VITE_API_BASE_URL` | HTTP API 基础地址 |
| `VITE_WS_URL` | WebSocket 基础地址 |
| `VITE_ANALYTICS_ENDPOINT` | telemetry mock 的环境上下文字段 |

`.env.example`、`.env.development`、`.env.production` 已存在，便于后续按环境切换。

## 三、 当前路由结构

```
BrowserRouter
├── /login          → LoginPage（公开）
├── / (AuthGuard)   → 重定向到 /dashboard
│   ├── /dashboard  → DashboardPage
│   ├── /rooms      → RoomsPage
│   ├── /users      → UsersPage
│   └── /logs       → LogsPage
└── *               → 404 / 重定向 /login
```

- `AuthGuard` 从 `useAuthStore` 检查 JWT 有效性，未登录重定向到 `/login`
- 登录成功后跳转 `/dashboard`

## 四、 当前测试覆盖

- **测试文件**：29 个（`.test.ts` / `.test.tsx` / `.integration.test.tsx`）
- 覆盖范围：页面组件渲染、Hook 逻辑、API 客户端、URL 安全拼接、AuthGuard 守卫、Store 状态管理
