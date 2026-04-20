/**
 * useUserDetail — 用户详情数据 Hook（T-20007）
 *
 * 职责：
 *   - userId 非 null 时发起 adminGetUserDetail 请求
 *   - userId = null 时清空 detail/error，不发请求
 *   - userId 变化时 abort 旧请求，建立新 AbortController
 *   - 成功 → setDetail(data), setLoading(false)
 *   - AbortError → 静默忽略（不修改 error 和 loading）
 *   - 其他错误 → setError, setDetail(null), setLoading(false)
 *   - cleanup → controller.abort()
 */

import { useState, useEffect } from 'react';
import {
  adminGetUserDetail,
  type AdminUserDetailResponse,
} from '../../core/network/apiClient';

export function useUserDetail(userId: string | null): {
  detail: AdminUserDetailResponse | null;
  loading: boolean;
  error: Error | null;
} {
  const [detail, setDetail] = useState<AdminUserDetailResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    // userId=null：清空状态，不发请求
    if (userId === null) {
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
        const data = await adminGetUserDetail(userId, controller.signal);
        // 只在未 abort 时更新状态，防止竞态闪烁
        if (!controller.signal.aborted) {
          setDetail(data);
          setLoading(false);
        }
      } catch (err) {
        // AbortError：静默忽略
        const errName = (err as { name?: string }).name;
        if (errName === 'AbortError') {
          return;
        }
        if (!controller.signal.aborted) {
          setError(err instanceof Error ? err : new Error(String(err)));
          setDetail(null);
          setLoading(false);
        }
      }
    };

    void run();

    // cleanup：abort 挂起请求
    return () => {
      controller.abort();
    };
  }, [userId]);

  return { detail, loading, error };
}
