/**
 * useFinancialReportsPage — 财务报表数据 hook (T-20033)
 */

import { useState, useEffect, useCallback } from 'react';
import {
  getPaymentReport,
  type ReportQueryParams,
  type ReportData,
} from '../../api/payment';

export function useFinancialReportsPage() {
  const [report, setReport] = useState<ReportData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetch = useCallback(
    async (params: ReportQueryParams, signal?: AbortSignal) => {
      setLoading(true);
      setError(null);
      try {
        const result = await getPaymentReport(params, signal);
        setReport(result);
      } catch (e: unknown) {
        if (e instanceof DOMException && e.name === 'AbortError') return;
        setError(e instanceof Error ? e.message : 'Unknown error');
      } finally {
        setLoading(false);
      }
    },
    [],
  );

  // Cleanup abort on unmount
  useEffect(() => {
    return () => {
      // no-op: fetch is manual, not auto
    };
  }, []);

  return { report, loading, error, fetch };
}
