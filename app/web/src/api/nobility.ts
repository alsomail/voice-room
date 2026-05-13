/**
 * Nobility Admin API 函数 (T-20035~36)
 *
 * 协议契约：doc/protocol/nobility_api.md §10 Admin REST
 */

import { z } from 'zod';
import { adminFetch, validateResponse } from '../core/network/apiClient';

// ─── Zod Schemas ────────────────────────────────────────────────────────────────

const TierResponseSchema = z.object({
  tier_id: z.string(),
  name_en: z.string(),
  name_ar: z.string(),
  level: z.number(),
  monthly_diamonds: z.number(),
  monthly_usd: z.string(),
  usd_sku_id: z.string().nullable(),
  privileges: z.unknown(),
  icon_url: z.string(),
  frame_url: z.string(),
  entrance_animation_url: z.string().nullable(),
  bgm_url: z.string().nullable(),
  badge_color: z.string(),
  bubble_style_id: z.string(),
  is_active: z.boolean(),
  created_at: z.string(),
  updated_at: z.string(),
});

const ListTiersDataSchema = z.object({
  items: z.array(TierResponseSchema),
  total: z.number(),
  page: z.number(),
  size: z.number(),
});

const NobleUserItemSchema = z.object({
  user_id: z.string().uuid(),
  nickname: z.string(),
  avatar_url: z.string().nullable(),
  tier_id: z.string(),
  tier_name_en: z.string(),
  tier_name_ar: z.string(),
  tier_level: z.number(),
  badge_color: z.string(),
  start_at: z.string(),
  current_period_start: z.string(),
  expire_at: z.string(),
  auto_renew: z.boolean(),
  renew_channel: z.string(),
  total_paid_diamonds: z.number(),
  total_paid_usd_micros: z.number(),
});

const ListNobleUsersDataSchema = z.object({
  items: z.array(NobleUserItemSchema),
  total: z.number(),
  page: z.number(),
  size: z.number(),
});

const NobleHistoryItemSchema = z.object({
  event: z.string(),
  from_tier: z.string().nullable(),
  to_tier: z.string().nullable(),
  actor: z.string(),
  created_at: z.string(),
});

const GrantNobleResponseSchema = z.object({
  user_id: z.string().uuid(),
  tier_id: z.string(),
  expire_at: z.string(),
});

const RevokeNobleResponseSchema = z.object({
  user_id: z.string().uuid(),
});

// ─── Exported Types ──────────────────────────────────────────────────────────────

export type TierItem = z.infer<typeof TierResponseSchema>;
export type ListTiersData = z.infer<typeof ListTiersDataSchema>;
export type NobleUserItem = z.infer<typeof NobleUserItemSchema>;
export type ListNobleUsersData = z.infer<typeof ListNobleUsersDataSchema>;
export type NobleHistoryItem = z.infer<typeof NobleHistoryItemSchema>;

// ─── Request Types ───────────────────────────────────────────────────────────────

export interface CreateTierRequest {
  tier_id: string;
  name_en: string;
  name_ar: string;
  level: number;
  monthly_diamonds: number;
  monthly_usd: string;
  usd_sku_id?: string;
  privileges: unknown;
  icon_url: string;
  frame_url: string;
  entrance_animation_url?: string;
  bgm_url?: string;
  badge_color: string;
  bubble_style_id: string;
}

export interface UpdateTierRequest {
  name_en?: string;
  name_ar?: string;
  monthly_diamonds?: number;
  monthly_usd?: string;
  usd_sku_id?: string;
  privileges?: unknown;
  icon_url?: string;
  frame_url?: string;
  entrance_animation_url?: string;
  bgm_url?: string;
  badge_color?: string;
  bubble_style_id?: string;
}

export interface ListNobleUsersParams {
  tier_id?: string;
  status?: 'active' | 'expired'; // 对应后端 NobleStatusFilter
  expire_before?: string;
  page?: number;
  size?: number;
}

// ─── API Functions ───────────────────────────────────────────────────────────────

