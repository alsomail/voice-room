/**
 * usePaymentOrdersPage — 订单列表数据 hook (T-20030)
 */

import { useState, useEffect, useCallback } from 'react';
import {
  listPaymentOrders,
  type ListOrdersParams,
  type PaymentOrderListItem,
} from '../../api/payment';

export function usePaymentOrdersPage() {
  const [items, setItems] = useState<PaymentOrderListItem[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(20);
  const [filters, setFilters] = useState<ListOrdersParams>({});
  const [refreshKey, setRefreshKey] = useState(0);

  const fetch = useCallback(
    async (signal?: AbortSignal) => {
      setLoading(true);
      setError(null);
      try {
        const result = await listPaymentOrders(
          { page, page_size: pageSize, ...filters },
          signal,
        );
        setItems(result.data);
        setTotal(result.total);
      } catch (e: unknown) {
        if (e instanceof DOMException && e.name === 'AbortError') return;
        setError(e instanceof Error ? e.message : 'Unknown error');
      } finally {
        setLoading(false);
      }
    },
    [page, pageSize, filters, refreshKey],
  );

  useEffect(() => {
    const controller = new AbortController();
    fetch(controller.signal);
    return () => controller.abort();
  }, [fetch]);

  const refresh = useCallback(() => setRefreshKey((k) => k + 1), []);

  const applyFilters = useCallback((f: ListOrdersParams) => {
    setFilters(f);
    setPage(1);
  }, []);

  const resetFilters = useCallback(() => {
    setFilters({});
    setPage(1);
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
    setPageSize,
    applyFilters,
    resetFilters,
    refresh,
  };
}
