/**
 * T-20004: RoomsPage 集成测试
 *
 * 验收用例：
 *   I01: mock API 返回 3 条 → Table 显示 3 行
 *   I02: mock API 失败 → 显示 data-testid="rooms-error"
 *   I03: Select 切换活跃 → adminGetRooms 以 status=active 重新调用
 *   I04: 搜索输入 + 300ms → adminGetRooms 以 keyword 调用
 *   I05: 确认关闭 → adminCloseRoom 调用 → 列表刷新
 *   I06: 点击行 → selectedRoomId 被设置
 *   I07: adminCloseRoom 失败 → 关闭失败 Alert，列表不刷新
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';

// ── i18n mock ─────────────────────────────────────────────────────────────
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { changeLanguage: vi.fn(), language: 'zh' },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ── apiClient mock ─────────────────────────────────────────────────────────
vi.mock('../../core/network/apiClient', () => ({
  adminGetRooms: vi.fn(),
  adminCloseRoom: vi.fn(),
  adminGetRoomDetail: vi.fn(),
}));

import { adminGetRooms, adminCloseRoom, adminGetRoomDetail } from '../../core/network/apiClient';
import { RoomsPage } from './index';
import type { AdminRoomItem, AdminRoomsData, AdminRoomDetail } from '../../core/network/apiClient';

const mockAdminGetRooms = adminGetRooms as ReturnType<typeof vi.fn>;
const mockAdminCloseRoom = adminCloseRoom as ReturnType<typeof vi.fn>;
const mockAdminGetRoomDetail = adminGetRoomDetail as ReturnType<typeof vi.fn>;

// ── 测试数据工厂 ───────────────────────────────────────────────────────────
function makeRoom(id: number, status: 'active' | 'closed' = 'active'): AdminRoomItem {
  return {
    room_id: `room-${id}`,
    title: `Room ${id}`,
    room_type: 'normal',
    member_count: 5,
    max_members: 20,
    status,
    owner_id: `user-${id}`,
    owner_nickname: `Owner${id}`,
    owner_avatar: null,
    created_at: '2025-01-01T00:00:00Z',
  };
}

function makeRoomsData(count: number): AdminRoomsData {
  return {
    total: count,
    page: 1,
    page_size: 20,
    items: Array.from({ length: count }, (_, i) => makeRoom(i + 1)),
  };
}

function makeDetail(roomId: string): AdminRoomDetail {
  const num = roomId.replace('room-', '');
  return {
    room_id: roomId,
    title: `Room ${num} Detail`,
    status: 'active',
    room_type: 'normal',
    member_count: 5,
    max_members: 20,
    owner: {
      user_id: `user-${num}`,
      nickname: `DetailOwner${num}`,
      avatar: null,
    },
    mic_slots: [],
    created_at: '2025-01-01T00:00:00Z',
    updated_at: '2025-01-01T00:00:00Z',
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  // 为触发 modal 的测试提供安全默认值（如 I06 点击行）
  mockAdminGetRoomDetail.mockReturnValue(new Promise(() => {})); // pending，不干扰其他断言
});

afterEach(() => {
  vi.useRealTimers();
});

// ── I01: API 成功 → Table 显示 3 行 ──────────────────────────────────────
describe('RoomsPage — I01: API 成功', () => {
  it('mock API 返回 3 条，Table 显示 3 行', async () => {
    mockAdminGetRooms.mockResolvedValue(makeRoomsData(3));

    render(<RoomsPage />);

    await waitFor(() => expect(screen.getByTestId('rooms-table')).toBeInTheDocument());

    const table = screen.getByTestId('rooms-table');
    await waitFor(() => {
      const rows = within(table).getAllByRole('row');
      expect(rows.length - 1).toBe(3);
    });
  });
});

// ── I02: API 失败 → 显示 rooms-error ────────────────────────────────────
describe('RoomsPage — I02: API 失败', () => {
  it('mock API 失败时显示 data-testid="rooms-error"', async () => {
    mockAdminGetRooms.mockRejectedValue(new Error('Server Down'));

    render(<RoomsPage />);

    await waitFor(() => {
      expect(screen.getByTestId('rooms-error')).toBeInTheDocument();
    });
  });
});

// ── I03: Select 切换活跃 → adminGetRooms 以 status=active ────────────────
describe('RoomsPage — I03: status 过滤', () => {
  it('Select 切换到"活跃"后 adminGetRooms 以 status=active 被调用', async () => {
    mockAdminGetRooms.mockResolvedValue(makeRoomsData(3));

    render(<RoomsPage />);

    await waitFor(() => expect(screen.getByTestId('rooms-table')).toBeInTheDocument());

    mockAdminGetRooms.mockClear();

    // 找到 status filter 并交互
    const filterContainer = screen.getByTestId('status-filter');
    const combobox = within(filterContainer).getByRole('combobox');
    await userEvent.click(combobox);

    // Ant Design Select 下拉选项渲染到 document.body
    // 使用 within(dropdown) 定位，避免与表格中的 StatusTag 文本冲突
    await waitFor(() => {
      expect(document.querySelector('.ant-select-dropdown')).toBeInTheDocument();
    });
    const dropdown = document.querySelector('.ant-select-dropdown') as HTMLElement;
    const activeOption = within(dropdown).getByText('rooms.statusActive');
    await userEvent.click(activeOption);

    await waitFor(() => {
      expect(mockAdminGetRooms).toHaveBeenCalled();
      const lastCall = mockAdminGetRooms.mock.calls[mockAdminGetRooms.mock.calls.length - 1];
      expect(lastCall[0]).toMatchObject({ status: 'active' });
    });
  });
});

// ── I04: 搜索输入 + 300ms → adminGetRooms 以 keyword 调用 ─────────────────
describe('RoomsPage — I04: 搜索 debounce', () => {
  it('输入 keyword 后（debounce 300ms）触发 adminGetRooms', async () => {
    mockAdminGetRooms.mockResolvedValue(makeRoomsData(3));

    render(<RoomsPage />);

    await waitFor(() => expect(screen.getByTestId('rooms-table')).toBeInTheDocument());
    await waitFor(() => expect(mockAdminGetRooms).toHaveBeenCalled());

    mockAdminGetRooms.mockClear();

    const searchInput = screen.getByPlaceholderText('rooms.search');
    fireEvent.change(searchInput, { target: { value: 'test' } });

    // debounce 300ms 后应该调用 adminGetRooms，设置 2000ms 超时等待
    await waitFor(
      () => {
        expect(mockAdminGetRooms).toHaveBeenCalled();
        const lastCall = mockAdminGetRooms.mock.calls[mockAdminGetRooms.mock.calls.length - 1];
        expect(lastCall[0]).toMatchObject({ keyword: 'test' });
      },
      { timeout: 2000 },
    );
  });
});

// ── I05: 确认关闭 → adminCloseRoom + 列表刷新 ────────────────────────────
describe('RoomsPage — I05: 确认关闭', () => {
  it('确认 Popconfirm 后 adminCloseRoom 被调用，列表刷新', async () => {
    const user = userEvent.setup();
    mockAdminGetRooms.mockResolvedValue(makeRoomsData(3));
    mockAdminCloseRoom.mockResolvedValue(undefined);

    render(<RoomsPage />);

    await waitFor(() => expect(screen.getByTestId('rooms-table')).toBeInTheDocument());

    const initialCalls = mockAdminGetRooms.mock.calls.length;

    // 点击第一个关闭按钮（room-1，active）
    const closeBtn = screen.getByTestId('close-btn-room-1');
    await user.click(closeBtn);

    const confirmBtn = await screen.findByText('rooms.confirmCloseOk');
    await user.click(confirmBtn);

    await waitFor(() => expect(mockAdminCloseRoom).toHaveBeenCalledWith('room-1'));
    await waitFor(() =>
      expect(mockAdminGetRooms.mock.calls.length).toBeGreaterThan(initialCalls),
    );
  });
});

// ── I06: 点击行 → selectedRoomId 被设置 ──────────────────────────────────
describe('RoomsPage — I06: 行点击设置 selectedRoomId', () => {
  it('点击行后 selected-room-id 显示对应 roomId', async () => {
    const user = userEvent.setup();
    mockAdminGetRooms.mockResolvedValue(makeRoomsData(3));

    render(<RoomsPage />);

    await waitFor(() => expect(screen.getByText('Room 1')).toBeInTheDocument());

    await user.click(screen.getByText('Room 1'));

    await waitFor(() =>
      expect(screen.getByTestId('selected-room-id')).toHaveTextContent('room-1'),
    );
  });
});

// ── I07: adminCloseRoom 失败 → 关闭失败 Alert，列表不刷新 ─────────────────
describe('RoomsPage — I07: 关闭失败', () => {
  it('adminCloseRoom 失败后显示 rooms-error，列表不刷新', async () => {
    const user = userEvent.setup();
    mockAdminGetRooms.mockResolvedValue(makeRoomsData(3));
    mockAdminCloseRoom.mockRejectedValue(new Error('Close failed'));

    render(<RoomsPage />);

    await waitFor(() => expect(screen.getByTestId('rooms-table')).toBeInTheDocument());

    const initialCalls = mockAdminGetRooms.mock.calls.length;

    const closeBtn = screen.getByTestId('close-btn-room-1');
    await user.click(closeBtn);

    const confirmBtn = await screen.findByText('rooms.confirmCloseOk');
    await user.click(confirmBtn);

    await waitFor(() => expect(screen.getByTestId('rooms-error')).toBeInTheDocument());

    // 列表不应刷新（adminGetRooms 调用次数不变）
    expect(mockAdminGetRooms.mock.calls.length).toBe(initialCalls);
  });
});

// ── helpers ───────────────────────────────────────────────────────────────
// (reserved for future use)

// ════════════════════════════════════════════════════════════════════════════
// T-20005 集成测试 — RoomDetailModal 与 RoomsPage 的集成
// ════════════════════════════════════════════════════════════════════════════

import { Modal } from 'antd';

// ── T-20005 I01: 点击行 → Modal 可见 ─────────────────────────────────────
describe('RoomsPage (T-20005) — I01: 点击行打开 Modal', () => {
  it('点击表格行后 RoomDetailModal dialog 可见', async () => {
    const user = userEvent.setup();
    mockAdminGetRooms.mockResolvedValue(makeRoomsData(3));
    mockAdminGetRoomDetail.mockResolvedValue(makeDetail('room-1'));

    render(<RoomsPage />);
    await waitFor(() => expect(screen.getByText('Room 1')).toBeInTheDocument());

    await user.click(screen.getByText('Room 1'));

    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
  });
});

// ── T-20005 I02: 详情加载 → 展示 owner.nickname ──────────────────────────
describe('RoomsPage (T-20005) — I02: 展示房间详情', () => {
  it('adminGetRoomDetail 返回详情后 Modal 内展示 owner.nickname', async () => {
    const user = userEvent.setup();
    mockAdminGetRooms.mockResolvedValue(makeRoomsData(3));
    mockAdminGetRoomDetail.mockResolvedValue(makeDetail('room-1'));

    render(<RoomsPage />);
    await waitFor(() => expect(screen.getByText('Room 1')).toBeInTheDocument());

    await user.click(screen.getByText('Room 1'));

    await waitFor(() =>
      expect(screen.getByTestId('detail-basic-info')).toHaveTextContent('DetailOwner1'),
    );
  });
});

// ── T-20005 I03: 关闭 Modal → selectedRoomId=null，Modal 隐藏 ────────────
describe('RoomsPage (T-20005) — I03: 关闭 Modal', () => {
  it('点击 Modal 关闭按钮后 dialog 不可见', async () => {
    const user = userEvent.setup();
    mockAdminGetRooms.mockResolvedValue(makeRoomsData(3));
    mockAdminGetRoomDetail.mockResolvedValue(makeDetail('room-1'));

    render(<RoomsPage />);
    await waitFor(() => expect(screen.getByText('Room 1')).toBeInTheDocument());

    await user.click(screen.getByText('Room 1'));
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    const closeBtn = document.querySelector('.ant-modal-close') as HTMLElement;
    expect(closeBtn).toBeTruthy();
    await user.click(closeBtn);

    // selectedRoomId=null → selected-room-id span 消失（Modal 已关闭）
    await waitFor(() =>
      expect(screen.queryByTestId('selected-room-id')).not.toBeInTheDocument(),
    );
  });
});

// ── T-20005 I04: 确认强制关闭 → adminCloseRoom + Modal 关闭 + 列表刷新 ─────
describe('RoomsPage (T-20005) — I04: 确认强制关闭', () => {
  it('确认 Modal.confirm → adminCloseRoom 调用，Modal 关闭，列表刷新', async () => {
    const user = userEvent.setup();
    mockAdminGetRooms.mockResolvedValue(makeRoomsData(3));
    mockAdminGetRoomDetail.mockResolvedValue(makeDetail('room-1'));
    mockAdminCloseRoom.mockResolvedValue(undefined);

    let capturedOnOk: (() => Promise<void> | void) | undefined;
    const confirmSpy = vi.spyOn(Modal, 'confirm').mockImplementation((config) => {
      capturedOnOk = config?.onOk as (() => Promise<void>) | undefined;
      return { destroy: vi.fn(), update: vi.fn() } as ReturnType<typeof Modal.confirm>;
    });

    render(<RoomsPage />);
    await waitFor(() => expect(screen.getByText('Room 1')).toBeInTheDocument());

    await user.click(screen.getByText('Room 1'));

    // 等待详情加载和按钮可用
    await waitFor(() =>
      expect(screen.getByTestId('close-room-btn')).not.toBeDisabled(),
    );

    const initialCalls = mockAdminGetRooms.mock.calls.length;

    await user.click(screen.getByTestId('close-room-btn'));
    expect(capturedOnOk).toBeDefined();

    await capturedOnOk!();

    await waitFor(() => expect(mockAdminCloseRoom).toHaveBeenCalledWith('room-1'));
    // Modal 已关闭：onClose() 调用后 selectedRoomId=null，隐藏 span 消失
    await waitFor(() =>
      expect(screen.queryByTestId('selected-room-id')).not.toBeInTheDocument(),
    );
    // 列表刷新
    await waitFor(() =>
      expect(mockAdminGetRooms.mock.calls.length).toBeGreaterThan(initialCalls),
    );

    confirmSpy.mockRestore();
  });
});

// ── T-20005 I05: adminCloseRoom 失败 → Modal 不关闭，error 可见 ───────────
describe('RoomsPage (T-20005) — I05: 关闭失败 Modal 保持', () => {
  it('adminCloseRoom 失败后 Modal 不关闭，rooms-error 显示', async () => {
    const user = userEvent.setup();
    mockAdminGetRooms.mockResolvedValue(makeRoomsData(3));
    mockAdminGetRoomDetail.mockResolvedValue(makeDetail('room-1'));
    mockAdminCloseRoom.mockRejectedValue(new Error('Server Error'));

    let capturedOnOk: (() => Promise<void> | void) | undefined;
    const confirmSpy = vi.spyOn(Modal, 'confirm').mockImplementation((config) => {
      capturedOnOk = config?.onOk as (() => Promise<void>) | undefined;
      return { destroy: vi.fn(), update: vi.fn() } as ReturnType<typeof Modal.confirm>;
    });

    render(<RoomsPage />);
    await waitFor(() => expect(screen.getByText('Room 1')).toBeInTheDocument());

    await user.click(screen.getByText('Room 1'));
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
    await waitFor(() =>
      expect(screen.getByTestId('close-room-btn')).not.toBeDisabled(),
    );

    await user.click(screen.getByTestId('close-room-btn'));
    expect(capturedOnOk).toBeDefined();

    // 触发 onOk（会 throw，Modal.confirm 应保持打开）
    await expect(capturedOnOk!()).rejects.toThrow('Server Error');

    // Modal 仍然可见（onClose 未被调用）
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    // 列表页面显示错误
    await waitFor(() => expect(screen.getByTestId('rooms-error')).toBeInTheDocument());

    confirmSpy.mockRestore();
  });
});

// ── T-20005 I06: 快速点击两行 → 旧请求 abort，新请求有效 ─────────────────
describe('RoomsPage (T-20005) — I06: 快速切换行 abort 旧请求', () => {
  it('快速点击两行时旧 AbortController.abort 被调用', async () => {
    const user = userEvent.setup();
    mockAdminGetRooms.mockResolvedValue(makeRoomsData(3));

    // room-1 请求永不 resolve（模拟慢请求）
    mockAdminGetRoomDetail.mockReturnValueOnce(new Promise(() => {}));
    // room-2 请求正常 resolve
    mockAdminGetRoomDetail.mockResolvedValueOnce(makeDetail('room-2'));

    const abortSpy = vi.spyOn(AbortController.prototype, 'abort');

    render(<RoomsPage />);
    await waitFor(() => expect(screen.getByText('Room 1')).toBeInTheDocument());

    // 快速点击两行
    await user.click(screen.getByText('Room 1'));
    await user.click(screen.getByText('Room 2'));

    // 旧请求（room-1）的 controller 应被 abort
    await waitFor(() => expect(abortSpy).toHaveBeenCalled());

    abortSpy.mockRestore();
  });
});

// ════════════════════════════════════════════════════════════════════════════
// T-20011: RoomsPage 集成测试（I08–I13）
// ════════════════════════════════════════════════════════════════════════════

/** 生成 N 分钟前的 ISO 时间字符串 */
const buildCreatedAt = (minsAgo: number) =>
  new Date(Date.now() - minsAgo * 60 * 1000).toISOString();

