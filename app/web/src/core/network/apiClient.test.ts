/**
 * T-20001 Review + T-20002 Review — apiClient TDD 测试套件
 *
 * 覆盖：
 *   - [MEDIUM-1] HTTP 非 2xx 状态时抛出 Error（含 body.message 透传）
 *   - [MEDIUM-1] JSON 解析失败时回退到 "HTTP Error <status>"
 *   - [MEDIUM-2] 15 秒超时后 abort 并抛出 Error
 *   - 正常 2xx + code=0 时返回 data
 *   - code !== 0 时抛出携带 message 的 Error（已有逻辑的回归保护）
 *   - 自动附加 Authorization 头（localStorage 有 token 时）
 *   - [HIGH-H01] 401 + token 存在时：logout() + 跳转 /login + 抛 Unauthorized
 *   - [HIGH-H01] 401 + 无 token（登录请求）时：抛 body.message，不触发 logout
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  adminLogin,
  adminCloseRoom,
  adminGetRoomDetail,
  adminListGifts,
  adminCreateGift,
  adminUpdateGift,
  adminDeleteGift,
} from './apiClient';

// ── 常量 ────────────────────────────────────────────────────────────────────
const ADMIN_TOKEN_KEY = 'adminToken';

// ── mock useAuthStore（H01 修复后 apiClient 会导入此模块）──────────────────
const { mockLogout } = vi.hoisted(() => ({ mockLogout: vi.fn() }));

vi.mock('../../stores/useAuthStore', () => ({
  useAuthStore: {
    getState: () => ({ logout: mockLogout }),
  },
  ADMIN_TOKEN_KEY: 'adminToken',
}));

// ── fetch mock 工具 ──────────────────────────────────────────────────────────

/** 构造一个模拟的 Response 对象 */
function mockResponse(
  body: unknown,
  options: { status?: number; ok?: boolean } = {},
): Response {
  const status = options.status ?? 200;
  const ok = options.ok ?? (status >= 200 && status < 300);
  return {
    ok,
    status,
    json: vi.fn().mockResolvedValue(body),
  } as unknown as Response;
}

/** 构造一个 JSON 解析失败的 Response（非 JSON body） */
function mockBadJsonResponse(status: number): Response {
  return {
    ok: false,
    status,
    json: vi.fn().mockRejectedValue(new SyntaxError('Unexpected token')),
  } as unknown as Response;
}

beforeEach(() => {
  localStorage.clear();
  mockLogout.mockClear();
  vi.useFakeTimers();
  vi.stubGlobal('fetch', vi.fn());
});

afterEach(() => {
  localStorage.clear();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

// ── 1. HTTP 状态码检查（MEDIUM-1）────────────────────────────────────────────
describe('adminFetch — HTTP 响应状态检查', () => {
  it('HTTP 401 时，从 body.message 提取错误信息并抛出', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ message: '无效的管理员凭据' }, { status: 401, ok: false }),
    );

    await expect(adminLogin({ username: 'admin', password: 'wrong' })).rejects.toThrow(
      '无效的管理员凭据',
    );
  });

  it('HTTP 500 时，body.message 为空则回退到 "HTTP Error 500"', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({}, { status: 500, ok: false }),
    );

    await expect(adminLogin({ username: 'admin', password: 'pass' })).rejects.toThrow(
      'HTTP Error 500',
    );
  });

  it('HTTP 403 时，body JSON 解析失败则回退到 "HTTP Error 403"', async () => {
    vi.mocked(fetch).mockResolvedValue(mockBadJsonResponse(403));

    await expect(adminLogin({ username: 'admin', password: 'pass' })).rejects.toThrow(
      'HTTP Error 403',
    );
  });

  it('HTTP 404 时抛出错误，不返回 data', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ message: 'Not Found' }, { status: 404, ok: false }),
    );

    await expect(adminLogin({ username: 'admin', password: 'pass' })).rejects.toThrow(
      'Not Found',
    );
  });
});

