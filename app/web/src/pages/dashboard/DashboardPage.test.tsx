/**
 * T-20003: DashboardPage 集成测试
 *
 * 验收用例：
 *   I01: mock API 全部成功 → 页面上可见卡片数值
 *   I02: mock API 全部失败 → 显示错误提示，刷新按钮可用
 *   I03: 点击刷新按钮 → API 函数被重新调用
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';

// ── i18n mock ─────────────────────────────────────────────────────────────────
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { changeLanguage: vi.fn(), language: 'zh' },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ── ECharts mock ──────────────────────────────────────────────────────────────
vi.mock('echarts-for-react', () => ({
  default: ({ 'data-testid': testId }: { 'data-testid'?: string }) => (
    <div data-testid={testId ?? 'echarts-mock'} />
  ),
}));

// ── apiClient mock ────────────────────────────────────────────────────────────
vi.mock('../../core/network/apiClient', () => ({
  adminGetRooms: vi.fn(),
  adminGetStatsOverview: vi.fn(),
}));

import { adminGetRooms, adminGetStatsOverview } from '../../core/network/apiClient';
import { DashboardPage } from './index';

const mockAdminGetRooms = adminGetRooms as ReturnType<typeof vi.fn>;
const mockAdminGetStatsOverview = adminGetStatsOverview as ReturnType<typeof vi.fn>;

const ROOMS_ALL = { total: 50, page: 1, page_size: 20, items: [] };
const ROOMS_ACTIVE = { total: 12, page: 1, page_size: 20, items: [] };
const STATS_OVERVIEW = {
  online_users: 340,
  dau: 1200,
  new_users_today: 88,
  trend: [{ date: '2025-05-11', dau: 1200, new_users: 88 }],
};

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  vi.clearAllTimers();
});

// ── I01: API 成功 → 页面显示数值 ──────────────────────────────────────────────
describe('DashboardPage — I01: API 成功', () => {
  it('渲染后卡片上可见在线人数', async () => {
    mockAdminGetRooms
      .mockResolvedValueOnce(ROOMS_ALL)
      .mockResolvedValueOnce(ROOMS_ACTIVE);
    mockAdminGetStatsOverview.mockResolvedValueOnce(STATS_OVERVIEW);

    render(<DashboardPage />);

    await waitFor(() => {
      expect(screen.getByTestId('card-online-users')).toHaveTextContent('340');
    });
  });

  it('渲染后卡片上可见活跃房间数', async () => {
    mockAdminGetRooms
      .mockResolvedValueOnce(ROOMS_ALL)
      .mockResolvedValueOnce(ROOMS_ACTIVE);
    mockAdminGetStatsOverview.mockResolvedValueOnce(STATS_OVERVIEW);

    render(<DashboardPage />);

    await waitFor(() => {
      expect(screen.getByTestId('card-active-rooms')).toHaveTextContent('12');
    });
  });
});

// ── I02: API 全失败 → 错误提示 + 刷新按钮可用 ────────────────────────────────
describe('DashboardPage — I02: API 全失败', () => {
  it('显示错误提示，刷新按钮不被禁用', async () => {
    mockAdminGetRooms.mockRejectedValue(new Error('Server Down'));
    mockAdminGetStatsOverview.mockRejectedValue(new Error('Server Down'));

    render(<DashboardPage />);

    await waitFor(() => {
      expect(screen.getByTestId('dashboard-error')).toBeInTheDocument();
    });

    expect(screen.getByTestId('btn-refresh')).not.toBeDisabled();
  });
});

// ── I03: 点击刷新按钮 → API 重新调用 ─────────────────────────────────────────
describe('DashboardPage — I03: 点击刷新', () => {
  it('点击刷新按钮后 adminGetRooms 被再次调用', async () => {
    mockAdminGetRooms
      .mockResolvedValueOnce(ROOMS_ALL)
      .mockResolvedValueOnce(ROOMS_ACTIVE)
      .mockResolvedValueOnce(ROOMS_ALL)
      .mockResolvedValueOnce(ROOMS_ACTIVE);
    mockAdminGetStatsOverview.mockResolvedValue(STATS_OVERVIEW);

    render(<DashboardPage />);
    await waitFor(() => expect(screen.getByTestId('card-online-users')).toBeInTheDocument());

    const callsBefore = mockAdminGetRooms.mock.calls.length;
    await userEvent.click(screen.getByTestId('btn-refresh'));

    await waitFor(() => {
      expect(mockAdminGetRooms.mock.calls.length).toBeGreaterThan(callsBefore);
    });
  });
});
