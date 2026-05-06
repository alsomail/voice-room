/**
 * T-20004: useRoomsPage Hook — TDD 测试套件
 *
 * 验收用例：
 *   H01: 初始挂载 → 发起 adminGetRooms，items 填充
 *   H02: setFilters(status=active) → page 重置为 1，带 status 参数请求
 *   H03: setFilters(status=undefined) → page=1，不含 status 参数
 *   H04: setFilters(keyword=test) → 300ms 后带 keyword 参数请求，page=1
 *   H05: keyword 快速变化两次 → 只发一次请求（debounce）
 *   H06: setPage(2, 20) → 请求 page=2
 *   H07: closeRoom 成功 → adminCloseRoom 调用，refresh 触发
 *   H08: closeRoom 失败 → error 非 null，closingId=null，不刷新
 *   H09: adminGetRooms 失败 → error 非 null，items=[]
 *   H10: 卸载 → AbortController.abort 被调用
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

// ── mock apiClient ─────────────────────────────────────────────────────────
vi.mock('../../core/network/apiClient', () => ({
  adminGetRooms: vi.fn(),
  adminCloseRoom: vi.fn(),
}));

import { adminGetRooms, adminCloseRoom } from '../../core/network/apiClient';
import { useRoomsPage } from './useRoomsPage';
import type { AdminRoomItem, AdminRoomsData } from '../../core/network/apiClient';

const mockAdminGetRooms = adminGetRooms as ReturnType<typeof vi.fn>;
const mockAdminCloseRoom = adminCloseRoom as ReturnType<typeof vi.fn>;

// ── 测试数据工厂 ───────────────────────────────────────────────────────────
function makeRoom(id: number, status: 'active' | 'closed' = 'active'): AdminRoomItem {
  return {
    id: `room-${id}`,
    room_id: `room-${id}`,
    title: `Room ${id}`,
    room_type: 'normal',
    member_count: 5,
    max_members: 20,
    status,
    owner_id: `user-${id}`,
    owner_nickname: `Owner${id}`,
    owner_avatar: null,
    created_at: '2025-01-01T00:00:00Z',
  };
}

function makeRoomsData(count: number): AdminRoomsData {
  return {
    total: count,
    page: 1,
    page_size: 20,
    items: Array.from({ length: count }, (_, i) => makeRoom(i + 1)),
  };
}

const ROOMS_DATA = makeRoomsData(3);

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  vi.useRealTimers();
});

// ── H01: 初始挂载 ──────────────────────────────────────────────────────────
describe('useRoomsPage — H01: 初始挂载', () => {
  it('发起 adminGetRooms，items 被填充，loading=false', async () => {
    mockAdminGetRooms.mockResolvedValue(ROOMS_DATA);

    const { result } = renderHook(() => useRoomsPage());

    expect(result.current.loading).toBe(true);

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(mockAdminGetRooms).toHaveBeenCalled();
    expect(result.current.items).toHaveLength(3);
    expect(result.current.total).toBe(3);
    expect(result.current.error).toBeNull();
  });
});

// ── H02: setFilters(status=active) → page=1，带 status 请求 ───────────────
describe('useRoomsPage — H02: status 过滤', () => {
  it('setFilters({status:"active"}) → page 重置为 1，请求携带 status=active', async () => {
    mockAdminGetRooms.mockResolvedValue(ROOMS_DATA);

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    // 先把 page 改成 2
    act(() => {
      result.current.setPage(2, 20);
    });
    await waitFor(() => expect(result.current.page).toBe(2));

    mockAdminGetRooms.mockClear();

    act(() => {
      result.current.setFilters({ status: 'active' });
    });

    await waitFor(() => expect(mockAdminGetRooms).toHaveBeenCalled());

    // page 应该重置为 1
    expect(result.current.page).toBe(1);

    const lastCall = mockAdminGetRooms.mock.calls[mockAdminGetRooms.mock.calls.length - 1];
    expect(lastCall[0]).toMatchObject({ status: 'active', page: 1 });
  });
});

// ── H03: setFilters(status=undefined) → page=1，不含 status ──────────────
describe('useRoomsPage — H03: 清除 status 过滤', () => {
  it('setFilters({status:undefined}) → page=1，请求不含 status 参数', async () => {
    mockAdminGetRooms.mockResolvedValue(ROOMS_DATA);

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    // 先设置 status
    act(() => { result.current.setFilters({ status: 'active' }); });
    await waitFor(() => expect(result.current.filters.status).toBe('active'));

    mockAdminGetRooms.mockClear();

    act(() => { result.current.setFilters({ status: undefined }); });

    await waitFor(() => expect(mockAdminGetRooms).toHaveBeenCalled());

    expect(result.current.page).toBe(1);

    const lastCall = mockAdminGetRooms.mock.calls[mockAdminGetRooms.mock.calls.length - 1];
    expect(lastCall[0]).not.toHaveProperty('status');
  });
});

// ── H04: setFilters(keyword=test) → 300ms 后带 keyword 请求，page=1 ────────
describe('useRoomsPage — H04: keyword debounce', () => {
  it('设置 keyword 后 300ms 才发请求，且 page=1', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    mockAdminGetRooms.mockResolvedValue(ROOMS_DATA);

    const { result } = renderHook(() => useRoomsPage());

    // 推进初始 debounce（300ms）并等待首次 fetch 完成
    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });

    expect(result.current.loading).toBe(false);
    const callsBefore = mockAdminGetRooms.mock.calls.length;

    // 设置 keyword
    act(() => {
      result.current.setFilters({ keyword: 'test' });
    });

    // 未到 300ms，不发新请求
    act(() => { vi.advanceTimersByTime(100); });
    expect(mockAdminGetRooms.mock.calls.length).toBe(callsBefore);

    // 再过 200ms（共 300ms），触发 debounce
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });

    expect(mockAdminGetRooms.mock.calls.length).toBeGreaterThan(callsBefore);

    const lastCall = mockAdminGetRooms.mock.calls[mockAdminGetRooms.mock.calls.length - 1];
    expect(lastCall[0]).toMatchObject({ keyword: 'test', page: 1 });

    vi.useRealTimers();
  });
});

// ── H05: keyword 快速变化两次 → 只发一次请求（debounce） ─────────────────
describe('useRoomsPage — H05: debounce 消抖', () => {
  it('快速变化 keyword 两次，只发一次请求', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    mockAdminGetRooms.mockResolvedValue(ROOMS_DATA);

    const { result } = renderHook(() => useRoomsPage());

    await act(async () => { await vi.advanceTimersByTimeAsync(400); });

    const callsBefore = mockAdminGetRooms.mock.calls.length;

    act(() => { result.current.setFilters({ keyword: 'te' }); });
    act(() => { vi.advanceTimersByTime(100); });
    act(() => { result.current.setFilters({ keyword: 'test' }); });

    // 推进 300ms（从最后一次 keyword 变化起）
    await act(async () => { await vi.advanceTimersByTimeAsync(300); });

    // 只多出一次调用
    expect(mockAdminGetRooms.mock.calls.length).toBe(callsBefore + 1);

    const lastCall = mockAdminGetRooms.mock.calls[mockAdminGetRooms.mock.calls.length - 1];
    expect(lastCall[0]).toMatchObject({ keyword: 'test' });

    vi.useRealTimers();
  });
});

// ── H06: setPage(2, 20) → 请求 page=2 ────────────────────────────────────
describe('useRoomsPage — H06: 翻页', () => {
  it('setPage(2, 20) → 发起 page=2 的请求', async () => {
    mockAdminGetRooms.mockResolvedValue(ROOMS_DATA);

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    mockAdminGetRooms.mockClear();

    act(() => { result.current.setPage(2, 20); });

    await waitFor(() => expect(mockAdminGetRooms).toHaveBeenCalled());

    const lastCall = mockAdminGetRooms.mock.calls[0];
    expect(lastCall[0]).toMatchObject({ page: 2, page_size: 20 });
  });
});

// ── H07: closeRoom 成功 → adminCloseRoom 调用，refresh 触发 ──────────────
describe('useRoomsPage — H07: closeRoom 成功', () => {
  it('adminCloseRoom 被调用，列表刷新（adminGetRooms 再次调用）', async () => {
    mockAdminGetRooms.mockResolvedValue(ROOMS_DATA);
    mockAdminCloseRoom.mockResolvedValue(undefined);

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    const callsBefore = mockAdminGetRooms.mock.calls.length;

    await act(async () => {
      await result.current.closeRoom('room-1');
    });

    expect(mockAdminCloseRoom).toHaveBeenCalledWith('room-1');
    await waitFor(() =>
      expect(mockAdminGetRooms.mock.calls.length).toBeGreaterThan(callsBefore),
    );
    expect(result.current.closingId).toBeNull();
  });
});

// ── H08: closeRoom 失败 → error 非 null，closingId=null，不刷新 ──────────
describe('useRoomsPage — H08: closeRoom 失败', () => {
  it('error 非 null，closingId=null，adminGetRooms 不额外调用', async () => {
    mockAdminGetRooms.mockResolvedValue(ROOMS_DATA);
    mockAdminCloseRoom.mockRejectedValue(new Error('Close failed'));

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    const callsBefore = mockAdminGetRooms.mock.calls.length;

    await act(async () => {
      await result.current.closeRoom('room-1').catch(() => {}); // closeRoom now re-throws
    });

    expect(result.current.error).not.toBeNull();
    expect(result.current.closingId).toBeNull();
    expect(mockAdminGetRooms.mock.calls.length).toBe(callsBefore);
  });
});

// ── H09: adminGetRooms 失败 → error 非 null，items=[] ──────────────────
describe('useRoomsPage — H09: fetch 失败', () => {
  it('error 非 null，items=[]', async () => {
    mockAdminGetRooms.mockRejectedValue(new Error('Network Error'));

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).not.toBeNull();
    expect(result.current.items).toEqual([]);
  });
});

// ── H10: 卸载 → AbortController.abort 被调用 ─────────────────────────────
describe('useRoomsPage — H10: 卸载清理', () => {
  it('unmount 时 AbortController.abort 被调用', () => {
    mockAdminGetRooms.mockResolvedValue(ROOMS_DATA);

    const abortSpy = vi.spyOn(AbortController.prototype, 'abort');

    const { unmount } = renderHook(() => useRoomsPage());
    unmount();

    expect(abortSpy).toHaveBeenCalled();

    abortSpy.mockRestore();
  });
});

// ── H11: adminGetRooms 抛出非 Error 类型 → error.message 有值 ─────────────
describe('useRoomsPage — H11: fetch 抛出非 Error 类型（字符串）', () => {
  it('adminGetRooms 抛出字符串时 error.message 仍等于该字符串', async () => {
    mockAdminGetRooms.mockRejectedValue('network error');

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).not.toBeNull();
    expect(result.current.error?.message).toBe('network error');
  });
});

// ── H12: adminCloseRoom 抛出非 Error 类型 → error.message 不为 undefined ──
describe('useRoomsPage — H12: closeRoom 抛出非 Error 类型', () => {
  it('adminCloseRoom 抛出字符串时 error.message 不为 undefined', async () => {
    mockAdminGetRooms.mockResolvedValue(ROOMS_DATA);
    mockAdminCloseRoom.mockRejectedValue('close error');

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => {
      await result.current.closeRoom('room-1').catch(() => {}); // closeRoom now re-throws
    });

    expect(result.current.error).not.toBeNull();
    expect(result.current.error?.message).not.toBeUndefined();
    expect(result.current.error?.message).toBe('close error');
  });
});

// ── H13: debouncedKeyword 为空字符串时不含 keyword 字段 ────────────────────
describe('useRoomsPage — H13: debouncedKeyword 空字符串不传 keyword', () => {
  it('keyword 为空字符串时 adminGetRooms 调用参数不含 keyword 字段', async () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    mockAdminGetRooms.mockResolvedValue(ROOMS_DATA);

    const { result } = renderHook(() => useRoomsPage());

    // 推进初始 debounce
    await act(async () => { await vi.advanceTimersByTimeAsync(400); });

    // 先设 keyword='test'
    act(() => { result.current.setFilters({ keyword: 'test' }); });
    await act(async () => { await vi.advanceTimersByTimeAsync(400); });

    mockAdminGetRooms.mockClear();

    // 清空 keyword（空字符串）
    act(() => { result.current.setFilters({ keyword: '' }); });
    await act(async () => { await vi.advanceTimersByTimeAsync(400); });

    const lastCall = mockAdminGetRooms.mock.calls[mockAdminGetRooms.mock.calls.length - 1];
    expect(lastCall[0]).not.toHaveProperty('keyword');

    vi.useRealTimers();
  });
});

// ── H14: filters.status 为 undefined 时不含 status 字段（直接构造参数验证）─
describe('useRoomsPage — H14: filters.status 为 undefined 时不传 status', () => {
  it('初始挂载（无 status 过滤）adminGetRooms 调用参数不含 status 字段', async () => {
    mockAdminGetRooms.mockResolvedValue(ROOMS_DATA);

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    const firstCall = mockAdminGetRooms.mock.calls[0];
    expect(firstCall[0]).not.toHaveProperty('status');
  });
});

// ════════════════════════════════════════════════════════════════════════════
// T-20011: 活跃度筛选相关测试（H11–H17 in T-20011 context）
// ════════════════════════════════════════════════════════════════════════════

/** 构造不同活跃状态的测试房间（created_at 相对当前时间计算，确保时间有效） */
function makeActivityRoom(
  id: string,
  memberCount: number,
  status: 'active' | 'closed',
  minsAgo: number,
): AdminRoomItem {
  return {
    id,
    room_id: id,
    title: `Room ${id}`,
    room_type: 'normal',
    member_count: memberCount,
    max_members: 20,
    status,
    owner_id: `user-${id}`,
    owner_nickname: `Owner-${id}`,
    owner_avatar: null,
    created_at: new Date(Date.now() - minsAgo * 60 * 1000).toISOString(),
  };
}

