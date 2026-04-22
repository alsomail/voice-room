/**
 * Admin Server HTTP 客户端
 *
 * Base URL：VITE_ADMIN_API_BASE_URL（默认 http://localhost:3001/api/v1/admin）
 * 协议契约：doc/protocol.md §三 Admin 认证模块
 *
 * 功能：
 *   - 自动附加 Content-Type: application/json
 *   - 若 localStorage 存有 JWT，自动附加 Authorization: Bearer <token>
 *   - 统一解析响应结构 { code, message, data }
 *   - code !== 0 时抛出携带 message 的 Error
 *   - [HIGH-H01] 401 且有 session token 时：logout() + 跳转 /login
 *
 * 依赖方向：apiClient → useAuthStore（useAuthStore 通过 adminLogin 反向依赖，形成循环）
 * 循环依赖是安全的：useAuthStore 仅在函数体内使用，不在模块初始化时调用。
 */

// [L01] 从 useAuthStore 导入共享常量，消除重复定义
import { ADMIN_TOKEN_KEY, useAuthStore } from '../../stores/useAuthStore';

/** 统一响应结构（protocol.md §1.3） */
export interface ApiResponse<T = unknown> {
  code: number;
  message: string;
  data: T;
  request_id?: string;
}

/** 管理员登录请求体 */
export interface AdminLoginRequest {
  username: string;
  password: string;
}

/** 管理员登录响应 data */
export interface AdminLoginData {
  token: string;
  expires_in: number;
  admin: {
    id: string;
    username: string;
    role: string;
    display_name: string;
    last_login_at: string;
  };
}

function getAdminApiBaseUrl(): string {
  return (
    (import.meta.env.VITE_ADMIN_API_BASE_URL as string | undefined) ??
    'http://localhost:3001/api/v1/admin'
  );
}

async function adminFetch<T>(
  path: string,
  init?: RequestInit,
): Promise<ApiResponse<T>> {
  const base = getAdminApiBaseUrl().replace(/\/$/, '');
  const url = `${base}${path}`;

  const token = localStorage.getItem(ADMIN_TOKEN_KEY);
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...(init?.headers as Record<string, string> | undefined),
  };

  // [MEDIUM-2] 15 秒超时控制
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 15_000);

  // [M-02] 若调用方传入外部 signal（如卸载时的 AbortController），
  //        则在其触发时同步中止内部 controller，两者任一触发均可取消 fetch
  if (init?.signal) {
    if (init.signal.aborted) {
      clearTimeout(timer);
      controller.abort();
    } else {
      init.signal.addEventListener('abort', () => controller.abort(), {
        once: true,
      });
    }
  }

  try {
    const response = await fetch(url, { ...init, headers, signal: controller.signal });

    // [MEDIUM-1] 检查 HTTP 响应状态，非 2xx 时提取错误信息并抛出
    if (!response.ok) {
      // [HIGH-H01] 401 且存在 session token：说明 token 已过期，自动 logout + 跳转 /login
      // 若无 token（例如登录请求返回 401），则走普通错误路径，向 UI 透传 body.message
      if (response.status === 401 && token) {
        useAuthStore.getState().logout();
        window.location.href = '/login';
        throw new Error('Unauthorized');
      }

      let message = `HTTP Error ${response.status}`;
      try {
        const errBody = await response.json();
        message = (errBody as { message?: string }).message || message;
      } catch {
        // JSON 解析失败时保留 "HTTP Error <status>" 默认信息
      }
      throw new Error(message);
    }

    const body = (await response.json()) as ApiResponse<T>;

    if (body.code !== 0) {
      throw new Error(body.message);
    }

    return body;
  } finally {
    clearTimeout(timer);
  }
}

/**
 * POST /login — 管理员账号密码登录（protocol.md §3.1）
 */
