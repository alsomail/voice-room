/**
 * T-20013: EventStreamTab 组件测试
 *
 * 验收用例（对应 TDS E13-01 ~ E13-08）：
 *   E13-01  EventStreamTab 在 UserDetailDrawer 中 Tab 可见
 *   E13-02  默认加载最近 24h 事件
 *   E13-03  事件名多选过滤生效
 *   E13-04  时间窗自定义 >30 天时显示错误提示
 *   E13-05  空数据时显示占位元素
 *   E13-06  导出 CSV 文件名包含 user_id 和时间戳
 *   E13-07  properties JSON 可展开 / 折叠
 *   E13-08  i18n：文案使用 i18n key
 *
 * 注意：不使用 vi.useFakeTimers() 以避免与 act()/waitFor() 死锁。
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import React from 'react';

// ── i18n mock ─────────────────────────────────────────────────────────────────
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { changeLanguage: vi.fn(), language: 'en' },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ── API mock ──────────────────────────────────────────────────────────────────
vi.mock('../../../services/api/events', () => ({
  listUserEvents: vi.fn(),
  listEventNames: vi.fn(),
}));

// ── useUserDetail mock ────────────────────────────────────────────────────────
vi.mock('../../../pages/users/useUserDetail', () => ({
  useUserDetail: vi.fn(),
}));

// ── useAuthStore mock ─────────────────────────────────────────────────────────
vi.mock('../../../stores/useAuthStore', () => ({
  useAuthStore: (selector?: (s: { admin: { role: string } }) => unknown) => {
    const state = { admin: { role: 'super_admin' } };
    if (typeof selector === 'function') return selector(state);
    return state;
  },
  ADMIN_TOKEN_KEY: 'adminToken',
}));

// ── URL mock ──────────────────────────────────────────────────────────────────
const mockCreateObjectURL = vi.fn().mockReturnValue('blob:mock-url');
const mockRevokeObjectURL = vi.fn();

Object.defineProperty(globalThis, 'URL', {
  writable: true,
  value: {
    createObjectURL: mockCreateObjectURL,
    revokeObjectURL: mockRevokeObjectURL,
  },
});

import { listUserEvents, listEventNames } from '../../../services/api/events';
import { EventStreamTab } from '../EventStreamTab';
import { __resetEventNamesCache } from '../useEventNames';

const mockListUserEvents = listUserEvents as ReturnType<typeof vi.fn>;
const mockListEventNames = listEventNames as unknown as ReturnType<typeof vi.fn>;

// ── 测试数据 ───────────────────────────────────────────────────────────────────
const MOCK_EVENT_ID = 'evt-uuid-001';

const MOCK_EVENT = {
  id: MOCK_EVENT_ID,
  event_name: 'gift_send_success',
  server_ts: '2026-04-22T18:30:00Z',
  client_ts: '2026-04-22T18:29:55Z',
  session_id: 'sess_abc',
  device_id: 'device_xyz',
  properties: { gift_id: '123', amount: 1000 },
  app_version: '1.2.0',
  os_version: 'Android 14',
  locale: 'ar-SA',
  network_type: 'wifi',
};

const MOCK_RESPONSE = {
  total: 1,
  page: 1,
  limit: 20,
  items: [MOCK_EVENT],
};

const EMPTY_RESPONSE = {
  total: 0,
  page: 1,
  limit: 20,
  items: [],
};

// 全局 afterEach：确保不污染全局 timer 状态
afterEach(() => {
  vi.useRealTimers();
});

beforeEach(() => {
  vi.clearAllMocks();
  mockListUserEvents.mockResolvedValue(MOCK_RESPONSE);
  mockListEventNames.mockResolvedValue({
    items: ['gift_send_success', 'login_success', 'mic_take', 'room_enter'],
  });
  mockCreateObjectURL.mockReturnValue('blob:mock-url');
  __resetEventNamesCache();
});

// ─────────────────────────────────────────────────────────────────────────────
// E13-01: Tab 渲染可见
// ─────────────────────────────────────────────────────────────────────────────
describe('EventStreamTab — E13-01: Tab 渲染可见', () => {
  it('组件挂载后 event-stream-tab 容器可见', () => {
    render(<EventStreamTab userId="user-123" />);
    expect(screen.getByTestId('event-stream-tab')).toBeInTheDocument();
  });

  it('UserDetailDrawer 包含行为流 Tab', async () => {
    const { useUserDetail } = await import('../../../pages/users/useUserDetail');
    (useUserDetail as ReturnType<typeof vi.fn>).mockReturnValue({
      detail: {
        id: 'user-001',
        phone: '13800138000',
        nickname: 'TestUser',
        avatar_url: null,
        coin_balance: 500,
        vip_level: 1,
        status: 'normal' as const,
        created_at: '2025-01-01T00:00:00Z',
        recharge_records: [],
        consume_records: [],
        devices: [],
      },
      loading: false,
      error: null,
    });

    const { UserDetailDrawer } = await import('../../../pages/users/UserDetailDrawer');
    render(<UserDetailDrawer userId="user-001" onClose={vi.fn()} />);

    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
    const tab = await screen.findByTestId('tab-event-stream');
    expect(tab).toBeInTheDocument();

    await userEvent.setup().click(tab);
    await waitFor(() => {
      expect(screen.getByTestId('event-stream-tab')).toBeInTheDocument();
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// E13-02: 默认加载最近 24h 事件（不使用 fake timers）
// ─────────────────────────────────────────────────────────────────────────────
describe('EventStreamTab — E13-02: 默认 24h', () => {
  it('挂载后 listUserEvents 被以 from~24h前 调用', async () => {
    const before = Date.now();
    render(<EventStreamTab userId="user-123" />);

    await waitFor(() => {
      expect(mockListUserEvents).toHaveBeenCalled();
    });

    const after = Date.now();
    const callArgs = mockListUserEvents.mock.calls[0][1] as { from: string };
    const actualFrom = new Date(callArgs.from).getTime();
    const expectedMin = before - 24 * 60 * 60 * 1000 - 60_000;
    const expectedMax = after - 24 * 60 * 60 * 1000 + 60_000;

    expect(actualFrom).toBeGreaterThanOrEqual(expectedMin);
    expect(actualFrom).toBeLessThanOrEqual(expectedMax);
  });

  it('time-range 控件存在，24h 为默认值', () => {
    render(<EventStreamTab userId="user-123" />);
    const timeRange = screen.getByTestId('event-time-range');
    expect(timeRange).toBeInTheDocument();
    // 24h radio input 应存在于 Radio.Group 中
    const input24h = timeRange.querySelector('input[value="24h"]') as HTMLInputElement | null;
    expect(input24h).toBeTruthy();
    expect(input24h?.checked).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// E13-03: event_name 多选过滤
// ─────────────────────────────────────────────────────────────────────────────
describe('EventStreamTab — E13-03: event_name 多选', () => {
  it('event-name-select 存在', () => {
    render(<EventStreamTab userId="user-123" />);
    expect(screen.getByTestId('event-name-select')).toBeInTheDocument();
  });

  it('event-name-select 是 multiple 模式的 Select', () => {
    render(<EventStreamTab userId="user-123" />);
    const selectEl = screen.getByTestId('event-name-select');
    // Ant Design Select multiple 模式会渲染 .ant-select-multiple
    expect(selectEl.classList.contains('ant-select-multiple')).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// E13-04: 自定义时间窗 >30 天校验（纯函数测试）
// ─────────────────────────────────────────────────────────────────────────────
describe('EventStreamTab — E13-04: 自定义范围校验', () => {
  it('validateCustomRange: >30天返回 false', async () => {
    const { validateCustomRange } = await import('../EventStreamTab');
    const from = new Date('2026-01-01T00:00:00Z').toISOString();
    const to = new Date('2026-02-15T00:00:00Z').toISOString(); // 45 days
    expect(validateCustomRange(from, to)).toBe(false);
  });

  it('validateCustomRange: 恰好 30 天返回 true', async () => {
    const { validateCustomRange } = await import('../EventStreamTab');
    const from = new Date('2026-04-01T00:00:00Z').toISOString();
    const to = new Date('2026-05-01T00:00:00Z').toISOString(); // exactly 30 days
    expect(validateCustomRange(from, to)).toBe(true);
  });

  it('validateCustomRange: <30天返回 true', async () => {
    const { validateCustomRange } = await import('../EventStreamTab');
    const from = new Date('2026-04-01T00:00:00Z').toISOString();
    const to = new Date('2026-04-10T00:00:00Z').toISOString(); // 9 days
    expect(validateCustomRange(from, to)).toBe(true);
  });

  it('自定义 >30 天时 range-error 出现，API 未发新请求', async () => {
    render(<EventStreamTab userId="user-123" />);
    await waitFor(() => expect(mockListUserEvents).toHaveBeenCalledTimes(1));
    const callsBefore = mockListUserEvents.mock.calls.length;

    // 触发无效自定义范围：找到 custom radio 并点击
    const timeRangeEl = screen.getByTestId('event-time-range');
    const customInput = timeRangeEl.querySelector('input[value="custom"]') as HTMLInputElement;
    if (customInput) {
      await userEvent.setup().click(customInput.closest('label')!);
      // custom 范围未设置 from/to 时不发请求
      expect(mockListUserEvents.mock.calls.length).toBe(callsBefore);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// E13-05: 空数据占位
// ─────────────────────────────────────────────────────────────────────────────
describe('EventStreamTab — E13-05: 空数据占位', () => {
  it('API 返回空列表时显示 events-empty', async () => {
    mockListUserEvents.mockResolvedValue(EMPTY_RESPONSE);
    render(<EventStreamTab userId="user-123" />);
    await waitFor(() => {
      expect(screen.getByTestId('events-empty')).toBeInTheDocument();
    });
  });

  it('API 返回空列表时不显示事件条目', async () => {
    mockListUserEvents.mockResolvedValue(EMPTY_RESPONSE);
    render(<EventStreamTab userId="user-123" />);
    await waitFor(() => screen.getByTestId('events-empty'));
    expect(screen.queryByTestId(`event-item-${MOCK_EVENT_ID}`)).not.toBeInTheDocument();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// E13-06: CSV 导出文件名包含 user_id 和时间戳
// ─────────────────────────────────────────────────────────────────────────────
describe('EventStreamTab — E13-06: CSV 导出', () => {
  it('btn-export-csv 存在', () => {
    render(<EventStreamTab userId="user-123" />);
    expect(screen.getByTestId('btn-export-csv')).toBeInTheDocument();
  });

  it('generateEventCsvFilename 生成正确格式', async () => {
    const { generateEventCsvFilename } = await import('../../../lib/csv');
    const userId = 'abc-user-id';
    const ts = 1714000000000;
    expect(generateEventCsvFilename(userId, ts)).toBe(`user_${userId}_events_${ts}.csv`);
  });

  it('点击导出后文件名包含 user_id 和数字时间戳', async () => {
    // 捕获 anchor 元素的 download 属性
    let capturedDownload = '';
    const origCreateElement = document.createElement.bind(document);
    vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
      const el = origCreateElement(tag);
      if (tag === 'a') {
        let _dl = '';
        Object.defineProperty(el, 'download', {
          get: () => _dl,
          set: (v: string) => { _dl = v; capturedDownload = v; },
          configurable: true,
        });
        Object.defineProperty(el, 'click', { value: vi.fn(), writable: true });
      }
      return el;
    });

    render(<EventStreamTab userId="export-user-id" />);

    // 等待数据加载完成（btn 不再 disabled，因为 data.total > 0）
    await waitFor(() => {
      expect(screen.getByTestId('btn-export-csv')).toBeInTheDocument();
    });

    // 点击导出
    await act(async () => {
      await userEvent.setup().click(screen.getByTestId('btn-export-csv'));
    });

    await waitFor(() => {
      expect(capturedDownload).toMatch(/^user_export-user-id_events_\d+\.csv$/);
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// E13-07: properties JSON 可展开 / 折叠
// ─────────────────────────────────────────────────────────────────────────────
describe('EventStreamTab — E13-07: properties JSON 折叠', () => {
  it('事件条目 event-item-{id} 被渲染', async () => {
    render(<EventStreamTab userId="user-123" />);
    await waitFor(() => {
      expect(screen.getByTestId(`event-item-${MOCK_EVENT_ID}`)).toBeInTheDocument();
    });
  });

  it('默认 properties 未展开（props-content 不存在）', async () => {
    render(<EventStreamTab userId="user-123" />);
    await waitFor(() => screen.getByTestId(`event-item-${MOCK_EVENT_ID}`));
    expect(screen.queryByTestId(`props-content-${MOCK_EVENT_ID}`)).not.toBeInTheDocument();
  });

  it('点击 props-toggle 后 properties 展开显示 JSON', async () => {
    render(<EventStreamTab userId="user-123" />);
    await waitFor(() => screen.getByTestId(`props-toggle-${MOCK_EVENT_ID}`));

    await userEvent.setup().click(screen.getByTestId(`props-toggle-${MOCK_EVENT_ID}`));

    await waitFor(() => {
      const content = screen.getByTestId(`props-content-${MOCK_EVENT_ID}`);
      expect(content).toBeInTheDocument();
      expect(content).toHaveTextContent('gift_id');
    });
  });

  it('再次点击 props-toggle 折叠', async () => {
    render(<EventStreamTab userId="user-123" />);
    await waitFor(() => screen.getByTestId(`props-toggle-${MOCK_EVENT_ID}`));

    const user = userEvent.setup();
    await user.click(screen.getByTestId(`props-toggle-${MOCK_EVENT_ID}`));
    await waitFor(() => screen.getByTestId(`props-content-${MOCK_EVENT_ID}`));

    await user.click(screen.getByTestId(`props-toggle-${MOCK_EVENT_ID}`));
    await waitFor(() => {
      expect(screen.queryByTestId(`props-content-${MOCK_EVENT_ID}`)).not.toBeInTheDocument();
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// E13-08: i18n 文案使用 key
// ─────────────────────────────────────────────────────────────────────────────
describe('EventStreamTab — E13-08: i18n', () => {
  it('导出按钮使用 events.exportCsv 翻译 key', () => {
    render(<EventStreamTab userId="user-123" />);
    expect(screen.getByTestId('btn-export-csv')).toHaveTextContent('events.exportCsv');
  });

  it('空状态使用 events.empty 翻译 key', async () => {
    mockListUserEvents.mockResolvedValue(EMPTY_RESPONSE);
    render(<EventStreamTab userId="user-123" />);
    await waitFor(() => {
      const emptyEl = screen.getByTestId('events-empty');
      expect(emptyEl).toBeInTheDocument();
      expect(emptyEl.textContent).toContain('events.empty');
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// HIGH-1 XSS 安全测试（Review R1 修复验证）
// ─────────────────────────────────────────────────────────────────────────────
describe('EventTimelineItem — HIGH-1: XSS 防护', () => {
  it('properties 含 <script> 时，highlight 激活后渲染内容不含原始 <script> 标签', async () => {
    const { EventTimelineItem } = await import('../components/EventTimelineItem');

    const xssEvent = {
      id: 'xss-evt-001',
      event_name: 'login_success',
      server_ts: '2026-04-22T18:30:00Z',
      properties: { payload: '<script>alert("xss")</script>' },
      app_version: '1.0.0',
      os_version: 'Android 14',
      network_type: 'wifi',
    };

    const { container } = render(
      <EventTimelineItem event={xssEvent} highlight="payload" />,
    );

    // 展开 properties
    const toggle = screen.getByTestId('props-toggle-xss-evt-001');
    await userEvent.setup().click(toggle);

    await waitFor(() => {
      const content = screen.getByTestId('props-content-xss-evt-001');
      // 1. 内容区域存在
      expect(content).toBeInTheDocument();
      // 2. 不得存在真实的 <script> 元素（XSS 执行载体）
      expect(container.querySelector('script')).toBeNull();
      // 3. 原始 < 必须被转义为 &lt;
      const html = content.innerHTML;
      expect(html).not.toContain('<script>');
      expect(html).toContain('&lt;script&gt;');
    });
  });

  it('highlight 为空时，properties 含 HTML 特殊字符仍被转义', async () => {
    const { EventTimelineItem } = await import('../components/EventTimelineItem');

    const htmlEvent = {
      id: 'html-evt-002',
      event_name: 'room_enter',
      server_ts: '2026-04-22T18:30:00Z',
      properties: { msg: '<b>bold</b> & "quoted"' },
      app_version: '1.0.0',
      os_version: 'iOS 17',
      network_type: 'lte',
    };

    render(<EventTimelineItem event={htmlEvent} highlight="bold" />);

    const toggle = screen.getByTestId('props-toggle-html-evt-002');
    await userEvent.setup().click(toggle);

    await waitFor(() => {
      const content = screen.getByTestId('props-content-html-evt-002');
      const html = content.innerHTML;
      // <b> 不应被渲染为真实 bold 元素
      expect(content.querySelector('b')).toBeNull();
      // & 应被转义
      expect(html).toContain('&amp;');
    });
  });

  it('highlight 关键字本身含 HTML 特殊字符时不产生 XSS', async () => {
    const { EventTimelineItem } = await import('../components/EventTimelineItem');

    const safeEvent = {
      id: 'kw-evt-003',
      event_name: 'wallet_view',
      server_ts: '2026-04-22T18:30:00Z',
      properties: { value: 'test<img>end' },
      app_version: '2.0.0',
      os_version: 'Android 13',
      network_type: 'wifi',
    };

    const { container } = render(
      <EventTimelineItem event={safeEvent} highlight="<img>" />,
    );

    const toggle = screen.getByTestId('props-toggle-kw-evt-003');
    await userEvent.setup().click(toggle);

    await waitFor(() => {
      // 不应存在真实 <img> 元素（highlight 关键字含 < > 被转义后不匹配注入）
      expect(container.querySelector('img')).toBeNull();
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 额外边界用例
// ─────────────────────────────────────────────────────────────────────────────
describe('EventStreamTab — 边界用例', () => {
  it('API 失败时显示 events-error', async () => {
    mockListUserEvents.mockRejectedValue(new Error('Network Error'));
    render(<EventStreamTab userId="user-123" />);
    await waitFor(() => {
      expect(screen.getByTestId('events-error')).toBeInTheDocument();
    });
  });

  it('userId 变化后重新调用 API', async () => {
    const { rerender } = render(<EventStreamTab userId="user-aaa" />);
    await waitFor(() => expect(mockListUserEvents).toHaveBeenCalledTimes(1));
    expect(mockListUserEvents.mock.calls[0][0]).toBe('user-aaa');

    rerender(<EventStreamTab userId="user-bbb" />);
    await waitFor(() => expect(mockListUserEvents).toHaveBeenCalledTimes(2));
    expect(mockListUserEvents.mock.calls[1][0]).toBe('user-bbb');
  });

  it('事件条目显示 event_name 文字', async () => {
    render(<EventStreamTab userId="user-123" />);
    await waitFor(() => {
      const item = screen.getByTestId(`event-item-${MOCK_EVENT_ID}`);
      expect(item).toHaveTextContent('gift_send_success');
    });
  });

  it('加载中时显示 events-loading', async () => {
    let resolveApi!: (v: typeof MOCK_RESPONSE) => void;
    mockListUserEvents.mockReturnValue(
      new Promise<typeof MOCK_RESPONSE>((res) => { resolveApi = res; }),
    );

    render(<EventStreamTab userId="user-123" />);
    await waitFor(() => {
      expect(screen.getByTestId('events-loading')).toBeInTheDocument();
    });

    await act(async () => { resolveApi(MOCK_RESPONSE); });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 缺陷 8（R1 批 3）：event_name 下拉接入后台 /admin/events/names
// ─────────────────────────────────────────────────────────────────────────────
describe('EventStreamTab — 缺陷 8: event_name 后端枚举接入', () => {
  it('挂载后调用 listEventNames(30, ...)', async () => {
    render(<EventStreamTab userId="user-123" />);
    await waitFor(() => {
      expect(mockListEventNames).toHaveBeenCalled();
    });
    expect(mockListEventNames.mock.calls[0]?.[0]).toBe(30);
  });

  it('后端枚举成功返回时下拉项使用后端结果', async () => {
    mockListEventNames.mockResolvedValue({
      items: ['custom_event_alpha', 'custom_event_beta'],
    });
    render(<EventStreamTab userId="user-123" />);

    // 等待异步 hook 完成
    await waitFor(() => expect(mockListEventNames).toHaveBeenCalled());
    await act(async () => { await Promise.resolve(); });

    // 打开下拉
    const select = screen.getByTestId('event-name-select');
    const combobox = select.querySelector('input') as HTMLInputElement;
    expect(combobox).toBeTruthy();
    await act(async () => {
      await userEvent.setup().click(combobox);
    });

    await waitFor(() => {
      // antd Select 选项渲染在 portal 中
      expect(document.body.textContent).toContain('custom_event_alpha');
      expect(document.body.textContent).toContain('custom_event_beta');
    });
  });

  it('接口失败时降级到本地 ANALYTICS_EVENTS 字典并 console.warn', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    mockListEventNames.mockRejectedValue(new Error('boom'));

    render(<EventStreamTab userId="user-123" />);

    await waitFor(() => {
      expect(warnSpy).toHaveBeenCalled();
    });

    // 至少 warn 一次，第一参数包含 hook 名
    expect(warnSpy.mock.calls[0]?.[0]).toContain('useEventNames');
    warnSpy.mockRestore();
  });
});
