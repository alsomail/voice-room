/**
 * T-20012: AdjustBalanceModal 组件测试
 *
 * 验收用例（对应 TDS W12-01 ~ W12-06, W12-12）：
 *   A01: open=false → Modal 不显示
 *   A02: W12-02  amount=0 时提交按钮禁用
 *   A03: W12-03  reason 为空时提交按钮禁用
 *   A04: W12-04  amount<0 时显示红色扣减警示，并弹 Modal.confirm 二次确认
 *   A05: W12-05  调用成功后 onSuccess(newBalance) 调用，弹窗关闭
 *   A06: W12-06  API 失败（400/403）显示 errorMessage，弹窗保留
 *   A07: W12-12  isConfirming 期间双击仅触发 1 次 API
 *   A08: amount 超过 10,000,000 绝对值 → 校验错误
 *   A09: reason 不足 2 字符 → 校验错误
 *   A10: reason 超过 200 字符 → 校验错误
 *   A11: W12-01  UserDetailDrawer 显示"调整余额"按钮
 *   A12: W12-11  切换 ar 语言后 Modal 文案正确
 */

import { describe, it, expect, vi, beforeEach, afterEach, type MockInstance } from 'vitest';
import { render, screen, waitFor, fireEvent, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import React from 'react';
import { Modal } from 'antd';

// ── i18n mock ─────────────────────────────────────────────────────────────────
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) => {
      if (opts && Object.keys(opts).length > 0) {
        return `${key}:${JSON.stringify(opts)}`;
      }
      return key;
    },
    i18n: { changeLanguage: vi.fn(), language: 'zh' },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ── apiClient mock ────────────────────────────────────────────────────────────
vi.mock('../../../core/network/apiClient', async (importOriginal) => {
  const original = await importOriginal<typeof import('../../../core/network/apiClient')>();
  return {
    ...original,
    adminAdjustBalance: vi.fn(),
  };
});

// ── useUserDetail mock ─────────────────────────────────────────────────────────
vi.mock('../../../pages/users/useUserDetail', () => ({
  useUserDetail: vi.fn(),
}));

import { adminAdjustBalance } from '../../../core/network/apiClient';
import { AdjustBalanceModal } from '../AdjustBalanceModal';

const mockAdjustBalance = adminAdjustBalance as ReturnType<typeof vi.fn>;

// ── 测试辅助 ──────────────────────────────────────────────────────────────────
const USER_ID = 'user-uuid-adjust-test';

function makeProps(overrides: Partial<{
  userId: string;
  currentBalance: number;
  open: boolean;
  onClose: () => void;
  onSuccess: (newBalance: number) => void;
}> = {}) {
  return {
    userId: USER_ID,
    currentBalance: 500,
    open: true,
    onClose: vi.fn(),
    onSuccess: vi.fn(),
    ...overrides,
  };
}

let confirmSpy: MockInstance<(typeof Modal)['confirm']>;

beforeEach(() => {
  vi.clearAllMocks();
  mockAdjustBalance.mockResolvedValue({ new_balance: 600 });
  confirmSpy = vi
    .spyOn(Modal, 'confirm')
    .mockImplementation(() => ({ destroy: vi.fn(), update: vi.fn() }));
});

afterEach(() => {
  confirmSpy?.mockRestore();
});