export async function adminLogin(
  req: AdminLoginRequest,
): Promise<AdminLoginData> {
  const res = await adminFetch<AdminLoginData>('/login', {
    method: 'POST',
    body: JSON.stringify(req),
  });
  return res.data;
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin Rooms（T-20003）
// ─────────────────────────────────────────────────────────────────────────────

/** GET /admin/rooms 查询参数（protocol.md §4.4） */
export interface AdminGetRoomsParams {
  page?: number;
  page_size?: number;
  status?: 'active' | 'closed';
  keyword?: string;
}

/** 单个房间条目 */
export interface AdminRoomItem {
  room_id: string;
  title: string;
  room_type: 'normal' | 'password' | 'paid';
  member_count: number;
  max_members: number;
  status: 'active' | 'closed';
  owner_id: string;
  owner_nickname: string;
  owner_avatar: string | null;
  created_at: string;
}

/** GET /admin/rooms 响应 data */
export interface AdminRoomsData {
  total: number;
  page: number;
  page_size: number;
  items: AdminRoomItem[];
}

/**
 * GET /admin/rooms — 管理员查询房间列表（protocol.md §4.4）
 * T-20003: 用于统计总房间数和活跃房间数
 */
export async function adminGetRooms(
  params?: AdminGetRoomsParams,
  signal?: AbortSignal,
): Promise<AdminRoomsData> {
  const query = params
    ? '?' + new URLSearchParams(
        Object.entries(params)
          .filter(([, v]) => v !== undefined)
          .map(([k, v]) => [k, String(v)]),
      ).toString()
    : '';
  const res = await adminFetch<AdminRoomsData>(`/rooms${query}`, { signal });
  return res.data;
}

/**
 * DELETE /rooms/:id — 管理员强制关闭房间（T-20004）
 */
export async function adminCloseRoom(roomId: string): Promise<void> {
  await adminFetch<null>(`/rooms/${encodeURIComponent(roomId)}`, {
    method: 'DELETE',
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin Room Detail（T-20005，对应 T-10005 后端接口）
// ─────────────────────────────────────────────────────────────────────────────

/** 房间详情中的房主信息（protocol.md §4.5） */
export interface AdminRoomDetailOwner {
  user_id: string;
  nickname: string;
  avatar: string | null;
}

/** GET /admin/rooms/:id 响应 data（protocol.md §4.5） */
export interface AdminRoomDetail {
  room_id: string;
  title: string;
  status: 'active' | 'closed';
  room_type: 'normal' | 'password' | 'paid';
  member_count: number;
  max_members: number;
  owner: AdminRoomDetailOwner;
  mic_slots: unknown[];
  created_at: string;
  updated_at: string;
}

/**
 * GET /admin/rooms/:id — 管理员获取房间详情（protocol.md §4.5）
 * T-20005: 用于 RoomDetailModal 展示房间详细信息
 */
export async function adminGetRoomDetail(
  roomId: string,
  signal?: AbortSignal,
): Promise<AdminRoomDetail> {
  const res = await adminFetch<AdminRoomDetail>(
    `/rooms/${encodeURIComponent(roomId)}`,
    { signal },
  );
  return res.data;
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin Stats Overview（T-20003，对应 T-10010 后端接口）
// ─────────────────────────────────────────────────────────────────────────────

/** 趋势图单个数据点 */
export interface AdminStatsTrendPoint {
  date: string;
  dau: number;
  new_users: number;
}

/** GET /admin/stats/overview 响应 data */
export interface AdminStatsOverviewData {
  /** 当前在线人数（来自 Redis 实时统计） */
  online_users: number;
  /** 今日 DAU */
  dau: number;
  /** 今日新增用户数 */
  new_users_today: number;
  /** 历史趋势（最近 N 天） */
  trend: AdminStatsTrendPoint[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin Users（T-20006，对应 T-10007 后端接口）
// ─────────────────────────────────────────────────────────────────────────────

/** GET /admin/users 查询参数（T-10007） */
export interface AdminGetUsersParams {
  phone?: string;
  nickname?: string;
  user_id?: string;
  status?: 'normal' | 'banned';
  page?: number;
  size?: number;
}

/** 单个用户条目（T-10007） */
export interface AdminUserItem {
  id: string;
  phone: string;
  nickname?: string;
  avatar?: string;
  coin_balance: number;
  vip_level: number;
  status: 'normal' | 'banned';
  created_at: string;
}

/** GET /admin/users 响应 data */
export interface AdminUsersData {
  total: number;
  page: number;
  size: number;
  items: AdminUserItem[];
}

/**
 * GET /admin/users — 管理员查询用户列表（T-10007）
 * T-20006: 用于用户管理页面展示用户列表
 */
export async function adminGetUsers(
  params?: AdminGetUsersParams,
  signal?: AbortSignal,
): Promise<AdminUsersData> {
  const query = params
    ? '?' + new URLSearchParams(
        Object.entries(params)
          .filter(([, v]) => v !== undefined)
          .map(([k, v]) => [k, String(v)]),
      ).toString()
    : '';
  const res = await adminFetch<AdminUsersData>(`/users${query}`, { signal });
  return res.data;
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin User Detail（T-20007，对应 T-10008 后端接口）
// ─────────────────────────────────────────────────────────────────────────────

/** GET /admin/users/:id 响应 data */
export interface AdminUserDetailResponse {
  id: string;
  phone: string;
  nickname: string;
  avatar_url: string | null;
  coin_balance: number;
  vip_level: number;
  status: 'normal' | 'banned';
  created_at: string;
  recharge_records: unknown[];
  consume_records: unknown[];
  devices: unknown[];
}

/**
 * GET /admin/users/:id — 管理员获取用户详情（T-20007）
 * 用于 UserDetailDrawer 展示用户详细信息
 */
export async function adminGetUserDetail(
  userId: string,
  signal?: AbortSignal,
): Promise<AdminUserDetailResponse> {
  const res = await adminFetch<AdminUserDetailResponse>(
    `/users/${encodeURIComponent(userId)}`,
    { signal },
  );
  return res.data;
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin Ban User（T-20008，对应 T-10009 后端接口）
// ─────────────────────────────────────────────────────────────────────────────

/** POST /admin/users/:id/ban 请求体 */
export interface AdminBanUserRequest {
  action: 'ban' | 'unban';
  duration?: number | null; // 分钟，null=永久
  reason?: string;
  remark?: string;
}

/**
 * POST /admin/users/:id/ban — 管理员封禁/解封用户（T-20008）
 * action='ban'   → 封禁用户，duration=null 表示永久封禁
 * action='unban' → 解封用户
 */
export async function adminBanUser(
  userId: string,
  req: AdminBanUserRequest,
  signal?: AbortSignal,
): Promise<void> {
  await adminFetch<null>(`/users/${encodeURIComponent(userId)}/ban`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
    signal,
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin Logs（T-20009，对应 T-10012 后端接口）
// ─────────────────────────────────────────────────────────────────────────────

/** GET /admin/logs 查询参数（T-10012） */
export interface AdminGetLogsParams {
  admin_id?: string;
  action?: string;
  start_date?: string;  // RFC3339
  end_date?: string;    // RFC3339
  page?: number;
  size?: number;
}

/** 单条审计日志条目（T-10012） */
export interface AdminLogItem {
  id: string;
  admin_id: string;
  action: string;
  target_type?: string;
  target_id?: string;
  ip_address?: string;
  detail?: Record<string, unknown>;
  created_at: string;
}

/** GET /admin/logs 响应 data */
export interface AdminLogsData {
  total: number;
  page: number;
  size: number;
  items: AdminLogItem[];
}

/**
 * GET /admin/logs — 管理员查询操作审计日志（T-10012）
 * T-20009: 用于操作日志页面展示审计列表
 */
export async function adminGetLogs(
  params?: AdminGetLogsParams,
  signal?: AbortSignal,
): Promise<AdminLogsData> {
  const query = params
    ? '?' + new URLSearchParams(
        Object.entries(params)
          .filter(([, v]) => v !== undefined)
          .map(([k, v]) => [k, String(v)]),
      ).toString()
    : '';
  const res = await adminFetch<AdminLogsData>(`/logs${query}`, { signal });
  return res.data;
}

/**
 * GET /admin/stats/overview — 数据统计概览（T-10010 契约）
 * T-20003: 用于在线人数、DAU、新增用户、趋势图
 */
export async function adminGetStatsOverview(
  signal?: AbortSignal,
): Promise<AdminStatsOverviewData> {
  const res = await adminFetch<AdminStatsOverviewData>('/stats/overview', {
    signal,
  });
  return res.data;
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin Unban User（T-20010，对应 PUT /admin/users/:id/unban）
// ─────────────────────────────────────────────────────────────────────────────

/** PUT /admin/users/:id/unban 请求体 */
export interface AdminUnbanUserRequest {
  reason: string;
  remark?: string;
}

/**
 * PUT /admin/users/:id/unban — 管理员解封用户（T-20010）
 * 成功 200：{ code: 0, data: null }
 * 40901：用户当前未被封禁（幂等）
 */
export async function adminUnbanUser(
  userId: string,
  req: AdminUnbanUserRequest,
  signal?: AbortSignal,
): Promise<void> {
  await adminFetch<null>(`/users/${encodeURIComponent(userId)}/unban`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
    signal,
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin Wallet Adjust（T-20012，对应 T-10013 后端接口）
// ─────────────────────────────────────────────────────────────────────────────

/** POST /admin/users/:id/wallet/adjust 请求体 */
export interface AdminAdjustBalanceRequest {
  amount: number;   // 正数=增加，负数=扣减，非零，|amount| ≤ 10,000,000
  reason: string;   // 2-200 字符，必填
}

/** POST /admin/users/:id/wallet/adjust 响应 data */
export interface AdminAdjustBalanceResponse {
  new_balance: number;
}

/**
 * POST /admin/users/:id/wallet/adjust — 手动调整用户余额（T-20012）
 * 正数=充值，负数=扣减；成功返回调整后的新余额
 */
export async function adminAdjustBalance(
  userId: string,
  req: AdminAdjustBalanceRequest,
): Promise<AdminAdjustBalanceResponse> {
  const res = await adminFetch<AdminAdjustBalanceResponse>(
    `/users/${encodeURIComponent(userId)}/wallet/adjust`,
    {
      method: 'POST',
      body: JSON.stringify(req),
    },
  );
  return res.data;
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin Gift CRUD（T-20012，对应 T-10014 后端接口）
// ─────────────────────────────────────────────────────────────────────────────

/** 单个礼物条目（T-10014） */
export interface AdminGiftItem {
  id: string;
  code: string;
  name_en: string;
  name_ar: string;
  icon_url: string;
  price: number;
  tier: number;
  effect_level: number;
  animation_url: string | null;
  is_active: boolean;
  sort_order: number;
  is_deleted: boolean;
  created_at: string;
  updated_at: string;
}

/** GET /admin/gifts 查询参数 */
export interface AdminListGiftsParams {
  include_inactive?: boolean;
  page?: number;
  size?: number;
  tier?: number;
}

/** GET /admin/gifts 响应 data */
export interface AdminGiftsData {
  total: number;
  page: number;
  size: number;
  items: AdminGiftItem[];
}

/** POST /admin/gifts 请求体 */
export interface AdminCreateGiftRequest {
  code: string;
  name_en: string;
  name_ar: string;
  icon_url: string;
  price: number;
  tier: number;
  effect_level: number;
  animation_url?: string;
  sort_order?: number;
  is_active?: boolean;
}

/** PUT /admin/gifts/:id 请求体（所有字段可选） */
export type AdminUpdateGiftRequest = Partial<AdminCreateGiftRequest>;

/** POST /admin/gifts/upload 响应 data */
export interface AdminUploadGiftAssetResponse {
  url: string;
  file_name: string;
}

/**
 * GET /admin/gifts — 管理员查询礼物列表（T-20012）
 */
export async function adminListGifts(
  params?: AdminListGiftsParams,
  signal?: AbortSignal,
): Promise<AdminGiftsData> {
  const query = params
    ? '?' + new URLSearchParams(
        Object.entries(params)
          .filter(([, v]) => v !== undefined)
          .map(([k, v]) => [k, String(v)]),
      ).toString()
    : '';
  const res = await adminFetch<AdminGiftsData>(`/gifts${query}`, { signal });
  return res.data;
}

/**
 * POST /admin/gifts — 管理员创建礼物（T-20012）
 */
export async function adminCreateGift(
  req: AdminCreateGiftRequest,
  signal?: AbortSignal,
): Promise<AdminGiftItem> {
  const res = await adminFetch<AdminGiftItem>('/gifts', {
    method: 'POST',
    body: JSON.stringify(req),
    signal,
  });
  return res.data;
}

/**
 * PUT /admin/gifts/:id — 管理员更新礼物（T-20012）
 */
export async function adminUpdateGift(
  id: string,
  req: AdminUpdateGiftRequest,
  signal?: AbortSignal,
): Promise<AdminGiftItem> {
  const res = await adminFetch<AdminGiftItem>(
    `/gifts/${encodeURIComponent(id)}`,
    {
      method: 'PUT',
      body: JSON.stringify(req),
      signal,
    },
  );
  return res.data;
}

/**
 * DELETE /admin/gifts/:id — 管理员软删除礼物（T-20012）
 */
export async function adminDeleteGift(id: string, signal?: AbortSignal): Promise<void> {
  await adminFetch<null>(`/gifts/${encodeURIComponent(id)}`, {
    method: 'DELETE',
    signal,
  });
}

/**
 * POST /admin/gifts/upload — 管理员上传礼物图片/Lottie（T-20012）
 * kind='icon'      → 图片（PNG/JPEG/WEBP，≤1MB）
 * kind='animation' → Lottie JSON，≤2MB
 */
export async function adminUploadGiftAsset(
  file: File,
  kind: 'icon' | 'animation',
): Promise<AdminUploadGiftAssetResponse> {
  const base = getAdminApiBaseUrl().replace(/\/$/, '');
  const url = `${base}/gifts/upload`;

  const token = localStorage.getItem(ADMIN_TOKEN_KEY);
  const headers: Record<string, string> = token
    ? { Authorization: `Bearer ${token}` }
    : {};

  const formData = new FormData();
  formData.append('file', file);
  formData.append('kind', kind);

  const response = await fetch(url, {
    method: 'POST',
    headers,
    body: formData,
  });

  if (!response.ok) {
    let message = `HTTP Error ${response.status}`;
    try {
      const errBody = await response.json();
      message = (errBody as { message?: string }).message || message;
    } catch {
      // ignore
    }
    throw new Error(message);
  }

  const body = (await response.json()) as ApiResponse<AdminUploadGiftAssetResponse>;
  if (body.code !== 0) throw new Error(body.message);
  return body.data;
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin User Events（T-20013，对应 T-10015 analytics 接口）
// ─────────────────────────────────────────────────────────────────────────────

/** 单条事件条目（T-10015 analytics.md §2.2） */
export interface EventItem {
  id: string;
  event_name: string;
  server_ts: string;        // ISO8601
  client_ts: string | null; // ISO8601
  session_id: string | null;
  device_id: string | null;
  properties: Record<string, unknown> | null;
  app_version: string | null;
  os_version: string | null;
  locale: string | null;
  network_type: string | null;
}

/** GET /admin/users/:id/events 查询参数（T-10015） */
export interface EventListParams {
  event_name?: string;  // 逗号分隔，如 "gift_send_success,coin_exchange"
  from?: string;        // ISO8601，默认 24h 前
  to?: string;          // ISO8601，默认现在
  page?: number;        // 默认 1
  limit?: number;       // 默认 20，max 100
}

/** GET /admin/users/:id/events 响应 data（T-10015） */
export interface EventListResponse {
  total: number;
  page: number;
  limit: number;
  items: EventItem[];
}

/**
 * GET /admin/users/:id/events — 查询用户埋点事件流（T-20013）
 * 对应 T-10015 analytics 接口
 */
export async function listUserEvents(
  userId: string,
  params?: EventListParams,
  signal?: AbortSignal,
): Promise<EventListResponse> {
  const query = params
    ? '?' + new URLSearchParams(
        Object.entries(params)
          .filter(([, v]) => v !== undefined)
          .map(([k, v]) => [k, String(v)]),
      ).toString()
    : '';
  const res = await adminFetch<EventListResponse>(
    `/users/${encodeURIComponent(userId)}/events${query}`,
    { signal },
  );
  return res.data;
}

// ─────────────────────────────────────────────────────────────────────────────
// Admin Governance（T-20014，对应 T-10016 后端接口）
// ─────────────────────────────────────────────────────────────────────────────

/** 踢人记录条目（T-10016） */
export interface KickLogItem {
  id: string;
  room_id: string;
  room_title: string;
  target_user_id: string;
  target_nickname: string;
  operator_user_id: string;
  operator_nickname: string;
  reason: string | null;
  created_at: string;
}

/** 禁言记录条目（T-10016）*/
export interface MuteLogItem {
  id: string;
  room_id: string;
  room_title: string;
  target_user_id: string;
  target_nickname: string;
  operator_user_id: string;
  operator_nickname: string;
  type: 'mic' | 'chat';
  duration_sec: number | null;
  reason: string | null;
  created_at: string;
}

/** GET /admin/governance/kicks + /mutes 通用查询参数（T-10016） */
export interface GovernanceListParams {
  room_id?: string;
  target_user_id?: string;
  operator_user_id?: string;
  from?: string;   // ISO8601，默认 7 天前
  to?: string;     // ISO8601，默认现在
  page?: number;
  limit?: number;
}

/** GET /admin/governance/mutes 专属参数 */
export interface MuteListParams extends GovernanceListParams {
  type?: 'mic' | 'chat';
}

/** 通用治理日志分页响应 */
export interface GovernanceListResponse<T> {
  total: number;
  page: number;
  limit: number;
  items: T[];
}

/**
 * GET /admin/governance/kicks — 踢人记录查询（T-20014）
 * 权限：super_admin / operator / cs；finance 禁止（403）
 */
export async function listKicks(
  params?: GovernanceListParams,
  signal?: AbortSignal,
): Promise<GovernanceListResponse<KickLogItem>> {
  const query = params
    ? '?' + new URLSearchParams(
        Object.entries(params)
          .filter(([, v]) => v !== undefined)
          .map(([k, v]) => [k, String(v)]),
      ).toString()
    : '';
  const res = await adminFetch<GovernanceListResponse<KickLogItem>>(
    `/governance/kicks${query}`,
    { signal },
  );
  return res.data;
}

/**
 * GET /admin/governance/mutes — 禁言记录查询（T-20014）
 * 权限：super_admin / operator / cs；finance 禁止（403）
 */
export async function listMutes(
  params?: MuteListParams,
  signal?: AbortSignal,
): Promise<GovernanceListResponse<MuteLogItem>> {
  const query = params
    ? '?' + new URLSearchParams(
        Object.entries(params)
          .filter(([, v]) => v !== undefined)
          .map(([k, v]) => [k, String(v)]),
      ).toString()
    : '';
  const res = await adminFetch<GovernanceListResponse<MuteLogItem>>(
    `/governance/mutes${query}`,
    { signal },
  );
  return res.data;
}
