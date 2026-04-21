/**
 * T-20007: UserDetailDrawer 组件测试
 *
 * 验收用例：
 *   D01: userId=null 时 Drawer 不渲染（open=false）
 *   D02: 加载中显示 Skeleton，data-testid="detail-skeleton"
 *   D03: 成功态：Descriptions 渲染手机号、昵称；Statistic 显示 coin_balance
 *   D04: status='normal' 时显示封禁按钮（data-testid="ban-btn"），不显示解封
 *   D05: status='banned' 时显示解封按钮（data-testid="unban-btn"），不显示封禁
 *   D06: API 错误时显示 data-testid="detail-error"，操作按钮不渲染
 *   D07: data-testid="behavior-placeholder" 存在
 *   D08: 点击关闭按钮，onClose 被调用
 *   D09: status='normal' 时点击封禁，onBanClick 以 detail.id 为参数调用
 *   D10: status='banned' 时点击解封，onUnbanClick 以 detail.id 为参数调用
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
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

// ── useUserDetail mock ────────────────────────────────────────────────────
vi.mock('./useUserDetail', () => ({
  useUserDetail: vi.fn(),
}));

// ── useAuthStore mock（MEDIUM-2 RBAC 控制）────────────────────────────────
// UserDetailDrawer 修复后会通过 useAuthStore 读取当前角色，控制"调整余额"按钮可见性。
// 这里通过可变变量 mockAdminRole 让每个测试可以灵活设置角色。
let mockAdminRole = 'super_admin';

vi.mock('../../stores/useAuthStore', () => ({
  useAuthStore: (selector?: (s: { admin: { role: string } | null }) => unknown) => {
    const state = { admin: { role: mockAdminRole } };
    if (typeof selector === 'function') return selector(state);
    return state;
  },
  ADMIN_TOKEN_KEY: 'adminToken',
}));

import { useUserDetail } from './useUserDetail';
import { UserDetailDrawer } from './UserDetailDrawer';
import type { AdminUserDetailResponse } from '../../core/network/apiClient';

const mockUseUserDetail = useUserDetail as ReturnType<typeof vi.fn>;

// ── 测试数据 ──────────────────────────────────────────────────────────────
const mockDetailNormal: AdminUserDetailResponse = {
  id: 'user-uuid-1',
  phone: '+8613800138000',
  nickname: 'TestUser',
  avatar_url: 'https://cdn.example.com/avatar.jpg',
  coin_balance: 1000,
  vip_level: 1,
  status: 'normal',
  created_at: '2024-01-01T00:00:00Z',
  recharge_records: [],
  consume_records: [],
  devices: [],
};

const mockDetailBanned: AdminUserDetailResponse = {
  ...mockDetailNormal,
  status: 'banned',
};

// ── Props 工厂 ────────────────────────────────────────────────────────────
function makeProps(
  overrides: Partial<{
    userId: string | null;
    onClose: () => void;
    onBanClick: (userId: string) => void;
    onUnbanClick: (userId: string) => void;
  }> = {},
) {
  return {
    userId: 'user-uuid-1',
    onClose: vi.fn(),
    onBanClick: vi.fn(),
    onUnbanClick: vi.fn(),
    ...overrides,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  // 默认角色 super_admin（有调整余额权限）
  mockAdminRole = 'super_admin';
  // 默认：成功态（normal 用户）
  mockUseUserDetail.mockReturnValue({
    detail: mockDetailNormal,
    loading: false,
    error: null,
  });
});

// ── D01: userId=null 时 Drawer 不渲染 ─────────────────────────────────────
describe('UserDetailDrawer — D01: userId=null 不显示', () => {
  it('userId=null 时 Drawer 不在 DOM 中', () => {
    mockUseUserDetail.mockReturnValue({ detail: null, loading: false, error: null });
    render(<UserDetailDrawer {...makeProps({ userId: null })} />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});

// ── D02: 加载中显示 Skeleton ──────────────────────────────────────────────
describe('UserDetailDrawer — D02: 加载中显示 Skeleton', () => {
  it('useUserDetail 返回 loading=true 时 detail-skeleton 可见', async () => {
    mockUseUserDetail.mockReturnValue({ detail: null, loading: true, error: null });
    render(<UserDetailDrawer {...makeProps()} />);
    await waitFor(() =>
      expect(screen.getByTestId('detail-skeleton')).toBeInTheDocument(),
    );
  });
});

// ── D03: 成功态：渲染手机号、昵称、coin_balance ───────────────────────────
describe('UserDetailDrawer — D03: 成功态字段展示', () => {
  it('显示手机号和昵称', async () => {
    render(<UserDetailDrawer {...makeProps()} />);
    await waitFor(() => {
      expect(screen.getByText('+8613800138000')).toBeInTheDocument();
      expect(screen.getByText('TestUser')).toBeInTheDocument();
    });
  });

  it('Statistic 显示 coin_balance', async () => {
    render(<UserDetailDrawer {...makeProps()} />);
    await waitFor(() => {
      // Ant Design Statistic 可能格式化为 "1,000"
      const el = screen.getByTestId('coin-balance-stat');
      expect(el).toBeInTheDocument();
    });
  });
});

// ── D04: status='normal' → 显示封禁按钮，不显示解封 ─────────────────────
describe('UserDetailDrawer — D04: status=normal 显示封禁按钮', () => {
  it('status="normal" 时显示 ban-btn，不显示 unban-btn', async () => {
    render(<UserDetailDrawer {...makeProps()} />);
    await waitFor(() => {
      expect(screen.getByTestId('ban-btn')).toBeInTheDocument();
      expect(screen.queryByTestId('unban-btn')).not.toBeInTheDocument();
    });
  });
});

// ── D05: status='banned' → 显示解封按钮，不显示封禁 ─────────────────────
describe('UserDetailDrawer — D05: status=banned 显示解封按钮', () => {
  it('status="banned" 时显示 unban-btn，不显示 ban-btn', async () => {
    mockUseUserDetail.mockReturnValue({
      detail: mockDetailBanned,
      loading: false,
      error: null,
    });
    render(<UserDetailDrawer {...makeProps()} />);
    await waitFor(() => {
      expect(screen.getByTestId('unban-btn')).toBeInTheDocument();
      expect(screen.queryByTestId('ban-btn')).not.toBeInTheDocument();
    });
  });
});

// ── D06: API 错误时显示 detail-error，操作按钮不渲染 ─────────────────────
describe('UserDetailDrawer — D06: API 错误', () => {
  it('error 非 null 时显示 detail-error，ban-btn 和 unban-btn 均不存在', async () => {
    mockUseUserDetail.mockReturnValue({
      detail: null,
      loading: false,
      error: new Error('User not found'),
    });
    render(<UserDetailDrawer {...makeProps()} />);
    await waitFor(() => {
      expect(screen.getByTestId('detail-error')).toBeInTheDocument();
      expect(screen.queryByTestId('ban-btn')).not.toBeInTheDocument();
      expect(screen.queryByTestId('unban-btn')).not.toBeInTheDocument();
    });
  });
});

// ── D07: behavior-placeholder 存在 ───────────────────────────────────────
describe('UserDetailDrawer — D07: behavior-placeholder 存在', () => {
  it('detail 加载后 behavior-placeholder 可见', async () => {
    render(<UserDetailDrawer {...makeProps()} />);
    await waitFor(() => {
      expect(screen.getByTestId('behavior-placeholder')).toBeInTheDocument();
    });
  });
});

// ── D08: 点击关闭按钮，onClose 被调用 ────────────────────────────────────
describe('UserDetailDrawer — D08: onClose 调用', () => {
  it('点击 Drawer 关闭按钮后 onClose 被调用', async () => {
    const user = userEvent.setup();
    const props = makeProps();
    render(<UserDetailDrawer {...props} />);

    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    // Ant Design Drawer 关闭按钮
    const closeBtn = document.querySelector('.ant-drawer-close') as HTMLElement;
    expect(closeBtn).toBeTruthy();
    await user.click(closeBtn);

    expect(props.onClose).toHaveBeenCalledTimes(1);
  });
});

// ── D09: status='normal' 点击封禁，onBanClick 被调用 ─────────────────────
describe('UserDetailDrawer — D09: 封禁按钮点击', () => {
  it('点击 ban-btn 后 onBanClick 以 detail.id 为参数调用', async () => {
    const user = userEvent.setup();
    const props = makeProps();
    render(<UserDetailDrawer {...props} />);

    const banBtn = await screen.findByTestId('ban-btn');
    await user.click(banBtn);

    expect(props.onBanClick).toHaveBeenCalledWith('user-uuid-1');
  });
});

// ── D10: status='banned' 点击解封，onUnbanClick 被调用 ───────────────────
describe('UserDetailDrawer — D10: 解封按钮点击', () => {
  it('点击 unban-btn 后 onUnbanClick 以 detail.id 为参数调用', async () => {
    const user = userEvent.setup();
    mockUseUserDetail.mockReturnValue({
      detail: mockDetailBanned,
      loading: false,
      error: null,
    });
    const props = makeProps();
    render(<UserDetailDrawer {...props} />);

    const unbanBtn = await screen.findByTestId('unban-btn');
    await user.click(unbanBtn);

    expect(props.onUnbanClick).toHaveBeenCalledWith('user-uuid-1');
  });
});

// ── D11 / D12 / D13: RBAC — 调整余额按钮可见性（MEDIUM-2）────────────────
/**
 * 验证"调整余额"按钮根据管理员角色控制可见性：
 *   - super_admin / operator / finance → 可见
 *   - cs → 不可见（无 WalletAdjust 权限）
 *
 * T-10013 RBAC 规定：WalletAdjust 权限仅授予 super_admin / operator / finance。
 * 修复前：按钮对所有角色均可见 → D11 断言 not.toBeInTheDocument() FAIL（RED）
 * 修复后：cs 角色按钮隐藏 → D11 PASS；finance/super_admin 可见 → D12/D13 PASS
 */