// ── 2. 超时控制（MEDIUM-2）──────────────────────────────────────────────────
describe('adminFetch — 15 秒超时控制', () => {
  it('请求超过 15 秒时，fetch 被 abort 并抛出 AbortError', async () => {
    // fetch 永不 resolve，模拟挂起请求
    vi.mocked(fetch).mockImplementation(
      (_url, init) =>
        new Promise((_resolve, reject) => {
          // 监听 abort 信号
          (init?.signal as AbortSignal | undefined)?.addEventListener('abort', () => {
            reject(new DOMException('The user aborted a request.', 'AbortError'));
          });
        }),
    );

    const loginPromise = adminLogin({ username: 'admin', password: 'pass' });
    // 先绑定 rejection 处理器，防止"Unhandled Rejection"警告
    const assertion = expect(loginPromise).rejects.toThrow();

    // 推进 15 秒，触发 AbortController.abort()
    await vi.advanceTimersByTimeAsync(15_000);

    await assertion;
  });

  it('请求在 15 秒内完成时，定时器被清除，不触发 abort', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse(
        { code: 0, message: 'ok', data: { token: 'tok', expires_in: 3600, admin: { id: '1', username: 'admin', role: 'admin', display_name: 'Admin', last_login_at: '' } } },
        { status: 200, ok: true },
      ),
    );

    const result = await adminLogin({ username: 'admin', password: 'pass' });

    expect(result.token).toBe('tok');
    // 推进超过 15 秒，不应抛出（定时器已清除）
    await vi.advanceTimersByTimeAsync(20_000);
  });
});

// ── 3. 正常流程（回归保护）──────────────────────────────────────────────────
describe('adminFetch — 正常流程', () => {
  it('HTTP 200 + code=0 时返回 data', async () => {
    const mockData = {
      token: 'jwt-token',
      expires_in: 3600,
      admin: {
        id: '1',
        username: 'admin',
        role: 'super_admin',
        display_name: '超级管理员',
        last_login_at: '2024-01-01T00:00:00Z',
      },
    };

    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ code: 0, message: 'ok', data: mockData }, { status: 200, ok: true }),
    );

    const result = await adminLogin({ username: 'admin', password: 'correct' });

    expect(result).toEqual(mockData);
  });

  it('HTTP 200 但 code !== 0 时，抛出 body.message', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse(
        { code: 1001, message: '账号或密码错误', data: null },
        { status: 200, ok: true },
      ),
    );

    await expect(adminLogin({ username: 'admin', password: 'wrong' })).rejects.toThrow(
      '账号或密码错误',
    );
  });

  it('localStorage 有 token 时，自动附加 Authorization 头', async () => {
    localStorage.setItem(ADMIN_TOKEN_KEY, 'my-jwt');

    vi.mocked(fetch).mockResolvedValue(
      mockResponse(
        { code: 0, message: 'ok', data: { token: 't', expires_in: 1, admin: { id: '1', username: 'a', role: 'r', display_name: 'd', last_login_at: '' } } },
        { status: 200, ok: true },
      ),
    );

    await adminLogin({ username: 'admin', password: 'pass' });

    const [, init] = vi.mocked(fetch).mock.calls[0];
    expect((init?.headers as Record<string, string>)['Authorization']).toBe('Bearer my-jwt');
  });

  it('localStorage 无 token 时，不附加 Authorization 头', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse(
        { code: 0, message: 'ok', data: { token: 't', expires_in: 1, admin: { id: '1', username: 'a', role: 'r', display_name: 'd', last_login_at: '' } } },
        { status: 200, ok: true },
      ),
    );

    await adminLogin({ username: 'admin', password: 'pass' });

    const [, init] = vi.mocked(fetch).mock.calls[0];
    expect((init?.headers as Record<string, string>)['Authorization']).toBeUndefined();
  });
});

