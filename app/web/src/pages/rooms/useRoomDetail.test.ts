/**
 * T-20005: useRoomDetail Hook — TDD 测试套件
 *
 * 验收用例：
 *   H01: roomId='uuid-1' → loading=true 后 detail 填充
 *   H02: roomId=null → 不发 fetch，detail=null
 *   H03: roomId 从 'a' → 'b' → 旧 controller.abort 被调用
 *   H04: fetch 抛非 AbortError → error 非 null，loading=false
 *   H05: unmount → controller.abort 被调用
 *   H06: fetch 抛 AbortError → error 保持 null，loading 保持 true
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

// ── mock apiClient ─────────────────────────────────────────────────────────
vi.mock('../../core/network/apiClient', () => ({
  adminGetRoomDetail: vi.fn(),
}));

import { adminGetRoomDetail } from '../../core/network/apiClient';
import { useRoomDetail } from './useRoomDetail';
import type { AdminRoomDetail } from '../../core/network/apiClient';

const mockAdminGetRoomDetail = adminGetRoomDetail as ReturnType<typeof vi.fn>;

// ── 测试数据工厂 ───────────────────────────────────────────────────────────
function makeMockDetail(roomId = 'uuid-1'): AdminRoomDetail {
  return {
    room_id: roomId,
    title: `Room ${roomId}`,
    status: 'active',
    room_type: 'normal',
    member_count: 3,
    max_members: 20,
    owner: {
      user_id: 'user-1',
      nickname: 'TestOwner',
      avatar: null,
    },
    mic_slots: [],
    created_at: '2025-01-01T00:00:00Z',
    updated_at: '2025-01-01T00:00:00Z',
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

// ── H01: roomId='uuid-1' → loading=true 后 detail 填充 ───────────────────
describe('useRoomDetail — H01: 成功加载', () => {
  it('roomId="uuid-1" → loading=true 后 detail 被填充', async () => {
    const detail = makeMockDetail('uuid-1');
    mockAdminGetRoomDetail.mockResolvedValue(detail);

    const { result } = renderHook(() => useRoomDetail('uuid-1'));

    // 效果触发后，loading 应立即为 true
    expect(result.current.loading).toBe(true);

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(mockAdminGetRoomDetail).toHaveBeenCalledWith('uuid-1', expect.any(AbortSignal));
    expect(result.current.detail).not.toBeNull();
    expect(result.current.detail!.owner.nickname).toBe('TestOwner');
    expect(result.current.error).toBeNull();
  });
});

// ── H02: roomId=null → 不发 fetch，detail=null ────────────────────────────
describe('useRoomDetail — H02: roomId=null 不发请求', () => {
  it('roomId=null 时不调用 adminGetRoomDetail，detail=null，loading=false', () => {
    const { result } = renderHook(() => useRoomDetail(null));

    expect(result.current.loading).toBe(false);
    expect(result.current.detail).toBeNull();
    expect(result.current.error).toBeNull();
    expect(mockAdminGetRoomDetail).not.toHaveBeenCalled();
  });
});

// ── H03: roomId 从 'a' → 'b' → 旧 controller.abort 被调用 ─────────────────
describe('useRoomDetail — H03: roomId 变化时 abort 旧请求', () => {
  it('roomId 从 "a" 变为 "b" 时，旧 AbortController.abort 被调用', async () => {
    // 第一次调用：永不 resolve（模拟挂起请求）
    mockAdminGetRoomDetail.mockReturnValueOnce(new Promise(() => {}));
    // 第二次调用：正常 resolve
    mockAdminGetRoomDetail.mockResolvedValueOnce(makeMockDetail('b'));

    const abortSpy = vi.spyOn(AbortController.prototype, 'abort');

    const { rerender } = renderHook(
      ({ id }: { id: string | null }) => useRoomDetail(id),
      { initialProps: { id: 'a' as string | null } },
    );

    // loading 应为 true（fetch 挂起）
    expect(abortSpy).not.toHaveBeenCalled();

    // 切换 roomId → cleanup 触发 abort
    rerender({ id: 'b' });

    expect(abortSpy).toHaveBeenCalled();

    abortSpy.mockRestore();
  });
});

// ── H04: fetch 抛非 AbortError → error 非 null，loading=false ─────────────
describe('useRoomDetail — H04: 非 AbortError → error 非 null', () => {
  it('fetch 抛出 Network Error → error 非 null，loading=false，detail=null', async () => {
    mockAdminGetRoomDetail.mockRejectedValue(new Error('Network Error'));

    const { result } = renderHook(() => useRoomDetail('uuid-1'));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).not.toBeNull();
    expect(result.current.error!.message).toBe('Network Error');
    expect(result.current.detail).toBeNull();
  });
});

// ── H05: unmount → controller.abort 被调用 ────────────────────────────────
describe('useRoomDetail — H05: unmount 触发 cleanup', () => {
  it('unmount 时 AbortController.abort 被调用', () => {
    // 永不 resolve，模拟挂起
    mockAdminGetRoomDetail.mockReturnValue(new Promise(() => {}));

    const abortSpy = vi.spyOn(AbortController.prototype, 'abort');

    const { unmount } = renderHook(() => useRoomDetail('uuid-1'));
    unmount();

    expect(abortSpy).toHaveBeenCalled();

    abortSpy.mockRestore();
  });
});

// ── H06: fetch 抛 AbortError → error 保持 null ────────────────────────────
describe('useRoomDetail — H06: AbortError 静默忽略', () => {
  it('fetch 抛 AbortError → error 保持 null，loading 保持 true（不重置）', async () => {
    const abortErr = new DOMException('The user aborted a request.', 'AbortError');
    mockAdminGetRoomDetail.mockRejectedValue(abortErr);

    const { result } = renderHook(() => useRoomDetail('uuid-1'));

    // 推进微任务队列，等待 Promise reject 处理完成
    await act(async () => {
      await new Promise((r) => setTimeout(r, 0));
    });

    // AbortError 应静默忽略：error 保持 null，loading 保持 true
    expect(result.current.error).toBeNull();
    expect(result.current.loading).toBe(true);
  });
});
