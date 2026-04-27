# Web 端多 Profile 环境配置体系

**最后更新**: 2026-05-31  
**关联 Task**: [T-20020](../../tds/web/T-20020.md) — Web 多 profile env + VITE_ADMIN_API_BASE_URL 收口  
**状态**: ✅ Done (Review Round 1 通过)

---

## 一、配置架构概览

Web 端采用 **Vite 官方 mode 加载链** + **启动期 fail-fast 校验** 的双层设计，确保环境变量缺失时应用立即阻塞渲染（而非在 API 调用时才暴露 `undefined`）。

### 1.1 核心流向

```
vite --mode {dev|test|staging|production}
         ↓
Vite 按 mode 加载 .env.{mode} 文件
         ↓
仅 VITE_ 前缀字段被 inline 到 bundle（构建期嵌入）
         ↓
app/web/src/core/config/env.ts
  ├─ readWebEnv() 模块顶层立即执行
  ├─ requireEnv(name) 校验每个字段非空
  └─ 缺值 → throw [CONFIG ERROR] VITE_XXX must be set
         ↓
浏览器控制台首屏红色错误，React 不渲染（fail-fast）
         ↓
apiClient.ts 正常运行（env 已就绪）
```

### 1.2 关键不变量

1. **VITE_ 前缀字段在构建期 inline**（产物里是字符串字面量），运行时无法改值；多 profile 切换**必须通过构建/启动时的 `--mode` 参数**。
2. **启动期校验必须发生在 `webEnv` 模块顶层**，由 `main.tsx` → `App` 链路的 import 自动触发，不能延迟到 API 调用。
3. **错误信息统一前缀** `[CONFIG ERROR]`（含尾空格），便于跨端 grep 收敛诊断（与后端 T-00040/T-10020 的 `CONFIG ERROR:` 关键词一致）。

---

## 二、字段冻结表（Web env 4 字段）

| 字段 | 必填 | 默认值 (dev) | test/staging 占位 | production 占位 | apiClient 消费方 |
|------|------|---------|--------------------|------------------|-----------|
| `VITE_API_BASE_URL` | ✅ | `http://127.0.0.1:3000/api` | `https://stg-app.example.com/api` | `https://api.example.com/api` | `src/api/client.ts` |
| `VITE_WS_URL` | ✅ | `ws://127.0.0.1:3000/ws` | `wss://stg-app.example.com/ws` | `wss://api.example.com/ws` | (房间 WS，后续消费) |
| `VITE_ADMIN_API_BASE_URL` | ✅ 🆕 | `http://127.0.0.1:3001/api/v1/admin` | `https://stg-admin-api.example.com/api/v1/admin` | `https://admin-api.example.com/api/v1/admin` | `src/core/network/apiClient.ts` |
| `VITE_ANALYTICS_ENDPOINT` | ✅ | `https://analytics-dev.example.com/collect` | `https://stg-analytics.example.com/collect` | `https://analytics.example.com/collect` | (埋点上报，后续消费) |

**关键说明**：
- `VITE_ADMIN_API_BASE_URL` 是 Web bundle 内嵌的完整 URL（含 `/api/v1/admin` 路径段）。
- 与 `tests/scripts/env/.env.{mode}.example` 中的 `ADMIN_SERVER_BASE_URL` 不同（后者是 E2E 直连根 URL，不含路径）。
- 两者不能合并，各自语义独立。

---

## 三、.env 文件配置

### 3.1 五档配置文件

#### `.env.example`（开发者参考）
```env
VITE_API_BASE_URL=
VITE_WS_URL=
VITE_ADMIN_API_BASE_URL=
VITE_ANALYTICS_ENDPOINT=
```
- 所有字段留空
- 用于 git clone 后的 `cp .env.example .env.local` 参考模板

#### `.env.development`（开箱即用，入库）
```env
VITE_API_BASE_URL=http://127.0.0.1:3000/api
VITE_WS_URL=ws://127.0.0.1:3000/ws
VITE_ADMIN_API_BASE_URL=http://127.0.0.1:3001/api/v1/admin
VITE_ANALYTICS_ENDPOINT=https://analytics-dev.example.com/collect
```
- `npm run dev` 使用（默认 `--mode development`）
- 指向本机 localhost
- 入库无敏感信息

