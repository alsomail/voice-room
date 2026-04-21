/**
 * T-20010: UnbanModal 组件测试
 *
 * 验收用例：
 *   M01: userId=null，Modal 不显示（open=false）
 *   M02: userId 非 null，Modal 显示，data-testid="unban-modal" 存在
 *   M03: 未选择解封原因直接提交，显示校验错误，API 未被调用
 *   M04: 备注超过 200 字，显示校验错误
 *   M05: 表单填写完整，点击确认，触发 Modal.confirm 二次确认，isConfirming=true 期间按钮 disabled
 *   M06: 二次确认后，useUnbanUser.unban 以正确 userId 和 params 被调用
 *   M07: 解封成功，onSuccess 以 userId 为参数被调用
 *   M08: 解封失败（API 错误），Modal 不关闭，错误 Alert 可见
 *   M09: 解封失败（40901），错误信息展示友好文案
 *   M10: 点击取消，onClose 被调用
 *   M11: 并发防护：快速双击提交按钮，unban 仅被调用一次
 *   M12: afterClose 触发后 isConfirming 重置为 false，按钮恢复可用
 *   M13: 外层 Modal 关闭（直接 onClose）后重新打开，按钮不处于 disabled 状态
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, within, act, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import React from 'react';
import { Modal } from 'antd';

// ── i18n mock ─────────────────────────────────────────────────────────────
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { changeLanguage: vi.fn(), language: 'zh' },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ── useUnbanUser mock ──────────────────────────────────────────────────────
vi.mock('./useUnbanUser', () => ({
  useUnbanUser: vi.fn(),
}));

import { useUnbanUser } from './useUnbanUser';
import { UnbanModal } from './UnbanModal';

const mockUseUnbanUser = useUnbanUser as ReturnType<typeof vi.fn>;

// ── 测试辅助 ──────────────────────────────────────────────────────────────
const USER_ID = 'user-uuid-unban-test';

function makeProps(overrides: Partial<{
  userId: string | null;
  onClose: () => void;
  onSuccess: (userId: string) => void;
}> = {}) {
  return {
    userId: USER_ID,
    onClose: vi.fn(),
    onSuccess: vi.fn(),
    ...overrides,
  };
}

/** 选择 antd Select 选项 */
async function selectOption(
  user: ReturnType<typeof userEvent.setup>,
  testId: string,
  optionText: string,
) {
  const selectEl = screen.getByTestId(testId);
  const combobox = within(selectEl).getByRole('combobox');
  await user.click(combobox);
  await waitFor(() => {
    expect(document.querySelector('.ant-select-dropdown')).toBeInTheDocument();
  });
  const dropdowns = document.querySelectorAll(
    '.ant-select-dropdown:not(.ant-select-dropdown-hidden)',
  );
  const dropdown = dropdowns[dropdowns.length - 1] as HTMLElement;
  const option = within(dropdown).getByText(optionText);
  await user.click(option);
}

let mockUnban: ReturnType<typeof vi.fn>;
let confirmSpy: ReturnType<typeof vi.spyOn>;

beforeEach(() => {
  vi.clearAllMocks();
  mockUnban = vi.fn().mockResolvedValue(undefined);
  mockUseUnbanUser.mockReturnValue({ loading: false, error: null, unban: mockUnban });
  confirmSpy = vi
    .spyOn(Modal, 'confirm')
    .mockImplementation(() => ({ destroy: vi.fn(), update: vi.fn() }));
});

afterEach(() => {
  confirmSpy?.mockRestore();
});

