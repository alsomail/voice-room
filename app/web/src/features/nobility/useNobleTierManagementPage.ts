/**
 * useNobleTierManagementPage — 贵族等级列表数据 hook (T-20035)
 */

import { useState, useEffect, useCallback } from 'react';
import { listNobleTiers, type TierItem } from '../../api/nobility';

export function useNobleTierManagementPage() {
  const [items, setItems] = useState<TierItem[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [refreshKey, setRefreshKey] = useState(0);

  const fetch = useCallback(
    async (signal?: AbortSignal) => {
      setLoading(true);
      setError(null);
      try {
        const result = await listNobleTiers(page, pageSize, signal);
        setItems(result.items);
        setTotal(result.total);
      } catch (e: unknown) {
        if (e instanceof DOMException && e.name === 'AbortError') return;
        setError(e instanceof Error ? e.message : 'Unknown error');
      } finally {
        setLoading(false);
      }
    },
    [page, pageSize, refreshKey],
  );

  useEffect(() => {
    const controller = new AbortController();
    fetch(controller.signal);
    return () => controller.abort();
  }, [fetch]);

  const refresh = useCallback(() => setRefreshKey((k) => k + 1), []);

  return {
    items,
    total,
    loading,
    error,
    page,
    pageSize,
    setPage,
    setPageSize,
    refresh,
  };
}
