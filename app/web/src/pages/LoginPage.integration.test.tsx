/**
 * T-20002: LoginPage 集成测试（接入 useAuthStore）
 *
 * 覆盖范围：
 *   - 登录成功后跳转到 /dashboard
 *   - 登录失败后显示错误 Alert
 *   - 已认证用户直接访问 /login 时重定向到 /dashboard
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import { MemoryRouter, Routes, Route } from 'react-router-dom';

// ── i18n mock ─────────────────────────────────────────────────────────────────
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { changeLanguage: vi.fn(), language: 'en' },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ── useAuthStore mock ──────────────────────────────────────────────────────────
const mockLogin = vi.fn();
const mockAuthState = {
  isAuthenticated: false,
  token: null as string | null,
  admin: null,
  checkAuth: vi.fn().mockReturnValue(false),
  login: mockLogin,
  logout: vi.fn(),
};

vi.mock('../stores/useAuthStore', () => ({
  useAuthStore: (selector?: (s: typeof mockAuthState) => unknown) => {
    if (typeof selector === 'function') return selector(mockAuthState);
    return mockAuthState;
  },
}));

import { LoginPage } from './login/index';

// ── 辅助：完整路由渲染 ─────────────────────────────────────────────────────────
function renderLoginRoute(isAuthenticated = false) {
  mockAuthState.isAuthenticated = isAuthenticated;
  mockAuthState.checkAuth.mockReturnValue(isAuthenticated);
  return render(
    <MemoryRouter initialEntries={['/login']}>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route
          path="/dashboard"
          element={<div data-testid="dashboard-page">Dashboard</div>}
        />
      </Routes>
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

// ── 1. 登录成功跳转 /dashboard ─────────────────────────────────────────────────
describe('LoginPage — 登录成功跳转', () => {
  it('登录成功后跳转到 /dashboard', async () => {
    mockLogin.mockResolvedValueOnce(undefined);
    renderLoginRoute(false);

    await userEvent.type(screen.getByTestId('input-username'), 'admin');
    await userEvent.type(screen.getByTestId('input-password'), 'password123');
    await userEvent.click(screen.getByTestId('btn-submit'));

    await waitFor(() => {
      expect(screen.getByTestId('dashboard-page')).toBeInTheDocument();
    });
  });

  it('登录成功后以正确的 username/password 调用 useAuthStore.login', async () => {
    mockLogin.mockResolvedValueOnce(undefined);
    renderLoginRoute(false);

    await userEvent.type(screen.getByTestId('input-username'), 'admin');
    await userEvent.type(screen.getByTestId('input-password'), 'secret');
    await userEvent.click(screen.getByTestId('btn-submit'));

    await waitFor(() => {
      expect(mockLogin).toHaveBeenCalledWith('admin', 'secret');
    });
  });
});

// ── 2. 登录失败保持在 /login ────────────────────────────────────────────────────
describe('LoginPage — 登录失败', () => {
  it('登录失败后显示错误 Alert，保持在登录页', async () => {
    mockLogin.mockRejectedValueOnce(new Error('Invalid credentials'));
    renderLoginRoute(false);

    await userEvent.type(screen.getByTestId('input-username'), 'admin');
    await userEvent.type(screen.getByTestId('input-password'), 'wrong');
    await userEvent.click(screen.getByTestId('btn-submit'));

    await waitFor(() => {
      expect(screen.getByTestId('alert-error')).toBeInTheDocument();
      expect(screen.queryByTestId('dashboard-page')).not.toBeInTheDocument();
    });
  });
});

// ── 3. 已认证用户访问 /login 重定向 ─────────────────────────────────────────────
describe('LoginPage — 已认证用户重定向', () => {
  it('已认证用户访问 /login 时自动重定向到 /dashboard', () => {
    renderLoginRoute(true);
    expect(screen.getByTestId('dashboard-page')).toBeInTheDocument();
    expect(screen.queryByTestId('input-username')).not.toBeInTheDocument();
  });
});
