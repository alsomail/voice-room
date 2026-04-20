/**
 * T-20009: LogSearchForm 组件测试
 *
 * 验收用例：
 *   LF-01: 渲染搜索字段（操作人ID Input / 操作类型 Select / 搜索+重置按钮）
 *   LF-02: 点击搜索触发 onSearch 回调（携带 adminId）
 *   LF-03: 点击重置 → onReset 被调用，输入框清空
 *
 * 扩展用例：
 *   LF-04: 选择操作类型后点击搜索，onSearch 携带 action
 *   LF-05: 不输入任何内容点击搜索，onSearch 仍被调用
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
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

import { LogSearchForm } from './LogSearchForm';

const defaultProps = {
  onSearch: vi.fn(),
  onReset: vi.fn(),
};

beforeEach(() => {
  vi.clearAllMocks();
});

// ── LF-01: 渲染搜索字段 ────────────────────────────────────────────────────
describe('LogSearchForm — LF-01: 渲染搜索字段', () => {
  it('渲染操作人 ID 输入框', () => {
    render(<LogSearchForm {...defaultProps} />);
    expect(
      screen.getByPlaceholderText('logs.adminIdPlaceholder'),
    ).toBeInTheDocument();
  });

  it('渲染操作类型下拉框（有 combobox）', () => {
    render(<LogSearchForm {...defaultProps} />);
    // Select 渲染为 combobox
    const actionSelect = screen.getByTestId('action-select');
    expect(actionSelect).toBeInTheDocument();
  });

  it('渲染搜索按钮', () => {
    render(<LogSearchForm {...defaultProps} />);
    expect(screen.getByText('logs.search')).toBeInTheDocument();
  });

  it('渲染重置按钮', () => {
    render(<LogSearchForm {...defaultProps} />);
    expect(screen.getByText('logs.reset')).toBeInTheDocument();
  });
});

// ── LF-02: 搜索触发回调 ────────────────────────────────────────────────────
describe('LogSearchForm — LF-02: 搜索调用回调', () => {
  it('输入操作人 ID 并点击搜索，onSearch 以包含 adminId 的 filters 调用', async () => {
    const user = userEvent.setup();
    const onSearch = vi.fn();
    render(<LogSearchForm onSearch={onSearch} onReset={vi.fn()} />);

    await user.type(
      screen.getByPlaceholderText('logs.adminIdPlaceholder'),
      'uuid-admin-001',
    );
    await user.click(screen.getByText('logs.search'));

    expect(onSearch).toHaveBeenCalledWith(
      expect.objectContaining({ adminId: 'uuid-admin-001' }),
    );
  });

  it('不输入任何内容点击搜索，onSearch 仍被调用', async () => {
    const user = userEvent.setup();
    const onSearch = vi.fn();
    render(<LogSearchForm onSearch={onSearch} onReset={vi.fn()} />);

    await user.click(screen.getByText('logs.search'));

    expect(onSearch).toHaveBeenCalled();
  });
});

// ── LF-03: 重置清空表单 ────────────────────────────────────────────────────
describe('LogSearchForm — LF-03: 重置调用 onReset 并清空', () => {
  it('点击重置按钮 onReset 被调用', async () => {
    const user = userEvent.setup();
    const onReset = vi.fn();
    render(<LogSearchForm onSearch={vi.fn()} onReset={onReset} />);

    await user.click(screen.getByText('logs.reset'));

    expect(onReset).toHaveBeenCalled();
  });

  it('输入操作人 ID 后点击重置，输入框被清空', async () => {
    const user = userEvent.setup();
    render(<LogSearchForm {...defaultProps} />);

    const adminIdInput = screen.getByPlaceholderText('logs.adminIdPlaceholder');
    await user.type(adminIdInput, 'some-uuid');
    expect(adminIdInput).toHaveValue('some-uuid');

    await user.click(screen.getByText('logs.reset'));

    expect(adminIdInput).toHaveValue('');
  });
});

// ── LF-04: 操作类型筛选 ────────────────────────────────────────────────────
describe('LogSearchForm — LF-04: 操作类型筛选', () => {
  it('选择 ban_user 并点击搜索，onSearch 携带 { action: "ban_user" }', async () => {
    const user = userEvent.setup();
    const onSearch = vi.fn();
    render(<LogSearchForm onSearch={onSearch} onReset={vi.fn()} />);

    // 点击 Select combobox
    const actionSelectWrapper = screen.getByTestId('action-select');
    const combobox = actionSelectWrapper.querySelector('[role="combobox"]')!;
    await user.click(combobox);

    // 等待下拉出现并选择 ban_user
    await screen.findByText('logs.actionBanUser');
    await user.click(screen.getByText('logs.actionBanUser'));

    await user.click(screen.getByText('logs.search'));

    expect(onSearch).toHaveBeenCalledWith(
      expect.objectContaining({ action: 'ban_user' }),
    );
  });
});