// ── 4. 401 拦截器：logout + 跳转（HIGH-H01）──────────────────────────────────
describe('adminFetch — 401 拦截器：logout + 跳转（H01）', () => {
  it('401 响应且 localStorage 存有 token 时，调用 useAuthStore.getState().logout()', async () => {
    localStorage.setItem(ADMIN_TOKEN_KEY, 'existing-session-token');
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ message: 'Token expired' }, { status: 401, ok: false }),
    );

    await expect(adminLogin({ username: 'admin', password: 'pass' })).rejects.toThrow();

    expect(mockLogout).toHaveBeenCalledOnce();
  });

  it('401 响应且 localStorage 存有 token 时，将 window.location.href 设为 /login', async () => {
    localStorage.setItem(ADMIN_TOKEN_KEY, 'existing-session-token');

    // 用可写对象替换 window.location，捕获 href 赋值
    const mockLocation = { href: 'http://localhost/dashboard' };
    Object.defineProperty(window, 'location', {
      value: mockLocation,
      configurable: true,
      writable: true,
    });

    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ message: 'Token expired' }, { status: 401, ok: false }),
    );

    await expect(adminLogin({ username: 'admin', password: 'pass' })).rejects.toThrow(
      'Unauthorized',
    );

    expect(mockLocation.href).toBe('/login');
  });

  it('401 响应且 localStorage 无 token（登录请求）时，不触发 logout，抛出 body.message', async () => {
    // 无 token — 模拟用户输入错误密码的登录请求
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ message: '无效的管理员凭据' }, { status: 401, ok: false }),
    );

    await expect(adminLogin({ username: 'admin', password: 'wrong' })).rejects.toThrow(
      '无效的管理员凭据',
    );

    expect(mockLogout).not.toHaveBeenCalled();
  });
});

// ── 5. adminCloseRoom（T-20004 A01-A05）──────────────────────────────────────
describe('adminCloseRoom — T-20004', () => {
  // A01: 成功 → Promise resolve（无抛出）
  it('A01: DELETE 成功 → Promise resolve', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ code: 0, message: 'ok', data: null }, { status: 200, ok: true }),
    );

    await expect(adminCloseRoom('room-123')).resolves.toBeUndefined();
  });

  // A02: 后端 404 → 抛出 Error
  it('A02: 后端 404 → 抛出 Error', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ message: 'Room not found' }, { status: 404, ok: false }),
    );

    await expect(adminCloseRoom('room-404')).rejects.toThrow('Room not found');
  });

  // A03: 后端 409 → 抛出 Error，message 正确
  it('A03: 后端 409 → 抛出 Error，message 为 "Room already closed"', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ message: 'Room already closed' }, { status: 409, ok: false }),
    );

    await expect(adminCloseRoom('room-409')).rejects.toThrow('Room already closed');
  });

  // A04: 后端 403 → 抛出 Error，不触发 logout（无 token）
  it('A04: 后端 403（无 token）→ 抛出 Error，不触发 logout', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ message: 'Forbidden' }, { status: 403, ok: false }),
    );

    await expect(adminCloseRoom('room-403')).rejects.toThrow('Forbidden');
    expect(mockLogout).not.toHaveBeenCalled();
  });

  // A05: roomId 被 encodeURIComponent 编码
  it('A05: roomId 含特殊字符时 URL 被正确编码', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ code: 0, message: 'ok', data: null }, { status: 200, ok: true }),
    );

    await adminCloseRoom('room/with/slashes');

    const [url] = vi.mocked(fetch).mock.calls[0];
    expect(url as string).toContain('room%2Fwith%2Fslashes');
  });
});

