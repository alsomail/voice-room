/**
 * fixtures.ts — 跨语言 WS E2E 测试的环境与 HTTP API 辅助工具。
 *
 * PROTO-BINDING:
 *   Android: RoomViewModel.joinRoom → POST /api/v1/rooms (创建房间)
 *   Server:  app/server/src/room/handler/lifecycle.rs::handle_join_room
 *
 * 加载顺序（优先级高→低）：
 *   1. process.env (shell / CI 注入)
 *   2. tests/scripts/env/.env.local
 *   3. 内置默认值 (CROSS_LANG_SERVER_URL / API_URL)
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import * as dotenv from 'dotenv';

// ─────────────────────────────────────────────────────────────────────────────
// 环境变量加载
// ─────────────────────────────────────────────────────────────────────────────

/** 仅在首次调用时加载一次 .env.local */
let envLoaded = false;
function ensureEnvLoaded(): void {
  if (envLoaded) return;
  envLoaded = true;

  // 尝试加载 tests/scripts/env/.env.local（不强制，缺失时静默）
  const envPath = path.resolve(
    __dirname,
    '../../../../tests/scripts/env/.env.local',
  );
  if (fs.existsSync(envPath)) {
    dotenv.config({ path: envPath, override: false });
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// 公开配置读取
// ─────────────────────────────────────────────────────────────────────────────

/** 默认服务地址（与任务说明一致） */
const DEFAULT_WS_URL = 'ws://192.168.1.8:3000/ws';
const DEFAULT_API_URL = 'http://192.168.1.8:3000';

export interface CrossLangEnv {
  /** WebSocket 地址，例如 ws://192.168.1.8:3000/ws */
  wsUrl: string;
  /** HTTP API 地址，例如 http://192.168.1.8:3000 */
  apiUrl: string;
  /** 普通用户 JWT（来自 E2E_TOKEN_USER1 → E2E_VALID_TOKEN） */
  userToken: string;
  /** Admin JWT（来自 E2E_TOKEN_ADMIN → E2E_ADMIN_TOKEN） */
  adminToken: string;
}

/**
 * 读取跨语言测试所需的环境配置。
 * 所有字段均有默认值，确保读取不抛异常。
 */
export function getCrossLangEnv(): CrossLangEnv {
  ensureEnvLoaded();

  const get = (key: string): string => process.env[key] ?? '';

  // WS URL：CROSS_LANG_SERVER_URL → APP_WS_URL → 默认值
  const wsUrl =
    get('CROSS_LANG_SERVER_URL') ||
    get('APP_WS_URL') ||
    DEFAULT_WS_URL;

  // HTTP API URL：从 WS URL 推导 或 APP_SERVER_BASE_URL
  const apiUrl =
    get('APP_SERVER_BASE_URL') ||
    DEFAULT_API_URL;

  // Token：E2E_TOKEN_USER1 → E2E_VALID_TOKEN → ''
  const userToken =
    get('E2E_TOKEN_USER1') ||
    get('E2E_VALID_TOKEN') ||
    '';

  // Admin Token：E2E_TOKEN_ADMIN → E2E_ADMIN_TOKEN → ''
  const adminToken =
    get('E2E_TOKEN_ADMIN') ||
    get('E2E_ADMIN_TOKEN') ||
    '';

  return { wsUrl, apiUrl, userToken, adminToken };
}

// ─────────────────────────────────────────────────────────────────────────────
// HTTP API 辅助
// ─────────────────────────────────────────────────────────────────────────────

export interface CreateRoomResult {
  room_id: string;
  title: string;
}

/**
 * POST /api/v1/rooms — 创建测试专用房间。
 *
 * 若用户已有活跃房间 (HTTP 409)，尝试从房间列表中找到该用户的房间。
 * 若 HTTP 请求失败（服务不可达），抛出 Error。
 *
 * @param apiUrl  HTTP API 基地址
 * @param token   创建者 JWT
 * @param title   房间标题（可选，默认 "CrossLang-Test"）
 */
export async function createOrGetRoom(
  apiUrl: string,
  token: string,
  title = 'CrossLang-Test',
): Promise<CreateRoomResult> {
  const url = `${apiUrl}/api/v1/rooms`;
  let resp: Response;

  try {
    resp = await fetch(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify({ title, room_type: 'normal', password: null }),
      signal: AbortSignal.timeout(8000),
    });
  } catch (err) {
    throw new Error(
      `[fixtures] HTTP request failed (server unreachable?): ${String(err)}`,
    );
  }

  // 成功 201
  if (resp.status === 201) {
    const body = (await resp.json()) as {
      code: number;
      data: { room_id: string; title: string };
    };
    if (body.code === 0) {
      return { room_id: body.data.room_id, title: body.data.title };
    }
    throw new Error(`[fixtures] createRoom failed: code=${body.code}`);
  }

  // 409 — 用户已有活跃房间，从列表中找
  if (resp.status === 409) {
    return getRoomForUser(apiUrl, token);
  }

  const text = await resp.text();
  throw new Error(
    `[fixtures] createRoom unexpected HTTP ${resp.status}: ${text}`,
  );
}

/**
 * GET /api/v1/rooms — 获取房间列表，取第一个（用于 409 fallback）。
 */
async function getRoomForUser(
  apiUrl: string,
  token: string,
): Promise<CreateRoomResult> {
  const resp = await fetch(`${apiUrl}/api/v1/rooms`, {
    headers: { Authorization: `Bearer ${token}` },
    signal: AbortSignal.timeout(8000),
  });
  const body = (await resp.json()) as {
    code: number;
    data: { rooms: Array<{ room_id: string; title: string }> };
  };
  const rooms = body.data?.rooms ?? [];
  if (rooms.length === 0) {
    throw new Error('[fixtures] getRoomForUser: no rooms found after 409');
  }
  return { room_id: rooms[0].room_id, title: rooms[0].title };
}

/**
 * 尝试通过 HTTP GET / HEAD 检测服务器是否可达（不抛异常）。
 * 返回 true = 可达；false = 不可达（网络错误 / timeout）。
 */
export async function isServerReachable(apiUrl: string): Promise<boolean> {
  try {
    const resp = await fetch(`${apiUrl}/api/v1/rooms`, {
      method: 'GET',
      signal: AbortSignal.timeout(3000),
    });
    // 任何 HTTP 响应（包括 401/403/500）都说明服务器在运行
    return resp.status > 0;
  } catch {
    return false;
  }
}
