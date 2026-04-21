/**
 * T-20008: BanModal 组件测试
 *
 * 验收用例：
 *   M01: userId=null，Modal 不显示（open=false）
 *   M02: userId 非 null，Modal 显示，表单可见
 *   M03: 未选择时长直接提交，显示校验错误，Modal.confirm 不调用
 *   M04: 未选择原因直接提交，显示校验错误，Modal.confirm 不调用
 *   M05: 备注超过 200 字，显示校验错误，Modal.confirm 不调用
 *   M06: 表单填写完整，点击确认，触发 Modal.confirm 二次确认
 *   M07: 二次确认后，useBanUser.ban 以正确参数被调用
 *   M08: 封禁成功，onSuccess 以 userId 为参数被调用
 *   M09: 封禁失败（API 错误），Modal 不关闭，错误信息显示
 *   M10: 点击取消，onClose 被调用
 *   M11: 确认弹窗显示期间，提交按钮变为 disabled/loading，再次点击不触发新 confirm
 *   M12: confirm afterClose 触发后，按钮恢复可用（isConfirming=false）
 */

import { describe, it, expect, vi, beforeEach, afterEach, type MockInstance } from 'vitest';
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

// ── useBanUser mock ────────────────────────────────────────────────────────
vi.mock('./useBanUser', () => ({
  useBanUser: vi.fn(),
}));

import { useBanUser } from './useBanUser';
import { BanModal } from './BanModal';

const mockUseBanUser = useBanUser as ReturnType<typeof vi.fn>;

// ── 测试辅助 ──────────────────────────────────────────────────────────────
const USER_ID = 'user-uuid-ban-test';

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
  // 取最后一个可见的 dropdown
  const dropdowns = document.querySelectorAll(
    '.ant-select-dropdown:not(.ant-select-dropdown-hidden)',
  );
  const dropdown = dropdowns[dropdowns.length - 1] as HTMLElement;
  const option = within(dropdown).getByText(optionText);
  await user.click(option);
}

let mockBan: ReturnType<typeof vi.fn>;
let confirmSpy: MockInstance<(typeof Modal)['confirm']>;

beforeEach(() => {
  vi.clearAllMocks();
  mockBan = vi.fn().mockResolvedValue(undefined);
  mockUseBanUser.mockReturnValue({ loading: false, error: null, ban: mockBan });
  // 默认：confirm 不自动执行 onOk，仅记录调用
  confirmSpy = vi
    .spyOn(Modal, 'confirm')
    .mockImplementation(() => ({ destroy: vi.fn(), update: vi.fn() }));
});

afterEach(() => {
  confirmSpy?.mockRestore();
});

