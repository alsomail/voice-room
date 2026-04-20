/**
 * T-20007: useUserDetail Hook — TDD 测试套件
 *
 * 验收用例：
 *   H01: 传入有效 userId → loading=true，请求完成后 loading=false, detail 为数据
 *   H02: detail.status 为 'normal'/'banned'；detail.coin_balance 为数字；recharge_records 为空数组
 *   H03: fetch 抛出错误 → loading=false, error 不为 null, detail=null
 *   H04: API 返回 404 → error.message 非空，detail=null
 *   H05: userId=null → 不发请求，loading=false, detail=null
 *   H06: userId 快速切换 → 旧请求被 abort，detail 为新 userId 的数据
 *   H07: userId 变为 null → detail 重置为 null，error 重置为 null
 *   H08: 组件卸载时 fetch 被 abort，不触发 setState
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

// ── mock apiClient ─────────────────────────────────────────────────────────
vi.mock('../../core/network/apiClient', () => ({
  adminGetUserDetail: vi.fn(),
}));

import { adminGetUserDetail } from '../../core/network/apiClient';
import { useUserDetail } from './useUserDetail';
import type { AdminUserDetailResponse } from '../../core/network/apiClient';

const mockAdminGetUserDetail = adminGetUserDetail as ReturnType<typeof vi.fn>;

// ── 测试数据工厂 ───────────────────────────────────────────────────────────
function makeMockDetail(
  userId = 'user-uuid-1',
  status: 'normal' | 'banned' = 'normal',
): AdminUserDetailResponse {
  return {
    id: userId,
    phone: '+8613800138000',
    nickname: 'TestUser',
    avatar_url: 'https://cdn.example.com/avatar.jpg',
    coin_balance: 1000,
    vip_level: 1,
    status,
    created_at: '2024-01-01T00:00:00Z',
    recharge_records: [],
    consume_records: [],
    devices: [],
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

// ── H01: 传入有效 userId → loading=true，完成后 detail 填充 ────────────────
describe('useUserDetail — H01: 成功加载', () => {
  it('userId="user-1" → loading=true 后 detail 被填充，loading=false', async () => {
    const detail = makeMockDetail('user-1');
    mockAdminGetUserDetail.mockResolvedValue(detail);

    const { result } = renderHook(() => useUserDetail('user-1'));

    // 立即应为 loading=true
    expect(result.current.loading).toBe(true);

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(mockAdminGetUserDetail).toHaveBeenCalledWith(
      'user-1',
      expect.any(AbortSignal),
    );
    expect(result.current.detail).not.toBeNull();
    expect(result.current.detail!.nickname).toBe('TestUser');
    expect(result.current.error).toBeNull();
  });
});

// ── H02: detail 字段类型校验 ──────────────────────────────────────────────
describe('useUserDetail — H02: detail 字段类型', () => {
  it('detail.status 为 "normal"，coin_balance 为数字，recharge_records 为数组', async () => {
    mockAdminGetUserDetail.mockResolvedValue(makeMockDetail('u1', 'normal'));
    const { result } = renderHook(() => useUserDetail('u1'));
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.detail!.status).toBe('normal');
    expect(typeof result.current.detail!.coin_balance).toBe('number');
    expect(Array.isArray(result.current.detail!.recharge_records)).toBe(true);
  });

  it('detail.status 可以为 "banned"', async () => {
    mockAdminGetUserDetail.mockResolvedValue(makeMockDetail('u2', 'banned'));
    const { result } = renderHook(() => useUserDetail('u2'));
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.detail!.status).toBe('banned');
  });
});

// ── H03: fetch 抛出错误 ────────────────────────────────────────────────────
describe('useUserDetail — H03: 请求失败', () => {
  it('fetch 抛出错误 → loading=false, error 非 null, detail=null', async () => {
    mockAdminGetUserDetail.mockRejectedValue(new Error('Network Error'));
    const { result } = renderHook(() => useUserDetail('user-err'));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).not.toBeNull();
    expect(result.current.error!.message).toBe('Network Error');
    expect(result.current.detail).toBeNull();
  });
});

// ── H04: API 返回 404 ─────────────────────────────────────────────────────
describe('useUserDetail — H04: API 404', () => {
  it('API 返回 404 → error.message 非空，detail=null', async () => {
    mockAdminGetUserDetail.mockRejectedValue(new Error('User not found'));
    const { result } = renderHook(() => useUserDetail('nonexistent'));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).not.toBeNull();
    expect(result.current.error!.message).toBeTruthy();
    expect(result.current.detail).toBeNull();
  });
});

// ── H05: userId=null → 不发请求 ──────────────────────────────────────────
describe('useUserDetail — H05: userId=null', () => {
  it('userId=null 时不调用 adminGetUserDetail，detail=null，loading=false', () => {
    const { result } = renderHook(() => useUserDetail(null));

    expect(result.current.loading).toBe(false);
    expect(result.current.detail).toBeNull();
    expect(result.current.error).toBeNull();
    expect(mockAdminGetUserDetail).not.toHaveBeenCalled();
  });
});

// ── H06: userId 快速切换 → 旧请求被 abort ────────────────────────────────
describe('useUserDetail — H06: userId 快速切换', () => {
  it('userId 从 "a" 变为 "b" 时旧 AbortController.abort 被调用，detail 为 "b" 的数据', async () => {
    // 第一次调用：永不 resolve（模拟挂起请求）
    mockAdminGetUserDetail.mockReturnValueOnce(new Promise(() => {}));
    // 第二次调用：正常 resolve
    mockAdminGetUserDetail.mockResolvedValueOnce(makeMockDetail('b'));

    const abortSpy = vi.spyOn(AbortController.prototype, 'abort');

    const { rerender, result } = renderHook(
      ({ id }: { id: string | null }) => useUserDetail(id),
      { initialProps: { id: 'a' as string | null } },
    );

    // 初始请求挂起，尚未 abort
    expect(abortSpy).not.toHaveBeenCalled();

    act(() => {
      rerender({ id: 'b' });
    });

    // 旧请求的 abort 被调用
    expect(abortSpy).toHaveBeenCalledTimes(1);

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.detail!.id).toBe('b');

    abortSpy.mockRestore();
  });
});

// ── H07: userId 变为 null → detail/error 重置 ─────────────────────────────
describe('useUserDetail — H07: userId 变为 null', () => {
  it('userId 从 "user-1" 变为 null → detail 和 error 重置为 null', async () => {
    mockAdminGetUserDetail.mockResolvedValue(makeMockDetail('user-1'));

    const { rerender, result } = renderHook(
      ({ id }: { id: string | null }) => useUserDetail(id),
      { initialProps: { id: 'user-1' as string | null } },
    );

    await waitFor(() => expect(result.current.detail).not.toBeNull());

    act(() => {
      rerender({ id: null });
    });

    expect(result.current.detail).toBeNull();
    expect(result.current.error).toBeNull();
    expect(result.current.loading).toBe(false);
  });
});

// ── H08: 组件卸载时 fetch 被 abort ────────────────────────────────────────
describe('useUserDetail — H08: 组件卸载', () => {
  it('unmount → AbortController.abort 被调用', () => {
    // 永不 resolve，保持挂起状态
    mockAdminGetUserDetail.mockReturnValue(new Promise(() => {}));

    const abortSpy = vi.spyOn(AbortController.prototype, 'abort');

    const { unmount } = renderHook(() => useUserDetail('user-1'));

    expect(abortSpy).not.toHaveBeenCalled();

    unmount();

    expect(abortSpy).toHaveBeenCalledTimes(1);

    abortSpy.mockRestore();
  });
});
