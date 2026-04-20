/**
 * useDashboardStats — 数据看板统计数据 Hook（T-20003）
 *
 * 职责：
 *   - 并发发起三个 API 请求（Promise.allSettled，任一失败不影响其他）
 *   - 仅当三个请求全部失败时才将 error 置为非 null
 *   - 每 30 秒自动刷新；组件卸载时清理定时器并取消飞行中请求
 *   - 导出 { stats, loading, error, refresh, lastUpdatedAt }
 */

import { useState, useEffect, useCallback } from 'react';
import {
  adminGetRooms,
  adminGetStatsOverview,
  type AdminStatsTrendPoint,
} from '../../core/network/apiClient';

export type TrendPoint = AdminStatsTrendPoint;

/** 数据看板统计字段（null 表示对应请求失败或尚未加载） */
export interface DashboardStats {
  /** 总房间数 */
  totalRooms: number | null;
  /** 活跃房间数 */
  activeRooms: number | null;
  /** 当前在线人数 */
  onlineUsers: number | null;
  /** 今日 DAU */
  dau: number | null;
  /** 今日新增用户 */
  newUsersToday: number | null;
  /** 历史趋势 */
  trend: TrendPoint[];
}

const INITIAL_STATS: DashboardStats = {
  totalRooms: null,
  activeRooms: null,
  onlineUsers: null,
  dau: null,
  newUsersToday: null,
  trend: [],
};

const AUTO_REFRESH_MS = 30_000;

export interface UseDashboardStatsReturn {
  stats: DashboardStats;
  loading: boolean;
  error: Error | null;
  refresh: () => void;
  lastUpdatedAt: Date | null;
}

export function useDashboardStats(): UseDashboardStatsReturn {
  const [stats, setStats] = useState<DashboardStats>(INITIAL_STATS);
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<Error | null>(null);
  const [lastUpdatedAt, setLastUpdatedAt] = useState<Date | null>(null);

  // [M-02] 接受外部 AbortSignal，卸载时取消飞行中请求
  const fetchData = useCallback(async (signal?: AbortSignal) => {
    setLoading(true);

    const [totalResult, activeResult, statsResult] = await Promise.allSettled([
      adminGetRooms(undefined, signal),
      adminGetRooms({ status: 'active' }, signal),
      adminGetStatsOverview(signal),
    ]);

    // 判断是否全部失败
    const allFailed =
      totalResult.status === 'rejected' &&
      activeResult.status === 'rejected' &&
      statsResult.status === 'rejected';

    if (allFailed) {
      setError(
        totalResult.status === 'rejected'
          ? (totalResult.reason as Error)
          : new Error('All requests failed'),
      );
      setStats(INITIAL_STATS);
    } else {
      setError(null);
      setStats({
        totalRooms:
          totalResult.status === 'fulfilled' ? totalResult.value.total : null,
        activeRooms:
          activeResult.status === 'fulfilled' ? activeResult.value.total : null,
        onlineUsers:
          statsResult.status === 'fulfilled'
            ? statsResult.value.online_users
            : null,
        dau:
          statsResult.status === 'fulfilled' ? statsResult.value.dau : null,
        newUsersToday:
          statsResult.status === 'fulfilled'
            ? statsResult.value.new_users_today
            : null,
        trend:
          statsResult.status === 'fulfilled' ? statsResult.value.trend : [],
      });
      setLastUpdatedAt(new Date());
    }

    setLoading(false);
  }, []);

  // [M-02] 每次 effect 建立独立的 AbortController；卸载时 abort() 取消飞行中请求
  // [M-03] 删除冗余的 fetchDataRef，直接依赖 fetchData（其引用永远稳定）
  useEffect(() => {
    const abortController = new AbortController();

    void fetchData(abortController.signal);

    const timer = setInterval(() => {
      void fetchData(abortController.signal);
    }, AUTO_REFRESH_MS);

    return () => {
      abortController.abort();
      clearInterval(timer);
    };
  }, [fetchData]);

  const refresh = useCallback(() => {
    void fetchData();
  }, [fetchData]);

  return { stats, loading, error, refresh, lastUpdatedAt };
}