// 混合活跃状态的测试数据
function makeMixedRoomsData(): AdminRoomsData {
  return {
    total: 4,
    page: 1,
    page_size: 20,
    items: [
      makeActivityRoom('act-1', 5, 'active', 30),    // active
      makeActivityRoom('abn-1', 0, 'active', 30),    // abnormal
      makeActivityRoom('qui-1', 2, 'active', 90),    // quiet (>1h)
      makeActivityRoom('nor-1', 3, 'active', 30),    // normal (<1h)
    ],
  };
}

// ── T20011-H11: 初始 activityFilter='all'，filteredItems 与 items 相同 ────────
describe('useRoomsPage — T20011-H11: 初始 activityFilter 为 all', () => {
  it('activityFilter 默认值为 "all"，filteredItems 长度等于 items 长度', async () => {
    mockAdminGetRooms.mockResolvedValue(makeMixedRoomsData());

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.activityFilter).toBe('all');
    expect(result.current.filteredItems).toHaveLength(result.current.items.length);
    expect(result.current.filteredItems).toHaveLength(4);
  });
});

// ── T20011-H12: setActivityFilter('active') → 只含 member_count≥5 的房间 ─────
describe('useRoomsPage — T20011-H12: 筛选 active', () => {
  it('setActivityFilter("active") 后 filteredItems 只含 member_count≥5 的房间', async () => {
    mockAdminGetRooms.mockResolvedValue(makeMixedRoomsData());

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.setActivityFilter('active');
    });

    expect(result.current.filteredItems).toHaveLength(1);
    expect(result.current.filteredItems[0].room_id).toBe('act-1');
    expect(result.current.filteredItems[0].member_count).toBeGreaterThanOrEqual(5);
  });
});