/** 构造含不同活跃状态的混合测试数据 */
function makeMixedRoomsData(): AdminRoomsData {
  return {
    total: 3,
    page: 1,
    page_size: 20,
    items: [
      // active: member_count=5
      {
        room_id: 'room-act',
        title: 'Active Room',
        room_type: 'normal',
        member_count: 5,
        max_members: 20,
        status: 'active',
        owner_id: 'u1',
        owner_nickname: 'Owner1',
        owner_avatar: null,
        created_at: buildCreatedAt(30),
      },
      // abnormal: member_count=0, status=active
      {
        room_id: 'room-abn',
        title: 'Abnormal Room',
        room_type: 'normal',
        member_count: 0,
        max_members: 20,
        status: 'active',
        owner_id: 'u2',
        owner_nickname: 'Owner2',
        owner_avatar: null,
        created_at: buildCreatedAt(30),
      },
      // quiet: member_count=2, 155min前（即2h35m）→ duration "2h 35m"
      {
        room_id: 'room-qui',
        title: 'Quiet Room',
        room_type: 'normal',
        member_count: 2,
        max_members: 20,
        status: 'active',
        owner_id: 'u3',
        owner_nickname: 'Owner3',
        owner_avatar: null,
        created_at: buildCreatedAt(155),
      },
    ],
  };
}

