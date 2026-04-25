/**
 * T-20003: useDashboardStats Hook — TDD 测试套件
 *
 * 验收用例：
 *   U01: 三个请求全部成功 → stats 有值，loading=false
 *   U02: adminGetStatsOverview 成功 → onlineUsers/dau/newUsersToday/trend 正确
 *   U03: adminGetStatsOverview 失败 → onlineUsers=null, trend=[], error=null（降级）
 *   U04: 三个请求全部失败 → error 非 null，loading=false
 *   U05: 调用 refresh() → loading 先 true，请求重发，loading 后 false
 *   U06: 推进 30s 定时器 → 请求被第二次调用
 *   U07: 组件卸载 → clearInterval 被调用
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

// ── mock apiClient ────────────────────────────────────────────────────────────
vi.mock('../../core/network/apiClient', () => ({
  adminGetRooms: vi.fn(),
  adminGetStatsOverview: vi.fn(),
}));

import { adminGetRooms, adminGetStatsOverview } from '../../core/network/apiClient';
import { useDashboardStats } from './useDashboardStats';

const mockAdminGetRooms = adminGetRooms as ReturnType<typeof vi.fn>;
const mockAdminGetStatsOverview = adminGetStatsOverview as ReturnType<typeof vi.fn>;

// ── 测试数据 ──────────────────────────────────────────────────────────────────
const ROOMS_ALL = { total: 50, page: 1, page_size: 20, items: [] };
const ROOMS_ACTIVE = { total: 12, page: 1, page_size: 20, items: [] };
const STATS_OVERVIEW = {
  online_users: 340,
  dau: 1200,
  new_users_today: 88,
  trend: [
    { date: '2025-05-10', dau: 1100, new_users: 80 },
    { date: '2025-05-11', dau: 1200, new_users: 88 },
  ],
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.restoreAllMocks();
});

afterEach(() => {
  vi.restoreAllMocks();
});

// ── U01: 三个请求全部成功 ──────────────────────────────────────────────────────
describe('useDashboardStats — U01: 全部成功', () => {
  it('stats 各字段正确，loading=false', async () => {
    mockAdminGetRooms
      .mockResolvedValueOnce(ROOMS_ALL)
      .mockResolvedValueOnce(ROOMS_ACTIVE);
    mockAdminGetStatsOverview.mockResolvedValueOnce(STATS_OVERVIEW);

    const { result } = renderHook(() => useDashboardStats());

    // 初始 loading=true
    expect(result.current.loading).toBe(true);

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.stats.totalRooms).toBe(50);
    expect(result.current.stats.activeRooms).toBe(12);
    expect(result.current.stats.onlineUsers).toBe(340);
    expect(result.current.stats.dau).toBe(1200);
    expect(result.current.stats.newUsersToday).toBe(88);
    expect(result.current.error).toBeNull();
  });
});

// ── U02: adminGetStatsOverview 成功 → trend 非空 ──────────────────────────────
describe('useDashboardStats — U02: stats overview 成功', () => {
  it('trend 为非空数组', async () => {
    mockAdminGetRooms
      .mockResolvedValueOnce(ROOMS_ALL)
      .mockResolvedValueOnce(ROOMS_ACTIVE);
    mockAdminGetStatsOverview.mockResolvedValueOnce(STATS_OVERVIEW);

    const { result } = renderHook(() => useDashboardStats());
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.stats.trend).toHaveLength(2);
    expect(result.current.stats.trend[0]).toMatchObject({ date: '2025-05-10', dau: 1100 });
  });
});

// ── U03: adminGetStatsOverview 失败 → 降级，error=null ───────────────────────
describe('useDashboardStats — U03: stats overview 失败，降级', () => {
  it('onlineUsers=null, trend=[], error=null（rooms 请求仍成功）', async () => {
    mockAdminGetRooms
      .mockResolvedValueOnce(ROOMS_ALL)
      .mockResolvedValueOnce(ROOMS_ACTIVE);
    mockAdminGetStatsOverview.mockRejectedValueOnce(new Error('Network Error'));

    const { result } = renderHook(() => useDashboardStats());
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.stats.onlineUsers).toBeNull();
    expect(result.current.stats.dau).toBeNull();
    expect(result.current.stats.newUsersToday).toBeNull();
    expect(result.current.stats.trend).toEqual([]);
    // rooms 请求成功，error 应为 null
    expect(result.current.error).toBeNull();
    // rooms 数据仍然正确
    expect(result.current.stats.totalRooms).toBe(50);
    expect(result.current.stats.activeRooms).toBe(12);
  });
});

// ── U04: 三个请求全部失败 → error 非 null ─────────────────────────────────────
describe('useDashboardStats — U04: 全部失败', () => {
  it('error 非 null，loading=false', async () => {
    mockAdminGetRooms.mockRejectedValue(new Error('Server Down'));
    mockAdminGetStatsOverview.mockRejectedValueOnce(new Error('Server Down'));

    const { result } = renderHook(() => useDashboardStats());
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).not.toBeNull();
    expect(result.current.stats.totalRooms).toBeNull();
    expect(result.current.stats.activeRooms).toBeNull();
  });
});

// ── U05: 调用 refresh() ────────────────────────────────────────────────────────
describe('useDashboardStats — U05: refresh()', () => {
  it('refresh() 触发 loading=true 然后 false，API 被再次调用', async () => {
    mockAdminGetRooms.mockResolvedValue(ROOMS_ALL);
    mockAdminGetStatsOverview.mockResolvedValue(STATS_OVERVIEW);

    const { result } = renderHook(() => useDashboardStats());
    await waitFor(() => expect(result.current.loading).toBe(false));

    const callsBefore = mockAdminGetRooms.mock.calls.length;

    act(() => {
      result.current.refresh();
    });

    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(mockAdminGetRooms.mock.calls.length).toBeGreaterThan(callsBefore);
  });
});

// ── U06: 30s 定时器（fake timers + advanceTimersByTimeAsync）─────────────────
describe('useDashboardStats — U06: 30s 自动刷新', () => {
  it('推进 30s 后 API 被第二次调用', async () => {
    // 只 fake setInterval/clearInterval，保留 setTimeout 供 waitFor 使用
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });

    mockAdminGetRooms.mockResolvedValue(ROOMS_ALL);
    mockAdminGetStatsOverview.mockResolvedValue(STATS_OVERVIEW);

    renderHook(() => useDashboardStats());

    // 等待初始请求完成（setTimeout 是真实的，waitFor 正常工作）
    await waitFor(() => expect(mockAdminGetRooms).toHaveBeenCalledTimes(2));

    const callsBefore = mockAdminGetRooms.mock.calls.length; // = 2

    // 推进 30s：触发 fake setInterval 的回调
    act(() => {
      vi.advanceTimersByTime(30_000);
    });

    // waitFor 等待 interval 触发的异步 fetch 完成
    await waitFor(() => {
      expect(mockAdminGetRooms.mock.calls.length).toBeGreaterThan(callsBefore);
    });

    vi.useRealTimers();
  });
});

// ── U08: 缺陷 #6 回归 — refresh() 取消上一次飞行中请求 ──────────────────────
describe('useDashboardStats — U08: refresh() 取消旧请求（缺陷 #6）', () => {
  it('连续调用 refresh() 应 abort 上一次飞行中请求', async () => {
    mockAdminGetRooms.mockResolvedValue(ROOMS_ALL);
    mockAdminGetStatsOverview.mockResolvedValue(STATS_OVERVIEW);

    const { result } = renderHook(() => useDashboardStats());
    await waitFor(() => expect(result.current.loading).toBe(false));

    // 监听后续创建的 AbortController 的 abort 调用
    const abortSpy = vi.spyOn(AbortController.prototype, 'abort');
    abortSpy.mockClear();

    // 第一次手动 refresh：建立新 controller
    act(() => {
      result.current.refresh();
    });
    // 第二次手动 refresh：必须 abort 第一次的 controller
    act(() => {
      result.current.refresh();
    });

    // 至少有一次 abort 来自第二次 refresh 取消第一次（不计 effect cleanup）
    expect(abortSpy).toHaveBeenCalled();
    abortSpy.mockRestore();
  });

  it('refresh() 调用时传入 AbortSignal 给 API（不再裸调用）', async () => {
    mockAdminGetRooms.mockResolvedValue(ROOMS_ALL);
    mockAdminGetStatsOverview.mockResolvedValue(STATS_OVERVIEW);

    const { result } = renderHook(() => useDashboardStats());
    await waitFor(() => expect(result.current.loading).toBe(false));
    mockAdminGetRooms.mockClear();
    mockAdminGetStatsOverview.mockClear();

    act(() => {
      result.current.refresh();
    });
    await waitFor(() => expect(result.current.loading).toBe(false));

    // refresh 触发的请求第二个参数应为 AbortSignal（不再是 undefined）
    const lastCall =
      mockAdminGetRooms.mock.calls[mockAdminGetRooms.mock.calls.length - 1];
    expect(lastCall?.[1]).toBeInstanceOf(AbortSignal);
  });
});
describe('useDashboardStats — U07: 卸载清理', () => {
  it('unmount 时 clearInterval 和 AbortController.abort 均被调用', () => {
    // clearInterval 在 useEffect cleanup 中同步执行，无需 async
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });

    const clearIntervalSpy = vi.spyOn(globalThis, 'clearInterval');
    // 监听 AbortController.prototype.abort 验证卸载时取消飞行中请求
    const abortSpy = vi.spyOn(AbortController.prototype, 'abort');

    mockAdminGetRooms.mockResolvedValue(ROOMS_ALL);
    mockAdminGetStatsOverview.mockResolvedValue(STATS_OVERVIEW);

    const { unmount } = renderHook(() => useDashboardStats());

    unmount();

    expect(clearIntervalSpy).toHaveBeenCalled();
    expect(abortSpy).toHaveBeenCalled();

    abortSpy.mockRestore();
    clearIntervalSpy.mockRestore();
    vi.useRealTimers();
  });
});
