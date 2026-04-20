/**
 * useBanUser — 封禁/解封用户 Hook（T-20008）
 *
 * 职责：
 *   - 包装 adminBanUser API 调用
 *   - 管理 loading / error 状态
 *   - ban() 失败时 re-throw，让调用方（BanModal）决定如何处理
 */

import { useState, useCallback } from 'react';
import { adminBanUser, type AdminBanUserRequest } from '../../core/network/apiClient';

export interface UseBanUserReturn {
  loading: boolean;
  error: Error | null;
  ban: (userId: string, req: AdminBanUserRequest) => Promise<void>;
}

export function useBanUser(): UseBanUserReturn {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const ban = useCallback(async (userId: string, req: AdminBanUserRequest) => {
    setLoading(true);
    setError(null);
    try {
      await adminBanUser(userId, req);
    } catch (err) {
      const e = err instanceof Error ? err : new Error(String(err));
      setError(e);
      throw e; // 让 BanModal 知道失败了
    } finally {
      setLoading(false);
    }
  }, []);

  return { loading, error, ban };
}