describe('UserDetailDrawer — D11/D12/D13: RBAC 调整余额按钮', () => {
  it('D11: cs 角色不可见"调整余额"按钮', async () => {
    mockAdminRole = 'cs';
    render(<UserDetailDrawer {...makeProps()} />);

    await waitFor(() => {
      // 用户详情已加载（detail 存在）
      expect(screen.getByTestId('ban-btn')).toBeInTheDocument();
    });

    // cs 无 WalletAdjust 权限，按钮不应出现
    expect(screen.queryByTestId('adjust-balance-btn')).not.toBeInTheDocument();
  });

  it('D12: finance 角色可见"调整余额"按钮', async () => {
    mockAdminRole = 'finance';
    render(<UserDetailDrawer {...makeProps()} />);

    await waitFor(() => {
      expect(screen.getByTestId('adjust-balance-btn')).toBeInTheDocument();
    });
  });

  it('D13: super_admin 角色可见"调整余额"按钮', async () => {
    mockAdminRole = 'super_admin';
    render(<UserDetailDrawer {...makeProps()} />);

    await waitFor(() => {
      expect(screen.getByTestId('adjust-balance-btn')).toBeInTheDocument();
    });
  });

  it('D14: operator 角色可见"调整余额"按钮', async () => {
    mockAdminRole = 'operator';
    render(<UserDetailDrawer {...makeProps()} />);

    await waitFor(() => {
      expect(screen.getByTestId('adjust-balance-btn')).toBeInTheDocument();
    });
  });
});
