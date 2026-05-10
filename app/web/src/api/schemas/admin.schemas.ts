// PROTO-BINDING: doc/protocol/schemas/http/RoomDetail.schema.json

import { z } from 'zod';

// ─── Auth ────────────────────────────────────────────────────────────────────

export const AdminLoginDataSchema = z
  .object({
    token: z.string(),
    expires_in: z.number(),
    admin: z
      .object({
        id: z.string(),
        username: z.string(),
        role: z.string(),
        display_name: z.string(),
        last_login_at: z.string(),
      })
      .passthrough(),
  })
  .passthrough();

// ─── Rooms ───────────────────────────────────────────────────────────────────

export const AdminRoomItemSchema = z
  .object({
    id: z.string(),
    room_id: z.string().optional(),
    title: z.string(),
    room_type: z.enum(['normal', 'password', 'paid']),
    member_count: z.number(),
    max_members: z.number(),
    status: z.enum(['active', 'closed']),
    owner_id: z.string(),
    owner_nickname: z.string(),
    owner_avatar: z.string().nullable(),
    created_at: z.string(),
  })
  .passthrough();

export const AdminRoomsDataSchema = z
  .object({
    total: z.number(),
    page: z.number(),
    page_size: z.number(),
    items: z.array(AdminRoomItemSchema),
  })
  .passthrough();

// ─── Room Detail (Admin endpoint — different from app RoomDetail) ─────────────

// PROTO-BINDING: doc/protocol/schemas/http/RoomDetail.schema.json
// mic_slots items require mic_index, locked, muted as per JSON-Schema spec
export const MicSlotSchema = z
  .object({
    mic_index: z.number().int().min(0).max(8),
    user_id: z.string().nullable().optional(),
    locked: z.boolean(),
    muted: z.boolean(),
  })
  .passthrough();

export const AdminRoomDetailAdminSchema = z
  .object({
    room_id: z.string(),
    title: z.string(),
    status: z.enum(['active', 'closed']),
    room_type: z.enum(['normal', 'password', 'paid']),
    member_count: z.number(),
    max_members: z.number(),
    owner: z
      .object({
        user_id: z.string(),
        nickname: z.string(),
        avatar: z.string().nullable(),
      })
      .passthrough(),
    mic_slots: z.array(MicSlotSchema),
    created_at: z.string(),
    updated_at: z.string(),
  })
  .passthrough();

// ─── Stats ───────────────────────────────────────────────────────────────────

export const AdminStatsTrendPointSchema = z
  .object({
    date: z.string(),
    dau: z.number(),
    new_users: z.number(),
  })
  .passthrough();

export const AdminStatsOverviewDataSchema = z
  .object({
    online_users: z.number(),
    dau: z.number(),
    new_users: z.number(),
    trend: z.array(AdminStatsTrendPointSchema).optional(),
  })
  .passthrough();

// ─── Users ───────────────────────────────────────────────────────────────────

export const AdminUserItemSchema = z
  .object({
    id: z.string(),
    phone: z.string(),
    nickname: z.string().nullable().optional(),
    avatar: z.string().nullable().optional(),
    coin_balance: z.number(),
    vip_level: z.number(),
    status: z.enum(['normal', 'banned']),
    created_at: z.string(),
  })
  .passthrough();

export const AdminUsersDataSchema = z
  .object({
    total: z.number(),
    page: z.number(),
    size: z.number(),
    items: z.array(AdminUserItemSchema),
  })
  .passthrough();

export const AdminUserDetailResponseSchema = z
  .object({
    id: z.string(),
    phone: z.string(),
    nickname: z.string(),
    avatar_url: z.string().nullable(),
    coin_balance: z.number(),
    vip_level: z.number(),
    status: z.enum(['normal', 'banned']),
    created_at: z.string(),
    recharge_records: z.array(z.unknown()),
    consume_records: z.array(z.unknown()),
    devices: z.array(z.unknown()),
  })
  .passthrough();

// ─── Wallet ──────────────────────────────────────────────────────────────────

export const AdminAdjustBalanceResponseSchema = z
  .object({
    new_balance: z.number(),
  })
  .passthrough();

// ─── Gifts ───────────────────────────────────────────────────────────────────

export const AdminGiftItemSchema = z
  .object({
    id: z.string(),
    code: z.string(),
    name_en: z.string(),
    name_ar: z.string(),
    icon_url: z.string(),
    price: z.number(),
    tier: z.number(),
    effect_level: z.number(),
    animation_url: z.string().nullable(),
    is_active: z.boolean(),
    sort_order: z.number(),
    is_deleted: z.boolean(),
    created_at: z.string(),
    updated_at: z.string(),
  })
  .passthrough();

export const AdminGiftsDataSchema = z
  .object({
    total: z.number(),
    page: z.number(),
    size: z.number(),
    items: z.array(AdminGiftItemSchema),
  })
  .passthrough();

export const AdminUploadGiftAssetResponseSchema = z
  .object({
    url: z.string(),
    file_name: z.string(),
  })
  .passthrough();

// ─── Events ──────────────────────────────────────────────────────────────────

export const EventItemSchema = z
  .object({
    id: z.string(),
    event_name: z.string(),
    server_ts: z.string(),
    client_ts: z.string().nullable(),
    session_id: z.string().nullable(),
    device_id: z.string().nullable(),
    properties: z.record(z.string(), z.unknown()).nullable(),
    app_version: z.string().nullable(),
    os_version: z.string().nullable(),
    locale: z.string().nullable(),
    network_type: z.string().nullable(),
  })
  .passthrough();

export const EventListResponseSchema = z
  .object({
    total: z.number(),
    page: z.number(),
    limit: z.number(),
    items: z.array(EventItemSchema),
  })
  .passthrough();

export const EventNamesResponseSchema = z
  .object({
    items: z.array(z.string()),
  })
  .passthrough();

// ─── Governance ──────────────────────────────────────────────────────────────

export const KickLogItemSchema = z
  .object({
    id: z.string(),
    room_id: z.string(),
    room_title: z.string(),
    target_user_id: z.string(),
    target_nickname: z.string(),
    operator_user_id: z.string(),
    operator_nickname: z.string(),
    reason: z.string().nullable(),
    created_at: z.string(),
  })
  .passthrough();

export const MuteLogItemSchema = z
  .object({
    id: z.string(),
    room_id: z.string(),
    room_title: z.string(),
    target_user_id: z.string(),
    target_nickname: z.string(),
    operator_user_id: z.string(),
    operator_nickname: z.string(),
    type: z.enum(['mic', 'chat']),
    duration_sec: z.number().nullable(),
    reason: z.string().nullable(),
    created_at: z.string(),
  })
  .passthrough();

export function makeGovernanceListResponseSchema<T extends z.ZodTypeAny>(itemSchema: T) {
  return z
    .object({
      total: z.number(),
      page: z.number(),
      limit: z.number(),
      items: z.array(itemSchema),
    })
    .passthrough();
}

// ─── Logs ────────────────────────────────────────────────────────────────────

export const AdminLogItemSchema = z
  .object({
    id: z.string(),
    admin_id: z.string(),
    action: z.string(),
    target_type: z.string().optional(),
    target_id: z.string().optional(),
    ip_address: z.string().optional(),
    detail: z.record(z.string(), z.unknown()).optional(),
    created_at: z.string(),
  })
  .passthrough();

export const AdminLogsDataSchema = z
  .object({
    total: z.number(),
    page: z.number(),
    size: z.number(),
    items: z.array(AdminLogItemSchema),
  })
  .passthrough();
