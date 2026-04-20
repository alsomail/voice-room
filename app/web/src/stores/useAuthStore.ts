/**
 * useAuthStore — 管理员认证状态（Zustand）
 *
 * 职责（T-20002 TDS §二）：
 *   - 从 localStorage 初始化 token，检查 JWT exp 有效性
 *   - login()：调用 adminLogin API → 存储 token + admin 到 store & localStorage
 *   - logout()：清除 token / admin / localStorage
 *   - checkAuth()：检查当前 token 是否过期（过期时自动 logout）
 *   - isAuthenticated：token 非 null 且未过期
 */

import { create } from 'zustand';
import { adminLogin } from '../core/network/apiClient';
import type { AdminLoginData } from '../core/network/apiClient';

/**
 * ⚠️ XSS 风险说明：JWT 存储在 localStorage 中，可被同域 XSS 脚本读取。
 * 缓解措施：
 *   1. 严格实施 Content-Security-Policy (CSP)，禁止内联脚本（'unsafe-inline'）
 *   2. 所有用户输入必须经过 HTML 转义，避免注入点
 *   3. 生产环境启用 Subresource Integrity (SRI) 校验第三方脚本
 * 如需更高安全级别，可考虑改用 HttpOnly Cookie（需后端配合）。
 */
export const ADMIN_TOKEN_KEY = 'adminToken';

export interface AuthStore {
  token: string | null;
  admin: AdminLoginData['admin'] | null;
  isAuthenticated: boolean;
  login: (username: string, password: string) => Promise<void>;
  logout: () => void;
  checkAuth: () => boolean;
}

/**
 * 解码 JWT payload（base64url decode），失败时返回 null
 * 不验证签名——仅用于读取 exp 字段
 */
function decodeJwtPayload(token: string): Record<string, unknown> | null {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return null;
    // base64url → base64 standard，并补足 '=' padding（M01：Safari atob 严格要求 padding）
    const base64 = parts[1].replace(/-/g, '+').replace(/_/g, '/');
    const padded = base64.padEnd(base64.length + (4 - (base64.length % 4)) % 4, '=');
    const json = atob(padded);
    return JSON.parse(json) as Record<string, unknown>;
  } catch {
    return null;
  }
}

/**
 * 检查 token 是否有效（有效 = token 非 null + exp 在未来）
 */
function isTokenValid(token: string | null): boolean {
  if (!token) return false;
  const payload = decodeJwtPayload(token);
  if (!payload) return false;
  const exp = payload['exp'];
  if (typeof exp !== 'number') return false;
  return exp > Math.floor(Date.now() / 1000);
}

/** 从 localStorage 读取初始 token */
const initialToken = localStorage.getItem(ADMIN_TOKEN_KEY);

export const useAuthStore = create<AuthStore>((set, get) => ({
  token: initialToken,
  admin: null,
  isAuthenticated: isTokenValid(initialToken),

  login: async (username: string, password: string) => {
    const data = await adminLogin({ username, password });
    localStorage.setItem(ADMIN_TOKEN_KEY, data.token);
    set({
      token: data.token,
      admin: data.admin,
      isAuthenticated: isTokenValid(data.token),
    });
  },

  logout: () => {
    localStorage.removeItem(ADMIN_TOKEN_KEY);
    set({ token: null, admin: null, isAuthenticated: false });
  },

  checkAuth: () => {
    const { token } = get();
    if (isTokenValid(token)) return true;
    // token 过期或无效，自动 logout
    if (token !== null) {
      get().logout();
    }
    return false;
  },
}));
