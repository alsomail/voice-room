/**
 * useUsersPage — 用户管理页面数据 Hook（T-20006）
 *
 * 职责：
 *   - 分页展示用户列表（page, pageSize 默认 20）
 *   - 支持 phone / nickname / userId / status 过滤
 *   - 搜索参数变化时重置 page=1
 *   - 每次 fetch 建立独立 AbortController，cleanup 时 abort 防竞态
 *   - refreshKey 自增触发强制刷新
 *   - 搜索参数双向同步 URL Query String（使用 react-router-dom useSearchParams）
 */

import { useState, useEffect, useCallback } from 'react';
import { useSearchParams } from 'react-router-dom';
import {
  adminGetUsers,
  type AdminUserItem,
} from '../../core/network/apiClient';

export interface UsersPageFilters {
  phone?: string;
  nickname?: string;
  userId?: string;
  status?: 'normal' | 'banned';
}

export interface UseUsersPageReturn {
  items: AdminUserItem[];
  total: number;
  loading: boolean;
  error: Error | null;
  page: number;
  pageSize: number;
  filters: UsersPageFilters;
  setPage: (page: number, pageSize: number) => void;
  setFilters: (filters: UsersPageFilters) => void;
  refresh: () => void;
}

export function useUsersPage(): UseUsersPageReturn {
  const [searchParams, setSearchParams] = useSearchParams();

  // ── 初始状态从 URL 读取 ──────────────────────────────────────────────────
  const [filters, setFiltersState] = useState<UsersPageFilters>(() => ({
    phone:    searchParams.get('phone')    ?? undefined,
    nickname: searchParams.get('nickname') ?? undefined,
    userId:   searchParams.get('user_id')  ?? undefined,
    status:   (searchParams.get('status') as 'normal' | 'banned') ?? undefined,
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
  const [items, setItems] = useState<AdminUserItem[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  // ── URL 同步：filters / page / pageSize 变化时更新 URL ──────────────────
  useEffect(() => {
    const params: Record<string, string> = {};
    if (filters.phone)    params.phone    = filters.phone;
    if (filters.nickname) params.nickname = filters.nickname;
    if (filters.userId)   params.user_id  = filters.userId;
    if (filters.status)   params.status   = filters.status;
    if (page > 1)         params.page     = String(page);
    if (pageSize !== 20)  params.size     = String(pageSize);
    setSearchParams(params, { replace: true });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filters.phone, filters.nickname, filters.userId, filters.status, page, pageSize]);

  // ── fetch effect：filters / page / pageSize / refreshKey 变化时触发 ──────
  useEffect(() => {
    const controller = new AbortController();

    const fetchUsers = async () => {
      setLoading(true);
      setError(null);
      try {
        const data = await adminGetUsers(
          {
            page,
            size: pageSize,
            ...(filters.phone    && { phone:    filters.phone }),
            ...(filters.nickname && { nickname: filters.nickname }),
            ...(filters.userId   && { user_id:  filters.userId }),
            ...(filters.status   && { status:   filters.status }),
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

    void fetchUsers();

    return () => {
      controller.abort();
    };
    // 订阅各个 filter 字段而非 filters 整体，避免对象引用变化引起的多余 refetch
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filters.phone, filters.nickname, filters.userId, filters.status, page, pageSize, refreshKey]);

  // ── setFilters：同时重置 page=1 ──────────────────────────────────────────
  const setFilters = useCallback((newFilters: UsersPageFilters) => {
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
