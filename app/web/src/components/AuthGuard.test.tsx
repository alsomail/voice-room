/**
 * T-20002: AuthGuard — TDD 测试套件
 *
 * 覆盖范围：
 *   - 已认证时渲染子组件（Outlet）
 *   - 未认证时重定向到 /login
 *   - token 过期（isAuthenticated=false）时重定向到 /login
 *   - [HIGH-H02] 每次渲染调用 checkAuth()，而非读取 isAuthenticated 快照
 *   - [HIGH-H02] isAuthenticated=true 但 checkAuth()=false（运行时 token 过期）→ 重定向 /login
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';
import { MemoryRouter, Routes, Route } from 'react-router-dom';

// ── mock useAuthStore ─────────────────────────────────────────────────────────
const mockCheckAuth = vi.fn();
const mockAuthState = {
  isAuthenticated: false,
  token: null as string | null,
  admin: null,
  checkAuth: mockCheckAuth,
  login: vi.fn(),
  logout: vi.fn(),
};

vi.mock('../stores/useAuthStore', () => ({
  useAuthStore: (selector?: (s: typeof mockAuthState) => unknown) => {
    if (typeof selector === 'function') return selector(mockAuthState);
    return mockAuthState;
  },
}));

import { AuthGuard } from './AuthGuard';

// ── 辅助：构建路由测试树 ────────────────────────────────────────────────────────
function renderWithRouter(
  initialPath: string,
  isAuthenticated: boolean,
) {
  mockAuthState.isAuthenticated = isAuthenticated;
  mockCheckAuth.mockReturnValue(isAuthenticated);

  return render(
    <MemoryRouter initialEntries={[initialPath]}>
      <Routes>
        <Route path="/login" element={<div data-testid="login-page">LoginPage</div>} />
        <Route element={<AuthGuard />}>
          <Route
            path="/dashboard"
            element={<div data-testid="dashboard-page">Dashboard</div>}
          />
          <Route
            path="/protected"
            element={<div data-testid="protected-page">Protected</div>}
          />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  localStorage.clear();
});

// ── 1. 已认证时渲染子路由 ───────────────────────────────────────────────────────
describe('AuthGuard — 已认证', () => {
  it('已认证用户访问 /dashboard 时渲染 dashboard 内容', () => {
    renderWithRouter('/dashboard', true);
    expect(screen.getByTestId('dashboard-page')).toBeInTheDocument();
    expect(screen.queryByTestId('login-page')).not.toBeInTheDocument();
  });

  it('已认证用户访问 /protected 时渲染受保护内容', () => {
    renderWithRouter('/protected', true);
    expect(screen.getByTestId('protected-page')).toBeInTheDocument();
  });
});

// ── 2. 未认证时重定向到 /login ──────────────────────────────────────────────────
describe('AuthGuard — 未认证', () => {
  it('未认证用户访问 /dashboard 时自动重定向到 /login', () => {
    renderWithRouter('/dashboard', false);
    expect(screen.getByTestId('login-page')).toBeInTheDocument();
    expect(screen.queryByTestId('dashboard-page')).not.toBeInTheDocument();
  });

  it('未认证用户访问 /protected 时自动重定向到 /login', () => {
    renderWithRouter('/protected', false);
    expect(screen.getByTestId('login-page')).toBeInTheDocument();
    expect(screen.queryByTestId('protected-page')).not.toBeInTheDocument();
  });
});

// ── 3. checkAuth() 调用验证（HIGH-H02）────────────────────────────────────────
describe('AuthGuard — checkAuth() 调用验证（H02）', () => {
  it('每次渲染时调用 checkAuth()，而非仅读取 isAuthenticated 属性快照', () => {
    renderWithRouter('/dashboard', true);
    // 若 AuthGuard 仍用 isAuthenticated selector，mockCheckAuth 不会被调用 → 本测试 FAIL
    expect(mockCheckAuth).toHaveBeenCalled();
  });

  it('isAuthenticated=true 但 checkAuth()=false（token 在使用中过期）时，必须重定向到 /login', () => {
    // 故意设置 isAuthenticated 为 true（旧快照），但 checkAuth() 返回 false（token 已过期）
    // 若 AuthGuard 读 isAuthenticated → 渲染 dashboard（错误）
    // 若 AuthGuard 调 checkAuth() → 重定向 /login（正确）
    mockAuthState.isAuthenticated = true;
    mockCheckAuth.mockReturnValue(false);

    render(
      <MemoryRouter initialEntries={['/dashboard']}>
        <Routes>
          <Route path="/login" element={<div data-testid="login-page">LoginPage</div>} />
          <Route element={<AuthGuard />}>
            <Route
              path="/dashboard"
              element={<div data-testid="dashboard-page">Dashboard</div>}
            />
          </Route>
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByTestId('login-page')).toBeInTheDocument();
    expect(screen.queryByTestId('dashboard-page')).not.toBeInTheDocument();
  });
});

