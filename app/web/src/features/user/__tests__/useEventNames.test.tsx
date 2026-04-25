/**
 * useEventNames — 单元测试（缺陷 8 / R1 批 3）
 *
 * 覆盖：
 *   - 成功：items 返回后端枚举（已字典序），loading 切换正确
 *   - 失败：降级到 ANALYTICS_EVENTS 并 console.warn
 *   - 缓存：5 分钟内重复 mount 不发新请求
 *   - 空数组：后端返回空 → 降级到本地字典
 *   - 卸载：组件卸载后不会 setState（abort 路径）
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, waitFor, act } from '@testing-library/react';

vi.mock('../../../services/api/events', () => ({
  listEventNames: vi.fn(),
}));

import { listEventNames } from '../../../services/api/events';
import { useEventNames, __resetEventNamesCache } from '../useEventNames';
import { ANALYTICS_EVENTS } from '../events.dict';

const mockListEventNames = listEventNames as unknown as ReturnType<typeof vi.fn>;

describe('useEventNames — 缺陷 8 / R1 批 3', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    __resetEventNamesCache();
  });

  afterEach(() => {
    __resetEventNamesCache();
  });

  it('成功路径：items 来自后端枚举（字典序）且 loading 最终为 false', async () => {
    mockListEventNames.mockResolvedValue({
      items: ['gift_send_success', 'login_success', 'mic_take'],
    });

    const { result } = renderHook(() => useEventNames());
    expect(result.current.loading).toBe(true);

    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.failed).toBe(false);
    expect(result.current.items).toEqual([
      'gift_send_success',
      'login_success',
      'mic_take',
    ]);
    expect(mockListEventNames).toHaveBeenCalledTimes(1);
    expect(mockListEventNames).toHaveBeenCalledWith(30, expect.anything());
  });

  it('失败路径：降级到 ANALYTICS_EVENTS 并 console.warn', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    mockListEventNames.mockRejectedValue(new Error('network down'));

    const { result } = renderHook(() => useEventNames());
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.failed).toBe(true);
    expect(result.current.items).toEqual(ANALYTICS_EVENTS);
    expect(warnSpy).toHaveBeenCalledTimes(1);
    expect(warnSpy.mock.calls[0]?.[0]).toContain('useEventNames');
    warnSpy.mockRestore();
  });

  it('缓存：5 分钟内第二次 mount 不再触发网络请求', async () => {
    mockListEventNames.mockResolvedValue({ items: ['a', 'b'] });

    const first = renderHook(() => useEventNames());
    await waitFor(() => expect(first.result.current.loading).toBe(false));
    expect(mockListEventNames).toHaveBeenCalledTimes(1);

    first.unmount();

    const second = renderHook(() => useEventNames());
    // 命中缓存 → 立即可用，loading 为 false
    expect(second.result.current.loading).toBe(false);
    expect(second.result.current.items).toEqual(['a', 'b']);
    expect(mockListEventNames).toHaveBeenCalledTimes(1);
  });

  it('后端返回空数组：items 降级到 ANALYTICS_EVENTS（保证下拉可用）', async () => {
    mockListEventNames.mockResolvedValue({ items: [] });

    const { result } = renderHook(() => useEventNames());
    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.failed).toBe(false);
    expect(result.current.items).toEqual(ANALYTICS_EVENTS);
  });

  it('AbortError 不被视为失败（不 warn、不降级）', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const abortErr = new Error('aborted');
    abortErr.name = 'AbortError';
    mockListEventNames.mockRejectedValue(abortErr);

    const { result, unmount } = renderHook(() => useEventNames());
    // 立刻卸载，触发 abort 路径
    unmount();
    // 等微任务清空
    await act(async () => {
      await Promise.resolve();
    });

    expect(warnSpy).not.toHaveBeenCalled();
    // 卸载后不再 setState，残留状态保持 loading=true 也是可以接受的（不会再被读取）
    expect(result.current.failed).toBe(false);
    warnSpy.mockRestore();
  });
});