// ── I08: mock 混合数据 → 三行渲染，活跃状态正确 ──────────────────────────────
describe('RoomsPage — I08: 混合数据渲染活跃状态', () => {
  it('三种活跃状态的房间都正确渲染对应 activity tag', async () => {
    mockAdminGetRooms.mockResolvedValue(makeMixedRoomsData());

    render(<RoomsPage />);

    await waitFor(() => expect(screen.getByText('Active Room')).toBeInTheDocument());

    // 三个 activity tag 都存在
    expect(screen.getByTestId('room-activity-tag-room-act')).toBeInTheDocument();
    expect(screen.getByTestId('room-activity-tag-room-abn')).toBeInTheDocument();
    expect(screen.getByTestId('room-activity-tag-room-qui')).toBeInTheDocument();

    // 颜色正确
    expect(screen.getByTestId('room-activity-tag-room-act').className).toContain('ant-tag-success');
    expect(screen.getByTestId('room-activity-tag-room-abn').className).toContain('ant-tag-error');
    expect(screen.getByTestId('room-activity-tag-room-qui').className).toContain('ant-tag-warning');
  });
});

// ── I09: 筛选"异常" → 只显示异常行，不发新 API 请求 ─────────────────────────
describe('RoomsPage — I09: 前端活跃度筛选', () => {
  it('选择 abnormal 过滤后只显示异常行，adminGetRooms 调用次数不变', async () => {
    const user = userEvent.setup();
    mockAdminGetRooms.mockResolvedValue(makeMixedRoomsData());

    render(<RoomsPage />);

    await waitFor(() => expect(screen.getByText('Active Room')).toBeInTheDocument());

    const apiCallsBefore = mockAdminGetRooms.mock.calls.length;

    // 打开活跃度筛选
    const filterContainer = screen.getByTestId('activity-filter');
    const combobox = within(filterContainer).getByRole('combobox');
    await user.click(combobox);

    await waitFor(() =>
      expect(document.querySelector('.ant-select-dropdown')).toBeInTheDocument(),
    );
    const dropdown = document.querySelector('.ant-select-dropdown') as HTMLElement;
    const abnormalOption = within(dropdown).getByText('rooms.activityLevelAbnormal');
    await user.click(abnormalOption);

    // 只有异常行可见
    await waitFor(() =>
      expect(screen.queryByText('Active Room')).not.toBeInTheDocument(),
    );
    expect(screen.getByText('Abnormal Room')).toBeInTheDocument();
    expect(screen.queryByText('Quiet Room')).not.toBeInTheDocument();

    // API 调用次数不变（纯前端过滤）
    expect(mockAdminGetRooms.mock.calls.length).toBe(apiCallsBefore);
  });
});