// ── T20011-H13: setActivityFilter('abnormal') → 只含 0人+active 的房间 ────────
describe('useRoomsPage — T20011-H13: 筛选 abnormal', () => {
  it('setActivityFilter("abnormal") 后 filteredItems 只含 0人且 active 的房间', async () => {
    mockAdminGetRooms.mockResolvedValue(makeMixedRoomsData());

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.setActivityFilter('abnormal');
    });

    expect(result.current.filteredItems).toHaveLength(1);
    expect(result.current.filteredItems[0].room_id).toBe('abn-1');
    expect(result.current.filteredItems[0].member_count).toBe(0);
    expect(result.current.filteredItems[0].status).toBe('active');
  });
});

// ── T20011-H14: setActivityFilter('quiet') → 只含冷清条件的房间 ───────────────
describe('useRoomsPage — T20011-H14: 筛选 quiet', () => {
  it('setActivityFilter("quiet") 后 filteredItems 只含 1-4人且>1h 的房间', async () => {
    mockAdminGetRooms.mockResolvedValue(makeMixedRoomsData());

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    act(() => {
      result.current.setActivityFilter('quiet');
    });

    expect(result.current.filteredItems).toHaveLength(1);
    expect(result.current.filteredItems[0].room_id).toBe('qui-1');
    expect(result.current.filteredItems[0].member_count).toBe(2);
  });
});

