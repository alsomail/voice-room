/**
 * T-20001: 管理员登录页 UI — TDD 测试套件
 *
 * 覆盖范围：
 *   - 渲染：用户名/密码输入框、记住密码复选框、登录按钮
 *   - 记住密码：localStorage 读写（REMEMBER_KEY = 'adminLoginUsername'）
 *   - 登录失败：onSubmit 抛出异常时展示 Alert 错误提示
 *   - 表单校验：空用户名/空密码提交时展示校验信息
 *   - i18n：文本由 useTranslation 提供，mock 后返回 key 本身
 *   - 边界：空表单、网络异常、已记住用户名的回填
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';

// ── i18n mock：t(key) 直接返回 key，便于断言 ──────────────────────────────────
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: {
      changeLanguage: vi.fn(),
      language: 'en',
    },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

import { LoginForm } from './LoginForm';

// ── 常量 ────────────────────────────────────────────────────────────────────
const REMEMBER_KEY = 'adminLoginUsername';

// ── 辅助：清理 DOM & localStorage ────────────────────────────────────────────
beforeEach(() => {
  localStorage.clear();
});
afterEach(() => {
  localStorage.clear();
});

// ── 1. 渲染测试 ───────────────────────────────────────────────────────────────
describe('LoginForm — 渲染', () => {
  it('渲染用户名输入框', () => {
    render(<LoginForm onSubmit={vi.fn()} />);
    expect(screen.getByTestId('input-username')).toBeInTheDocument();
  });

  it('渲染密码输入框', () => {
    render(<LoginForm onSubmit={vi.fn()} />);
    expect(screen.getByTestId('input-password')).toBeInTheDocument();
  });

  it('渲染"记住密码"复选框', () => {
    render(<LoginForm onSubmit={vi.fn()} />);
    expect(screen.getByRole('checkbox')).toBeInTheDocument();
  });

  it('渲染登录提交按钮', () => {
    render(<LoginForm onSubmit={vi.fn()} />);
    expect(screen.getByTestId('btn-submit')).toBeInTheDocument();
  });

  it('使用 i18n key 作为记住密码文本（中英文支持）', () => {
    render(<LoginForm onSubmit={vi.fn()} />);
    expect(screen.getByText('login.rememberMe')).toBeInTheDocument();
  });

  it('使用 i18n key 作为按钮文本（中英文支持）', () => {
    render(<LoginForm onSubmit={vi.fn()} />);
    expect(screen.getByTestId('btn-submit')).toHaveTextContent('login.submit');
  });
});

// ── 2. 记住密码 —— localStorage 回填 ─────────────────────────────────────────
describe('LoginForm — 记住密码', () => {
  it('localStorage 有记住的用户名时，用户名输入框自动回填', () => {
    localStorage.setItem(REMEMBER_KEY, 'admin_user');
    render(<LoginForm onSubmit={vi.fn()} />);
    expect(screen.getByTestId('input-username')).toHaveValue('admin_user');
  });

  it('localStorage 有记住的用户名时，复选框默认选中', () => {
    localStorage.setItem(REMEMBER_KEY, 'admin_user');
    render(<LoginForm onSubmit={vi.fn()} />);
    expect(screen.getByRole('checkbox')).toBeChecked();
  });

  it('localStorage 无记住用户名时，用户名输入框为空', () => {
    render(<LoginForm onSubmit={vi.fn()} />);
    expect(screen.getByTestId('input-username')).toHaveValue('');
  });

  it('勾选记住密码并提交成功后，将用户名写入 localStorage', async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(<LoginForm onSubmit={onSubmit} />);

    await userEvent.type(screen.getByTestId('input-username'), 'admin_user');
    await userEvent.type(screen.getByTestId('input-password'), 'secret123');
    await userEvent.click(screen.getByRole('checkbox'));
    await userEvent.click(screen.getByTestId('btn-submit'));

    await waitFor(() => {
      expect(localStorage.getItem(REMEMBER_KEY)).toBe('admin_user');
    });
  });

  it('不勾选记住密码并提交成功后，从 localStorage 删除用户名', async () => {
    localStorage.setItem(REMEMBER_KEY, 'old_user');
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(<LoginForm onSubmit={onSubmit} />);

    // 已回填：取消勾选复选框
    await userEvent.click(screen.getByRole('checkbox'));
    await userEvent.type(screen.getByTestId('input-password'), 'secret123');
    await userEvent.click(screen.getByTestId('btn-submit'));

    await waitFor(() => {
      expect(localStorage.getItem(REMEMBER_KEY)).toBeNull();
    });
  });
});

// ── 3. 表单提交 ───────────────────────────────────────────────────────────────
describe('LoginForm — 表单提交', () => {
  it('填入合法值后点击登录，以正确参数调用 onSubmit', async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(<LoginForm onSubmit={onSubmit} />);

    await userEvent.type(screen.getByTestId('input-username'), 'admin');
    await userEvent.type(screen.getByTestId('input-password'), 'password123');
    await userEvent.click(screen.getByTestId('btn-submit'));

    await waitFor(() => {
      expect(onSubmit).toHaveBeenCalledTimes(1);
      expect(onSubmit).toHaveBeenCalledWith({
        username: 'admin',
        password: 'password123',
        remember: false,
      });
    });
  });

  it('提交期间按钮显示 loading 状态（onSubmit 为悬挂 Promise）', async () => {
    let resolve!: () => void;
    const onSubmit = vi.fn(
      () => new Promise<void>((r) => { resolve = r; }),
    );
    render(<LoginForm onSubmit={onSubmit} />);

    await userEvent.type(screen.getByTestId('input-username'), 'admin');
    await userEvent.type(screen.getByTestId('input-password'), 'pass');
    await userEvent.click(screen.getByTestId('btn-submit'));

    // 按钮应处于 disabled/loading
    await waitFor(() => {
      expect(screen.getByTestId('btn-submit')).toBeDisabled();
    });

    // 恢复
    act(() => { resolve(); });
  });
});

// ── 4. 登录失败提示 ────────────────────────────────────────────────────────────
describe('LoginForm — 登录失败提示', () => {
  it('onSubmit 抛出 Error 时展示错误信息（Alert）', async () => {
    const onSubmit = vi.fn().mockRejectedValue(new Error('Invalid admin credentials'));
    render(<LoginForm onSubmit={onSubmit} />);

    await userEvent.type(screen.getByTestId('input-username'), 'admin');
    await userEvent.type(screen.getByTestId('input-password'), 'wrong');
    await userEvent.click(screen.getByTestId('btn-submit'));

    await waitFor(() => {
      expect(screen.getByTestId('alert-error')).toBeInTheDocument();
      expect(screen.getByTestId('alert-error')).toHaveTextContent(
        'Invalid admin credentials',
      );
    });
  });

  it('onSubmit 抛出非 Error 对象时展示通用错误文案（i18n key）', async () => {
    const onSubmit = vi.fn().mockRejectedValue('string_error');
    render(<LoginForm onSubmit={onSubmit} />);

    await userEvent.type(screen.getByTestId('input-username'), 'admin');
    await userEvent.type(screen.getByTestId('input-password'), 'wrong');
    await userEvent.click(screen.getByTestId('btn-submit'));

    await waitFor(() => {
      expect(screen.getByTestId('alert-error')).toHaveTextContent(
        'login.error.unknown',
      );
    });
  });

  it('第二次成功提交后，清除之前的错误 Alert', async () => {
    const onSubmit = vi
      .fn()
      .mockRejectedValueOnce(new Error('bad'))
      .mockResolvedValueOnce(undefined);
    render(<LoginForm onSubmit={onSubmit} />);

    await userEvent.type(screen.getByTestId('input-username'), 'admin');
    await userEvent.type(screen.getByTestId('input-password'), 'wrong');
    await userEvent.click(screen.getByTestId('btn-submit'));
    await waitFor(() => expect(screen.getByTestId('alert-error')).toBeInTheDocument());

    await userEvent.click(screen.getByTestId('btn-submit'));
    await waitFor(() => {
      expect(screen.queryByTestId('alert-error')).not.toBeInTheDocument();
    });
  });
});

// ── 5. 表单校验（空值提交）────────────────────────────────────────────────────
describe('LoginForm — 表单校验', () => {
  it('用户名为空提交时展示校验信息', async () => {
    render(<LoginForm onSubmit={vi.fn()} />);

    await userEvent.type(screen.getByTestId('input-password'), 'password123');
    await userEvent.click(screen.getByTestId('btn-submit'));

    await waitFor(() => {
      expect(
        screen.getByText('login.validation.usernameRequired'),
      ).toBeInTheDocument();
    });
  });

  it('密码为空提交时展示校验信息', async () => {
    render(<LoginForm onSubmit={vi.fn()} />);

    await userEvent.type(screen.getByTestId('input-username'), 'admin');
    await userEvent.click(screen.getByTestId('btn-submit'));

    await waitFor(() => {
      expect(
        screen.getByText('login.validation.passwordRequired'),
      ).toBeInTheDocument();
    });
  });

  it('空表单提交时不调用 onSubmit', async () => {
    const onSubmit = vi.fn();
    render(<LoginForm onSubmit={onSubmit} />);

    await userEvent.click(screen.getByTestId('btn-submit'));

    await waitFor(() => {
      expect(onSubmit).not.toHaveBeenCalled();
    });
  });
});
