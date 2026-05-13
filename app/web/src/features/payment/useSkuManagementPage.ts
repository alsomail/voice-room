/**
 * useSkuManagementPage — SKU 列表数据 hook (T-20032)
 */

import { useState, useEffect, useCallback } from 'react';
import { listSkus, type SkuItem } from '../../api/payment';

export function useSkuManagementPage() {
  const [items, setItems] = useState<SkuItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  const fetch = useCallback(
    async (signal?: AbortSignal) => {
      setLoading(true);
      setError(null);
      try {
        const result = await listSkus(signal);
        setItems(result);
      } catch (e: unknown) {
        if (e instanceof DOMException && e.name === 'AbortError') return;
        setError(e instanceof Error ? e.message : 'Unknown error');
      } finally {
        setLoading(false);
      }
    },
    [refreshKey],
  );

  useEffect(() => {
    const controller = new AbortController();
    fetch(controller.signal);
    return () => controller.abort();
  }, [fetch]);

  const refresh = useCallback(() => setRefreshKey((k) => k + 1), []);

  return { items, loading, error, refresh };
}
