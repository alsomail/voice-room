/**
 * T-20008: useBanUser Hook 测试
 *
 * 验收用例：
 *   B01: 调用 ban() 成功，loading 由 true → false，error=null
 *   B02: API 抛出错误，loading=false，error 非空
 *   B03: 409（已封禁），错误被正确捕获并设置 error
 *   B04: ban() 执行中，loading=true（防止重复提交）
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';

// ── apiClient mock ─────────────────────────────────────────────────────────
vi.mock('../../core/network/apiClient', () => ({
  adminBanUser: vi.fn(),
}));

import { adminBanUser } from '../../core/network/apiClient';
import { useBanUser } from './useBanUser';

const mockAdminBanUser = adminBanUser as ReturnType<typeof vi.fn>;

beforeEach(() => {
  vi.clearAllMocks();
});

// ── B01: 成功 ──────────────────────────────────────────────────────────────
describe('useBanUser — B01: 成功', () => {
  it('ban() 成功后 loading=false，error=null，adminBanUser 以正确参数调用', async () => {
    mockAdminBanUser.mockResolvedValue(undefined);
    const { result } = renderHook(() => useBanUser());

    await act(async () => {
      await result.current.ban('user-1', {
        action: 'ban',
        ban_type: 'temporary', duration_hours: 24,
        reason: '违规内容',
      });
    });

    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
    expect(mockAdminBanUser).toHaveBeenCalledWith('user-1', {
      action: 'ban',
      ban_type: 'temporary', duration_hours: 24,
      reason: '违规内容',
    });
  });
});

// ── B02: API 错误 ──────────────────────────────────────────────────────────
describe('useBanUser — B02: API 错误', () => {
  it('API 抛出错误后 loading=false，error 非空，错误信息正确', async () => {
    mockAdminBanUser.mockRejectedValue(new Error('Server Error'));
    const { result } = renderHook(() => useBanUser());

    await act(async () => {
      await result.current
        .ban('user-1', { action: 'ban', ban_type: 'temporary', duration_hours: 24, reason: '违规内容' })
        .catch(() => {});
    });

    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeInstanceOf(Error);
    expect(result.current.error?.message).toBe('Server Error');
  });
});

// ── B03: 409 错误 ──────────────────────────────────────────────────────────
describe('useBanUser — B03: 409 错误捕获', () => {
  it('409（已封禁）时 error 对象与抛出的 Error 一致', async () => {
    const conflictError = new Error('Already banned');
    mockAdminBanUser.mockRejectedValue(conflictError);
    const { result } = renderHook(() => useBanUser());

    await act(async () => {
      await result.current
        .ban('user-1', { action: 'ban', ban_type: 'permanent' })
        .catch(() => {});
    });

    expect(result.current.error).toBe(conflictError);
  });
});

// ── B04: loading=true 期间 ────────────────────────────────────────────────
describe('useBanUser — B04: loading=true 期间', () => {
  it('ban() 执行中 loading=true，完成后 loading=false', async () => {
    let resolvePromise!: () => void;
    const pending = new Promise<void>((resolve) => {
      resolvePromise = resolve;
    });
    mockAdminBanUser.mockReturnValue(pending);

    const { result } = renderHook(() => useBanUser());

    act(() => {
      void result.current.ban('user-1', {
        action: 'ban',
        ban_type: 'temporary', duration_hours: 24,
        reason: '违规内容',
      });
    });

    // 执行中 loading 应为 true
    expect(result.current.loading).toBe(true);

    await act(async () => {
      resolvePromise();
    });

    expect(result.current.loading).toBe(false);
  });
});
