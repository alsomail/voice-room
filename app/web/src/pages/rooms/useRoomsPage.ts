/**
 * useRoomsPage — 房间管理页面数据 Hook（T-20004 + T-20011）
 *
 * 职责：
 *   - 分页展示房间列表（page, pageSize 默认 20）
 *   - 支持 status 过滤（立即生效，重置 page=1）
 *   - 支持 keyword 搜索（300ms debounce，重置 page=1）
 *   - 每次 fetch 建立独立 AbortController，cleanup 时 abort
 *   - closeRoom：细粒度 loading（closingId），成功后 refresh，失败后 setError
 *   - selectedRoomId：T-20005 占位 state
 *   - activityFilter（T-20011）：纯前端活跃度过滤，不触发新 API 请求
 */

import { useState, useEffect, useCallback, useMemo } from 'react';
import {
  adminGetRooms,
  adminCloseRoom,
  type AdminRoomItem,
} from '../../core/network/apiClient';
import { filterByActivity, type ActivityFilter } from './roomUtils';

export interface RoomsPageFilters {
  status?: 'active' | 'closed';
  keyword?: string;
}

export interface UseRoomsPageReturn {
  items: AdminRoomItem[];
  filteredItems: AdminRoomItem[];
  total: number;
  loading: boolean;
  error: Error | null;
  page: number;
  pageSize: number;
  filters: RoomsPageFilters;
  activityFilter: ActivityFilter;
  closingId: string | null;
  selectedRoomId: string | null;
  setPage: (page: number, pageSize: number) => void;
  setFilters: (patch: Partial<RoomsPageFilters>) => void;
  setActivityFilter: (filter: ActivityFilter) => void;
  closeRoom: (roomId: string) => Promise<void>;
  refresh: () => void;
  setSelectedRoomId: (id: string | null) => void;
}

export function useRoomsPage(): UseRoomsPageReturn {
  const [items, setItems] = useState<AdminRoomItem[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [page, setPageState] = useState(1);
  const [pageSize, setPageSizeState] = useState(20);
  const [filters, setFiltersState] = useState<RoomsPageFilters>({});
  const [debouncedKeyword, setDebouncedKeyword] = useState<string | undefined>(undefined);
  const [refreshKey, setRefreshKey] = useState(0);
  const [closingId, setClosingId] = useState<string | null>(null);
  const [selectedRoomId, setSelectedRoomId] = useState<string | null>(null);
  // T-20011: 活跃度筛选（纯前端，不触发 API）
  const [activityFilter, setActivityFilter] = useState<ActivityFilter>('all');

  // ── debounce keyword：300ms 后更新 debouncedKeyword 并重置 page=1 ──────
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedKeyword(filters.keyword);
      setPageState(1);
    }, 300);
    return () => clearTimeout(timer);
  }, [filters.keyword]);

  // ── fetch effect：debouncedKeyword / status / page / pageSize / refreshKey 变化时触发 ──
  useEffect(() => {
    const controller = new AbortController();

    const fetchRooms = async () => {
      setLoading(true);
      setError(null);
      try {
        const data = await adminGetRooms(
          {
            page,
            page_size: pageSize,
            ...(filters.status   && { status:  filters.status }),
            ...(debouncedKeyword && { keyword: debouncedKeyword }),
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

    void fetchRooms();

    return () => {
      controller.abort();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
    // 故意订阅 filters.status 而非 filters 整体——filters 对象引用在每次 setFilters 时都会更新，
    // 订阅整体会导致多余 refetch。status/keyword 变化已通过各自依赖项精确捕获。
  }, [debouncedKeyword, filters.status, page, pageSize, refreshKey]);

  // ── T-20011: filteredItems — 纯前端活跃度过滤，依赖 items + activityFilter ──
  const filteredItems = useMemo(
    () => filterByActivity(items, activityFilter),
    [items, activityFilter],
  );

  // ── setFilters：status 变化时立即重置 page=1 ──────────────────────────
  const setFilters = useCallback((patch: Partial<RoomsPageFilters>) => {
    setFiltersState((prev) => ({ ...prev, ...patch }));
    if ('status' in patch) {
      setPageState(1);
    }
  }, []);

  // ── setPage ───────────────────────────────────────────────────────────
  const setPage = useCallback((newPage: number, newPageSize: number) => {
    setPageState(newPage);
    setPageSizeState(newPageSize);
  }, []);

  // ── refresh：通过 refreshKey 强制触发 fetch effect ──────────────────
  const refresh = useCallback(() => {
    setRefreshKey((prev) => prev + 1);
  }, []);

  // ── closeRoom ─────────────────────────────────────────────────────────
  const closeRoom = useCallback(
    async (roomId: string) => {
      setClosingId(roomId);
      try {
        await adminCloseRoom(roomId);
        refresh();
      } catch (err) {
        const e = err instanceof Error ? err : new Error(String(err));
        setError(e);
        throw e; // re-throw，让调用方（如 RoomDetailModal onOk）可感知失败
      } finally {
        setClosingId(null);
      }
    },
    [refresh],
  );

  return {
    items,
    filteredItems,
    total,
    loading,
    error,
    page,
    pageSize,
    filters,
    activityFilter,
    closingId,
    selectedRoomId,
    setPage,
    setFilters,
    setActivityFilter,
    closeRoom,
    refresh,
    setSelectedRoomId,
  };
}
