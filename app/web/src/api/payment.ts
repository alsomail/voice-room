/**
 * Payment Admin API 函数 (T-20030~33)
 *
 * 协议契约：doc/protocol/payment_api.md §9 Admin REST
 */

import { z } from 'zod';
import { adminFetch } from '../core/network/apiClient';
import { validateResponse } from '../core/network/apiClient';

// ─── Zod Schemas ────────────────────────────────────────────────────────────────

const PaymentOrderListItemSchema = z.object({
  order_id: z.string().uuid(),
  user_id: z.string().uuid(),
  sku_id: z.string(),
  provider: z.string(),
  amount_micros: z.number().nullable(),
  currency: z.string().nullable(),
  country_code: z.string().nullable(),
  state: z.string(),
  purchase_token_masked: z.string().nullable(),
  provider_order_id: z.string().nullable(),
  created_at: z.string(),
  credited_at: z.string().nullable(),
  acked_at: z.string().nullable(),
  failed_at: z.string().nullable(),
});

const ListOrdersDataSchema = z.object({
  data: z.array(PaymentOrderListItemSchema),
  total: z.number(),
  page: z.number(),
  page_size: z.number(),
});

const OrderDetailSchema = z.object({
  order_id: z.string().uuid(),
  user_id: z.string().uuid(),
  sku_id: z.string(),
  provider: z.string(),
  amount_micros: z.number().nullable(),
  currency: z.string().nullable(),
  country_code: z.string().nullable(),
  state: z.string(),
  state_history: z.unknown(),
  provider_response_raw: z.unknown().nullable(),
  purchase_token_masked: z.string().nullable(),
  provider_order_id: z.string().nullable(),
  risk_flags: z.array(z.string()),
  created_at: z.string(),
  verified_at: z.string().nullable(),
  credited_at: z.string().nullable(),
  acked_at: z.string().nullable(),
  failed_at: z.string().nullable(),
  failed_reason: z.string().nullable(),
});

const SkuResponseSchema = z.object({
  sku_id: z.string(),
  provider: z.string(),
  diamonds: z.number(),
  display_price_usd: z.string(),
  display_price_local: z.string().nullable(),
  display_currency: z.string().nullable(),
  is_active: z.boolean(),
  sort_order: z.number(),
  tag: z.string().nullable(),
  created_at: z.string(),
  updated_at: z.string(),
});

const ListSkusDataSchema = z.object({
  skus: z.array(SkuResponseSchema),
});

const CreateSkuDataSchema = z.object({
  sku: SkuResponseSchema,
  warning: z.string().nullable().optional(),
});

const ReportSeriesItemSchema = z.object({
  date: z.string(),
  gmv_usd: z.string(),
  gmv_by_currency: z.record(z.string(), z.string()),
  order_count: z.number(),
  refund_count: z.number(),
  refund_amount_usd: z.string(),
  avg_ticket_usd: z.string(),
});

const ReportTotalsSchema = z.object({
  gmv_usd: z.string(),
  order_count: z.number(),
  refund_count: z.number(),
  refund_amount_usd: z.string(),
  avg_ticket_usd: z.string(),
});

const ReportDataSchema = z.object({
  granularity: z.string(),
  from: z.string(),
  to: z.string(),
  series: z.array(ReportSeriesItemSchema),
  totals: ReportTotalsSchema,
});

// ─── Mutation response schemas ──────────────────────────────────────────────────

const RecreditResponseSchema = z.object({
  order_id: z.string().uuid(),
  new_state: z.string(),
  diamonds_credited: z.number(),
});

const RefundResponseSchema = z.object({
  order_id: z.string().uuid(),
  new_state: z.string(),
  diamonds_deducted: z.number(),
});

// ─── Exported Types ──────────────────────────────────────────────────────────────

export type PaymentOrderListItem = z.infer<typeof PaymentOrderListItemSchema>;
export type ListOrdersData = z.infer<typeof ListOrdersDataSchema>;
export type OrderDetail = z.infer<typeof OrderDetailSchema>;
export type SkuItem = z.infer<typeof SkuResponseSchema>;
export type ReportSeriesItem = z.infer<typeof ReportSeriesItemSchema>;
export type ReportTotals = z.infer<typeof ReportTotalsSchema>;
export type ReportData = z.infer<typeof ReportDataSchema>;

// ─── Query Params ────────────────────────────────────────────────────────────────

export interface ListOrdersParams {
  page?: number;
  page_size?: number;
  user_id?: string;
  state?: string;
  provider?: string;
  created_from?: string;
  created_to?: string;
  amount_min?: number;
  amount_max?: number;
}

export interface ReportQueryParams {
  granularity: 'day' | 'month';
  from: string; // YYYY-MM-DD
  to: string; // YYYY-MM-DD
  currency?: string;
}

export interface SkuCreateRequest {
  sku_id: string;
  provider: string;
  diamonds: number;
  display_price_usd: string;
  display_price_local?: string;
  display_currency?: string;
  sort_order?: number;
  tag?: string;
  is_active?: boolean;
}

