<!--
[AI 读写指令与维护规约]
1. 本文件记录 Web 端 Auth 模块的架构现状，由 DoD Agent 生成，内容来源于实际代码。
2. 当 Auth 相关文件发生变更（新增路由守卫、接入 Zustand useAuthStore 等），必须同步更新本文件。
3. 禁止在本文件内粘贴完整业务代码；仅记录结构、接口与关键设计决策。
-->

# Web Auth 模块架构文档

**Last Updated:** 2025-05-10  
**覆盖 Task：** T-20001（管理员登录页 UI）、T-20002（登录逻辑与路由守卫）  
**状态：** 🟢 登录页 UI 已完成（T-20001）| 🟢 登录逻辑与路由守卫已完成（T-20002）

---

## 一、模块概述

Auth 模块负责管理员身份认证的前端 UI、状态管理与路由鉴权。当前已完成：

- 登录页静态 UI（`LoginPage` + `LoginForm`）← T-20001
- Admin Server HTTP 客户端（`apiClient.ts`）← T-20001
- i18n 双语配置（en / zh）← T-20001
- Zustand `useAuthStore`（token 管理、JWT 校验、login/logout/checkAuth）← T-20002
- `AuthGuard` 路由守卫（每次渲染调用 `checkAuth()`，过期 token 实时拦截）← T-20002
- 路由配置（react-router-dom：`/ → /dashboard`、`/login`、受保护 `/dashboard`）← T-20002
- 401 拦截器（有 session token 时自动 `logout()` + `window.location.href='/login'`）← T-20002

---

## 二、组件与模块结构

```
src/
├── stores/
│   └── useAuthStore.ts        ← Zustand store：token/admin/isAuthenticated/login/logout/checkAuth
├── components/
│   └── AuthGuard.tsx          ← 路由守卫：每次渲染调用 checkAuth()，未认证跳转 /login
├── router/
│   └── index.tsx              ← AppRoutes：/ /login /dashboard 路由配置
├── pages/
│   ├── login/
│   │   ├── index.tsx          ← LoginPage（接入 useAuthStore.login + useNavigate）
│   │   ├── LoginForm.tsx      ← LoginForm（Ant Design Form 表单逻辑）
│   │   ├── LoginForm.test.tsx ← 测试套件
│   │   └── login.module.css   ← CSS Modules 样式
│   └── dashboard/
│       └── index.tsx          ← DashboardPage（占位页，T-20003 待实现）
└── core/network/
    └── apiClient.ts           ← HTTP 客户端（含 JWT 自动附加 + 401 拦截器）
```

### 组件职责

| 组件/模块 | 文件 | 职责 |
|-----------|------|------|
| `LoginPage` | `pages/login/index.tsx` | 全屏居中卡片布局；调用 `useAuthStore.login`；登录成功后 `Navigate` 到 `/dashboard` |
| `LoginForm` | `pages/login/LoginForm.tsx` | Ant Design Form；用户名/密码/记住账号/提交；错误 `Alert`；loading 状态；i18n |
| `useAuthStore` | `stores/useAuthStore.ts` | Zustand store：管理 token/admin/isAuthenticated；login/logout/checkAuth |
| `AuthGuard` | `components/AuthGuard.tsx` | 路由守卫：每次渲染调用 `checkAuth()`；未认证返回 `<Navigate to="/login" replace />` |
| `AppRoutes` | `router/index.tsx` | react-router-dom Routes 配置；/ → /dashboard 重定向；AuthGuard 包裹受保护路由 |
| `adminFetch` | `core/network/apiClient.ts` | HTTP 客户端核心；自动附加 Bearer token；401 + session token 时触发 logout + 跳转 |

### 完整认证数据流

```
App 启动
  └─► useAuthStore 初始化
        ├─► localStorage.getItem('adminToken')
        └─► isTokenValid(token) → isAuthenticated 初始值

用户登录
  └─► LoginForm.handleFinish(values)
        ├─► setLoading(true)
        ├─► props.onSubmit(values)
        │     └─► useAuthStore.login(username, password)
        │           ├─► apiClient.adminLogin({ username, password })
        │           │     └─► POST /api/v1/admin/login → Admin Server
        │           ├─► 成功：localStorage.setItem('adminToken', token)
        │           │         set({ token, admin, isAuthenticated: true })
        │           └─► 失败：抛出 Error(message)
        ├─► 成功：localStorage.setItem / removeItem (REMEMBER_KEY) + Navigate('/dashboard')
        └─► 失败：setError(err.message) → <Alert title={error} />

路由切换（受保护页面）
  └─► AuthGuard 渲染
        ├─► checkAuth() → isTokenValid(token)
        │     ├─► true：<Outlet />（渲染子路由）
        │     └─► false（含 token 过期）：logout() → <Navigate to="/login" replace />

API 请求（已认证）
  └─► adminFetch(path, init)
        ├─► 自动附加 Authorization: Bearer <token>
        └─► 响应 401 + token 存在
              └─► useAuthStore.getState().logout()
                    └─► window.location.href = '/login'
```