// ── M01: userId=null → Modal 不显示 ──────────────────────────────────────
describe('UnbanModal — M01: userId=null 不显示', () => {
  it('userId=null 时 Modal 不在 DOM 中', () => {
    render(<UnbanModal {...makeProps({ userId: null })} />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});

// ── M02: userId 非 null → Modal 显示，data-testid 存在 ────────────────────
describe('UnbanModal — M02: 显示并包含表单', () => {
  it('userId 非 null 时 Modal 显示，data-testid="unban-modal" 可见，表单字段存在', async () => {
    render(<UnbanModal {...makeProps()} />);
    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });
    expect(screen.getByTestId('unban-modal')).toBeInTheDocument();
    expect(screen.getByTestId('unban-reason-select')).toBeInTheDocument();
    expect(screen.getByTestId('unban-remark-input')).toBeInTheDocument();
  });
});

// ── M03: 未选解封原因 → 校验错误，API 不调用 ─────────────────────────────
describe('UnbanModal — M03: 未选原因校验失败', () => {
  it('未选择解封原因点击提交，出现校验错误，Modal.confirm 不调用', async () => {
    const user = userEvent.setup();
    render(<UnbanModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await user.click(screen.getByTestId('unban-confirm-btn'));

    await waitFor(() => {
      expect(document.querySelector('.ant-form-item-explain-error')).toBeInTheDocument();
    });
    expect(confirmSpy).not.toHaveBeenCalled();
  });
});

// ── M04: 备注超过 200 字 → 校验错误 ──────────────────────────────────────
describe('UnbanModal — M04: 备注超 200 字校验失败', () => {
  it('选了原因，备注超过 200 字，出现校验错误，Modal.confirm 不调用', async () => {
    const user = userEvent.setup();
    render(<UnbanModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'unban-reason-select', 'users.unban.reasonExpired');

    const remark = screen.getByTestId('unban-remark-input');
    fireEvent.change(remark, { target: { value: 'a'.repeat(201) } });
    await user.click(screen.getByTestId('unban-confirm-btn'));

    await waitFor(() => {
      expect(document.querySelector('.ant-form-item-explain-error')).toBeInTheDocument();
    });
    expect(confirmSpy).not.toHaveBeenCalled();
  });
});

// ── M05: 表单完整 → 触发 Modal.confirm，isConfirming 期间按钮 disabled ────
describe('UnbanModal — M05: 表单完整触发 Modal.confirm', () => {
  it('选择原因后点击提交，Modal.confirm 被调用，isConfirming=true 期间按钮 disabled', async () => {
    const user = userEvent.setup();
    render(<UnbanModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'unban-reason-select', 'users.unban.reasonExpired');
    await user.click(screen.getByTestId('unban-confirm-btn'));

    await waitFor(() => {
      expect(confirmSpy).toHaveBeenCalled();
    });

    // isConfirming=true 期间按钮应 disabled
    const submitBtn = screen.getByTestId('unban-confirm-btn');
    await waitFor(() => {
      expect(submitBtn).toBeDisabled();
    });
  });
});

// ── M06: 二次确认后，unban 以正确参数调用 ────────────────────────────────
describe('UnbanModal — M06: 确认后 unban 以正确参数调用', () => {
  it('Modal.confirm 确认后，useUnbanUser.unban 以 userId 和 req 被调用', async () => {
    confirmSpy.mockImplementation(({ onOk }: Parameters<typeof Modal.confirm>[0]) => {
      void (onOk as (() => Promise<void>) | undefined)?.();
      return { destroy: vi.fn(), update: vi.fn() };
    });
    const user = userEvent.setup();
    render(<UnbanModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'unban-reason-select', 'users.unban.reasonExpired');
    await user.click(screen.getByTestId('unban-confirm-btn'));

    await waitFor(() => {
      expect(mockUnban).toHaveBeenCalledWith(USER_ID, {
        reason: '处罚到期',
        remark: undefined,
      });
    });
  });
});

// ── M07: 解封成功 → onSuccess 以 userId 调用 ─────────────────────────────
describe('UnbanModal — M07: 解封成功 onSuccess 调用', () => {
  it('unban() 成功后 onSuccess 以 userId 为参数被调用', async () => {
    confirmSpy.mockImplementation(({ onOk }: Parameters<typeof Modal.confirm>[0]) => {
      void (onOk as (() => Promise<void>) | undefined)?.();
      return { destroy: vi.fn(), update: vi.fn() };
    });
    mockUnban.mockResolvedValue(undefined);
    const user = userEvent.setup();
    const props = makeProps();
    render(<UnbanModal {...props} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'unban-reason-select', 'users.unban.reasonExpired');
    await user.click(screen.getByTestId('unban-confirm-btn'));

    await waitFor(() => {
      expect(props.onSuccess).toHaveBeenCalledWith(USER_ID);
    });
  });
});

// ── M08: 解封失败 → Modal 不关闭，错误 Alert 可见 ─────────────────────────
describe('UnbanModal — M08: 解封失败不关闭', () => {
  it('unban() 失败时 onClose 不调用，错误 Alert 显示', async () => {
    confirmSpy.mockImplementation(({ onOk }: Parameters<typeof Modal.confirm>[0]) => {
      void (onOk as (() => Promise<void>) | undefined)?.();
      return { destroy: vi.fn(), update: vi.fn() };
    });
    mockUnban.mockRejectedValue(new Error('解封失败：服务器错误'));
    const user = userEvent.setup();
    const props = makeProps();
    render(<UnbanModal {...props} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'unban-reason-select', 'users.unban.reasonExpired');
    await user.click(screen.getByTestId('unban-confirm-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('unban-error-alert')).toBeInTheDocument();
    });
    expect(props.onClose).not.toHaveBeenCalled();
    expect(props.onSuccess).not.toHaveBeenCalled();
  });
});

// ── M09: 解封失败（40901） → 友好文案 ────────────────────────────────────
describe('UnbanModal — M09: 40901 错误友好文案', () => {
  it('40901 错误时错误 Alert 显示 users.unban.alreadyNormal 文案', async () => {
    confirmSpy.mockImplementation(({ onOk }: Parameters<typeof Modal.confirm>[0]) => {
      void (onOk as (() => Promise<void>) | undefined)?.();
      return { destroy: vi.fn(), update: vi.fn() };
    });
    mockUnban.mockRejectedValue(new Error('[40901] 用户当前未被封禁'));
    const user = userEvent.setup();
    render(<UnbanModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'unban-reason-select', 'users.unban.reasonExpired');
    await user.click(screen.getByTestId('unban-confirm-btn'));

    await waitFor(() => {
      const alert = screen.getByTestId('unban-error-alert');
      expect(alert).toBeInTheDocument();
      // t('users.unban.alreadyNormal') 在 mock 中返回 key 本身
      expect(alert.textContent).toContain('users.unban.alreadyNormal');
    });
  });
});

// ── M10: 点击取消 → onClose 调用 ─────────────────────────────────────────
describe('UnbanModal — M10: 取消按钮', () => {
  it('点击取消按钮后 onClose 被调用', async () => {
    const user = userEvent.setup();
    const props = makeProps();
    render(<UnbanModal {...props} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await user.click(screen.getByTestId('unban-cancel-btn'));

    expect(props.onClose).toHaveBeenCalledTimes(1);
  });
});

// ── M11: 并发防护：快速双击，unban 仅调用一次 ────────────────────────────
describe('UnbanModal — M11: 并发防护快速双击', () => {
  it('Modal.confirm 弹出期间，提交按钮变为 disabled，再次点击不触发新 confirm', async () => {
    const user = userEvent.setup();
    render(<UnbanModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'unban-reason-select', 'users.unban.reasonExpired');

    await user.click(screen.getByTestId('unban-confirm-btn'));

    await waitFor(() => {
      expect(confirmSpy).toHaveBeenCalledTimes(1);
    });

    const submitBtn = screen.getByTestId('unban-confirm-btn');
    await waitFor(() => {
      expect(submitBtn).toBeDisabled();
    });

    fireEvent.click(submitBtn);
    expect(confirmSpy).toHaveBeenCalledTimes(1);
  });
});

// ── M12: afterClose 触发后按钮恢复可用 ───────────────────────────────────
describe('UnbanModal — M12: afterClose 后 isConfirming 恢复 false', () => {
  it('Modal.confirm 的 afterClose 回调触发后，提交按钮恢复可用', async () => {
    let capturedAfterClose: (() => void) | undefined;
    confirmSpy.mockImplementation((config: Parameters<typeof Modal.confirm>[0]) => {
      capturedAfterClose = config.afterClose;
      return { destroy: vi.fn(), update: vi.fn() };
    });

    const user = userEvent.setup();
    render(<UnbanModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'unban-reason-select', 'users.unban.reasonExpired');
    await user.click(screen.getByTestId('unban-confirm-btn'));

    await waitFor(() => {
      expect(confirmSpy).toHaveBeenCalledTimes(1);
    });

    const submitBtn = screen.getByTestId('unban-confirm-btn');
    await waitFor(() => {
      expect(submitBtn).toBeDisabled();
    });

    expect(capturedAfterClose).toBeDefined();
    act(() => {
      capturedAfterClose!();
    });

    await waitFor(() => {
      expect(submitBtn).not.toBeDisabled();
    });
  });
});

// ── M13: 关闭后再打开 isConfirming 已重置 ───────────────────────────────
describe('UnbanModal — M13: 关闭后再打开 isConfirming 已重置', () => {
  it('isConfirming=true 时点击取消，再次打开后提交按钮不 disabled', async () => {
    const user = userEvent.setup();
    const props = makeProps();
    const { rerender } = render(<UnbanModal {...props} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'unban-reason-select', 'users.unban.reasonExpired');

    await user.click(screen.getByTestId('unban-confirm-btn'));
    await waitFor(() => expect(confirmSpy).toHaveBeenCalledTimes(1));

    await waitFor(() => {
      expect(screen.getByTestId('unban-confirm-btn')).toBeDisabled();
    });

    await user.click(screen.getByTestId('unban-cancel-btn'));
    expect(props.onClose).toHaveBeenCalledTimes(1);

    rerender(<UnbanModal {...props} userId={null} />);
    rerender(<UnbanModal {...props} userId={USER_ID} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    const btn = screen.getByTestId('unban-confirm-btn');
    await waitFor(() => expect(btn).not.toBeDisabled());
  });
});
