/**
 * T-20004: RoomsTable 组件测试
 *
 * 验收用例：
 *   C01: 传入 3 条 items → 渲染 3 行
 *   C02: loading=true → Table loading 态
 *   C03: status=active → 绿色 Tag，data-testid="status-tag-active"
 *   C04: status=closed → 灰色 Tag，关闭按钮 disabled
 *   C05: 点击 active 行关闭按钮 → Popconfirm 出现
 *   C06: Popconfirm 确认 → onCloseRoom 以 roomId 调用
 *   C07: Popconfirm 取消 → onCloseRoom 不调用
 *   C08: closingId === row.room_id → 该按钮 loading
 *   C09: Input.Search 输入 → onFiltersChange({keyword}) 调用
 *   C10: total=0, items=[] → 空状态
 *   C11: 点击表格行 → onRowClick 以 room_id 调用
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, within, waitFor } from '@testing-library/react';
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

import { RoomsTable } from './RoomsTable';
import type { AdminRoomItem } from '../../core/network/apiClient';
import type { RoomsPageFilters } from './useRoomsPage';

// ── 测试数据 ──────────────────────────────────────────────────────────────
const makeRoom = (id: number, status: 'active' | 'closed' = 'active'): AdminRoomItem => ({
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
});

const items3: AdminRoomItem[] = [
  makeRoom(1, 'active'),
  makeRoom(2, 'active'),
  makeRoom(3, 'closed'),
];

const defaultProps = {
  items: items3,
  total: 3,
  page: 1,
  pageSize: 20,
  filters: {} as RoomsPageFilters,
  loading: false,
  closingId: null,
  onPageChange: vi.fn(),
  onFiltersChange: vi.fn(),
  onCloseRoom: vi.fn(),
  onRowClick: vi.fn(),
  onRefresh: vi.fn(),
};

beforeEach(() => {
  vi.clearAllMocks();
});

// ── C01: 3 条 items → 3 行 ────────────────────────────────────────────────
describe('RoomsTable — C01: 渲染行数', () => {
  it('传入 3 条 items 渲染 3 行数据', () => {
    render(<RoomsTable {...defaultProps} />);
    const table = screen.getByTestId('rooms-table');
    const rows = within(table).getAllByRole('row');
    // 数据行（去掉 header row）
    expect(rows.length - 1).toBe(3);
  });
});

// ── C02: loading=true → Table loading ────────────────────────────────────
describe('RoomsTable — C02: loading 状态', () => {
  it('loading=true 时表格外层容器 aria-busy=true', () => {
    const { container } = render(<RoomsTable {...defaultProps} loading={true} />);
    // RoomsTable 外层 div 添加 aria-busy={loading}
    const busyContainer = container.querySelector('[aria-busy="true"]');
    expect(busyContainer).toBeInTheDocument();
  });
});

// ── C03: status=active → green tag ───────────────────────────────────────
describe('RoomsTable — C03: active 状态 Tag', () => {
  it('active 行有 data-testid="status-tag-active"', () => {
    render(<RoomsTable {...defaultProps} />);
    const activeTags = screen.getAllByTestId('status-tag-active');
    expect(activeTags.length).toBeGreaterThan(0);
  });
});

// ── C04: status=closed → gray tag + button disabled ──────────────────────
describe('RoomsTable — C04: closed 状态 Tag 及按钮', () => {
  it('closed 行有 data-testid="status-tag-closed" 且关闭按钮 disabled', () => {
    render(<RoomsTable {...defaultProps} />);
    expect(screen.getByTestId('status-tag-closed')).toBeInTheDocument();

    const closedBtn = screen.getByTestId('close-btn-room-3');
    expect(closedBtn).toBeDisabled();
  });
});

// ── C05: 点击 active 行关闭按钮 → Popconfirm 出现 ─────────────────────
describe('RoomsTable — C05: 关闭按钮触发 Popconfirm', () => {
  it('点击 active 行关闭按钮后 Popconfirm 出现', async () => {
    const user = userEvent.setup();
    render(<RoomsTable {...defaultProps} />);

    const closeBtn = screen.getByTestId('close-btn-room-1');
    await user.click(closeBtn);

    expect(await screen.findByText('rooms.confirmClose')).toBeInTheDocument();
  });
});

// ── C06: Popconfirm 确认 → onCloseRoom 调用 ──────────────────────────────
describe('RoomsTable — C06: Popconfirm 确认', () => {
  it('点击确认后 onCloseRoom 以 roomId 调用', async () => {
    const user = userEvent.setup();
    render(<RoomsTable {...defaultProps} />);

    await user.click(screen.getByTestId('close-btn-room-1'));
    const confirmBtn = await screen.findByText('rooms.confirmCloseOk');
    await user.click(confirmBtn);

    expect(defaultProps.onCloseRoom).toHaveBeenCalledWith('room-1');
  });
});

// ── C07: Popconfirm 取消 → onCloseRoom 不调用 ────────────────────────────
describe('RoomsTable — C07: Popconfirm 取消', () => {
  it('点击取消后 onCloseRoom 不调用', async () => {
    const user = userEvent.setup();
    render(<RoomsTable {...defaultProps} />);

    await user.click(screen.getByTestId('close-btn-room-1'));
    const cancelBtn = await screen.findByText('rooms.confirmCloseCancel');
    await user.click(cancelBtn);

    expect(defaultProps.onCloseRoom).not.toHaveBeenCalled();
  });
});

// ── C08: closingId === row.room_id → 该按钮 loading ──────────────────────
describe('RoomsTable — C08: closingId loading 状态', () => {
  it('closingId="room-1" 时 room-1 的关闭按钮处于 loading', () => {
    render(<RoomsTable {...defaultProps} closingId="room-1" />);
    const btn = screen.getByTestId('close-btn-room-1');
    // Ant Design Button with loading 会设置 aria-busy 或包含 loading span
    // 检查按钮不可点击（loading 按钮 disabled）
    const hasLoadingIndicator =
      btn.getAttribute('aria-busy') === 'true' ||
      btn.querySelector('[class*="loading"]') !== null ||
      btn.classList.contains('ant-btn-loading');
    expect(hasLoadingIndicator).toBe(true);
  });
});

// ── C09: Input.Search 输入 → onFiltersChange 调用 ────────────────────────
describe('RoomsTable — C09: 搜索输入', () => {
  it('Input.Search 输入后 onFiltersChange({keyword}) 被调用', () => {
    render(<RoomsTable {...defaultProps} />);
    const searchInput = screen.getByPlaceholderText('rooms.search');
    fireEvent.change(searchInput, { target: { value: 'keyword' } });
    expect(defaultProps.onFiltersChange).toHaveBeenCalledWith({ keyword: 'keyword' });
  });
});

// ── C10: total=0, items=[] → 空状态 ──────────────────────────────────────
describe('RoomsTable — C10: 空状态', () => {
  it('items=[] 时没有数据行内容', () => {
    render(<RoomsTable {...defaultProps} items={[]} total={0} />);
    // 无任何房间数据行（通过检查无 Room 标题来验证）
    expect(screen.queryByText(/^Room \d+$/)).not.toBeInTheDocument();
  });
});

// ── C11: 点击表格行 → onRowClick 调用 ────────────────────────────────────
describe('RoomsTable — C11: 行点击', () => {
  it('点击行上的标题单元格 → onRowClick 以 room_id 调用', async () => {
    const user = userEvent.setup();
    render(<RoomsTable {...defaultProps} />);

    // 点击第一行的 title 列（非按钮区域）
    const titleCell = screen.getByText('Room 1');
    await user.click(titleCell);

    expect(defaultProps.onRowClick).toHaveBeenCalledWith('room-1');
  });
});

// ── C12: 受控搜索框 — filters.keyword 从 'test' 变为 '' 时搜索框清空 ────────
describe('RoomsTable — C12: 受控搜索框随 filters.keyword 更新', () => {
  it('filters.keyword 从 "test" 变为 "" 时搜索框值清空', () => {
    const { rerender } = render(
      <RoomsTable {...defaultProps} filters={{ keyword: 'test' }} />,
    );
    const searchInput = screen.getByPlaceholderText('rooms.search');
    expect(searchInput).toHaveValue('test');

    rerender(<RoomsTable {...defaultProps} filters={{ keyword: '' }} />);
    expect(searchInput).toHaveValue('');
  });
});

// ── C13: active 状态 Tag 显示 i18n 翻译文本（非字面量 "active"）────────────
describe('RoomsTable — C13: active Tag 使用 i18n 文本', () => {
  it('active 状态 Tag 文本为 t("rooms.statusActive")，而非字面量 "active"', () => {
    render(<RoomsTable {...defaultProps} />);
    const activeTags = screen.getAllByTestId('status-tag-active');
    // mock 中 t(key) => key，所以期望文本为 'rooms.statusActive'
    expect(activeTags[0]).toHaveTextContent('rooms.statusActive');
    expect(activeTags[0]).not.toHaveTextContent('active');
  });
});

// ── C14: closed 状态 Tag 显示 i18n 翻译文本（非字面量 "closed"）────────────
describe('RoomsTable — C14: closed Tag 使用 i18n 文本', () => {
  it('closed 状态 Tag 文本为 t("rooms.statusClosed")，而非字面量 "closed"', () => {
    render(<RoomsTable {...defaultProps} />);
    const closedTag = screen.getByTestId('status-tag-closed');
    expect(closedTag).toHaveTextContent('rooms.statusClosed');
    expect(closedTag).not.toHaveTextContent('closed');
  });
});

// ════════════════════════════════════════════════════════════════════════════
// T-20011: 新增列/筛选/行高亮测试（C15–C26）
// ════════════════════════════════════════════════════════════════════════════

/** 辅助：生成 N 分钟前的 ISO 时间（相对于测试执行时间，确保时间判断稳定） */
const buildCreatedAt = (minsAgo: number) =>
  new Date(Date.now() - minsAgo * 60 * 1000).toISOString();