// ── I10: 异常房间行高亮背景可见 ──────────────────────────────────────────────
describe('RoomsPage — I10: 异常行高亮背景', () => {
  it('异常房间所在 tr 有高亮背景 rgba(231, 76, 60, 0.1)', async () => {
    mockAdminGetRooms.mockResolvedValue(makeMixedRoomsData());

    render(<RoomsPage />);

    await waitFor(() =>
      expect(screen.getByTestId('room-activity-tag-room-abn')).toBeInTheDocument(),
    );

    const tag = screen.getByTestId('room-activity-tag-room-abn');
    const row = tag.closest('tr');
    expect(row).not.toBeNull();
    expect(row!.style.background).toBe('rgba(231, 76, 60, 0.1)');
  });
});

// ── I11: 持续时长格式正确（2h35m 前的房间显示 "2h 35m"） ─────────────────────
describe('RoomsPage — I11: 持续时长格式', () => {
  it('155分钟前创建的房间显示 "2h 35m"', async () => {
    mockAdminGetRooms.mockResolvedValue(makeMixedRoomsData());

    render(<RoomsPage />);

    await waitFor(() =>
      expect(screen.getByTestId('room-duration-room-qui')).toBeInTheDocument(),
    );

    const durationText = screen.getByTestId('room-duration-room-qui').textContent ?? '';
    expect(durationText).toBe('2h 35m');
  });
});

