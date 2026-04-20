/**
 * useRoomDetail — 房间详情数据 Hook（T-20005）
 *
 * 职责：
 *   - roomId 非 null 时发起 adminGetRoomDetail 请求
 *   - roomId = null 时清空 detail/error，不发请求
 *   - roomId 变化时 abort 旧请求，建立新 AbortController
 *   - 成功 → setDetail(data), setLoading(false)
 *   - AbortError → 静默忽略（不修改 error 和 loading）
 *   - 其他错误 → setError, setDetail(null), setLoading(false)
 *   - cleanup → controller.abort()
 */

import { useState, useEffect } from 'react';
import { adminGetRoomDetail, type AdminRoomDetail } from '../../core/network/apiClient';

export function useRoomDetail(roomId: string | null): {
  detail: AdminRoomDetail | null;
  loading: boolean;
  error: Error | null;
} {
  const [detail, setDetail] = useState<AdminRoomDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    // roomId=null：清空状态，不发请求
    if (roomId === null) {
      setDetail(null);
      setError(null);
      setLoading(false);
      return;
    }

    const controller = new AbortController();

    // 开始加载：重置状态
    setLoading(true);
    setDetail(null);
    setError(null);

    const run = async () => {
      try {
        const data = await adminGetRoomDetail(roomId, controller.signal);
        setDetail(data);
        setLoading(false);
      } catch (err) {
        // AbortError：静默忽略（不管是 DOMException 还是 Error 子类）
        // jsdom 中 DOMException 不继承 Error，故不使用 instanceof Error 判断
        const errName = (err as { name?: string }).name;
        if (errName === 'AbortError') {
          return;
        }
        setError(err instanceof Error ? err : new Error(String(err)));
        setDetail(null);
        setLoading(false);
      }
    };

    void run();

    // cleanup：abort 挂起请求
    return () => {
      controller.abort();
    };
  }, [roomId]);

  return { detail, loading, error };
}