#### `.env.test`（E2E 测试用，入库）
```env
VITE_API_BASE_URL=https://stg-app.example.com/api
VITE_WS_URL=wss://stg-app.example.com/ws
VITE_ADMIN_API_BASE_URL=https://stg-admin-api.example.com/api/v1/admin
VITE_ANALYTICS_ENDPOINT=https://stg-analytics.example.com/collect
```
- `vite --mode test` 使用（E2E 默认 mode）
- 指向 staging 环境占位
- 与 `.env.staging` 当前同源，后续可分离

#### `.env.staging`（staging 构建，入库）
```env
VITE_API_BASE_URL=https://stg-app.example.com/api
VITE_WS_URL=wss://stg-app.example.com/ws
VITE_ADMIN_API_BASE_URL=https://stg-admin-api.example.com/api/v1/admin
VITE_ANALYTICS_ENDPOINT=https://stg-analytics.example.com/collect
```
- `vite build --mode staging` 使用
- 指向 staging 环境占位
- 与 `.env.test` 默认同源

#### `.env.production`（生产构建，入库）
```env
VITE_API_BASE_URL=https://api.example.com/api
VITE_WS_URL=wss://api.example.com/ws
VITE_ADMIN_API_BASE_URL=https://admin-api.example.com/api/v1/admin
VITE_ANALYTICS_ENDPOINT=https://analytics.example.com/collect
```
- `npm run build` 使用（默认 `--mode production`）
- 指向生产环境占位

### 3.2 Vite mode 加载链（实测验证）

| 命令 | mode 值 | 加载文件顺序 |
|------|---------|------------|
| `npm run dev` | `development` | `.env` → `.env.local` → `.env.development` → `.env.development.local` |
| `vite --mode test` | `test` | `.env` → `.env.local` → `.env.test` → `.env.test.local` |
| `vite --mode staging` | `staging` | `.env` → `.env.local` → `.env.staging` → `.env.staging.local` |
| `npm run build` | `production` | `.env` → `.env.local` → `.env.production` → `.env.production.local` |
| `vite build --mode staging` | `staging` | `.env` → `.env.local` → `.env.staging` → `.env.staging.local` |

**T-20020 实测验证**：
- ✅ `vite build --mode staging` 产物含 staging 占位 URL
- ✅ `vite build --mode test` 产物含 test 占位 URL
- ✅ `npm run build` (production) bundle **0 条 dev URL 泄露**（防信息泄露）
- ✅ `npm run dev` webEnv 正确读取 `.env.development` 值

---

## 四、启动期校验契约

### 4.1 核心实现（env.ts）

```typescript
// app/web/src/core/config/env.ts
function requireEnv(name: string): string {
  const v = (import.meta.env as Record<string, string | undefined>)[name];
  if (!v || v.trim() === '') {
    throw new Error(`[CONFIG ERROR] ${name} must be set`);
  }
  return v;
}

export const webEnv = Object.freeze({
  apiBaseUrl:        requireEnv('VITE_API_BASE_URL'),
  wsUrl:             requireEnv('VITE_WS_URL'),
  adminApiBaseUrl:   requireEnv('VITE_ADMIN_API_BASE_URL'),  // 🆕
  analyticsEndpoint: requireEnv('VITE_ANALYTICS_ENDPOINT'),
});
```

### 4.2 触发时机

| 场景 | 行为 | 表现 |
|------|------|------|
| 浏览器加载首个 JS chunk | `main.tsx` import `./core/config/env` → `readWebEnv()` 立即执行 → 缺值 throw | 首屏红色未捕获错误，React 不渲染（白屏 + 控制台红色） |
| vitest 单元测试 | `src/test/setup.ts` 通过 `vi.stubEnv` 注入四字段默认值 | 避免 `webEnv` 模块顶层 throw 导致所有用例集体红 |
| `vite build` 构建期 | VITE_ 字段空值不会让构建失败，但 bundle 中会内嵌 `undefined` | 运行时校验兜底 |

### 4.3 错误格式示例

```
[CONFIG ERROR] VITE_ADMIN_API_BASE_URL must be set
[CONFIG ERROR] VITE_API_BASE_URL must be set
[CONFIG ERROR] VITE_WS_URL must be set
[CONFIG ERROR] VITE_ANALYTICS_ENDPOINT must be set
```

**特点**：
- 前缀 `[CONFIG ERROR] ` 固定（含尾空格）
- 与后端 T-00040/T-10020 的 `CONFIG ERROR:` 关键词一致（cross-tier `grep -r "CONFIG ERROR"` 全收敛）