/** GET /api/v1/admin/nobles/tiers */
export async function listNobleTiers(
  page = 1,
  size = 20,
  signal?: AbortSignal,
): Promise<ListTiersData> {
  const q = new URLSearchParams({ page: String(page), size: String(size) });
  const result = await adminFetch<ListTiersData>(
    `/api/v1/admin/nobles/tiers?${q.toString()}`,
    { signal },
  );
  return validateResponse(result.data, ListTiersDataSchema);
}

/** GET /api/v1/admin/nobles/tiers/:id */
export async function getNobleTier(
  tierId: string,
  signal?: AbortSignal,
): Promise<TierItem> {
  const result = await adminFetch<TierItem>(
    `/api/v1/admin/nobles/tiers/${tierId}`,
    { signal },
  );
  return validateResponse(result.data, TierResponseSchema);
}

/** POST /api/v1/admin/nobles/tiers */
export async function createNobleTier(
  req: CreateTierRequest,
  signal?: AbortSignal,
): Promise<TierItem> {
  const result = await adminFetch<TierItem>(
    '/api/v1/admin/nobles/tiers',
    { method: 'POST', body: JSON.stringify(req), signal },
  );
  return validateResponse(result.data, TierResponseSchema);
}

/** PUT /api/v1/admin/nobles/tiers/:id */
export async function updateNobleTier(
  tierId: string,
  req: UpdateTierRequest,
  signal?: AbortSignal,
): Promise<TierItem> {
  const result = await adminFetch<TierItem>(
    `/api/v1/admin/nobles/tiers/${tierId}`,
    { method: 'PUT', body: JSON.stringify(req), signal },
  );
  return validateResponse(result.data, TierResponseSchema);
}

/** DELETE /api/v1/admin/nobles/tiers/:id */
export async function deleteNobleTier(
  tierId: string,
  signal?: AbortSignal,
): Promise<void> {
  await adminFetch(`/api/v1/admin/nobles/tiers/${tierId}`, {
    method: 'DELETE',
    signal,
  });
}

/** GET /api/v1/admin/nobles/users */
export async function listNobleUsers(
  params: ListNobleUsersParams,
  signal?: AbortSignal,
): Promise<ListNobleUsersData> {
  const q = new URLSearchParams();
  if (params.tier_id) q.set('tier_id', params.tier_id);
  if (params.status) q.set('status', params.status);
  if (params.expire_before) q.set('expire_before', params.expire_before);
  if (params.page) q.set('page', String(params.page));
  if (params.size) q.set('size', String(params.size));

  const result = await adminFetch<ListNobleUsersData>(
    `/api/v1/admin/nobles/users?${q.toString()}`,
    { signal },
  );
  return validateResponse(result.data, ListNobleUsersDataSchema);
}

/** GET /api/v1/admin/nobles/users/:user_id/history */
export async function getNobleHistory(
  userId: string,
  signal?: AbortSignal,
): Promise<NobleHistoryItem[]> {
  const result = await adminFetch<NobleHistoryItem[]>(
    `/api/v1/admin/nobles/users/${userId}/history`,
    { signal },
  );
  return z.array(NobleHistoryItemSchema).parse(result.data);
}

/** POST /api/v1/admin/users/:id/noble/grant */
export async function grantNoble(
  userId: string,
  tierId: string,
  durationDays: number,
  reason: string,
  signal?: AbortSignal,
): Promise<{ user_id: string; tier_id: string; expire_at: string }> {
  const result = await adminFetch<z.infer<typeof GrantNobleResponseSchema>>(
    `/api/v1/admin/users/${userId}/noble/grant`,
    {
      method: 'POST',
      body: JSON.stringify({ tier_id: tierId, duration_days: durationDays, reason }),
      signal,
    },
  );
  return validateResponse(result.data, GrantNobleResponseSchema);
}

/** POST /api/v1/admin/users/:id/noble/revoke */
export async function revokeNoble(
  userId: string,
  reason: string,
  signal?: AbortSignal,
): Promise<{ user_id: string }> {
  const result = await adminFetch<z.infer<typeof RevokeNobleResponseSchema>>(
    `/api/v1/admin/users/${userId}/noble/revoke`,
    {
      method: 'POST',
      body: JSON.stringify({ reason }),
      signal,
    },
  );
  return validateResponse(result.data, RevokeNobleResponseSchema);
}
