/**
 * T-20006: UserSearchForm 组件测试
 *
 * 验收用例：
 *   F01: 渲染手机号 / 用户ID / 昵称 输入框和搜索/重置按钮
 *   F02: 输入手机号后点击搜索 → onSearch 以 phone 调用
 *   F03: 不输入任何内容点击搜索 → onSearch 以空 filters 调用
 *   F04: 点击重置 → onReset 被调用
 *   F05: 点击重置 → 表单字段被清空
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

import { UserSearchForm } from './UserSearchForm';

const defaultProps = {
  onSearch: vi.fn(),
  onReset: vi.fn(),
};

beforeEach(() => {
  vi.clearAllMocks();
});

// ── F01: 渲染搜索字段 ──────────────────────────────────────────────────────
describe('UserSearchForm — F01: 渲染搜索字段', () => {
  it('渲染手机号输入框', () => {
    render(<UserSearchForm {...defaultProps} />);
    expect(screen.getByPlaceholderText('users.phonePlaceholder')).toBeInTheDocument();
  });

  it('渲染用户ID输入框', () => {
    render(<UserSearchForm {...defaultProps} />);
    expect(screen.getByPlaceholderText('users.userIdPlaceholder')).toBeInTheDocument();
  });

  it('渲染昵称输入框', () => {
    render(<UserSearchForm {...defaultProps} />);
    expect(screen.getByPlaceholderText('users.nicknamePlaceholder')).toBeInTheDocument();
  });

  it('渲染搜索按钮', () => {
    render(<UserSearchForm {...defaultProps} />);
    expect(screen.getByText('users.search')).toBeInTheDocument();
  });

  it('渲染重置按钮', () => {
    render(<UserSearchForm {...defaultProps} />);
    expect(screen.getByText('users.reset')).toBeInTheDocument();
  });
});

// ── F02: 搜索调用 onSearch ─────────────────────────────────────────────────
describe('UserSearchForm — F02: 搜索调用回调', () => {
  it('输入手机号并点击搜索，onSearch 以包含 phone 的 filters 调用', async () => {
    const user = userEvent.setup();
    const onSearch = vi.fn();
    render(<UserSearchForm onSearch={onSearch} onReset={vi.fn()} />);

    await user.type(screen.getByPlaceholderText('users.phonePlaceholder'), '13800138000');
    await user.click(screen.getByText('users.search'));

    expect(onSearch).toHaveBeenCalledWith(
      expect.objectContaining({ phone: '13800138000' }),
    );
  });

  it('输入昵称并点击搜索，onSearch 以包含 nickname 的 filters 调用', async () => {
    const user = userEvent.setup();
    const onSearch = vi.fn();
    render(<UserSearchForm onSearch={onSearch} onReset={vi.fn()} />);

    await user.type(screen.getByPlaceholderText('users.nicknamePlaceholder'), 'testuser');
    await user.click(screen.getByText('users.search'));

    expect(onSearch).toHaveBeenCalledWith(
      expect.objectContaining({ nickname: 'testuser' }),
    );
  });
});

// ── F03: 空搜索 ────────────────────────────────────────────────────────────
describe('UserSearchForm — F03: 空搜索', () => {
  it('不输入任何内容点击搜索，onSearch 仍被调用', async () => {
    const user = userEvent.setup();
    const onSearch = vi.fn();
    render(<UserSearchForm onSearch={onSearch} onReset={vi.fn()} />);

    await user.click(screen.getByText('users.search'));

    expect(onSearch).toHaveBeenCalled();
  });
});

// ── F04: 重置调用 onReset ──────────────────────────────────────────────────
describe('UserSearchForm — F04: 点击重置调用 onReset', () => {
  it('点击重置按钮 onReset 被调用', async () => {
    const user = userEvent.setup();
    const onReset = vi.fn();
    render(<UserSearchForm onSearch={vi.fn()} onReset={onReset} />);

    await user.click(screen.getByText('users.reset'));

    expect(onReset).toHaveBeenCalled();
  });
});

// ── F05: 重置清空表单 ──────────────────────────────────────────────────────
describe('UserSearchForm — F05: 点击重置清空表单', () => {
  it('输入手机号后点击重置，手机号输入框被清空', async () => {
    const user = userEvent.setup();
    render(<UserSearchForm {...defaultProps} />);

    const phoneInput = screen.getByPlaceholderText('users.phonePlaceholder');
    await user.type(phoneInput, '13800138000');
    expect(phoneInput).toHaveValue('13800138000');

    await user.click(screen.getByText('users.reset'));

    expect(phoneInput).toHaveValue('');
  });

  it('输入昵称后点击重置，昵称输入框被清空', async () => {
    const user = userEvent.setup();
    render(<UserSearchForm {...defaultProps} />);

    const nicknameInput = screen.getByPlaceholderText('users.nicknamePlaceholder');
    await user.type(nicknameInput, 'testuser');
    expect(nicknameInput).toHaveValue('testuser');

    await user.click(screen.getByText('users.reset'));

    expect(nicknameInput).toHaveValue('');
  });
});