// ── T20011-H15: 连续切换 all→active，filteredItems 正确变化 ───────────────────
describe('useRoomsPage — T20011-H15: 连续切换 activityFilter', () => {
  it('all→active 切换后 filteredItems 正确变化', async () => {
    mockAdminGetRooms.mockResolvedValue(makeMixedRoomsData());

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    // 初始 all → 4条
    expect(result.current.filteredItems).toHaveLength(4);

    // 切换为 active → 1条
    act(() => { result.current.setActivityFilter('active'); });
    expect(result.current.filteredItems).toHaveLength(1);

    // 切回 all → 4条
    act(() => { result.current.setActivityFilter('all'); });
    expect(result.current.filteredItems).toHaveLength(4);
  });
});

// ── T20011-H16: API 数据刷新后 filteredItems 随之重新计算 ─────────────────────
describe('useRoomsPage — T20011-H16: API 刷新后 filteredItems 重新计算', () => {
  it('items 更新后 filteredItems 依据新 items 重新过滤', async () => {
    // 第一次：返回 4 条混合数据
    mockAdminGetRooms.mockResolvedValueOnce(makeMixedRoomsData());
    // 第二次（refresh）：只返回 1 条 active 房间
    mockAdminGetRooms.mockResolvedValueOnce({
      total: 1,
      page: 1,
      page_size: 20,
      items: [makeActivityRoom('act-1', 5, 'active', 30)],
    });

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    // 设置 activityFilter='active' → 此时 filteredItems=1
    act(() => { result.current.setActivityFilter('active'); });
    expect(result.current.filteredItems).toHaveLength(1);

    // 刷新 → API 返回新数据（只有1条active）→ filteredItems 仍为 1
    act(() => { result.current.refresh(); });
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.filteredItems).toHaveLength(1);
    expect(result.current.items).toHaveLength(1);
  });
});

// ── T20011-H17: setActivityFilter 不触发 adminGetRooms 重新请求 ───────────────
describe('useRoomsPage — T20011-H17: 切换 activityFilter 不触发 API', () => {
  it('setActivityFilter 不增加 adminGetRooms 的调用次数', async () => {
    mockAdminGetRooms.mockResolvedValue(makeMixedRoomsData());

    const { result } = renderHook(() => useRoomsPage());
    await waitFor(() => expect(result.current.loading).toBe(false));

    const callsBeforeFilter = mockAdminGetRooms.mock.calls.length;

    // 多次切换 activityFilter
    act(() => { result.current.setActivityFilter('active'); });
    act(() => { result.current.setActivityFilter('abnormal'); });
    act(() => { result.current.setActivityFilter('all'); });

    // 等待任何可能的异步 effect 触发（应该没有）
    await new Promise((resolve) => setTimeout(resolve, 50));

    // adminGetRooms 调用次数不变
    expect(mockAdminGetRooms.mock.calls.length).toBe(callsBeforeFilter);
  });
});