---

## 三、useAuthStore（Zustand Store）

**文件**：`src/stores/useAuthStore.ts`

### 状态接口

```typescript
interface AuthStore {
  token: string | null;                      // JWT（从 localStorage 初始化）
  admin: AdminLoginData['admin'] | null;     // 登录成功后的管理员信息
  isAuthenticated: boolean;                  // token 非 null 且 exp 未过期（模块加载时计算）
  login: (username: string, password: string) => Promise<void>;
  logout: () => void;
  checkAuth: () => boolean;                  // 每次调用重新验证 exp（过期时自动 logout）
}
```

### 关键设计决策

| 决策 | 实现方式 | 原因 |
|------|----------|------|
| Token 持久化 | `localStorage['adminToken']` | 页面刷新后保持登录态 |
| 初始化 | 模块加载时 `localStorage.getItem` + `isTokenValid` | CSR 项目，无 SSR 风险 |
| `checkAuth()` vs `isAuthenticated` | 路由守卫使用 `checkAuth()` | `isAuthenticated` 仅初始化时计算一次；`checkAuth()` 每次调用重新检测 `exp`，实时感知运行时过期 |
| JWT 解码 | 仅解 payload（不验签） | 前端只需读取 `exp`，签名验证由后端负责 |
| Base64 padding | `padEnd(len + (4 - len%4)%4, '=')` | Safari `atob()` 严格要求标准 Base64 padding（M01 修复） |

### XSS 风险与缓解（L02 修复）

> ⚠️ JWT 存储在 `localStorage`，可被同域 XSS 脚本读取。

| 缓解措施 | 说明 |
|----------|------|
| 严格 CSP | 禁止 `'unsafe-inline'`，阻断内联脚本注入 |
| 输入转义 | 所有用户输入经 HTML 转义，消除注入点 |
| SRI 校验 | 生产环境启用 Subresource Integrity，防止第三方脚本篡改 |
| HttpOnly Cookie（备选） | 需后端配合，可彻底消除 JS 读取风险 |

---

## 四、AuthGuard（路由守卫）

**文件**：`src/components/AuthGuard.tsx`

```tsx
export function AuthGuard() {
  const checkAuth = useAuthStore((s) => s.checkAuth);
  if (!checkAuth()) {
    return <Navigate to="/login" replace />;
  }
  return <Outlet />;
}
```

### 设计要点

- **每次渲染**均调用 `checkAuth()`，而非读取 `isAuthenticated` 快照（H02 修复）。
- `checkAuth()` 内部调用 `isTokenValid(token)` 检测 JWT `exp` 字段，过期时自动 `logout()`（清除 store + localStorage）再返回 `false`。
- 使用 `replace` 防止登录页被加入历史栈，避免"返回"按钮回到受保护页面。

---

## 五、路由配置（react-router-dom）

**文件**：`src/router/index.tsx`

| 路径 | 元素 | 保护 | 说明 |
|------|------|------|------|
| `/` | `<Navigate to="/dashboard" replace />` | — | 根路由重定向；AuthGuard 会二次鉴权 |
| `/login` | `<LoginPage />` | 公开 | 已登录时 LoginPage 内部 Navigate 跳转 `/dashboard` |
| `/dashboard` | `<DashboardPage />` | `AuthGuard` | T-20003 待实现完整内容 |

`AppRoutes` 在 `App.tsx` 通过 `<BrowserRouter>` 包裹后挂载。

---

## 六、localStorage 记住账号机制

| Key | 常量 | 存储时机 | 清除时机 |
|-----|------|----------|----------|
| `adminLoginUsername` | `REMEMBER_KEY`（`LoginForm.tsx`） | 登录成功 且 `remember === true` | 登录成功 且 `remember === false` |

- **读取时机**：组件挂载时通过 `useState` 懒初始化读取一次（`() => localStorage.getItem(REMEMBER_KEY)`），避免每次重渲染重复读取。
- **注意**：仅存储 **用户名**，不存储密码（zh 文案为"记住账号"）。

---

## 七、i18n 双语实现

```
src/i18n/
├── index.ts           ← i18next + react-i18next 初始化（默认语言：en，fallback：en）
└── locales/
    ├── en.ts          ← 英文翻译资源
    └── zh.ts          ← 中文翻译资源
```

### 初始化配置（`src/i18n/index.ts`）

- 框架：`i18next` + `react-i18next`
- 默认语言：`en`；fallback：`en`
- 在 `main.tsx` 中 `import './i18n'` 一次即可全局生效
- 组件中通过 `const { t } = useTranslation()` 使用

### 当前翻译键（`login.*`）

| Key | EN | ZH |
|-----|----|----|
| `login.title` | Admin Login | 管理员登录 |
| `login.subtitle` | Voice Room Management | Voice Room 管理后台 |
| `login.username` | Username | 用户名 |
| `login.usernamePlaceholder` | Enter username | 请输入用户名 |
| `login.password` | Password | 密码 |
| `login.passwordPlaceholder` | Enter password | 请输入密码 |
| `login.rememberMe` | Remember me | 记住账号 |
| `login.submit` | Login | 登录 |
| `login.validation.usernameRequired` | Please enter your username | 请输入用户名 |
| `login.validation.passwordRequired` | Please enter your password | 请输入密码 |
| `login.error.unknown` | An unknown error occurred. Please try again. | 发生未知错误，请重试。 |