// ── 6. adminGetRoomDetail（T-20005 A01–A06）──────────────────────────────────
describe('adminGetRoomDetail — T-20005', () => {
  const mockDetail = {
    room_id: 'uuid-1',
    title: 'Test Room',
    status: 'active' as const,
    room_type: 'normal' as const,
    member_count: 5,
    max_members: 20,
    owner: {
      user_id: 'user-1',
      nickname: 'TestOwner',
      avatar: null,
    },
    mic_slots: [],
    created_at: '2025-01-01T00:00:00Z',
    updated_at: '2025-01-01T00:00:00Z',
  };

  // A01: 成功 → 返回 AdminRoomDetail，含嵌套 owner
  it('A01: 成功 → 返回 AdminRoomDetail，含嵌套 owner', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ code: 0, message: 'ok', data: mockDetail }, { status: 200, ok: true }),
    );

    const result = await adminGetRoomDetail('uuid-1');

    expect(result.room_id).toBe('uuid-1');
    expect(result.owner).toBeDefined();
    expect(result.owner.nickname).toBe('TestOwner');
    expect(result.owner.user_id).toBe('user-1');
  });

  // A02: 后端 404 → 抛出 Error
  it('A02: 后端 404 → 抛出 Error', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ message: 'Room not found' }, { status: 404, ok: false }),
    );

    await expect(adminGetRoomDetail('uuid-not-exist')).rejects.toThrow('Room not found');
  });

  // A03: 后端 401 + localStorage 有 token → logout 被调用
  it('A03: 后端 401 且 localStorage 有 token → logout 被调用', async () => {
    localStorage.setItem(ADMIN_TOKEN_KEY, 'my-token');
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ message: 'Token expired' }, { status: 401, ok: false }),
    );

    await expect(adminGetRoomDetail('uuid-1')).rejects.toThrow();
    expect(mockLogout).toHaveBeenCalled();
  });

  // A04: 后端 403 → 抛出 Error，logout 不被调用
  it('A04: 后端 403 → 抛出 Error，logout 不被调用', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ message: 'Forbidden' }, { status: 403, ok: false }),
    );

    await expect(adminGetRoomDetail('uuid-1')).rejects.toThrow('Forbidden');
    expect(mockLogout).not.toHaveBeenCalled();
  });

  // A05: roomId='abc/def' → URL 含 'abc%2Fdef'
  it('A05: roomId="abc/def" → URL 含 "abc%2Fdef"', async () => {
    vi.mocked(fetch).mockResolvedValue(
      mockResponse({ code: 0, message: 'ok', data: mockDetail }, { status: 200, ok: true }),
    );

    await adminGetRoomDetail('abc/def');

    const [url] = vi.mocked(fetch).mock.calls[0];
    expect(url as string).toContain('abc%2Fdef');
  });

  // A06: signal abort → 抛出 AbortError
  it('A06: signal abort → 抛出 AbortError', async () => {
    vi.mocked(fetch).mockImplementation(
      (_url, init) =>
        new Promise((_resolve, reject) => {
          (init?.signal as AbortSignal | undefined)?.addEventListener('abort', () => {
            reject(new DOMException('The user aborted a request.', 'AbortError'));
          });
        }),
    );

    const controller = new AbortController();
    const promise = adminGetRoomDetail('uuid-1', controller.signal);
    controller.abort();

    await expect(promise).rejects.toMatchObject({ name: 'AbortError' });
  });
});

