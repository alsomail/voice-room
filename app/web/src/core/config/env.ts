/**
 * T-20020 Web env 启动期校验入口（fail-fast）
 *
 * - 模块顶层立即执行 `readWebEnv()`，缺值/空白即抛
 *   `Error('[CONFIG ERROR] VITE_XXX must be set')`，阻塞 main.tsx 渲染。
 * - 字段冻结清单见 doc/tds/web/T-20020.md §2.3：4 字段。
 * - VITE_ 前缀字段构建期 inline，运行时无法热改；切换 profile 必须用
 *   `vite --mode {dev|test|staging|production}`。
 */

function requireEnv(name: keyof ImportMetaEnv): string {
  const raw = (import.meta.env as Record<string, string | undefined>)[name];
  if (raw === undefined || raw === null || String(raw).trim() === '') {
    throw new Error(`[CONFIG ERROR] ${String(name)} must be set`);
  }
  return String(raw);
}

function readWebEnv() {
  return {
    apiBaseUrl: requireEnv('VITE_API_BASE_URL'),
    wsUrl: requireEnv('VITE_WS_URL'),
    adminApiBaseUrl: requireEnv('VITE_ADMIN_API_BASE_URL'),
    analyticsEndpoint: requireEnv('VITE_ANALYTICS_ENDPOINT'),
  } as const;
}

export const webEnv = readWebEnv();
