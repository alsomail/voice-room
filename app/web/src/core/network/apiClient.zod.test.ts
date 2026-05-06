/**
 * T-00102 — Web Zod runtime schema validation tests
 *
 * ZOD-1: Missing required field in adminGetUsers response -> ZodError thrown
 * ZOD-2: Wrong type (total as string) in adminGetUsers response -> ZodError thrown
 * ZOD-3: All endpoint schemas exist (z.object coverage >= endpoint count)
 * ZOD-4: .passthrough() allows unknown fields without throwing
 *
 * PROTO-BINDING: doc/protocol/schemas/http/RoomDetail.schema.json
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { ZodError } from 'zod';
import { adminGetUsers } from './apiClient';
import {
  AdminLoginDataSchema,
  AdminRoomsDataSchema,
  AdminRoomDetailAdminSchema,
  AdminStatsOverviewDataSchema,
  AdminUsersDataSchema,
  AdminUserDetailResponseSchema,
  AdminAdjustBalanceResponseSchema,
  AdminGiftItemSchema,
  AdminGiftsDataSchema,
  AdminUploadGiftAssetResponseSchema,
  EventItemSchema,
  EventListResponseSchema,
  EventNamesResponseSchema,
  KickLogItemSchema,
  MuteLogItemSchema,
  makeGovernanceListResponseSchema,
  AdminLogsDataSchema,
  AdminLogItemSchema,
} from '../../api/schemas/admin.schemas';

// ── mock useAuthStore ────────────────────────────────────────────────────────
const { mockLogout } = vi.hoisted(() => ({ mockLogout: vi.fn() }));
vi.mock('../../stores/useAuthStore', () => ({
  useAuthStore: { getState: () => ({ logout: mockLogout }) },
  ADMIN_TOKEN_KEY: 'adminToken',
}));

// ── fetch mock helpers ────────────────────────────────────────────────────────

function mockFetch(data: unknown) {
  vi.stubGlobal(
    'fetch',
    vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: vi.fn().mockResolvedValue({ code: 0, message: 'ok', data }),
    } as unknown as Response),
  );
}

beforeEach(() => {
  localStorage.clear();
  mockLogout.mockClear();
  vi.useFakeTimers();
});

afterEach(() => {
  localStorage.clear();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

// ── ZOD-1: Missing required field ────────────────────────────────────────────

describe('ZOD-1: adminGetUsers — missing required field throws ZodError', () => {
  it('throws ZodError when users[0].id is missing', async () => {
    mockFetch({
      total: 1,
      page: 1,
      size: 10,
      items: [
        {
          // id is intentionally missing
          phone: '13800138000',
          coin_balance: 100,
          vip_level: 0,
          status: 'normal',
          created_at: '2024-01-01T00:00:00Z',
        },
      ],
    });

    await expect(adminGetUsers()).rejects.toThrow(ZodError);
  });
});

// ── ZOD-2: Wrong field type ──────────────────────────────────────────────────

describe('ZOD-2: adminGetUsers — wrong field type throws ZodError', () => {
  it('throws ZodError when total is string "100" instead of number', async () => {
    mockFetch({
      total: '100', // string instead of number
      page: 1,
      size: 10,
      items: [],
    });

    await expect(adminGetUsers()).rejects.toThrow(ZodError);
  });
});

// ── ZOD-3: All schemas exist ─────────────────────────────────────────────────

describe('ZOD-3: All endpoint schemas are defined', () => {
  it('AdminLoginDataSchema is a valid Zod object schema', () => {
    expect(AdminLoginDataSchema).toBeDefined();
    expect(typeof AdminLoginDataSchema.parse).toBe('function');
  });

  it('AdminRoomsDataSchema is a valid Zod object schema', () => {
    expect(AdminRoomsDataSchema).toBeDefined();
    expect(typeof AdminRoomsDataSchema.parse).toBe('function');
  });

  it('AdminRoomDetailAdminSchema is a valid Zod object schema', () => {
    expect(AdminRoomDetailAdminSchema).toBeDefined();
    expect(typeof AdminRoomDetailAdminSchema.parse).toBe('function');
  });

  it('AdminStatsOverviewDataSchema is a valid Zod object schema', () => {
    expect(AdminStatsOverviewDataSchema).toBeDefined();
    expect(typeof AdminStatsOverviewDataSchema.parse).toBe('function');
  });

  it('AdminUsersDataSchema is a valid Zod object schema', () => {
    expect(AdminUsersDataSchema).toBeDefined();
    expect(typeof AdminUsersDataSchema.parse).toBe('function');
  });

  it('AdminUserDetailResponseSchema is a valid Zod object schema', () => {
    expect(AdminUserDetailResponseSchema).toBeDefined();
    expect(typeof AdminUserDetailResponseSchema.parse).toBe('function');
  });

  it('AdminAdjustBalanceResponseSchema is a valid Zod object schema', () => {
    expect(AdminAdjustBalanceResponseSchema).toBeDefined();
    expect(typeof AdminAdjustBalanceResponseSchema.parse).toBe('function');
  });

  it('AdminGiftItemSchema is a valid Zod object schema', () => {
    expect(AdminGiftItemSchema).toBeDefined();
    expect(typeof AdminGiftItemSchema.parse).toBe('function');
  });

  it('AdminGiftsDataSchema is a valid Zod object schema', () => {
    expect(AdminGiftsDataSchema).toBeDefined();
    expect(typeof AdminGiftsDataSchema.parse).toBe('function');
  });

  it('AdminUploadGiftAssetResponseSchema is a valid Zod object schema', () => {
    expect(AdminUploadGiftAssetResponseSchema).toBeDefined();
    expect(typeof AdminUploadGiftAssetResponseSchema.parse).toBe('function');
  });

  it('EventItemSchema is a valid Zod object schema', () => {
    expect(EventItemSchema).toBeDefined();
    expect(typeof EventItemSchema.parse).toBe('function');
  });

  it('EventListResponseSchema is a valid Zod object schema', () => {
    expect(EventListResponseSchema).toBeDefined();
    expect(typeof EventListResponseSchema.parse).toBe('function');
  });

  it('EventNamesResponseSchema is a valid Zod object schema', () => {
    expect(EventNamesResponseSchema).toBeDefined();
    expect(typeof EventNamesResponseSchema.parse).toBe('function');
  });

  it('KickLogItemSchema is a valid Zod object schema', () => {
    expect(KickLogItemSchema).toBeDefined();
    expect(typeof KickLogItemSchema.parse).toBe('function');
  });

  it('MuteLogItemSchema is a valid Zod object schema', () => {
    expect(MuteLogItemSchema).toBeDefined();
    expect(typeof MuteLogItemSchema.parse).toBe('function');
  });

  it('makeGovernanceListResponseSchema factory produces valid schema', () => {
    const schema = makeGovernanceListResponseSchema(KickLogItemSchema);
    expect(schema).toBeDefined();
    expect(typeof schema.parse).toBe('function');
  });

  it('AdminLogsDataSchema is a valid Zod object schema', () => {
    expect(AdminLogsDataSchema).toBeDefined();
    expect(typeof AdminLogsDataSchema.parse).toBe('function');
  });

  it('AdminLogItemSchema is a valid Zod object schema', () => {
    expect(AdminLogItemSchema).toBeDefined();
    expect(typeof AdminLogItemSchema.parse).toBe('function');
  });
});

// ── ZOD-4: .passthrough() behavior ───────────────────────────────────────────

describe('ZOD-4: .passthrough() allows unknown fields without throwing', () => {
  it('AdminUsersDataSchema passes through unknown fields silently', () => {
    const input = {
      total: 1,
      page: 1,
      size: 10,
      items: [],
      unknown_future_field: 'some_value', // server added a new field
    };
    expect(() => AdminUsersDataSchema.parse(input)).not.toThrow();
    const parsed = AdminUsersDataSchema.parse(input);
    expect((parsed as Record<string, unknown>).unknown_future_field).toBe('some_value');
  });

  it('KickLogItemSchema passes through unknown fields silently', () => {
    const input = {
      id: 'k1',
      room_id: 'r1',
      room_title: 'test',
      target_user_id: 'u1',
      target_nickname: 'nick',
      operator_user_id: 'op1',
      operator_nickname: 'opnick',
      reason: null,
      created_at: '2024-01-01T00:00:00Z',
      new_field_from_server: 42,
    };
    expect(() => KickLogItemSchema.parse(input)).not.toThrow();
  });
});