// ── 7. MEDIUM-1: T-20012 新增 API 函数 AbortSignal 参数 ────────────────────────
// 验证 adminListGifts / adminCreateGift / adminUpdateGift / adminDeleteGift
// 支持外部 AbortSignal，并在信号预先 abort 时立即取消请求。
//
// 测试策略：
//   - 预先调用 controller.abort()（信号已 abort）
//   - mock fetch：若 init.signal 已 abort → 立即 reject（模拟真实 fetch 行为）
//             否则 → 正常 resolve（以区分"信号被正确转发"vs"信号被忽略"两种状态）
//   - RED：函数无 signal 参数 → 信号未传入 adminFetch → fetch 使用内部 signal（未 abort）
//           → mock 正常 resolve → expect rejects.toThrow() 断言 FAIL
//   - GREEN：函数接受 signal → adminFetch 检测到 init.signal.aborted=true → 立即 abort 内部 controller
//            → fetch 使用已 abort 的内部 signal → mock 立即 reject → 断言 PASS
describe('MEDIUM-1: T-20012 新增 API 函数 — AbortSignal 参数', () => {
  /** 构造一个当 signal 已 abort 时立即 reject，否则正常 resolve 的 fetch mock */
  function makeFetchMockWithSignalCheck(successData: unknown) {
    return vi.fn().mockImplementation((_url: string, init?: RequestInit) => {
      const signal = init?.signal as AbortSignal | undefined;
      if (signal?.aborted) {
        return Promise.reject(new DOMException('The user aborted a request.', 'AbortError'));
      }
      return Promise.resolve(
        mockResponse(
          { code: 0, message: 'ok', data: successData },
          { status: 200, ok: true },
        ),
      );
    });
  }

  // ── adminListGifts ─────────────────────────────────────────────────────────
  describe('adminListGifts', () => {
    it('传入预先 abort 的 signal，请求应立即失败（signal 被正确转发至 adminFetch）', async () => {
      const controller = new AbortController();
      controller.abort(); // 预先 abort

      vi.stubGlobal('fetch', makeFetchMockWithSignalCheck({
        total: 0, page: 1, size: 50, items: [],
      }));

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await expect((adminListGifts as any)(undefined, controller.signal)).rejects.toThrow();
    });

    it('未传 signal 时请求正常完成', async () => {
      vi.stubGlobal('fetch', makeFetchMockWithSignalCheck({
        total: 0, page: 1, size: 50, items: [],
      }));

      await expect(adminListGifts()).resolves.toEqual({
        total: 0, page: 1, size: 50, items: [],
      });
    });
  });

  // ── adminCreateGift ────────────────────────────────────────────────────────
  describe('adminCreateGift', () => {
    const MOCK_GIFT = {
      id: 'g-1', code: 'rose', name_en: 'Rose', name_ar: 'وردة',
      icon_url: '/rose.png', price: 10, tier: 1, effect_level: 1,
      animation_url: null, is_active: true, sort_order: 1,
      is_deleted: false, created_at: '', updated_at: '',
    };

    it('传入预先 abort 的 signal，请求应立即失败', async () => {
      const controller = new AbortController();
      controller.abort();

      vi.stubGlobal('fetch', makeFetchMockWithSignalCheck(MOCK_GIFT));

      const req = { code: 'rose', name_en: 'Rose', name_ar: 'وردة', icon_url: '/rose.png', price: 10, tier: 1, effect_level: 1 };
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await expect((adminCreateGift as any)(req, controller.signal)).rejects.toThrow();
    });

    it('未传 signal 时请求正常完成', async () => {
      vi.stubGlobal('fetch', makeFetchMockWithSignalCheck(MOCK_GIFT));

      const req = { code: 'rose', name_en: 'Rose', name_ar: 'وردة', icon_url: '/rose.png', price: 10, tier: 1, effect_level: 1 };
      const result = await adminCreateGift(req);
      expect(result.code).toBe('rose');
    });
  });

  // ── adminUpdateGift ────────────────────────────────────────────────────────
  describe('adminUpdateGift', () => {
    const MOCK_UPDATED = {
      id: 'g-1', code: 'rose', name_en: 'Rose Updated', name_ar: 'وردة',
      icon_url: '/rose.png', price: 20, tier: 1, effect_level: 1,
      animation_url: null, is_active: false, sort_order: 1,
      is_deleted: false, created_at: '', updated_at: '',
    };

    it('传入预先 abort 的 signal，请求应立即失败', async () => {
      const controller = new AbortController();
      controller.abort();

      vi.stubGlobal('fetch', makeFetchMockWithSignalCheck(MOCK_UPDATED));

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await expect((adminUpdateGift as any)('g-1', { is_active: false }, controller.signal)).rejects.toThrow();
    });

    it('未传 signal 时请求正常完成', async () => {
      vi.stubGlobal('fetch', makeFetchMockWithSignalCheck(MOCK_UPDATED));

      const result = await adminUpdateGift('g-1', { is_active: false });
      expect(result.is_active).toBe(false);
    });
  });

  // ── adminDeleteGift ────────────────────────────────────────────────────────
  describe('adminDeleteGift', () => {
    it('传入预先 abort 的 signal，请求应立即失败', async () => {
      const controller = new AbortController();
      controller.abort();

      vi.stubGlobal('fetch', makeFetchMockWithSignalCheck(null));

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      await expect((adminDeleteGift as any)('g-1', controller.signal)).rejects.toThrow();
    });

    it('未传 signal 时请求正常完成', async () => {
      vi.stubGlobal('fetch', makeFetchMockWithSignalCheck(null));

      await expect(adminDeleteGift('g-1')).resolves.toBeUndefined();
    });
  });
});
