/**
 * useLogsPage — 操作日志页面数据 Hook（T-20009）
 *
 * 职责：
 *   - 分页展示操作日志列表（page, pageSize 默认 20）
 *   - 支持 adminId / action / startDate / endDate 过滤
 *   - 搜索参数变化时重置 page=1
 *   - 每次 fetch 建立独立 AbortController，cleanup 时 abort 防竞态
 *   - refreshKey 自增触发强制刷新
 *   - 搜索参数双向同步 URL Query String（使用 react-router-dom useSearchParams）
 */

import { useState, useEffect, useCallback } from 'react';
import { useSearchParams } from 'react-router-dom';
import {
  adminGetLogs,
  type AdminLogItem,
} from '../../core/network/apiClient';

export interface LogsPageFilters {
  adminId?: string;
  action?: string;
  startDate?: string;  // ISO 8601
  endDate?: string;    // ISO 8601
}

export interface UseLogsPageReturn {
  items: AdminLogItem[];
  total: number;
  loading: boolean;
  error: Error | null;
  page: number;
  pageSize: number;
  filters: LogsPageFilters;
  setPage: (page: number, pageSize: number) => void;
  setFilters: (filters: LogsPageFilters) => void;
  refresh: () => void;
}

export function useLogsPage(): UseLogsPageReturn {
  const [searchParams, setSearchParams] = useSearchParams();

  // ── 初始状态从 URL 读取 ──────────────────────────────────────────────────
  const [filters, setFiltersState] = useState<LogsPageFilters>(() => ({
    adminId:   searchParams.get('admin_id')   ?? undefined,
    action:    searchParams.get('action')     ?? undefined,
    startDate: searchParams.get('start_date') ?? undefined,
    endDate:   searchParams.get('end_date')   ?? undefined,
  }));

  const [page, setPageState] = useState<number>(() => {
    const p = Number(searchParams.get('page'));
    return p > 0 ? p : 1;
  });

  const [pageSize, setPageSizeState] = useState<number>(() => {
    const s = Number(searchParams.get('size'));
    return s > 0 ? s : 20;
  });

  const [refreshKey, setRefreshKey] = useState(0);
  const [items, setItems] = useState<AdminLogItem[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  // ── URL 同步：filters / page / pageSize 变化时更新 URL ──────────────────
  useEffect(() => {
    const params: Record<string, string> = {};
    if (filters.adminId)   params.admin_id   = filters.adminId;
    if (filters.action)    params.action     = filters.action;
    if (filters.startDate) params.start_date = filters.startDate;
    if (filters.endDate)   params.end_date   = filters.endDate;
    if (page > 1)          params.page       = String(page);
    if (pageSize !== 20)   params.size       = String(pageSize);
    setSearchParams(params, { replace: true });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filters.adminId, filters.action, filters.startDate, filters.endDate, page, pageSize]);

  // ── fetch effect：filters / page / pageSize / refreshKey 变化时触发 ──────
  useEffect(() => {
    const controller = new AbortController();

    const fetchLogs = async () => {
      setLoading(true);
      setError(null);
      try {
        const data = await adminGetLogs(
          {
            page,
            size: pageSize,
            ...(filters.adminId   && { admin_id:   filters.adminId }),
            ...(filters.action    && { action:     filters.action }),
            ...(filters.startDate && { start_date: filters.startDate }),
            ...(filters.endDate   && { end_date:   filters.endDate }),
          },
          controller.signal,
        );
        setItems(data.items);
        setTotal(data.total);
      } catch (err) {
        const e = err instanceof Error ? err : new Error(String(err));
        if (e.name !== 'AbortError') {
          setError(e);
          setItems([]);
        }
      } finally {
        setLoading(false);
      }
    };

    void fetchLogs();

    return () => {
      controller.abort();
    };
    // 订阅各个 filter 字段而非 filters 整体，避免对象引用变化引起的多余 refetch
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filters.adminId, filters.action, filters.startDate, filters.endDate, page, pageSize, refreshKey]);

  // ── setFilters：同时重置 page=1 ──────────────────────────────────────────
  const setFilters = useCallback((newFilters: LogsPageFilters) => {
    setFiltersState(newFilters);
    setPageState(1);
  }, []);

  // ── setPage ───────────────────────────────────────────────────────────────
  const setPage = useCallback((newPage: number, newPageSize: number) => {
    setPageState(newPage);
    setPageSizeState(newPageSize);
  }, []);

  // ── refresh：refreshKey 自增触发 fetch ────────────────────────────────────
  const refresh = useCallback(() => {
    setRefreshKey((prev) => prev + 1);
  }, []);

  return {
    items,
    total,
    loading,
    error,
    page,
    pageSize,
    filters,
    setPage,
    setFilters,
    refresh,
  };
}
