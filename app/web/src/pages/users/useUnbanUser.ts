/**
 * useUnbanUser — 解封用户 Hook（T-20010）
 *
 * 职责：
 *   - 包装 adminUnbanUser API 调用
 *   - 管理 loading / error 状态
 *   - unban() 失败时 re-throw，让调用方（UnbanModal）决定如何处理
 */

import { useState, useCallback } from 'react';
import { adminUnbanUser, type AdminUnbanUserRequest } from '../../core/network/apiClient';

export interface UseUnbanUserReturn {
  loading: boolean;
  error: Error | null;
  unban: (userId: string, req: AdminUnbanUserRequest) => Promise<void>;
}

export function useUnbanUser(): UseUnbanUserReturn {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const unban = useCallback(async (userId: string, req: AdminUnbanUserRequest) => {
    setLoading(true);
    setError(null);
    try {
      await adminUnbanUser(userId, req);
    } catch (err) {
      const e = err instanceof Error ? err : new Error(String(err));
      setError(e);
      throw e;
    } finally {
      setLoading(false);
    }
  }, []);

  return { loading, error, unban };
}