export interface SkuUpdateRequest {
  diamonds?: number;
  display_price_usd?: string;
  display_price_local?: string;
  display_currency?: string;
  is_active?: boolean;
  sort_order?: number;
  tag?: string;
}

// ─── API Functions ───────────────────────────────────────────────────────────────

/** GET /api/v1/admin/payments/orders */
export async function listPaymentOrders(
  params: ListOrdersParams,
  signal?: AbortSignal,
): Promise<ListOrdersData> {
  const q = new URLSearchParams();
  if (params.page) q.set('page', String(params.page));
  if (params.page_size) q.set('page_size', String(params.page_size));
  if (params.user_id) q.set('user_id', params.user_id);
  if (params.state) q.set('state', params.state);
  if (params.provider) q.set('provider', params.provider);
  if (params.created_from) q.set('created_from', params.created_from);
  if (params.created_to) q.set('created_to', params.created_to);
  if (params.amount_min != null) q.set('amount_min', String(params.amount_min));
  if (params.amount_max != null) q.set('amount_max', String(params.amount_max));

  const result = await adminFetch<ListOrdersData>(
    `/api/v1/admin/payments/orders?${q.toString()}`,
    { signal },
  );
  return validateResponse(result.data, ListOrdersDataSchema);
}

/** GET /api/v1/admin/payments/orders/:id */
export async function getPaymentOrderDetail(
  orderId: string,
  signal?: AbortSignal,
): Promise<OrderDetail> {
  const result = await adminFetch<OrderDetail>(
    `/api/v1/admin/payments/orders/${orderId}`,
    { signal },
  );
  return validateResponse(result.data, OrderDetailSchema);
}

/** POST /api/v1/admin/payments/orders/:id/recredit */
export async function recreditOrder(
  orderId: string,
  reason: string,
  signal?: AbortSignal,
): Promise<{ order_id: string; new_state: string; diamonds_credited: number }> {
  const result = await adminFetch<z.infer<typeof RecreditResponseSchema>>(
    `/api/v1/admin/payments/orders/${orderId}/recredit`,
    {
      method: 'POST',
      body: JSON.stringify({ reason }),
      signal,
    },
  );
  return validateResponse(result.data, RecreditResponseSchema);
}

/** POST /api/v1/admin/payments/orders/:id/refund */
export async function refundOrder(
  orderId: string,
  reason: string,
  signal?: AbortSignal,
): Promise<{ order_id: string; new_state: string; diamonds_deducted: number }> {
  const result = await adminFetch<z.infer<typeof RefundResponseSchema>>(
    `/api/v1/admin/payments/orders/${orderId}/refund`,
    {
      method: 'POST',
      body: JSON.stringify({ reason }),
      signal,
    },
  );
  return validateResponse(result.data, RefundResponseSchema);
}

/** GET /api/v1/admin/payments/skus */
export async function listSkus(signal?: AbortSignal): Promise<SkuItem[]> {
  const result = await adminFetch<{ skus: SkuItem[] }>(
    '/api/v1/admin/payments/skus',
    { signal },
  );
  return validateResponse(result.data, ListSkusDataSchema).skus;
}

/** POST /api/v1/admin/payments/skus */
export async function createSku(
  req: SkuCreateRequest,
  signal?: AbortSignal,
): Promise<{ sku: SkuItem; warning?: string }> {
  const result = await adminFetch<{ sku: SkuItem; warning?: string }>(
    '/api/v1/admin/payments/skus',
    { method: 'POST', body: JSON.stringify(req), signal },
  );
  return validateResponse(result.data, CreateSkuDataSchema);
}

/** PUT /api/v1/admin/payments/skus/:id */
export async function updateSku(
  skuId: string,
  req: SkuUpdateRequest,
  confirm = false,
  signal?: AbortSignal,
): Promise<SkuItem> {
  const qs = confirm ? '?confirm=true' : '';
  const result = await adminFetch<SkuItem>(
    `/api/v1/admin/payments/skus/${skuId}${qs}`,
    { method: 'PUT', body: JSON.stringify(req), signal },
  );
  return validateResponse(result.data, SkuResponseSchema);
}

/** DELETE /api/v1/admin/payments/skus/:id */
export async function deleteSku(skuId: string, signal?: AbortSignal): Promise<void> {
  await adminFetch(`/api/v1/admin/payments/skus/${skuId}`, {
    method: 'DELETE',
    signal,
  });
}

/** GET /api/v1/admin/payments/reports */
export async function getPaymentReport(
  params: ReportQueryParams,
  signal?: AbortSignal,
): Promise<ReportData> {
  const q = new URLSearchParams({
    granularity: params.granularity,
    from: params.from,
    to: params.to,
  });
  if (params.currency) q.set('currency', params.currency);

  const result = await adminFetch<ReportData>(
    `/api/v1/admin/payments/reports?${q.toString()}`,
    { signal },
  );
  return validateResponse(result.data, ReportDataSchema);
}