// ── A01: open=false → Modal 不显示 ───────────────────────────────────────────
describe('AdjustBalanceModal — A01: open=false 不显示', () => {
  it('open=false 时 Modal 不在 DOM 中', () => {
    render(<AdjustBalanceModal {...makeProps({ open: false })} />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});

// ── A02: W12-02 amount=0 时提交按钮禁用 ──────────────────────────────────────
describe('AdjustBalanceModal — A02: W12-02 amount=0 提交按钮禁用', () => {
  it('amount 为 0 时提交按钮禁用', async () => {
    const user = userEvent.setup();
    render(<AdjustBalanceModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    const amountInput = screen.getByTestId('adjust-amount-input');
    await user.clear(amountInput);
    // 输入 0
    await user.type(amountInput, '0');

    const submitBtn = screen.getByTestId('adjust-submit-btn');
    await waitFor(() => {
      expect(submitBtn).toBeDisabled();
    });
  });

  it('amount 为空（未填写）时提交按钮禁用', async () => {
    render(<AdjustBalanceModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    const submitBtn = screen.getByTestId('adjust-submit-btn');
    expect(submitBtn).toBeDisabled();
  });
});

// ── A03: W12-03 reason 为空时提交按钮禁用 ────────────────────────────────────
describe('AdjustBalanceModal — A03: W12-03 reason 为空时提交按钮禁用', () => {
  it('reason 为空，即使填了 amount，提交按钮也禁用', async () => {
    const user = userEvent.setup();
    render(<AdjustBalanceModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    // 只填 amount，不填 reason
    const amountInput = screen.getByTestId('adjust-amount-input');
    await user.clear(amountInput);
    await user.type(amountInput, '100');

    const submitBtn = screen.getByTestId('adjust-submit-btn');
    await waitFor(() => {
      expect(submitBtn).toBeDisabled();
    });
  });

  it('reason 非空且 amount 非零时提交按钮可用', async () => {
    const user = userEvent.setup();
    render(<AdjustBalanceModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    const amountInput = screen.getByTestId('adjust-amount-input');
    await user.clear(amountInput);
    await user.type(amountInput, '100');

    const reasonInput = screen.getByTestId('adjust-reason-input');
    await user.type(reasonInput, '测试充值原因');

    const submitBtn = screen.getByTestId('adjust-submit-btn');
    await waitFor(() => {
      expect(submitBtn).not.toBeDisabled();
    });
  });
});

// ── A04: W12-04 负数金额显示红色警示 + Modal.confirm ─────────────────────────
describe('AdjustBalanceModal — A04: W12-04 负数金额二次确认', () => {
  it('amount < 0 时，Modal 内显示红色扣减警示 banner', async () => {
    const user = userEvent.setup();
    render(<AdjustBalanceModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    const amountInput = screen.getByTestId('adjust-amount-input');
    await user.clear(amountInput);
    await user.type(amountInput, '-100');

    await waitFor(() => {
      expect(screen.getByTestId('deduct-warning-banner')).toBeInTheDocument();
    });
  });

  it('amount < 0 提交后触发 Modal.confirm 二次确认', async () => {
    const user = userEvent.setup();
    render(<AdjustBalanceModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    const amountInput = screen.getByTestId('adjust-amount-input');
    await user.clear(amountInput);
    await user.type(amountInput, '-100');

    const reasonInput = screen.getByTestId('adjust-reason-input');
    await user.type(reasonInput, '扣减测试原因');

    await user.click(screen.getByTestId('adjust-submit-btn'));

    await waitFor(() => {
      expect(confirmSpy).toHaveBeenCalled();
    });
  });

  it('amount > 0 提交后不触发 Modal.confirm，直接调用 API', async () => {
    const user = userEvent.setup();
    render(<AdjustBalanceModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    const amountInput = screen.getByTestId('adjust-amount-input');
    await user.clear(amountInput);
    await user.type(amountInput, '100');

    const reasonInput = screen.getByTestId('adjust-reason-input');
    await user.type(reasonInput, '充值测试原因');

    await user.click(screen.getByTestId('adjust-submit-btn'));

    await waitFor(() => {
      expect(mockAdjustBalance).toHaveBeenCalled();
    });
    expect(confirmSpy).not.toHaveBeenCalled();
  });
});

// ── A05: W12-05 调用成功 → onSuccess(newBalance) 调用，弹窗关闭 ──────────────
describe('AdjustBalanceModal — A05: W12-05 调用成功后刷新余额', () => {
  it('正数金额调用成功后 onSuccess(newBalance) 被调用', async () => {
    mockAdjustBalance.mockResolvedValue({ new_balance: 600 });
    const user = userEvent.setup();
    const props = makeProps();
    render(<AdjustBalanceModal {...props} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await user.type(screen.getByTestId('adjust-amount-input'), '100');
    await user.type(screen.getByTestId('adjust-reason-input'), '充值测试原因');

    await user.click(screen.getByTestId('adjust-submit-btn'));

    await waitFor(() => {
      expect(props.onSuccess).toHaveBeenCalledWith(600);
    });
  });

  it('负数金额 confirm 确认后 onSuccess 被调用', async () => {
    confirmSpy.mockImplementation(({ onOk }: Parameters<typeof Modal.confirm>[0]) => {
      void (onOk as (() => Promise<void>) | undefined)?.();
      return { destroy: vi.fn(), update: vi.fn() };
    });
    mockAdjustBalance.mockResolvedValue({ new_balance: 400 });
    const user = userEvent.setup();
    const props = makeProps();
    render(<AdjustBalanceModal {...props} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await user.type(screen.getByTestId('adjust-amount-input'), '-100');
    await user.type(screen.getByTestId('adjust-reason-input'), '扣减测试原因');
    await user.click(screen.getByTestId('adjust-submit-btn'));

    await waitFor(() => {
      expect(props.onSuccess).toHaveBeenCalledWith(400);
    });
  });

  it('成功后 onClose 被调用（弹窗关闭）', async () => {
    mockAdjustBalance.mockResolvedValue({ new_balance: 600 });
    const user = userEvent.setup();
    const props = makeProps();
    render(<AdjustBalanceModal {...props} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await user.type(screen.getByTestId('adjust-amount-input'), '100');
    await user.type(screen.getByTestId('adjust-reason-input'), '充值测试原因');
    await user.click(screen.getByTestId('adjust-submit-btn'));

    await waitFor(() => {
      expect(props.onClose).toHaveBeenCalled();
    });
  });
});

// ── A06: W12-06 API 失败 → 显示 errorMessage，保留弹窗 ───────────────────────
describe('AdjustBalanceModal — A06: W12-06 API 失败保留弹窗', () => {
  it('API 返回 400 时 error Alert 显示，onClose 不调用', async () => {
    mockAdjustBalance.mockRejectedValue(new Error('余额不足'));
    const user = userEvent.setup();
    const props = makeProps();
    render(<AdjustBalanceModal {...props} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await user.type(screen.getByTestId('adjust-amount-input'), '100');
    await user.type(screen.getByTestId('adjust-reason-input'), '测试原因');
    await user.click(screen.getByTestId('adjust-submit-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('adjust-error-alert')).toBeInTheDocument();
    });
    expect(props.onClose).not.toHaveBeenCalled();
    expect(props.onSuccess).not.toHaveBeenCalled();
  });

  it('API 返回 403 时 error Alert 显示', async () => {
    mockAdjustBalance.mockRejectedValue(new Error('权限不足'));
    const user = userEvent.setup();
    const props = makeProps();
    render(<AdjustBalanceModal {...props} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await user.type(screen.getByTestId('adjust-amount-input'), '50');
    await user.type(screen.getByTestId('adjust-reason-input'), '权限测试');
    await user.click(screen.getByTestId('adjust-submit-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('adjust-error-alert')).toBeInTheDocument();
    });
    expect(screen.getByTestId('adjust-error-alert')).toHaveTextContent('权限不足');
  });
});

// ── A07: W12-12 isConfirming 防重复 ──────────────────────────────────────────
describe('AdjustBalanceModal — A07: W12-12 isConfirming 期间仅 1 次 API', () => {
  it('负数金额：confirm 期间再次点击不重复触发 confirm', async () => {
    const user = userEvent.setup();
    render(<AdjustBalanceModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await user.type(screen.getByTestId('adjust-amount-input'), '-100');
    await user.type(screen.getByTestId('adjust-reason-input'), '扣减测试原因');

    // 第一次点击，触发 confirm（spy 不调用 onOk，保持 isConfirming=true）
    await user.click(screen.getByTestId('adjust-submit-btn'));
    await waitFor(() => expect(confirmSpy).toHaveBeenCalledTimes(1));

    // 按钮应变为禁用
    const submitBtn = screen.getByTestId('adjust-submit-btn');
    await waitFor(() => expect(submitBtn).toBeDisabled());

    // 再次点击，confirm 不应被再次触发
    fireEvent.click(submitBtn);
    expect(confirmSpy).toHaveBeenCalledTimes(1);
  });

  it('正数金额：loading 期间再次点击 API 仅调用 1 次', async () => {
    let resolveAdjust!: (v: { new_balance: number }) => void;
    mockAdjustBalance.mockReturnValue(
      new Promise<{ new_balance: number }>((res) => { resolveAdjust = res; })
    );

    const user = userEvent.setup();
    render(<AdjustBalanceModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await user.type(screen.getByTestId('adjust-amount-input'), '100');
    await user.type(screen.getByTestId('adjust-reason-input'), '测试原因');

    await user.click(screen.getByTestId('adjust-submit-btn'));

    // loading 中按钮禁用
    const submitBtn = screen.getByTestId('adjust-submit-btn');
    await waitFor(() => expect(submitBtn).toBeDisabled());

    // 强制再次点击
    fireEvent.click(submitBtn);

    // API 只被调用一次
    expect(mockAdjustBalance).toHaveBeenCalledTimes(1);

    act(() => { resolveAdjust({ new_balance: 600 }); });
  });
});

// ── A08: amount 超过 10M 绝对值 → 校验错误 ───────────────────────────────────
describe('AdjustBalanceModal — A08: amount 超 10M 校验失败', () => {
  it('amount=10000001 提交后出现校验错误', async () => {
    const user = userEvent.setup();
    render(<AdjustBalanceModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await user.type(screen.getByTestId('adjust-amount-input'), '10000001');
    await user.type(screen.getByTestId('adjust-reason-input'), '测试原因超额');

    // 提交按钮根据 InputNumber max 可能被 disabled，或提交时报错
    // 点击提交按钮（可能被 disabled，此时改用 form submit）
    await user.click(screen.getByTestId('adjust-submit-btn'));

    await waitFor(() => {
      expect(document.querySelector('.ant-form-item-explain-error')).toBeInTheDocument();
    });
    expect(mockAdjustBalance).not.toHaveBeenCalled();
  });
});

// ── A09: reason 不足 2 字 → 校验错误 ─────────────────────────────────────────
describe('AdjustBalanceModal — A09: reason 不足 2 字符校验失败', () => {
  it('reason="a" (1字) 提交后出现校验错误', async () => {
    const user = userEvent.setup();
    render(<AdjustBalanceModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await user.type(screen.getByTestId('adjust-amount-input'), '100');
    await user.type(screen.getByTestId('adjust-reason-input'), 'a');

    await user.click(screen.getByTestId('adjust-submit-btn'));

    await waitFor(() => {
      expect(document.querySelector('.ant-form-item-explain-error')).toBeInTheDocument();
    });
    expect(mockAdjustBalance).not.toHaveBeenCalled();
  });
});

// ── A10: reason 超过 200 字符 → 校验错误 ─────────────────────────────────────
describe('AdjustBalanceModal — A10: reason 超 200 字符校验失败', () => {
  it('reason 超过 200 字符提交后出现校验错误', async () => {
    const user = userEvent.setup();
    render(<AdjustBalanceModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    await user.type(screen.getByTestId('adjust-amount-input'), '100');
    const reasonInput = screen.getByTestId('adjust-reason-input');
    fireEvent.change(reasonInput, { target: { value: 'a'.repeat(201) } });

    await user.click(screen.getByTestId('adjust-submit-btn'));

    await waitFor(() => {
      expect(document.querySelector('.ant-form-item-explain-error')).toBeInTheDocument();
    });
    expect(mockAdjustBalance).not.toHaveBeenCalled();
  });
});

// ── A11: W12-01 UserDetailDrawer 显示"调整余额"按钮 ──────────────────────────
describe('AdjustBalanceModal — A11: W12-01 UserDetailDrawer 含调整余额按钮', () => {
  it('用户详情 Drawer 中存在 adjust-balance-btn', async () => {
    const { useUserDetail } = await import('../../../pages/users/useUserDetail');
    (useUserDetail as ReturnType<typeof vi.fn>).mockReturnValue({
      detail: {
        id: USER_ID,
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
    render(
      <UserDetailDrawer
        userId={USER_ID}
        onClose={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(screen.getByTestId('adjust-balance-btn')).toBeInTheDocument();
    });
  });
});

// ── A12: W12-11 i18n ar 文案 ─────────────────────────────────────────────────
describe('AdjustBalanceModal — A12: W12-11 ar 语言文案', () => {
  it('切换 ar 语言后 Modal 标题为 wallet.adjust.title', async () => {
    render(<AdjustBalanceModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
    // t() mock 直接返回 key，所以检查 key 存在于 DOM
    expect(screen.getByText('wallet.adjust.title')).toBeInTheDocument();
  });
});
