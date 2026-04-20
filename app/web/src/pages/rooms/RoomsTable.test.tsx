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
import { render, screen, fireEvent, within } from '@testing-library/react';
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