// 含不同活跃状态的测试数据集
const activeRoom: AdminRoomItem = {
  room_id: 'room-active',
  title: 'Active Room',
  room_type: 'normal',
  member_count: 5,   // active: ≥5
  max_members: 20,
  status: 'active',
  owner_id: 'u1',
  owner_nickname: 'Owner1',
  owner_avatar: null,
  created_at: buildCreatedAt(30),
};

const abnormalRoom: AdminRoomItem = {
  room_id: 'room-abnormal',
  title: 'Abnormal Room',
  room_type: 'normal',
  member_count: 0,   // abnormal: 0人+active
  max_members: 20,
  status: 'active',
  owner_id: 'u2',
  owner_nickname: 'Owner2',
  owner_avatar: null,
  created_at: buildCreatedAt(30),
};

const quietRoom: AdminRoomItem = {
  room_id: 'room-quiet',
  title: 'Quiet Room',
  room_type: 'normal',
  member_count: 2,   // quiet: 1-4人且>1h
  max_members: 20,
  status: 'active',
  owner_id: 'u3',
  owner_nickname: 'Owner3',
  owner_avatar: null,
  created_at: buildCreatedAt(90),  // 90分钟前
};

const normalRoom: AdminRoomItem = {
  room_id: 'room-normal',
  title: 'Normal Room',
  room_type: 'normal',
  member_count: 3,   // normal: 1-4人且≤1h
  max_members: 20,
  status: 'active',
  owner_id: 'u4',
  owner_nickname: 'Owner4',
  owner_avatar: null,
  created_at: buildCreatedAt(30),  // 30分钟前
};

