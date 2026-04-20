/**
 * T-20002: useAuthStore — TDD 测试套件
 *
 * 覆盖范围：
 *   - 初始状态：从 localStorage 读取 adminToken，解码 JWT 判断有效性
 *   - login()：调用 adminLogin API → 成功时存 token + admin 到 store & localStorage
 *   - logout()：清除 token / admin / localStorage
 *   - checkAuth()：JWT exp 过期时返回 false，未过期返回 true
 *   - isAuthenticated 计算属性：token 非 null 且未过期
 *   - 边界：token 为 null / 无效格式 / 缺少 exp 字段
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { AdminLoginData } from '../core/network/apiClient';

// ── mock apiClient ────────────────────────────────────────────────────────────
vi.mock('../core/network/apiClient', () => ({
  adminLogin: vi.fn(),
}));

import { adminLogin } from '../core/network/apiClient';
const mockAdminLogin = vi.mocked(adminLogin);

// ── 辅助：构造 JWT ──────────────────────────────────────────────────────────────
function makeJwt(payload: Record<string, unknown>): string {
  const header = btoa(JSON.stringify({ alg: 'HS256', typ: 'JWT' }));
  const body = btoa(JSON.stringify(payload));
  return `${header}.${body}.fake_signature`;
}

const FUTURE_EXP = Math.floor(Date.now() / 1000) + 3600; // 1 小时后
const PAST_EXP = Math.floor(Date.now() / 1000) - 3600;   // 1 小时前

const VALID_TOKEN = makeJwt({ sub: 'admin-1', exp: FUTURE_EXP });
const EXPIRED_TOKEN = makeJwt({ sub: 'admin-1', exp: PAST_EXP });
const NO_EXP_TOKEN = makeJwt({ sub: 'admin-1' });
const INVALID_TOKEN = 'not.a.valid.jwt';

const MOCK_ADMIN: AdminLoginData['admin'] = {
  id: 'admin-1',
  username: 'admin',
  role: 'super_admin',
  display_name: 'Admin User',
  last_login_at: '2024-01-01T00:00:00Z',
};

// ── beforeEach：每个测试前清空 localStorage 并重置 store ──────────────────────
beforeEach(() => {
  localStorage.clear();
  vi.clearAllMocks();
});

afterEach(() => {
  localStorage.clear();
});

// ── 动态 import 使每个 describe 拿到新鲜 store ───────────────────────────────
async function freshStore() {
  vi.resetModules();
  const mod = await import('./useAuthStore');
  return mod.useAuthStore;
}

// ── 1. 初始状态 ────────────────────────────────────────────────────────────────
describe('useAuthStore — 初始状态', () => {
  it('localStorage 无 token 时，token 为 null，isAuthenticated 为 false', async () => {
    const useStore = await freshStore();
    const state = useStore.getState();
    expect(state.token).toBeNull();
    expect(state.isAuthenticated).toBe(false);
  });

  it('localStorage 有有效 token 时，初始化后 isAuthenticated 为 true', async () => {
    localStorage.setItem('adminToken', VALID_TOKEN);
    const useStore = await freshStore();
    const state = useStore.getState();
    expect(state.token).toBe(VALID_TOKEN);
    expect(state.isAuthenticated).toBe(true);
  });

  it('localStorage 有过期 token 时，初始化后 isAuthenticated 为 false', async () => {
    localStorage.setItem('adminToken', EXPIRED_TOKEN);
    const useStore = await freshStore();
    const state = useStore.getState();
    expect(state.isAuthenticated).toBe(false);
  });
});

// ── 2. login() ────────────────────────────────────────────────────────────────
describe('useAuthStore — login()', () => {
  it('登录成功后 token 和 admin 写入 store', async () => {
    const useStore = await freshStore();
    mockAdminLogin.mockResolvedValueOnce({
      token: VALID_TOKEN,
      expires_in: 3600,
      admin: MOCK_ADMIN,
    });

    await useStore.getState().login('admin', 'password123');

    const state = useStore.getState();
    expect(state.token).toBe(VALID_TOKEN);
    expect(state.admin).toEqual(MOCK_ADMIN);
    expect(state.isAuthenticated).toBe(true);
  });

  it('登录成功后 token 写入 localStorage["adminToken"]', async () => {
    const useStore = await freshStore();
    mockAdminLogin.mockResolvedValueOnce({
      token: VALID_TOKEN,
      expires_in: 3600,
      admin: MOCK_ADMIN,
    });

    await useStore.getState().login('admin', 'password123');

    expect(localStorage.getItem('adminToken')).toBe(VALID_TOKEN);
  });

  it('login() 以正确参数调用 adminLogin', async () => {
    const useStore = await freshStore();
    mockAdminLogin.mockResolvedValueOnce({
      token: VALID_TOKEN,
      expires_in: 3600,
      admin: MOCK_ADMIN,
    });

    await useStore.getState().login('testuser', 'testpass');

    expect(mockAdminLogin).toHaveBeenCalledOnce();
    expect(mockAdminLogin).toHaveBeenCalledWith({
      username: 'testuser',
      password: 'testpass',
    });
  });

  it('login() 失败时抛出错误，store 保持未认证状态', async () => {
    const useStore = await freshStore();
    mockAdminLogin.mockRejectedValueOnce(new Error('Invalid credentials'));

    await expect(
      useStore.getState().login('admin', 'wrong'),
    ).rejects.toThrow('Invalid credentials');

    const state = useStore.getState();
    expect(state.token).toBeNull();
    expect(state.isAuthenticated).toBe(false);
  });
});

// ── 3. logout() ───────────────────────────────────────────────────────────────
describe('useAuthStore — logout()', () => {
  it('logout() 后 token 和 admin 清空', async () => {
    localStorage.setItem('adminToken', VALID_TOKEN);
    const useStore = await freshStore();

    useStore.getState().logout();

    const state = useStore.getState();
    expect(state.token).toBeNull();
    expect(state.admin).toBeNull();
    expect(state.isAuthenticated).toBe(false);
  });

  it('logout() 后 localStorage["adminToken"] 被清除', async () => {
    localStorage.setItem('adminToken', VALID_TOKEN);
    const useStore = await freshStore();

    useStore.getState().logout();

    expect(localStorage.getItem('adminToken')).toBeNull();
  });
});

// ── 4. checkAuth() ────────────────────────────────────────────────────────────
describe('useAuthStore — checkAuth()', () => {
  it('有效 token 时 checkAuth() 返回 true', async () => {
    localStorage.setItem('adminToken', VALID_TOKEN);
    const useStore = await freshStore();
    expect(useStore.getState().checkAuth()).toBe(true);
  });

  it('过期 token 时 checkAuth() 返回 false', async () => {
    localStorage.setItem('adminToken', EXPIRED_TOKEN);
    const useStore = await freshStore();
    expect(useStore.getState().checkAuth()).toBe(false);
  });

  it('无 exp 字段的 token 时 checkAuth() 返回 false', async () => {
    localStorage.setItem('adminToken', NO_EXP_TOKEN);
    const useStore = await freshStore();
    expect(useStore.getState().checkAuth()).toBe(false);
  });

  it('无 token 时 checkAuth() 返回 false', async () => {
    const useStore = await freshStore();
    expect(useStore.getState().checkAuth()).toBe(false);
  });

  it('格式无效的 token checkAuth() 返回 false（不崩溃）', async () => {
    localStorage.setItem('adminToken', INVALID_TOKEN);
    const useStore = await freshStore();
    expect(useStore.getState().checkAuth()).toBe(false);
  });

  it('base64 payload 无法解析的 token checkAuth() 返回 false（atob 失败）', async () => {
    // 3 段式但 payload 是非法 base64 字符
    const badPayloadToken = 'header.!!!invalid-base64!!!.signature';
    localStorage.setItem('adminToken', badPayloadToken);
    const useStore = await freshStore();
    expect(useStore.getState().checkAuth()).toBe(false);
  });

  it('[M01] base64url payload 无 padding 时 checkAuth() 正确解码（Safari padding 修复）', async () => {
    // 真实 JWT 使用 base64url（无 '='  padding），Safari 的 atob 在缺少 padding 时会抛异常
    // 若未补位 → try/catch 吞掉错误 → decodeJwtPayload 返回 null → 合法 token 被误判为无效
    const payload = { sub: 'admin-1', exp: FUTURE_EXP };
    // 生成 base64url 编码（去掉所有 '=' padding）
    const base64urlPayload = btoa(JSON.stringify(payload))
      .replace(/\+/g, '-')
      .replace(/\//g, '_')
      .replace(/=/g, ''); // 故意去掉 padding，还原真实 JWT 场景

    // 确保此 payload 确实缺少 padding（length % 4 !== 0）
    expect(base64urlPayload.length % 4).not.toBe(0);

    const tokenWithoutPadding = `eyJhbGciOiJIUzI1NiJ9.${base64urlPayload}.fake_sig`;
    localStorage.setItem('adminToken', tokenWithoutPadding);
    const useStore = await freshStore();

    // 有效 token（exp 未过期），checkAuth() 应返回 true
    expect(useStore.getState().checkAuth()).toBe(true);
  });

  it('过期 token 调用 checkAuth() 后自动 logout（清空 store + localStorage）', async () => {
    localStorage.setItem('adminToken', EXPIRED_TOKEN);
    const useStore = await freshStore();

    useStore.getState().checkAuth();

    expect(useStore.getState().token).toBeNull();
    expect(localStorage.getItem('adminToken')).toBeNull();
  });
});