// ── M01: userId=null → Modal 不显示 ──────────────────────────────────────
describe('BanModal — M01: userId=null 不显示', () => {
  it('userId=null 时 Modal 不在 DOM 中', () => {
    render(<BanModal {...makeProps({ userId: null })} />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});

// ── M02: userId 非 null → Modal 显示，表单可见 ────────────────────────────
describe('BanModal — M02: 显示并包含表单', () => {
  it('userId 非 null 时 Modal 显示，duration/reason/remark 表单可见', async () => {
    render(<BanModal {...makeProps()} />);
    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });
    expect(screen.getByTestId('ban-duration-select')).toBeInTheDocument();
    expect(screen.getByTestId('ban-reason-select')).toBeInTheDocument();
    expect(screen.getByTestId('ban-remark-textarea')).toBeInTheDocument();
  });
});

// ── M03: 未选择时长 → 校验错误，不触发 Modal.confirm ──────────────────────
describe('BanModal — M03: 未选时长校验失败', () => {
  it('未选择封禁时长点击提交，出现校验错误，Modal.confirm 不调用', async () => {
    const user = userEvent.setup();
    render(<BanModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await user.click(screen.getByText('users.ban.submitBtn'));

    await waitFor(() => {
      expect(document.querySelector('.ant-form-item-explain-error')).toBeInTheDocument();
    });
    expect(confirmSpy).not.toHaveBeenCalled();
  });
});

// ── M04: 未选择原因 → 校验错误，不触发 Modal.confirm ──────────────────────
describe('BanModal — M04: 未选原因校验失败', () => {
  it('选了时长但未选原因点击提交，出现校验错误，Modal.confirm 不调用', async () => {
    const user = userEvent.setup();
    render(<BanModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    // 只选时长，不选原因
    await selectOption(user, 'ban-duration-select', 'users.ban.duration24h');
    await user.click(screen.getByText('users.ban.submitBtn'));

    await waitFor(() => {
      expect(document.querySelector('.ant-form-item-explain-error')).toBeInTheDocument();
    });
    expect(confirmSpy).not.toHaveBeenCalled();
  });
});

// ── M05: 备注超过 200 字 → 校验错误 ──────────────────────────────────────
describe('BanModal — M05: 备注超 200 字校验失败', () => {
  it('选了时长和原因，备注超过 200 字，出现校验错误，Modal.confirm 不调用', async () => {
    const user = userEvent.setup();
    render(<BanModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'ban-duration-select', 'users.ban.duration24h');
    await selectOption(user, 'ban-reason-select', 'users.ban.reasonViolation');

    // 使用 fireEvent.change 绕过 HTML maxLength 属性，直接设置超长值以触发 Form 规则校验
    const remark = screen.getByTestId('ban-remark-textarea');
    fireEvent.change(remark, { target: { value: 'a'.repeat(201) } });
    await user.click(screen.getByText('users.ban.submitBtn'));

    await waitFor(() => {
      expect(document.querySelector('.ant-form-item-explain-error')).toBeInTheDocument();
    });
    expect(confirmSpy).not.toHaveBeenCalled();
  });
});

// ── M06: 表单完整 → 触发 Modal.confirm ────────────────────────────────────
describe('BanModal — M06: 表单完整触发 Modal.confirm', () => {
  it('选择时长和原因后点击提交，Modal.confirm 被调用', async () => {
    const user = userEvent.setup();
    render(<BanModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'ban-duration-select', 'users.ban.duration24h');
    await selectOption(user, 'ban-reason-select', 'users.ban.reasonViolation');
    await user.click(screen.getByText('users.ban.submitBtn'));

    await waitFor(() => {
      expect(confirmSpy).toHaveBeenCalled();
    });
  });
});

// ── M07: 二次确认后，ban 以正确参数被调用 ─────────────────────────────────
describe('BanModal — M07: 确认后 ban 以正确参数调用', () => {
  it('Modal.confirm 确认后，useBanUser.ban 以 userId 和 req 被调用', async () => {
    confirmSpy.mockImplementation(({ onOk }: Parameters<typeof Modal.confirm>[0]) => {
      void (onOk as (() => Promise<void>) | undefined)?.();
      return { destroy: vi.fn(), update: vi.fn() };
    });
    const user = userEvent.setup();
    render(<BanModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'ban-duration-select', 'users.ban.duration24h');
    await selectOption(user, 'ban-reason-select', 'users.ban.reasonViolation');
    await user.click(screen.getByText('users.ban.submitBtn'));

    await waitFor(() => {
      expect(mockBan).toHaveBeenCalledWith(USER_ID, {
        action: 'ban',
        duration: 1440,
        reason: '违规内容',
        remark: '',
      });
    });
  });
});

// ── M08: 封禁成功 → onSuccess 以 userId 调用 ─────────────────────────────
describe('BanModal — M08: 封禁成功 onSuccess 调用', () => {
  it('ban() 成功后 onSuccess 以 userId 为参数被调用', async () => {
    confirmSpy.mockImplementation(({ onOk }: Parameters<typeof Modal.confirm>[0]) => {
      void (onOk as (() => Promise<void>) | undefined)?.();
      return { destroy: vi.fn(), update: vi.fn() };
    });
    mockBan.mockResolvedValue(undefined);
    const user = userEvent.setup();
    const props = makeProps();
    render(<BanModal {...props} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'ban-duration-select', 'users.ban.duration24h');
    await selectOption(user, 'ban-reason-select', 'users.ban.reasonViolation');
    await user.click(screen.getByText('users.ban.submitBtn'));

    await waitFor(() => {
      expect(props.onSuccess).toHaveBeenCalledWith(USER_ID);
    });
  });
});

// ── M09: 封禁失败 → Modal 不关闭，错误信息显示 ───────────────────────────
describe('BanModal — M09: 封禁失败不关闭', () => {
  it('ban() 失败时 onClose 不调用，错误 Alert 显示', async () => {
    confirmSpy.mockImplementation(({ onOk }: Parameters<typeof Modal.confirm>[0]) => {
      void (onOk as (() => Promise<void>) | undefined)?.();
      return { destroy: vi.fn(), update: vi.fn() };
    });
    mockBan.mockRejectedValue(new Error('封禁失败：服务器错误'));
    const user = userEvent.setup();
    const props = makeProps();
    render(<BanModal {...props} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'ban-duration-select', 'users.ban.duration24h');
    await selectOption(user, 'ban-reason-select', 'users.ban.reasonViolation');
    await user.click(screen.getByText('users.ban.submitBtn'));

    await waitFor(() => {
      expect(screen.getByTestId('ban-error-alert')).toBeInTheDocument();
    });
    expect(props.onClose).not.toHaveBeenCalled();
    expect(props.onSuccess).not.toHaveBeenCalled();
  });
});

// ── M10: 点击取消 → onClose 调用 ─────────────────────────────────────────
describe('BanModal — M10: 取消按钮', () => {
  it('点击取消按钮后 onClose 被调用', async () => {
    const user = userEvent.setup();
    const props = makeProps();
    render(<BanModal {...props} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await user.click(screen.getByText('users.ban.cancelBtn'));

    expect(props.onClose).toHaveBeenCalledTimes(1);
  });
});

// ── M11: 确认弹窗期间按钮禁用，再次点击不触发新 confirm ──────────────────
describe('BanModal — M11: 二次确认期间按钮禁用防并发', () => {
  it('Modal.confirm 弹出期间，提交按钮变为 disabled，再次点击不触发新 confirm', async () => {
    const user = userEvent.setup();
    render(<BanModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'ban-duration-select', 'users.ban.duration24h');
    await selectOption(user, 'ban-reason-select', 'users.ban.reasonViolation');

    // 第一次点击，触发 Modal.confirm（spy 不调用 afterClose，保持 isConfirming=true）
    await user.click(screen.getByText('users.ban.submitBtn'));

    await waitFor(() => {
      expect(confirmSpy).toHaveBeenCalledTimes(1);
    });

    // 按钮应变为 disabled（isConfirming=true）
    const submitBtn = screen.getByText('users.ban.submitBtn').closest('button')!;
    await waitFor(() => {
      expect(submitBtn).toBeDisabled();
    });

    // 再次点击（使用 fireEvent 绕过 pointer-events: none），验证 guard 生效，不触发新 confirm
    fireEvent.click(submitBtn);

    // 仍然只有 1 次调用
    expect(confirmSpy).toHaveBeenCalledTimes(1);
  });
});

// ── M12: afterClose 触发后按钮恢复可用 ───────────────────────────────────
describe('BanModal — M12: afterClose 后 isConfirming 恢复 false', () => {
  it('Modal.confirm 的 afterClose 回调触发后，提交按钮恢复可用', async () => {
    let capturedAfterClose: (() => void) | undefined;
    confirmSpy.mockImplementation((config: Parameters<typeof Modal.confirm>[0]) => {
      capturedAfterClose = config.afterClose;
      return { destroy: vi.fn(), update: vi.fn() };
    });

    const user = userEvent.setup();
    render(<BanModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'ban-duration-select', 'users.ban.duration24h');
    await selectOption(user, 'ban-reason-select', 'users.ban.reasonViolation');

    // 第一次点击，触发 Modal.confirm
    await user.click(screen.getByText('users.ban.submitBtn'));

    await waitFor(() => {
      expect(confirmSpy).toHaveBeenCalledTimes(1);
    });

    // 按钮应禁用
    const submitBtn = screen.getByText('users.ban.submitBtn').closest('button');
    await waitFor(() => {
      expect(submitBtn).toBeDisabled();
    });

    // 触发 afterClose（模拟用户关闭 confirm 弹窗）
    expect(capturedAfterClose).toBeDefined();
    act(() => {
      capturedAfterClose!();
    });

    // 按钮应恢复可用
    await waitFor(() => {
      expect(submitBtn).not.toBeDisabled();
    });
  });
});

// ── M13: 点击关闭按钮后 isConfirming 已重置，再次打开按钮可用 ────────────
describe('BanModal — M13: 关闭后再打开 isConfirming 已重置', () => {
  it('isConfirming=true 时点击取消按钮（handleClose），再次打开后提交按钮应恢复可用（不再 disabled）', async () => {
    const user = userEvent.setup();
    const props = makeProps();
    const { rerender } = render(<BanModal {...props} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await selectOption(user, 'ban-duration-select', 'users.ban.duration24h');
    await selectOption(user, 'ban-reason-select', 'users.ban.reasonViolation');

    // 触发 Modal.confirm，使 isConfirming=true
    await user.click(screen.getByText('users.ban.submitBtn'));
    await waitFor(() => expect(confirmSpy).toHaveBeenCalledTimes(1));

    await waitFor(() => {
      expect(screen.getByText('users.ban.submitBtn').closest('button')).toBeDisabled();
    });

    // 点击取消按钮，触发 handleClose → setIsConfirming(false)
    await user.click(screen.getByText('users.ban.cancelBtn'));
    expect(props.onClose).toHaveBeenCalledTimes(1);

    // 模拟父组件关闭后重新打开（userId: null → USER_ID）
    rerender(<BanModal {...props} userId={null} />);
    rerender(<BanModal {...props} userId={USER_ID} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    // 提交按钮应恢复可用（isConfirming 已被 handleClose 重置为 false）
    const btn = screen.getByText('users.ban.submitBtn').closest('button')!;
    await waitFor(() => expect(btn).not.toBeDisabled());
  });
});
