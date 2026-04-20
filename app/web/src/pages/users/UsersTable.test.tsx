/**
 * T-20006: UsersTable 组件测试
 *
 * 验收用例：
 *   C01: 渲染表格列标题
 *   C02: 传入 3 条 items → 渲染 3 行数据，显示正确手机号
 *   C03: loading=true → aria-busy=true
 *   C04: items=[] → 无数据行
 *   C05: normal 用户 → data-testid="status-tag-normal"
 *   C06: banned 用户 → data-testid="status-tag-banned"
 *   C07: 操作列"查看详情"按钮处于 disabled（T-20007 占位）
 *   C08: 点击刷新按钮调用 onRefresh
 *
 * T-20007 新增：
 *   T01: 传入 onViewDetail 时按钮可点击，点击后 onViewDetail 以该行 id 调用
 *   T02: 不传 onViewDetail 时按钮 disabled（向后兼容）
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, within } from '@testing-library/react';
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

import { UsersTable } from './UsersTable';
import type { AdminUserItem } from '../../core/network/apiClient';

// ── 测试数据 ──────────────────────────────────────────────────────────────
function makeUser(id: number, status: 'normal' | 'banned' = 'normal'): AdminUserItem {
  return {
    id: `user-${id}`,
    phone: `1380013800${id}`,
    nickname: `User${id}`,
    avatar: undefined,
    coin_balance: 100,
    vip_level: 0,
    status,
    created_at: '2025-01-01T00:00:00Z',
  };
}

const items3: AdminUserItem[] = [
  makeUser(1, 'normal'),
  makeUser(2, 'normal'),
  makeUser(3, 'banned'),
];

const defaultProps = {
  items: items3,
  total: 3,
  page: 1,
  pageSize: 20,
  loading: false,
  onPageChange: vi.fn(),
  onRefresh: vi.fn(),
};

beforeEach(() => {
  vi.clearAllMocks();
});

// ── C01: 列标题渲染 ────────────────────────────────────────────────────────
describe('UsersTable — C01: 列标题渲染', () => {
  it('显示手机号、昵称、状态、操作等列标题', () => {
    render(<UsersTable {...defaultProps} />);
    expect(screen.getByText('users.colPhone')).toBeInTheDocument();
    expect(screen.getByText('users.colNickname')).toBeInTheDocument();
    expect(screen.getByText('users.colStatus')).toBeInTheDocument();
    expect(screen.getByText('users.colActions')).toBeInTheDocument();
  });
});

// ── C02: 用户行渲染 ────────────────────────────────────────────────────────
describe('UsersTable — C02: 用户行渲染', () => {
  it('传入 3 条 items 渲染 3 行数据', () => {
    render(<UsersTable {...defaultProps} />);
    const table = screen.getByTestId('users-table');
    const rows = within(table).getAllByRole('row');
    // 数据行（去掉 header row）
    expect(rows.length - 1).toBe(3);
  });

  it('显示正确的手机号', () => {
    render(<UsersTable {...defaultProps} />);
    expect(screen.getByText('13800138001')).toBeInTheDocument();
  });

  it('显示正确的昵称', () => {
    render(<UsersTable {...defaultProps} />);
    expect(screen.getByText('User1')).toBeInTheDocument();
  });
});

// ── C03: loading 状态 ─────────────────────────────────────────────────────
describe('UsersTable — C03: loading 状态', () => {
  it('loading=true 时表格外层容器 aria-busy=true', () => {
    const { container } = render(<UsersTable {...defaultProps} loading={true} />);
    const busyContainer = container.querySelector('[aria-busy="true"]');
    expect(busyContainer).toBeInTheDocument();
  });
});

// ── C04: 空状态 ───────────────────────────────────────────────────────────
describe('UsersTable — C04: 空状态', () => {
  it('items=[] 时不渲染用户数据行', () => {
    render(<UsersTable {...defaultProps} items={[]} total={0} />);
    expect(screen.queryByText('13800138001')).not.toBeInTheDocument();
  });
});

// ── C05: normal 状态标签 ──────────────────────────────────────────────────
describe('UsersTable — C05: normal 状态 Tag', () => {
  it('normal 用户显示 data-testid="status-tag-normal"', () => {
    render(<UsersTable {...defaultProps} />);
    const normalTags = screen.getAllByTestId('status-tag-normal');
    expect(normalTags.length).toBeGreaterThan(0);
  });

  it('normal Tag 文本为 t("users.statusNormal")', () => {
    render(<UsersTable {...defaultProps} />);
    const normalTags = screen.getAllByTestId('status-tag-normal');
    expect(normalTags[0]).toHaveTextContent('users.statusNormal');
  });
});

// ── C06: banned 状态标签 ──────────────────────────────────────────────────
describe('UsersTable — C06: banned 状态 Tag', () => {
  it('banned 用户显示 data-testid="status-tag-banned"', () => {
    render(<UsersTable {...defaultProps} />);
    expect(screen.getByTestId('status-tag-banned')).toBeInTheDocument();
  });

  it('banned Tag 文本为 t("users.statusBanned")', () => {
    render(<UsersTable {...defaultProps} />);
    expect(screen.getByTestId('status-tag-banned')).toHaveTextContent('users.statusBanned');
  });
});

// ── C07: 查看详情按钮 disabled ────────────────────────────────────────────
describe('UsersTable — C07: 查看详情按钮 disabled', () => {
  it('操作列"查看详情"按钮处于 disabled（T-20007 占位）', () => {
    render(<UsersTable {...defaultProps} />);
    const detailBtns = screen.getAllByTestId('view-detail-btn');
    expect(detailBtns.length).toBe(3);
    detailBtns.forEach((btn) => expect(btn).toBeDisabled());
  });
});

// ── C08: 刷新按钮 ─────────────────────────────────────────────────────────
describe('UsersTable — C08: 刷新按钮', () => {
  it('点击刷新按钮调用 onRefresh', async () => {
    const user = userEvent.setup();
    render(<UsersTable {...defaultProps} />);

    const refreshBtn = screen.getByTestId('refresh-btn');
    await user.click(refreshBtn);

    expect(defaultProps.onRefresh).toHaveBeenCalledTimes(1);
  });
});

// ── T01: onViewDetail 传入时按钮可点击 ─────────────────────────────────────
describe('UsersTable — T01: onViewDetail 传入时按钮可点击', () => {
  it('传入 onViewDetail 时，点击第一行的查看详情按钮，onViewDetail 以该行 id 调用', async () => {
    const user = userEvent.setup();
    const onViewDetail = vi.fn();
    render(<UsersTable {...defaultProps} onViewDetail={onViewDetail} />);

    const detailBtns = screen.getAllByTestId('view-detail-btn');
    expect(detailBtns[0]).not.toBeDisabled();

    await user.click(detailBtns[0]);

    expect(onViewDetail).toHaveBeenCalledWith('user-1');
  });
});

// ── T02: 不传 onViewDetail 时按钮 disabled ────────────────────────────────
describe('UsersTable — T02: 不传 onViewDetail 按钮 disabled', () => {
  it('不传 onViewDetail 时所有查看详情按钮处于 disabled（向后兼容）', () => {
    render(<UsersTable {...defaultProps} />);

    const detailBtns = screen.getAllByTestId('view-detail-btn');
    detailBtns.forEach((btn) => expect(btn).toBeDisabled());
  });
});
