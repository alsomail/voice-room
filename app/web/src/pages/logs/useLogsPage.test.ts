/**
 * T-20009: useLogsPage Hook — TDD 测试套件
 *
 * 验收用例：
 *   LL-01: 初次加载 → 调用 adminGetLogs，items 填充，loading=false
 *   LL-02: setFilters({adminId: 'uuid-xxx', action: 'ban_user'}) → page 重置为 1，携带参数
 *   LL-03: setPage(2, 20) → 发起 page=2 的请求
 *   LL-04: 从 URL 读取初始参数（action/page）
 *
 * 扩展用例：
 *   LL-05: refresh() 重新发起请求
 *   LL-06: adminGetLogs 失败 → error 非 null，items=[]
 *   LL-07: 卸载 → AbortController.abort 被调用
 *   LL-08: setFilters 变化时 page 自动重置为 1
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

// ── mock react-router-dom（避免 Router context 依赖）─────────────────────────
vi.mock('react-router-dom', () => ({
  useSearchParams: vi.fn(() => [new URLSearchParams(), vi.fn()]),
}));

// ── mock apiClient ─────────────────────────────────────────────────────────
vi.mock('../../core/network/apiClient', () => ({
  adminGetLogs: vi.fn(),
}));

import { useSearchParams } from 'react-router-dom';
import { adminGetLogs } from '../../core/network/apiClient';
import { useLogsPage } from './useLogsPage';
import type { AdminLogItem, AdminLogsData } from '../../core/network/apiClient';

const mockAdminGetLogs = adminGetLogs as ReturnType<typeof vi.fn>;
const mockUseSearchParams = useSearchParams as ReturnType<typeof vi.fn>;

// ── 测试数据工厂 ───────────────────────────────────────────────────────────
function makeLogItem(id: number): AdminLogItem {
  return {
    id: `log-${id}`,
    admin_id: `admin-${id}`,
    action: 'ban_user',
    target_type: 'user',
    target_id: `target-${id}`,
    ip_address: `192.168.1.${id}`,
    detail: { reason: 'test' },
    created_at: '2025-01-01T00:00:00Z',
  };
}

function makeLogsData(count: number): AdminLogsData {
  return {
    total: count,
    page: 1,
    size: 20,
    items: Array.from({ length: count }, (_, i) => makeLogItem(i + 1)),
  };
}

const LOGS_DATA = makeLogsData(3);

beforeEach(() => {
  vi.clearAllMocks();
  mockUseSearchParams.mockReturnValue([new URLSearchParams(), vi.fn()]);
  mockAdminGetLogs.mockResolvedValue(LOGS_DATA);
});

// ── LL-01: 初始加载 ────────────────────────────────────────────────────────
describe('useLogsPage — LL-01: 初始加载', () => {
  it('挂载时调用 adminGetLogs，loading → false，items 填充', async () => {
    const { result } = renderHook(() => useLogsPage());

    expect(result.current.loading).toBe(true);

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(mockAdminGetLogs).toHaveBeenCalled();
    expect(result.current.items).toHaveLength(3);
    expect(result.current.total).toBe(3);
    expect(result.current.error).toBeNull();
  });

  it('初始请求携带 page=1 和 size=20', async () => {
    renderHook(() => useLogsPage());

    await waitFor(() => expect(mockAdminGetLogs).toHaveBeenCalled());

    const firstCall = mockAdminGetLogs.mock.calls[0];
    expect(firstCall[0]).toMatchObject({ page: 1, size: 20 });
  });
});

// ── LL-02: 过滤条件变化时重新请求 ─────────────────────────────────────────
describe('useLogsPage — LL-02: setFilters 重置 page=1', () => {
  it('setFilters({adminId, action}) → page 重置为 1，携带对应参数', async () => {
    const { result } = renderHook(() => useLogsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    // 先翻到第 2 页
    act(() => { result.current.setPage(2, 20); });
    await waitFor(() => expect(result.current.page).toBe(2));

    mockAdminGetLogs.mockClear();

    act(() => {
      result.current.setFilters({ adminId: 'uuid-xxx', action: 'ban_user' });
    });

    await waitFor(() => expect(mockAdminGetLogs).toHaveBeenCalled());

    expect(result.current.page).toBe(1);
    const lastCall = mockAdminGetLogs.mock.calls[mockAdminGetLogs.mock.calls.length - 1];
    expect(lastCall[0]).toMatchObject({ admin_id: 'uuid-xxx', action: 'ban_user', page: 1 });
  });
});

// ── LL-03: 分页切换 ────────────────────────────────────────────────────────
describe('useLogsPage — LL-03: 翻页', () => {
  it('setPage(2, 20) → 发起 page=2 的请求', async () => {
    const { result } = renderHook(() => useLogsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    mockAdminGetLogs.mockClear();

    act(() => { result.current.setPage(2, 20); });

    await waitFor(() => expect(mockAdminGetLogs).toHaveBeenCalled());

    const lastCall = mockAdminGetLogs.mock.calls[0];
    expect(lastCall[0]).toMatchObject({ page: 2, size: 20 });
  });
});

// ── LL-04: 从 URL 读取初始参数 ────────────────────────────────────────────
describe('useLogsPage — LL-04: 从 URL 初始化参数', () => {
  it('URL 含 action=ban_user&page=2 时，hook 以对应参数发起请求', async () => {
    mockUseSearchParams.mockReturnValue([
      new URLSearchParams('action=ban_user&page=2'),
      vi.fn(),
    ]);

    const { result } = renderHook(() => useLogsPage());

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.filters.action).toBe('ban_user');
    expect(result.current.page).toBe(2);

    const lastCall = mockAdminGetLogs.mock.calls[mockAdminGetLogs.mock.calls.length - 1];
    expect(lastCall[0]).toMatchObject({ action: 'ban_user', page: 2 });
  });

  it('URL 含 admin_id=uuid-yyy 时，filters.adminId 被设置', async () => {
    mockUseSearchParams.mockReturnValue([
      new URLSearchParams('admin_id=uuid-yyy'),
      vi.fn(),
    ]);

    const { result } = renderHook(() => useLogsPage());

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.filters.adminId).toBe('uuid-yyy');
  });
});

// ── LL-05: refresh() → 重新发起请求 ───────────────────────────────────────
describe('useLogsPage — LL-05: 刷新', () => {
  it('refresh() 以当前 filters 重新发起请求', async () => {
    const { result } = renderHook(() => useLogsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    const callsBefore = mockAdminGetLogs.mock.calls.length;

    act(() => { result.current.refresh(); });

    await waitFor(() =>
      expect(mockAdminGetLogs.mock.calls.length).toBeGreaterThan(callsBefore),
    );
  });
});

// ── LL-06: API 失败 → error 非 null，items=[] ─────────────────────────────
describe('useLogsPage — LL-06: API 失败', () => {
  it('adminGetLogs 失败 → error 非 null，items=[]', async () => {
    mockAdminGetLogs.mockRejectedValue(new Error('Network Error'));

    const { result } = renderHook(() => useLogsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).not.toBeNull();
    expect(result.current.items).toEqual([]);
  });
});

// ── LL-07: 卸载 → AbortController.abort 被调用 ────────────────────────────
describe('useLogsPage — LL-07: 卸载清理', () => {
  it('unmount 时 AbortController.abort 被调用', () => {
    const abortSpy = vi.spyOn(AbortController.prototype, 'abort');

    const { unmount } = renderHook(() => useLogsPage());
    unmount();

    expect(abortSpy).toHaveBeenCalled();

    abortSpy.mockRestore();
  });
});

// ── LL-08: setFilters 变化时 page 自动重置为 1 ────────────────────────────
describe('useLogsPage — LL-08: setFilters 重置 page=1', () => {
  it('处于第 3 页时 setFilters → page 自动重置为 1', async () => {
    const { result } = renderHook(() => useLogsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => { result.current.setPage(3, 20); });
    await waitFor(() => expect(result.current.page).toBe(3));

    act(() => { result.current.setFilters({ action: 'close_room' }); });

    await waitFor(() => expect(result.current.page).toBe(1));
  });
});
