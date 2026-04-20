/**
 * T-20006: useUsersPage Hook — TDD 测试套件
 *
 * 验收用例：
 *   H01: 初始挂载 → 调用 adminGetUsers，items 填充，loading=false
 *   H02: setFilters({phone:"138"}) → page 重置为 1，带 phone 参数请求
 *   H03: setFilters({status:"banned"}) → 请求携带 status=banned
 *   H04: setPage(2, 20) → 请求 page=2
 *   H05: refresh() → 以当前 filters 重新发起请求
 *   H06: adminGetUsers 失败 → error 非 null，items=[]
 *   H07: 卸载 → AbortController.abort 被调用
 *   H08: 从 URL 读取初始参数（phone/status/page）
 *   H09: setFilters 变化时 page 重置为 1
 *   H10: adminGetUsers 抛出非 Error 类型时 error.message 有值
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

// ── mock react-router-dom（避免 Router context 依赖）─────────────────────────
vi.mock('react-router-dom', () => ({
  useSearchParams: vi.fn(() => [new URLSearchParams(), vi.fn()]),
}));

// ── mock apiClient ─────────────────────────────────────────────────────────
vi.mock('../../core/network/apiClient', () => ({
  adminGetUsers: vi.fn(),
}));

import { useSearchParams } from 'react-router-dom';
import { adminGetUsers } from '../../core/network/apiClient';
import { useUsersPage } from './useUsersPage';
import type { AdminUserItem, AdminUsersData } from '../../core/network/apiClient';

const mockAdminGetUsers = adminGetUsers as ReturnType<typeof vi.fn>;
const mockUseSearchParams = useSearchParams as ReturnType<typeof vi.fn>;

// ── 测试数据工厂 ───────────────────────────────────────────────────────────
function makeUser(id: number, status: 'normal' | 'banned' = 'normal'): AdminUserItem {
  return {
    id: `user-${id}`,
    phone: `1380013800${id}`,
    nickname: `User${id}`,
    avatar: undefined,
    coin_balance: 100,
    vip_level: 0,
    status,
    created_at: '2025-01-01T00:00:00Z',
  };
}

function makeUsersData(count: number): AdminUsersData {
  return {
    total: count,
    page: 1,
    size: 20,
    items: Array.from({ length: count }, (_, i) => makeUser(i + 1)),
  };
}

const USERS_DATA = makeUsersData(3);

beforeEach(() => {
  vi.clearAllMocks();
  mockUseSearchParams.mockReturnValue([new URLSearchParams(), vi.fn()]);
  mockAdminGetUsers.mockResolvedValue(USERS_DATA);
});

// ── H01: 初始挂载 ──────────────────────────────────────────────────────────
describe('useUsersPage — H01: 初始加载', () => {
  it('挂载时调用 adminGetUsers，loading → false，items 填充', async () => {
    const { result } = renderHook(() => useUsersPage());

    expect(result.current.loading).toBe(true);

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(mockAdminGetUsers).toHaveBeenCalled();
    expect(result.current.items).toHaveLength(3);
    expect(result.current.total).toBe(3);
    expect(result.current.error).toBeNull();
  });
});

// ── H02: setFilters phone → page 重置，带 phone 参数 ──────────────────────
describe('useUsersPage — H02: setFilters phone 重置 page=1', () => {
  it('setFilters({phone:"138"}) → page 重置为 1，请求携带 phone', async () => {
    const { result } = renderHook(() => useUsersPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    // 先翻到第 2 页
    act(() => { result.current.setPage(2, 20); });
    await waitFor(() => expect(result.current.page).toBe(2));

    mockAdminGetUsers.mockClear();

    act(() => { result.current.setFilters({ phone: '138' }); });

    await waitFor(() => expect(mockAdminGetUsers).toHaveBeenCalled());

    expect(result.current.page).toBe(1);
    const lastCall = mockAdminGetUsers.mock.calls[mockAdminGetUsers.mock.calls.length - 1];
    expect(lastCall[0]).toMatchObject({ phone: '138', page: 1 });
  });
});

// ── H03: setFilters status → 请求携带 status ──────────────────────────────
describe('useUsersPage — H03: status 过滤', () => {
  it('setFilters({status:"banned"}) → 请求携带 status=banned', async () => {
    const { result } = renderHook(() => useUsersPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    mockAdminGetUsers.mockClear();

    act(() => { result.current.setFilters({ status: 'banned' }); });

    await waitFor(() => expect(mockAdminGetUsers).toHaveBeenCalled());

    const lastCall = mockAdminGetUsers.mock.calls[mockAdminGetUsers.mock.calls.length - 1];
    expect(lastCall[0]).toMatchObject({ status: 'banned' });
  });
});

// ── H04: setPage(2, 20) → 请求 page=2 ────────────────────────────────────
describe('useUsersPage — H04: 翻页', () => {
  it('setPage(2, 20) → 发起 page=2 的请求', async () => {
    const { result } = renderHook(() => useUsersPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    mockAdminGetUsers.mockClear();

    act(() => { result.current.setPage(2, 20); });

    await waitFor(() => expect(mockAdminGetUsers).toHaveBeenCalled());

    const lastCall = mockAdminGetUsers.mock.calls[0];
    expect(lastCall[0]).toMatchObject({ page: 2, size: 20 });
  });
});

// ── H05: refresh() → 重新发起请求 ─────────────────────────────────────────
describe('useUsersPage — H05: 刷新', () => {
  it('refresh() 以当前 filters 重新发起请求', async () => {
    const { result } = renderHook(() => useUsersPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    const callsBefore = mockAdminGetUsers.mock.calls.length;

    act(() => { result.current.refresh(); });

    await waitFor(() =>
      expect(mockAdminGetUsers.mock.calls.length).toBeGreaterThan(callsBefore),
    );
  });
});

// ── H06: API 失败 → error 非 null，items=[] ──────────────────────────────
describe('useUsersPage — H06: API 失败', () => {
  it('adminGetUsers 失败 → error 非 null，items=[]', async () => {
    mockAdminGetUsers.mockRejectedValue(new Error('Network Error'));

    const { result } = renderHook(() => useUsersPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).not.toBeNull();
    expect(result.current.items).toEqual([]);
  });
});

// ── H07: 卸载 → AbortController.abort 被调用 ─────────────────────────────
describe('useUsersPage — H07: 卸载清理', () => {
  it('unmount 时 AbortController.abort 被调用', () => {
    const abortSpy = vi.spyOn(AbortController.prototype, 'abort');

    const { unmount } = renderHook(() => useUsersPage());
    unmount();

    expect(abortSpy).toHaveBeenCalled();

    abortSpy.mockRestore();
  });
});

// ── H08: 从 URL 读取初始参数 ──────────────────────────────────────────────
describe('useUsersPage — H08: 从 URL 初始化参数', () => {
  it('URL 含 phone=138&status=banned&page=2 时，hook 以对应参数发起请求', async () => {
    mockUseSearchParams.mockReturnValue([
      new URLSearchParams('phone=138&status=banned&page=2'),
      vi.fn(),
    ]);

    const { result } = renderHook(() => useUsersPage());

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.filters.phone).toBe('138');
    expect(result.current.filters.status).toBe('banned');
    expect(result.current.page).toBe(2);

    const lastCall = mockAdminGetUsers.mock.calls[mockAdminGetUsers.mock.calls.length - 1];
    expect(lastCall[0]).toMatchObject({ phone: '138', status: 'banned', page: 2 });
  });
});

// ── H09: filters 变化时 page 自动重置为 1 ─────────────────────────────────
describe('useUsersPage — H09: setFilters 重置 page=1', () => {
  it('处于第 3 页时 setFilters → page 自动重置为 1', async () => {
    const { result } = renderHook(() => useUsersPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => { result.current.setPage(3, 20); });
    await waitFor(() => expect(result.current.page).toBe(3));

    act(() => { result.current.setFilters({ nickname: 'test' }); });

    await waitFor(() => expect(result.current.page).toBe(1));
  });
});

// ── H10: 非 Error 类型抛出 → error.message 有值 ──────────────────────────
describe('useUsersPage — H10: 非 Error 类型抛出', () => {
  it('adminGetUsers 抛出字符串时 error.message 为该字符串', async () => {
    mockAdminGetUsers.mockRejectedValue('network problem');

    const { result } = renderHook(() => useUsersPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).not.toBeNull();
    expect(result.current.error?.message).toBe('network problem');
  });
});