// ── I12: 状态筛选与活跃度筛选可同时生效 ─────────────────────────────────────
describe('RoomsPage — I12: 状态筛选与活跃度筛选联合使用', () => {
  it('status=active API 过滤 + activityFilter=abnormal 前端过滤同时生效', async () => {
    const user = userEvent.setup();
    mockAdminGetRooms.mockResolvedValue(makeMixedRoomsData());

    render(<RoomsPage />);

    await waitFor(() => expect(screen.getByText('Active Room')).toBeInTheDocument());

    // 先设状态过滤（API 侧）
    const statusContainer = screen.getByTestId('status-filter');
    const statusCombobox = within(statusContainer).getByRole('combobox');
    await user.click(statusCombobox);

    // 等待 status 下拉框出现并选择"活跃"
    await waitFor(() =>
      expect(document.querySelector('.ant-select-dropdown')).toBeInTheDocument(),
    );
    const statusDropdown = document.querySelector('.ant-select-dropdown') as HTMLElement;
    await user.click(within(statusDropdown).getByText('rooms.statusActive'));

    await waitFor(() => expect(mockAdminGetRooms).toHaveBeenCalledWith(
      expect.objectContaining({ status: 'active' }),
      expect.anything(),
    ));

    // 再设活跃度过滤（前端侧）
    const activityContainer = screen.getByTestId('activity-filter');
    const activityCombobox = within(activityContainer).getByRole('combobox');
    await user.click(activityCombobox);

    // 通过 'rooms.activityAll'（status 下拉框不含此项）定位活跃度专属下拉框
    // 避免 'rooms.activityLevelAbnormal' 被表格 Tag 误匹配
    await waitFor(() => {
      const allDropdowns = document.querySelectorAll('.ant-select-dropdown');
      const activityDropdown = Array.from(allDropdowns).find(
        (d) => d.textContent?.includes('rooms.activityAll'),
      );
      expect(activityDropdown).toBeTruthy();
    });
    const allDropdowns = document.querySelectorAll('.ant-select-dropdown');
    const activityDropdown = Array.from(allDropdowns).find(
      (d) => d.textContent?.includes('rooms.activityAll'),
    ) as HTMLElement;
    await user.click(within(activityDropdown).getByText('rooms.activityLevelAbnormal'));

    // 只有异常行可见
    await waitFor(() =>
      expect(screen.queryByText('Active Room')).not.toBeInTheDocument(),
    );
    expect(screen.getByText('Abnormal Room')).toBeInTheDocument();
  });
});