const activityItems = [activeRoom, abnormalRoom, quietRoom, normalRoom];

const activityProps = {
  ...defaultProps,
  items: activityItems,
  total: 4,
};

// ── C15: 3行数据 → 每行有 room-activity-tag testid ──────────────────────────
describe('RoomsTable — C15: 每行有活跃状态 Tag testid', () => {
  it('4条数据 → 4个 room-activity-tag testid', () => {
    render(<RoomsTable {...activityProps} />);
    expect(screen.getByTestId('room-activity-tag-room-active')).toBeInTheDocument();
    expect(screen.getByTestId('room-activity-tag-room-abnormal')).toBeInTheDocument();
    expect(screen.getByTestId('room-activity-tag-room-quiet')).toBeInTheDocument();
    expect(screen.getByTestId('room-activity-tag-room-normal')).toBeInTheDocument();
  });
});

// ── C16: member_count=5 → Tag level='active'（绿） ──────────────────────────
describe('RoomsTable — C16: 活跃房间 → success 色 Tag', () => {
  it('member_count=5 的房间 Tag 带有 ant-tag-success 类', () => {
    render(<RoomsTable {...activityProps} />);
    const tag = screen.getByTestId('room-activity-tag-room-active');
    expect(tag.className).toContain('ant-tag-success');
  });
});

// ── C17: member_count=0+active → level='abnormal'（红） ─────────────────────
describe('RoomsTable — C17: 异常房间 → error 色 Tag', () => {
  it('member_count=0 且 status=active 的房间 Tag 带有 ant-tag-error 类', () => {
    render(<RoomsTable {...activityProps} />);
    const tag = screen.getByTestId('room-activity-tag-room-abnormal');
    expect(tag.className).toContain('ant-tag-error');
  });
});

// ── C18: member_count=2, 90min前 → level='quiet'（黄） ──────────────────────
describe('RoomsTable — C18: 冷清房间 → warning 色 Tag', () => {
  it('member_count=2 且 created 90min 前的房间 Tag 带有 ant-tag-warning 类', () => {
    render(<RoomsTable {...activityProps} />);
    const tag = screen.getByTestId('room-activity-tag-room-quiet');
    expect(tag.className).toContain('ant-tag-warning');
  });
});

// ── C19: 正常房间 → level='normal'（蓝） ────────────────────────────────────
describe('RoomsTable — C19: 正常房间 → processing 色 Tag', () => {
  it('member_count=3 且 created 30min 前的房间 Tag 带有 ant-tag-processing 类', () => {
    render(<RoomsTable {...activityProps} />);
    const tag = screen.getByTestId('room-activity-tag-room-normal');
    expect(tag.className).toContain('ant-tag-processing');
  });
});