---

## 五、apiClient 改造

### 5.1 getAdminApiBaseUrl() 变化

**之前**（T-20020 前）:
```typescript
function getAdminApiBaseUrl(): string {
  return webEnv.adminApiBaseUrl ?? 'http://localhost:3001/api/v1/admin';
}
```

**现在**（T-20020 后）:
```typescript
function getAdminApiBaseUrl(): string {
  return webEnv.adminApiBaseUrl;  // 删除 ?? 默认值
}
```

### 5.2 删除默认值的意义

1. **强制环境配置**：dev/test/staging/prod 任何 mode 都必须显式配置，无兜底。
2. **fail-fast 保证**：缺值在启动期（not API 调用时）立即抛错。
3. **防信息污染**：production bundle 不会暴露 dev localhost 地址。

---

## 六、开发流程

### 6.1 首次启动（开发者）

```bash
# 1. 克隆仓库
git clone <repo>
cd app/web

# 2. 复制并配置
cp .env.example .env.local
# 编辑 .env.local，填入四个字段（或使用 .env.development 的值）

# 3. 启动开发服务
npm install
npm run dev
# 浏览器自动打开 http://localhost:5173，Admin 登录页正常加载
```

### 6.2 多环境构建

```bash
# development（默认，npm run dev 自动用）
npm run dev

# test mode（用于 E2E）
npx vite --mode test --port 4173

# staging 构建
npm run build -- --mode staging
# 产物 dist/ 包含 stg-*.example.com 占位 URL

# production 构建（默认）
npm run build
# 产物 dist/ 包含 admin-api.example.com 占位 URL
```

### 6.3 测试环境配置

**vitest 默认 stub 策略**（src/test/setup.ts）:
```typescript
import { vi } from 'vitest';
vi.stubEnv('VITE_API_BASE_URL', 'http://127.0.0.1:3000/api');
vi.stubEnv('VITE_WS_URL', 'ws://127.0.0.1:3000/ws');
vi.stubEnv('VITE_ADMIN_API_BASE_URL', 'http://127.0.0.1:3001/api/v1/admin');
vi.stubEnv('VITE_ANALYTICS_ENDPOINT', 'https://analytics-test.example.com/collect');
```

避免 vitest 跑时 `webEnv` 模块顶层 throw 导致全用例集体红。

### 6.4 单测中测 throw 路径

```typescript
import { vi, beforeEach, afterEach } from 'vitest';

describe('env.ts throw 分支', () => {
  beforeEach(() => { vi.unstubAllEnvs(); });
  afterEach(() => { vi.unstubAllEnvs(); });

  it('缺 VITE_ADMIN_API_BASE_URL 时 throw', () => {
    vi.stubEnv('VITE_API_BASE_URL', 'http://127.0.0.1:3000/api');
    vi.stubEnv('VITE_WS_URL', 'ws://127.0.0.1:3000/ws');
    vi.stubEnv('VITE_ANALYTICS_ENDPOINT', 'https://analytics-test.example.com/collect');
    // VITE_ADMIN_API_BASE_URL 不 stub，缺失

    expect(() => { vi.resetModules(); require('./env'); })
      .toThrow(/\[CONFIG ERROR\] VITE_ADMIN_API_BASE_URL must be set/);
  });
});
```

---

## 七、类型定义

### 7.1 vite-env.d.ts

```typescript
/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_API_BASE_URL: string;
  readonly VITE_WS_URL: string;
  readonly VITE_ADMIN_API_BASE_URL: string;  // 🆕
  readonly VITE_ANALYTICS_ENDPOINT: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
```

### 7.2 env.ts 导出类型

```typescript
export const webEnv: Readonly<{
  apiBaseUrl: string;
  wsUrl: string;
  adminApiBaseUrl: string;
  analyticsEndpoint: string;
}>;
```

使用 `as const` 或 `Object.freeze()` 保证只读性。

---

## 八、与其他 Epic 的关系

### 8.1 E2E 测试基建（模块 9）

- **T-0000E**: Web env 字段冻结四字段（本文档 §二）
- **T-0000F**: 根 `.env.example` + 三档 `tests/scripts/env/.env.*.example`（E2E 直连用）
- **T-20020**: Web 多 profile env + `VITE_ADMIN_API_BASE_URL` 收口（当前 Task）