---

## 八、API 客户端（`src/core/network/apiClient.ts`）

### 配置

| 项 | 值 |
|----|----|
| Base URL | `VITE_ADMIN_API_BASE_URL`（默认 `http://localhost:3001/api/v1/admin`） |
| Token 存储 Key | `adminToken`（localStorage） |
| 请求超时 | 15 秒（`AbortController`） |
| Content-Type | `application/json` |
| Auth Header | `Authorization: Bearer <token>`（存在 token 时自动附加） |

### 已实现接口

| 函数 | 路径 | 方法 | 说明 |
|------|------|------|------|
| `adminLogin(req)` | `POST /login` | POST | 账号密码登录，返回 `AdminLoginData`（token、expires_in、admin 信息） |

### 错误处理策略

1. **HTTP 非 2xx**：先尝试 `response.json()` 提取 `message` 字段，失败则回退为 `HTTP Error <status>`，统一抛出 `Error`。
2. **业务错误**（`code !== 0`）：直接抛出 `new Error(body.message)`。
3. **超时**：`AbortController` 15 秒后触发 `abort()`；`clearTimeout` 在 `finally` 块执行，无泄漏。
4. **401 + session token 存在**（H01 修复）：区分两种场景：
   - **有 token（token 过期）**：`useAuthStore.getState().logout()` → `window.location.href = '/login'`（全量重置，防止残留状态）
   - **无 token（登录请求本身返回 401）**：走普通错误路径，向 UI 透传 `body.message`（如"用户名或密码错误"）

### 关键类型

```ts
interface AdminLoginRequest { username: string; password: string; }
interface AdminLoginData {
  token: string; expires_in: number;
  admin: { id: string; username: string; role: string; display_name: string; last_login_at: string; };
}
interface ApiResponse<T> { code: number; message: string; data: T; request_id?: string; }
```

---

## 九、antd v6 组件使用注意

> ⚠️ **Alert prop 历史**（T-20001 三轮 Review 记录，避免重蹈覆辙）

| Prop | antd v6 状态 | 说明 |
|------|-------------|------|
| `title` | ✅ **正确用法** | `/** Content of Alert */`，渲染为可见文本内容 |
| `message` | 🚫 `@deprecated` | `please use 'title' instead`；向后兼容但会输出控制台警告 |

**结论**：antd v6 的 `Alert` 组件使用 `title={error}` 而非 `message={error}`。

```tsx
// ✅ antd v6 正确写法
<Alert data-testid="alert-error" type="error" title={error} showIcon />
```

---

## 十、测试覆盖

| 文件 | 测试文件 | 用例数 | 覆盖率 |
|------|----------|--------|--------|
| `LoginForm.tsx` | `LoginForm.test.tsx` | 40 个 | 全 100% |
| `useAuthStore.ts` | `useAuthStore.test.ts` | 16 个 | 全 100% |
| `AuthGuard.tsx` | `AuthGuard.test.tsx` | 4 个（含「isAuthenticated=true 但 checkAuth()=false」运行时过期场景） | 全 100% |
| `pages/login/index.tsx` | `LoginPage.integration.test.tsx` | 4 个集成测试 | 全 100% |
| `app/App.tsx` | `App.test.tsx` | 4 个路由测试 | — |

**全局**：8 个测试文件，**74 / 74** 全部通过，lint 零警告。

测试工具：`Vitest` + `@testing-library/react` + `@testing-library/user-event` + `@testing-library/jest-dom`

---

## 十一、已完成事项（T-20002 ✅）

- [x] `useAuthStore`（Zustand）实现 `login / logout / token / checkAuth` 状态管理
- [x] `LoginPage` 接入 `useAuthStore.login` + `useNavigate` 跳转 `/dashboard`
- [x] JWT 写入 `localStorage['adminToken']`，登录成功跳转 `/dashboard`
- [x] `AuthGuard` 路由守卫：每次渲染调用 `checkAuth()`，过期 token 实时退出
- [x] `apiClient.ts` 401 拦截器：有 session token 时 `logout()` + 跳转 `/login`
- [x] Base64 padding 修复（Safari `atob()` 兼容）
- [x] XSS 风险注释与缓解措施文档化
- [x] `ADMIN_TOKEN_KEY` 常量统一到 `useAuthStore.ts` 导出，消除重复定义
- [ ] Logo 占位替换为真实品牌资源（T-20002 范围外，待后续处理）

---

## 十二、相关文档

- [Web 架构总索引](./index.md)
- [目录结构与入口链路](./structure.md)
- [协议定义 §3.1 POST /admin/login](../../protocol.md)
- [TDS T-20001](../../tds/web/T-20001.md)
- [TDS T-20002](../../tds/web/T-20002.md)