// ── C20: 每行含 room-duration testid，格式正确 ──────────────────────────────
describe('RoomsTable — C20: 持续时长列', () => {
  it('每行都有 room-duration testid 且内容为时长格式', () => {
    render(<RoomsTable {...activityProps} />);
    // 验证存在
    expect(screen.getByTestId('room-duration-room-active')).toBeInTheDocument();
    expect(screen.getByTestId('room-duration-room-abnormal')).toBeInTheDocument();
    expect(screen.getByTestId('room-duration-room-quiet')).toBeInTheDocument();
    expect(screen.getByTestId('room-duration-room-normal')).toBeInTheDocument();
    // 验证格式（数字 + 单位）
    const durationText = screen.getByTestId('room-duration-room-active').textContent ?? '';
    expect(durationText).toMatch(/^\d+(m|\d+h \d+m|\d+d \d+h)$/);
  });
});

// ── C21: 异常房间行背景 rgba(231, 76, 60, 0.1) ──────────────────────────────
describe('RoomsTable — C21: 异常行高亮背景', () => {
  it('异常房间所在 tr 有高亮背景色', () => {
    render(<RoomsTable {...activityProps} />);
    const activityTag = screen.getByTestId('room-activity-tag-room-abnormal');
    const row = activityTag.closest('tr');
    expect(row).not.toBeNull();
    expect(row!.style.background).toBe('rgba(231, 76, 60, 0.1)');
  });
});

// ── C22: 非异常房间无高亮背景 ──────────────────────────────────────────────
describe('RoomsTable — C22: 非异常行无高亮', () => {
  it('active/quiet/normal 房间所在 tr 不含高亮背景色', () => {
    render(<RoomsTable {...activityProps} />);

    for (const roomId of ['room-active', 'room-quiet', 'room-normal']) {
      const tag = screen.getByTestId(`room-activity-tag-${roomId}`);
      const row = tag.closest('tr');
      expect(row).not.toBeNull();
      expect(row!.style.background).not.toBe('rgba(231, 76, 60, 0.1)');
    }
  });
});

// ── C23: 工具栏含 activity-filter Select ────────────────────────────────────
describe('RoomsTable — C23: 工具栏活跃度筛选', () => {
  it('工具栏存在 data-testid="activity-filter" 的 Select', () => {
    render(<RoomsTable {...defaultProps} />);
    expect(screen.getByTestId('activity-filter')).toBeInTheDocument();
  });
});

// ── C24: 选择"异常" → onActivityFilterChange('abnormal') 被调用 ─────────────
describe('RoomsTable — C24: 选择活跃度筛选选项', () => {
  it('选择异常选项后 onActivityFilterChange("abnormal") 被调用', async () => {
    const user = userEvent.setup();
    const onActivityFilterChange = vi.fn();
    render(
      <RoomsTable
        {...defaultProps}
        onActivityFilterChange={onActivityFilterChange}
      />,
    );

    const filterContainer = screen.getByTestId('activity-filter');
    const combobox = within(filterContainer).getByRole('combobox');
    await user.click(combobox);

    await waitFor(() => {
      expect(document.querySelector('.ant-select-dropdown')).toBeInTheDocument();
    });
    const dropdown = document.querySelector('.ant-select-dropdown') as HTMLElement;
    const abnormalOption = within(dropdown).getByText('rooms.activityLevelAbnormal');
    await user.click(abnormalOption);

    expect(onActivityFilterChange).toHaveBeenCalledWith('abnormal');
  });
});

// ── C25: activityFilter='active' → Select 显示受控值 ────────────────────────
describe('RoomsTable — C25: activityFilter 受控显示', () => {
  it('activityFilter="active" 时 Select 显示对应 label', () => {
    render(<RoomsTable {...defaultProps} activityFilter="active" />);
    const filterContainer = screen.getByTestId('activity-filter');
    // Ant Design Select 的选中项显示在 selection-item 中
    expect(filterContainer.textContent).toContain('rooms.activityLevelActive');
  });
});

// ── C26: 点击行 onRowClick 正常触发，不受活跃度筛选影响 ─────────────────────
describe('RoomsTable — C26: 筛选不影响行点击', () => {
  it('activityFilter 存在时点击行仍触发 onRowClick', async () => {
    const user = userEvent.setup();
    const onRowClick = vi.fn();
    render(
      <RoomsTable
        {...defaultProps}
        items={activityItems}
        total={4}
        activityFilter="active"
        onRowClick={onRowClick}
      />,
    );

    await user.click(screen.getByText('Active Room'));
    expect(onRowClick).toHaveBeenCalledWith('room-active');
  });
});
