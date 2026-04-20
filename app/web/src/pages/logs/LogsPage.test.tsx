/**
 * T-20009: LogsPage 集成测试
 *
 * 验收用例：
 *   LP-01: API 成功 → Table 显示 3 行数据
 *   LP-02: API 失败 → 显示 data-testid="logs-error" Alert
 *   LP-03: URL 含参数 ?action=ban_user&page=2 → adminGetLogs 携带对应参数
 *
 * 扩展用例：
 *   LP-04: 操作人ID搜索 → adminGetLogs 以 admin_id 参数调用
 *   LP-05: 重置 → 以空参数重新发起请求
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import { MemoryRouter } from 'react-router-dom';
import React from 'react';

// ── i18n mock ─────────────────────────────────────────────────────────────
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { changeLanguage: vi.fn(), language: 'zh' },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ── apiClient mock ─────────────────────────────────────────────────────────
vi.mock('../../core/network/apiClient', () => ({
  adminGetLogs: vi.fn(),
}));

import { adminGetLogs } from '../../core/network/apiClient';
import { LogsPage } from './index';
import type { AdminLogsData } from '../../core/network/apiClient';

const mockAdminGetLogs = adminGetLogs as ReturnType<typeof vi.fn>;

// ── 测试数据工厂 ───────────────────────────────────────────────────────────
function makeLogsData(count: number): AdminLogsData {
  return {
    total: count,
    page: 1,
    size: 20,
    items: Array.from({ length: count }, (_, i) => ({
      id: `log-${i + 1}`,
      admin_id: `admin-${i + 1}`,
      action: 'ban_user',
      target_type: 'user',
      target_id: `target-${i + 1}`,
      ip_address: `192.168.1.${i + 1}`,
      detail: { reason: 'test' },
      created_at: '2025-01-01T00:00:00Z',
    })),
  };
}

// ── 带路由的渲染 helper ─────────────────────────────────────────────────────
function renderWithRouter(route = '/') {
  return render(
    <MemoryRouter initialEntries={[route]}>
      <LogsPage />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  mockAdminGetLogs.mockResolvedValue(makeLogsData(3));
});

// ── LP-01: API 成功 → Table 显示 3 行 ─────────────────────────────────────
describe('LogsPage — LP-01: API 成功渲染', () => {
  it('API 返回 3 条，Table 显示 3 行', async () => {
    renderWithRouter();

    const table = await screen.findByTestId('logs-table');
    await waitFor(() => {
      const rows = within(table).getAllByRole('row');
      expect(rows.length - 1).toBe(3);
    });
  });

  it('loading 状态在请求完成后消失', async () => {
    renderWithRouter();

    await waitFor(() =>
      expect(screen.queryByTestId('logs-error')).not.toBeInTheDocument(),
    );
    // loading 完成后表格可见
    await screen.findByTestId('logs-table');
  });
});

// ── LP-02: API 失败 → 显示 logs-error ─────────────────────────────────────
describe('LogsPage — LP-02: API 失败', () => {
  it('API 失败时显示 data-testid="logs-error"', async () => {
    mockAdminGetLogs.mockRejectedValue(new Error('Network Error'));

    renderWithRouter();

    await waitFor(() => {
      expect(screen.getByTestId('logs-error')).toBeInTheDocument();
    });
  });

  it('API 失败时不渲染旧数据行', async () => {
    mockAdminGetLogs.mockRejectedValue(new Error('Server Down'));

    renderWithRouter();

    await waitFor(() => {
      expect(screen.getByTestId('logs-error')).toBeInTheDocument();
    });

    // items=[] → 表格无任何 action Tag（no ban_user data-testid）
    expect(screen.queryByTestId('action-tag-ban_user')).not.toBeInTheDocument();
  });
});

// ── LP-03: URL 状态持久化 ──────────────────────────────────────────────────
describe('LogsPage — LP-03: URL 状态持久化', () => {
  it('URL 含 ?action=ban_user&page=2，adminGetLogs 携带对应参数', async () => {
    renderWithRouter('/?action=ban_user&page=2');

    await waitFor(() => expect(mockAdminGetLogs).toHaveBeenCalled());

    const lastCall = mockAdminGetLogs.mock.calls[mockAdminGetLogs.mock.calls.length - 1];
    expect(lastCall[0]).toMatchObject({ action: 'ban_user', page: 2 });
  });
});

// ── LP-04: 操作人ID搜索 ────────────────────────────────────────────────────
describe('LogsPage — LP-04: 操作人ID搜索', () => {
  it('输入操作人 ID 并点击搜索，adminGetLogs 以 admin_id 参数调用', async () => {
    const user = userEvent.setup();
    renderWithRouter();

    await waitFor(() => expect(screen.getByTestId('logs-table')).toBeInTheDocument());

    mockAdminGetLogs.mockClear();

    const adminIdInput = screen.getByPlaceholderText('logs.adminIdPlaceholder');
    await user.type(adminIdInput, 'uuid-admin-001');
    await user.click(screen.getByText('logs.search'));

    await waitFor(() => {
      expect(mockAdminGetLogs).toHaveBeenCalled();
      const lastCall = mockAdminGetLogs.mock.calls[mockAdminGetLogs.mock.calls.length - 1];
      expect(lastCall[0]).toMatchObject({ admin_id: 'uuid-admin-001' });
    });
  });
});

// ── LP-05: 重置 ────────────────────────────────────────────────────────────
describe('LogsPage — LP-05: 重置', () => {
  it('输入操作人 ID 搜索后点击重置，再次以空参数发起请求', async () => {
    const user = userEvent.setup();
    renderWithRouter();

    await waitFor(() => expect(screen.getByTestId('logs-table')).toBeInTheDocument());

    // 先搜索
    const adminIdInput = screen.getByPlaceholderText('logs.adminIdPlaceholder');
    await user.type(adminIdInput, 'uuid-xxx');
    await user.click(screen.getByText('logs.search'));
    await waitFor(() => expect(mockAdminGetLogs).toHaveBeenCalled());

    mockAdminGetLogs.mockClear();

    // 重置
    await user.click(screen.getByText('logs.reset'));

    await waitFor(() => expect(mockAdminGetLogs).toHaveBeenCalled());

    const lastCall = mockAdminGetLogs.mock.calls[mockAdminGetLogs.mock.calls.length - 1];
    expect(lastCall[0]).not.toHaveProperty('admin_id');
  });
});
