/**
 * T-20010: useUnbanUser Hook 测试
 *
 * 验收用例：
 *   U01: 调用 unban() 成功，loading 经历 true → false，error=null
 *   U02: API 抛出错误，loading=false，error 非空，且 error 被 re-throw
 *   U03: 409/40900（已是正常状态），错误被正确捕获，error.message 非空
 *   U04: unban() 执行中 loading=true（防重复提交验证）
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';

// ── apiClient mock ─────────────────────────────────────────────────────────
vi.mock('../../core/network/apiClient', () => ({
  adminUnbanUser: vi.fn(),
}));

import { adminUnbanUser } from '../../core/network/apiClient';
import { useUnbanUser } from './useUnbanUser';

const mockAdminUnbanUser = adminUnbanUser as ReturnType<typeof vi.fn>;

beforeEach(() => {
  vi.clearAllMocks();
});

// ── U01: 成功 ──────────────────────────────────────────────────────────────
describe('useUnbanUser — U01: 成功', () => {
  it('unban() 成功后 loading=false，error=null，adminUnbanUser 以正确参数调用', async () => {
    mockAdminUnbanUser.mockResolvedValue(undefined);
    const { result } = renderHook(() => useUnbanUser());

    await act(async () => {
      await result.current.unban('user-1', {
        reason: '处罚到期: 备注',
      });
    });

    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
    expect(mockAdminUnbanUser).toHaveBeenCalledWith('user-1', {
      reason: '处罚到期: 备注',
    });
  });
});

// ── U02: API 错误 ──────────────────────────────────────────────────────────
describe('useUnbanUser — U02: API 错误', () => {
  it('API 抛出错误后 loading=false，error 非空，且错误被 re-throw', async () => {
    mockAdminUnbanUser.mockRejectedValue(new Error('Server Error'));
    const { result } = renderHook(() => useUnbanUser());

    let thrown: Error | null = null;
    await act(async () => {
      await result.current.unban('user-1', { reason: '处罚到期' }).catch((e: Error) => {
        thrown = e;
      });
    });

    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeInstanceOf(Error);
    expect(result.current.error?.message).toBe('Server Error');
    expect(thrown).toBeInstanceOf(Error);
    expect((thrown as unknown as Error).message).toBe('Server Error');
  });
});

// ── U03: 40900 错误 ────────────────────────────────────────────────────────
describe('useUnbanUser — U03: 40900 错误捕获（与 admin-server UserAlreadyNormal 对齐）', () => {
  it('40900（已是正常状态）时 error.message 非空，error 对象被正确设置', async () => {
    const conflictError = new Error('[40900] 用户当前未被封禁');
    mockAdminUnbanUser.mockRejectedValue(conflictError);
    const { result } = renderHook(() => useUnbanUser());

    await act(async () => {
      await result.current.unban('user-1', { reason: '误封' }).catch(() => {});
    });

    expect(result.current.error).toBe(conflictError);
    expect(result.current.error?.message).toBeTruthy();
  });
});

// ── U04: loading=true 期间 ────────────────────────────────────────────────
describe('useUnbanUser — U04: loading=true 期间', () => {
  it('unban() 执行中 loading=true，完成后 loading=false', async () => {
    let resolvePromise!: () => void;
    const pending = new Promise<void>((resolve) => {
      resolvePromise = resolve;
    });
    mockAdminUnbanUser.mockReturnValue(pending);

    const { result } = renderHook(() => useUnbanUser());

    act(() => {
      void result.current.unban('user-1', { reason: '处罚到期' });
    });

    expect(result.current.loading).toBe(true);

    await act(async () => {
      resolvePromise();
    });

    expect(result.current.loading).toBe(false);
  });
});
