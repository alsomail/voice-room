/**
 * T-20009: LogsTable 组件测试
 *
 * 验收用例：
 *   LT-01: 渲染列标题（操作类型/目标ID/IP地址/操作时间等）
 *   LT-02: 传入 items → 正确渲染行，action 列使用 Tag 渲染
 *   LT-03: loading=true → aria-busy=true
 *
 * 扩展用例：
 *   LT-04: 空字段降级 → target_id/ip_address/detail 为 null 时显示 "-"
 *   LT-05: 点击刷新按钮调用 onRefresh
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

import { LogsTable } from './LogsTable';
import type { AdminLogItem } from '../../core/network/apiClient';

// ── 测试数据 ──────────────────────────────────────────────────────────────
function makeLogItem(
  id: number,
  action: string = 'ban_user',
  partial: Partial<AdminLogItem> = {},
): AdminLogItem {
  return {
    id: `log-${id}`,
    admin_id: `admin-${id}`,
    action,
    target_type: 'user',
    target_id: `target-${id}`,
    ip_address: `192.168.1.${id}`,
    detail: { reason: 'test' },
    created_at: '2025-01-01T00:00:00Z',
    ...partial,
  };
}

const items3: AdminLogItem[] = [
  makeLogItem(1, 'ban_user'),
  makeLogItem(2, 'unban_user'),
  makeLogItem(3, 'close_room'),
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

// ── LT-01: 列标题渲染 ──────────────────────────────────────────────────────
describe('LogsTable — LT-01: 列标题渲染', () => {
  it('显示操作类型列标题', () => {
    render(<LogsTable {...defaultProps} />);
    expect(screen.getByText('logs.colAction')).toBeInTheDocument();
  });

  it('显示目标 ID 列标题', () => {
    render(<LogsTable {...defaultProps} />);
    expect(screen.getByText('logs.colTargetId')).toBeInTheDocument();
  });

  it('显示 IP 地址列标题', () => {
    render(<LogsTable {...defaultProps} />);
    expect(screen.getByText('logs.colIpAddress')).toBeInTheDocument();
  });

  it('显示操作时间列标题', () => {
    render(<LogsTable {...defaultProps} />);
    expect(screen.getByText('logs.colCreatedAt')).toBeInTheDocument();
  });

  it('显示操作人 ID 列标题', () => {
    render(<LogsTable {...defaultProps} />);
    expect(screen.getByText('logs.colAdminId')).toBeInTheDocument();
  });
});

// ── LT-02: 数据行渲染 ──────────────────────────────────────────────────────
describe('LogsTable — LT-02: 数据行渲染', () => {
  it('传入 3 条 items 渲染 3 行数据', () => {
    render(<LogsTable {...defaultProps} />);
    const table = screen.getByTestId('logs-table');
    const rows = within(table).getAllByRole('row');
    // 数据行（去掉 header row）
    expect(rows.length - 1).toBe(3);
  });

  it('action 列使用 Tag 渲染（ban_user 显示对应 tag）', () => {
    render(<LogsTable {...defaultProps} />);
    // 找到 ban_user 对应的 Tag
    const banTags = screen.getAllByTestId('action-tag-ban_user');
    expect(banTags.length).toBeGreaterThan(0);
  });

  it('action 列 unban_user Tag 存在', () => {
    render(<LogsTable {...defaultProps} />);
    expect(screen.getByTestId('action-tag-unban_user')).toBeInTheDocument();
  });

  it('action 列 close_room Tag 存在', () => {
    render(<LogsTable {...defaultProps} />);
    expect(screen.getByTestId('action-tag-close_room')).toBeInTheDocument();
  });

  it('显示正确的 IP 地址', () => {
    render(<LogsTable {...defaultProps} />);
    expect(screen.getByText('192.168.1.1')).toBeInTheDocument();
  });
});

// ── LT-03: loading 状态 ────────────────────────────────────────────────────
describe('LogsTable — LT-03: loading 状态', () => {
  it('loading=true 时表格外层容器 aria-busy=true', () => {
    const { container } = render(<LogsTable {...defaultProps} loading={true} />);
    const busyContainer = container.querySelector('[aria-busy="true"]');
    expect(busyContainer).toBeInTheDocument();
  });

  it('loading=false 时 aria-busy=false', () => {
    const { container } = render(<LogsTable {...defaultProps} loading={false} />);
    const busyContainer = container.querySelector('[aria-busy="true"]');
    expect(busyContainer).not.toBeInTheDocument();
  });
});

// ── LT-04: 空字段降级 ─────────────────────────────────────────────────────
describe('LogsTable — LT-04: 空字段降级显示 "-"', () => {
  it('target_id=undefined 时显示 "-"', () => {
    const itemWithNull = makeLogItem(99, 'ban_user', {
      target_id: undefined,
      ip_address: undefined,
      detail: undefined,
    });
    render(
      <LogsTable
        {...defaultProps}
        items={[itemWithNull]}
        total={1}
      />,
    );
    // 至少有一个 "-" 在表格中
    const dashes = screen.getAllByText('-');
    expect(dashes.length).toBeGreaterThan(0);
  });
});

// ── LT-05: 刷新按钮 ────────────────────────────────────────────────────────
describe('LogsTable — LT-05: 刷新按钮', () => {
  it('点击刷新按钮调用 onRefresh', async () => {
    const user = userEvent.setup();
    render(<LogsTable {...defaultProps} />);

    const refreshBtn = screen.getByTestId('refresh-btn');
    await user.click(refreshBtn);

    expect(defaultProps.onRefresh).toHaveBeenCalledTimes(1);
  });
});