// ── I13: 回归测试 — 原有功能（搜索/分页/关闭/详情/刷新）不可回归 ─────────────
describe('RoomsPage — I13: 回归验证（关键路径）', () => {
  it('添加活跃度筛选后搜索仍然正常工作', async () => {
    mockAdminGetRooms.mockResolvedValue(makeMixedRoomsData());

    render(<RoomsPage />);

    await waitFor(() => expect(screen.getByTestId('rooms-table')).toBeInTheDocument());
    await waitFor(() => expect(mockAdminGetRooms).toHaveBeenCalled());

    mockAdminGetRooms.mockClear();

    // 搜索功能
    const searchInput = screen.getByPlaceholderText('rooms.search');
    fireEvent.change(searchInput, { target: { value: 'test' } });

    await waitFor(
      () => {
        expect(mockAdminGetRooms).toHaveBeenCalled();
        const lastCall = mockAdminGetRooms.mock.calls[mockAdminGetRooms.mock.calls.length - 1];
        expect(lastCall[0]).toMatchObject({ keyword: 'test' });
      },
      { timeout: 2000 },
    );
  });

  it('添加活跃度筛选后关闭功能仍然正常', async () => {
    const user = userEvent.setup();
    mockAdminGetRooms.mockResolvedValue(makeMixedRoomsData());
    mockAdminCloseRoom.mockResolvedValue(undefined);

    render(<RoomsPage />);

    await waitFor(() => expect(screen.getByText('Active Room')).toBeInTheDocument());

    const closeBtn = screen.getByTestId('close-btn-room-act');
    await user.click(closeBtn);

    const confirmBtn = await screen.findByText('rooms.confirmCloseOk');
    await user.click(confirmBtn);

    await waitFor(() => expect(mockAdminCloseRoom).toHaveBeenCalledWith('room-act'));
  });
});