**关键差异**：
- Web `.env.{mode}` 中的 `VITE_ADMIN_API_BASE_URL` = 完整 URL + 路径（`/api/v1/admin`）
- E2E `tests/scripts/env/.env.{mode}.example` 中的 `ADMIN_SERVER_BASE_URL` = 根 URL（不含路径）

### 8.2 后端对称任务

- **T-00040**: AppServer 多 profile 配置体系（`app/server/config/*.toml`）
- **T-10020**: AdminServer 多 profile 配置体系（`app/adminServer/config/*.toml`）

三端（Web/AppServer/AdminServer）均采用 **fail-fast + [CONFIG ERROR] 前缀** 的统一错误格式。

---

## 九、验收清单（T-20020 DoD）

### 9.1 文件清单

| 文件 | 变更 | 状态 |
|------|------|------|
| `app/web/.env.example` | 增 `VITE_ADMIN_API_BASE_URL=` | ✅ |
| `app/web/.env.development` | 增 `VITE_ADMIN_API_BASE_URL=...` | ✅ |
| `app/web/.env.test` | 🆕 新增 | ✅ |
| `app/web/.env.staging` | 🆕 新增 | ✅ |
| `app/web/.env.production` | 增 `VITE_ADMIN_API_BASE_URL=...` | ✅ |
| `app/web/src/vite-env.d.ts` | 增 `VITE_ADMIN_API_BASE_URL` 类型 | ✅ |
| `app/web/src/core/config/env.ts` | 重写 `requireEnv` + `readWebEnv()` | ✅ |
| `app/web/src/core/network/apiClient.ts` | 删除默认值 + import webEnv | ✅ |
| `app/web/src/test/setup.ts` | 增 `vi.stubEnv` 注入 | ✅ |
| `app/web/src/core/config/env.test.ts` | 🆕 新增（6 tests） | ✅ |
| `app/web/src/main.tsx` | 增 `import './core/config/env'` | ✅ |

### 9.2 测试验收

- ✅ `npm test` — **517 / 517 passed**（41 files，包含新增 14 tests）
- ✅ `vite build --mode test` — 产物含 staging 占位 URL
- ✅ `vite build --mode staging` — 产物含 staging 占位 URL
- ✅ `npm run build` — production bundle **0 条 dev URL 泄露**
- ✅ `npm run dev` — webEnv 正确读取 `.env.development`，应用正常启动

### 9.3 fail-fast 验证

**干净 clone 后缺值测试**:
```bash
cd app/web
cp .env.example .env.local  # 不填任何值
npm run dev
# 预期：浏览器控制台首屏红色错误 [CONFIG ERROR] VITE_API_BASE_URL must be set
# 实际行为：React 不渲染（白屏）
```

**填充所有值后**:
```bash
# 编辑 .env.local，填入 4 字段
# 例：从 .env.development 复制
npm run dev
# 预期：Admin 登录页正常加载，无 [CONFIG ERROR] 错误
```

---

## 十、故障排查

### 常见问题 Q&A

| 问题 | 症状 | 解决方案 |
|------|------|----------|
| 启动时白屏 + 控制台 `[CONFIG ERROR] VITE_XXX must be set` | 某字段缺失 | 检查 `.env.{mode}` 或 `.env.local` 中该字段是否存在且非空 |
| `npm run dev` 后 API 请求返回 404 | 配置 URL 错误 | 确认 `VITE_ADMIN_API_BASE_URL` 指向正确的 admin server 地址（dev 环境 `http://127.0.0.1:3001`) |
| `vite build --mode staging` 生产文件中含 dev URL | 配置被错误覆盖 | 确认 `.env.staging` 不被 `.env.local` 或 `.env.staging.local` 覆盖；删除这些本地文件再重建 |
| vitest 跑不了（所有用例红） | `webEnv` 模块顶层 throw | 确认 `src/test/setup.ts` 顶部有 `vi.stubEnv` 注入（应自动工作） |

---

## 十一、相关文件引用

- **TDS**: [doc/tds/web/T-20020.md](../../tds/web/T-20020.md)
- **Web 架构索引**: [doc/arch/web/index.md](./index.md)
- **全局架构**: [doc/architecture/index.md](../../architecture/index.md)
- **产品进度**: [doc/product/index.md](../../product/index.md) (Phase 1.6)
- **任务看板**: [doc/tasks/index.md](../../tasks/index.md)

---

**记录人**: DoD Agent  
**完成日期**: 2026-05-31  
**Review 状态**: ✅ Round 1 通过 (commit ac908c9)
