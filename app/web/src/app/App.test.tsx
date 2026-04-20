/**
 * T-20002: App 路由配置测试
 *
 * 覆盖范围：
 *   - /login 路由渲染 LoginPage
 *   - /dashboard 路由被 AuthGuard 保护
 *   - 未认证访问 /dashboard 重定向到 /login
 *   - / 根路由重定向到 /dashboard（已认证）或 /login（未认证）
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';

// ── i18n mock ─────────────────────────────────────────────────────────────────
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { changeLanguage: vi.fn(), language: 'en' },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ── useAuthStore mock ──────────────────────────────────────────────────────────
const mockAuthState = {
  isAuthenticated: false,
  token: null as string | null,
  admin: null,
  checkAuth: vi.fn().mockReturnValue(false),
  login: vi.fn(),
  logout: vi.fn(),
};

vi.mock('../stores/useAuthStore', () => ({
  useAuthStore: (selector?: (s: typeof mockAuthState) => unknown) => {
    if (typeof selector === 'function') return selector(mockAuthState);
    return mockAuthState;
  },
}));

// ── react-router-dom: 用 MemoryRouter 替换 BrowserRouter ──────────────────────
import { MemoryRouter } from 'react-router-dom';
import { AppRoutes } from '../router/index';

function renderApp(initialPath: string, isAuthenticated = false) {
  mockAuthState.isAuthenticated = isAuthenticated;
  mockAuthState.checkAuth.mockReturnValue(isAuthenticated);
  return render(
    <MemoryRouter initialEntries={[initialPath]}>
      <AppRoutes />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
});

afterEach(() => {
  localStorage.clear();
});

// ── 1. 公开路由 ─────────────────────────────────────────────────────────────────
describe('App 路由 — 公开路由', () => {
  it('访问 /login 时渲染登录页（含用户名输入框）', () => {
    renderApp('/login', false);
    expect(screen.getByTestId('input-username')).toBeInTheDocument();
  });
});

// ── 2. 受保护路由 ────────────────────────────────────────────────────────────────
describe('App 路由 — 受保护路由', () => {
  it('未认证访问 /dashboard 时重定向到 /login', () => {
    renderApp('/dashboard', false);
    expect(screen.getByTestId('input-username')).toBeInTheDocument();
  });

  it('已认证访问 /dashboard 时渲染 dashboard 内容', () => {
    renderApp('/dashboard', true);
    expect(screen.getByTestId('dashboard-page')).toBeInTheDocument();
  });
});

// ── 3. 根路由重定向 ─────────────────────────────────────────────────────────────
describe('App 路由 — 根路由', () => {
  it('未认证访问 / 时重定向到 /login', () => {
    renderApp('/', false);
    expect(screen.getByTestId('input-username')).toBeInTheDocument();
  });
});
