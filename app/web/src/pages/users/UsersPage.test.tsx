/**
 * T-20006: UsersPage 集成测试
 * T-20007: UsersPage 集成测试（用户详情抽屉）
 * T-20008: UsersPage 集成测试（封禁对话框）
 * T-20010: UsersPage 集成测试（解封对话框）
 *
 * 验收用例（T-20006）：
 *   I01: API 成功 → Table 显示 3 行
 *   I02: API 失败 → 显示 data-testid="users-error"
 *   I03: 手机号搜索 → adminGetUsers 以 phone 参数调用
 *   I04: 状态筛选 → adminGetUsers 以 status=banned 调用
 *   I05: 重置 → 以空参数重新发起请求
 *   I06: URL 含参数时，搜索框显示对应初始值
 *
 * 验收用例（T-20007）：
 *   I07: 点击"查看详情" → Drawer open=true，GET 用户详情请求发出
 *   I08: Drawer 关闭 → Drawer open=false
 *
 * 验收用例（T-20008）：
 *   I09: UserDetailDrawer 封禁按钮点击 → BanModal 打开
 *   I10: BanModal 封禁成功 → Drawer 关闭 + 用户列表重新加载
 *
 * 验收用例（T-20010）：
 *   I11: UserDetailDrawer 解封按钮点击 → UnbanModal 打开
 *   I12: UnbanModal 解封成功 → Drawer 关闭 + 用户列表重新加载 + message.success 出现
 *   I13: UnbanModal 取消 → unbanUserId 重置为 null，Modal 关闭
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import { MemoryRouter } from 'react-router-dom';
import React from 'react';

// ── i18n mock ─────────────────────────────────────────────────────────────
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { changeLanguage: vi.fn(), language: 'zh' },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ── apiClient mock ─────────────────────────────────────────────────────────
vi.mock('../../core/network/apiClient', () => ({
  adminGetUsers: vi.fn(),
  adminGetUserDetail: vi.fn(),
  adminBanUser: vi.fn(),
  adminUnbanUser: vi.fn(),
}));

// ── BanModal mock（用于集成测试 I09/I10，聚焦 UsersPage 回调行为）──────────
vi.mock('./BanModal', () => ({
  BanModal: ({
    userId,
    onClose,
    onSuccess,
  }: {
    userId: string | null;
    onClose: () => void;
    onSuccess: (id: string) => void;
  }) => {
    if (!userId) return null;
    return (
      <div data-testid="ban-modal-mock">
        <button data-testid="mock-ban-close" onClick={onClose}>
          close
        </button>
        <button data-testid="mock-ban-success" onClick={() => onSuccess(userId)}>
          success
        </button>
      </div>
    );
  },
}));

// ── UnbanModal mock（用于集成测试 I11/I12/I13，聚焦 UsersPage 回调行为）────
vi.mock('./UnbanModal', () => ({
  UnbanModal: ({
    userId,
    onClose,
    onSuccess,
  }: {
    userId: string | null;
    onClose: () => void;
    onSuccess: (id: string) => void;
  }) => {
    if (!userId) return null;
    return (
      <div data-testid="unban-modal-mock">
        <button data-testid="mock-unban-close" onClick={onClose}>
          close
        </button>
        <button data-testid="mock-unban-success" onClick={() => onSuccess(userId)}>
          success
        </button>
      </div>
    );
  },
}));

import { adminGetUsers, adminGetUserDetail, adminBanUser } from '../../core/network/apiClient';
import { UsersPage } from './index';
import type { AdminUsersData } from '../../core/network/apiClient';
import * as antd from 'antd';

const mockAdminGetUsers = adminGetUsers as ReturnType<typeof vi.fn>;
const mockAdminGetUserDetail = adminGetUserDetail as ReturnType<typeof vi.fn>;
const mockAdminBanUser = adminBanUser as ReturnType<typeof vi.fn>;

// ── 测试数据工厂 ───────────────────────────────────────────────────────────
function makeUsersData(count: number): AdminUsersData {
  return {
    total: count,
    page: 1,
    size: 20,
    items: Array.from({ length: count }, (_, i) => ({
      id: `user-${i + 1}`,
      phone: `1380013800${i + 1}`,
      nickname: `User${i + 1}`,
      avatar: undefined,
      coin_balance: 100,
      vip_level: 0,
      status: 'normal' as const,
      created_at: '2025-01-01T00:00:00Z',
    })),
  };
}

// ── 带路由的渲染 helper ─────────────────────────────────────────────────────
function renderWithRouter(route = '/') {
  return render(
    <MemoryRouter initialEntries={[route]}>
      <UsersPage />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  mockAdminGetUsers.mockResolvedValue(makeUsersData(3));
  mockAdminBanUser.mockResolvedValue(undefined);
  mockAdminGetUserDetail.mockResolvedValue({
    id: 'user-1',
    phone: '+8613800138001',
    nickname: 'User1',
    avatar_url: null,
    coin_balance: 100,
    vip_level: 0,
    status: 'normal',
    created_at: '2025-01-01T00:00:00Z',
    recharge_records: [],
    consume_records: [],
    devices: [],
  });
});

// ── I01: API 成功 → Table 显示 3 行 ──────────────────────────────────────
describe('UsersPage — I01: API 成功渲染', () => {
  it('API 返回 3 条，Table 显示 3 行', async () => {
    renderWithRouter();

    const table = await screen.findByTestId('users-table');
    await waitFor(() => {
      const rows = within(table).getAllByRole('row');
      expect(rows.length - 1).toBe(3);
    });
  });
});

// ── I02: API 失败 → 显示 users-error ─────────────────────────────────────
describe('UsersPage — I02: API 失败', () => {
  it('API 失败时显示 data-testid="users-error"', async () => {
    mockAdminGetUsers.mockRejectedValue(new Error('Network Error'));

    renderWithRouter();

    await waitFor(() => {
      expect(screen.getByTestId('users-error')).toBeInTheDocument();
    });
  });
});

// ── I03: 手机号搜索 ───────────────────────────────────────────────────────
describe('UsersPage — I03: 手机号搜索', () => {
  it('输入手机号并点击搜索，adminGetUsers 以 phone 参数调用', async () => {
    const user = userEvent.setup();
    renderWithRouter();

    await waitFor(() => expect(screen.getByTestId('users-table')).toBeInTheDocument());

    mockAdminGetUsers.mockClear();

    const phoneInput = screen.getByPlaceholderText('users.phonePlaceholder');
    await user.type(phoneInput, '13800138000');
    await user.click(screen.getByText('users.search'));

    await waitFor(() => {
      expect(mockAdminGetUsers).toHaveBeenCalled();
      const lastCall = mockAdminGetUsers.mock.calls[mockAdminGetUsers.mock.calls.length - 1];
      expect(lastCall[0]).toMatchObject({ phone: '13800138000' });
    });
  });
});

// ── I04: 状态筛选 ─────────────────────────────────────────────────────────
describe('UsersPage — I04: 状态筛选', () => {
  it('选择"封禁"状态并搜索，adminGetUsers 以 status=banned 调用', async () => {
    const user = userEvent.setup();
    renderWithRouter();

    await waitFor(() => expect(screen.getByTestId('users-table')).toBeInTheDocument());

    mockAdminGetUsers.mockClear();

    // 找到 status Select combobox
    const statusSelect = screen.getByTestId('status-select');
    const combobox = within(statusSelect).getByRole('combobox');
    await user.click(combobox);

    // 等待下拉出现
    await waitFor(() => {
      expect(document.querySelector('.ant-select-dropdown')).toBeInTheDocument();
    });
    const dropdown = document.querySelector('.ant-select-dropdown') as HTMLElement;
    const bannedOption = within(dropdown).getByText('users.statusBanned');
    await user.click(bannedOption);

    // 点击搜索
    await user.click(screen.getByText('users.search'));

    await waitFor(() => {
      expect(mockAdminGetUsers).toHaveBeenCalled();
      const lastCall = mockAdminGetUsers.mock.calls[mockAdminGetUsers.mock.calls.length - 1];
      expect(lastCall[0]).toMatchObject({ status: 'banned' });
    });
  });
});

// ── I05: 重置 ────────────────────────────────────────────────────────────
describe('UsersPage — I05: 重置', () => {
  it('输入手机号搜索后点击重置，再次以空参数发起请求', async () => {
    const user = userEvent.setup();
    renderWithRouter();

    await waitFor(() => expect(screen.getByTestId('users-table')).toBeInTheDocument());

    // 先搜索
    const phoneInput = screen.getByPlaceholderText('users.phonePlaceholder');
    await user.type(phoneInput, '138');
    await user.click(screen.getByText('users.search'));
    await waitFor(() => expect(mockAdminGetUsers).toHaveBeenCalled());

    mockAdminGetUsers.mockClear();

    // 重置
    await user.click(screen.getByText('users.reset'));

    await waitFor(() => expect(mockAdminGetUsers).toHaveBeenCalled());

    const lastCall = mockAdminGetUsers.mock.calls[mockAdminGetUsers.mock.calls.length - 1];
    expect(lastCall[0]).not.toHaveProperty('phone');
  });
});

// ── I06: URL 状态恢复 ─────────────────────────────────────────────────────
describe('UsersPage — I06: URL 状态恢复', () => {
  it('URL 含 phone=138 时，搜索框显示对应初始值', async () => {
    renderWithRouter('/?phone=138');

    await waitFor(() => {
      expect(screen.getByTestId('users-table')).toBeInTheDocument();
    });

    await waitFor(() => {
      const phoneInput = screen.getByPlaceholderText('users.phonePlaceholder');
      expect(phoneInput).toHaveValue('138');
    });
  });

  it('URL 含 status=banned 时，adminGetUsers 以 status=banned 调用', async () => {
    renderWithRouter('/?status=banned');

    await waitFor(() => expect(mockAdminGetUsers).toHaveBeenCalled());

    const lastCall = mockAdminGetUsers.mock.calls[mockAdminGetUsers.mock.calls.length - 1];
    expect(lastCall[0]).toMatchObject({ status: 'banned' });
  });
});

// ── I07: 点击"查看详情" → Drawer open，GET 用户详情请求发出 ───────────────
describe('UsersPage — I07: 查看详情打开 Drawer', () => {
  it('点击"查看详情"按钮 → Drawer 打开，adminGetUserDetail 以用户 id 调用', async () => {
    const user = userEvent.setup();
    renderWithRouter();

    // 等待表格渲染
    const table = await screen.findByTestId('users-table');
    await waitFor(() => {
      const rows = table.querySelectorAll('tbody tr');
      expect(rows.length).toBeGreaterThan(0);
    });

    // 点击第一行"查看详情"
    const detailBtns = screen.getAllByTestId('view-detail-btn');
    await user.click(detailBtns[0]);

    // Drawer 应打开（role="dialog"）
    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });

    // adminGetUserDetail 应以对应 userId 调用
    expect(mockAdminGetUserDetail).toHaveBeenCalledWith(
      'user-1',
      expect.any(AbortSignal),
    );
  });
});

// ── I08: Drawer 关闭 → Drawer open=false ─────────────────────────────────
describe('UsersPage — I08: 关闭 Drawer', () => {
  it('点击 Drawer 关闭按钮后 Drawer 消失', async () => {
    const user = userEvent.setup();
    renderWithRouter();

    const table = await screen.findByTestId('users-table');
    await waitFor(() => {
      const rows = table.querySelectorAll('tbody tr');
      expect(rows.length).toBeGreaterThan(0);
    });

    // 打开 Drawer
    const detailBtns = screen.getAllByTestId('view-detail-btn');
    await user.click(detailBtns[0]);

    await waitFor(() => {
      expect(screen.getByRole('dialog')).toBeInTheDocument();
    });

    // 点击 Drawer 关闭按钮
    const closeBtn = document.querySelector('.ant-drawer-close') as HTMLElement;
    expect(closeBtn).toBeTruthy();
    await user.click(closeBtn);

    // Drawer 应消失
    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });
  });
});

// ── I09: 封禁按钮点击 → BanModal 打开 ────────────────────────────────────
describe('UsersPage — I09 (T-20008): 封禁按钮打开 BanModal', () => {
  it('点击 Drawer 中的封禁按钮，BanModal 出现（banUserId 被设置）', async () => {
    const user = userEvent.setup();
    renderWithRouter();

    const table = await screen.findByTestId('users-table');
    await waitFor(() => {
      const rows = table.querySelectorAll('tbody tr');
      expect(rows.length).toBeGreaterThan(0);
    });

    // 打开 Drawer
    const detailBtns = screen.getAllByTestId('view-detail-btn');
    await user.click(detailBtns[0]);

    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    // 等待用户详情加载（normal 状态，有 ban-btn）
    const banBtn = await screen.findByTestId('ban-btn');
    await user.click(banBtn);

    // BanModal mock 应出现
    await waitFor(() => {
      expect(screen.getByTestId('ban-modal-mock')).toBeInTheDocument();
    });
  });
});

// ── I10: BanModal 成功 → Drawer 关闭 + 列表刷新 ──────────────────────────
describe('UsersPage — I10 (T-20008): BanModal 成功后关闭 Drawer 并刷新列表', () => {
  it('BanModal onSuccess 被调用后 Drawer 关闭且 adminGetUsers 重新调用', async () => {
    const user = userEvent.setup();
    renderWithRouter();

    const table = await screen.findByTestId('users-table');
    await waitFor(() => {
      const rows = table.querySelectorAll('tbody tr');
      expect(rows.length).toBeGreaterThan(0);
    });

    // 打开 Drawer → 点击封禁 → BanModal 出现
    await user.click(screen.getAllByTestId('view-detail-btn')[0]);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
    const banBtn = await screen.findByTestId('ban-btn');
    await user.click(banBtn);
    await waitFor(() => expect(screen.getByTestId('ban-modal-mock')).toBeInTheDocument());

    const callCountBefore = mockAdminGetUsers.mock.calls.length;

    // 点击 mock 成功按钮 → onSuccess 触发
    await user.click(screen.getByTestId('mock-ban-success'));

    // Drawer 应关闭（dialog 消失）
    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });

    // 列表应刷新（adminGetUsers 再次被调用）
    await waitFor(() => {
      expect(mockAdminGetUsers.mock.calls.length).toBeGreaterThan(callCountBefore);
    });
  });
});

// ── I11: 解封按钮 → UnbanModal 打开（T-20010）────────────────────────────
describe('UsersPage — I11 (T-20010): 解封按钮点击 → UnbanModal 打开', () => {
  it('点击解封按钮，UnbanModal 打开（unban-modal-mock 出现）', async () => {
    mockAdminGetUserDetail.mockResolvedValue({
      id: 'user-1',
      phone: '+8613800138001',
      nickname: 'User1',
      avatar_url: null,
      coin_balance: 100,
      vip_level: 0,
      status: 'banned',
      created_at: '2025-01-01T00:00:00Z',
      recharge_records: [],
      consume_records: [],
      devices: [],
    });

    const user = userEvent.setup();
    renderWithRouter();

    const table = await screen.findByTestId('users-table');
    await waitFor(() => {
      const rows = table.querySelectorAll('tbody tr');
      expect(rows.length).toBeGreaterThan(0);
    });

    await user.click(screen.getAllByTestId('view-detail-btn')[0]);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    const unbanBtn = await screen.findByTestId('unban-btn');
    await user.click(unbanBtn);

    await waitFor(() => {
      expect(screen.getByTestId('unban-modal-mock')).toBeInTheDocument();
    });
  });
});

// ── I12: UnbanModal 成功 → Drawer 关闭 + 用户列表刷新 + message.success ───
describe('UsersPage — I12 (T-20010): UnbanModal 解封成功后关闭 Drawer 并刷新列表', () => {
  it('UnbanModal onSuccess 被调用后 Drawer 关闭且 adminGetUsers 重新调用', async () => {
    mockAdminGetUserDetail.mockResolvedValue({
      id: 'user-1',
      phone: '+8613800138001',
      nickname: 'User1',
      avatar_url: null,
      coin_balance: 100,
      vip_level: 0,
      status: 'banned',
      created_at: '2025-01-01T00:00:00Z',
      recharge_records: [],
      consume_records: [],
      devices: [],
    });

    const messageSuccessSpy = vi
      .spyOn(antd.message, 'success')
      .mockImplementation(() => ({ then: vi.fn() } as unknown as ReturnType<typeof antd.message.success>));

    const user = userEvent.setup();
    renderWithRouter();

    const table = await screen.findByTestId('users-table');
    await waitFor(() => {
      const rows = table.querySelectorAll('tbody tr');
      expect(rows.length).toBeGreaterThan(0);
    });

    await user.click(screen.getAllByTestId('view-detail-btn')[0]);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
    const unbanBtn = await screen.findByTestId('unban-btn');
    await user.click(unbanBtn);
    await waitFor(() => expect(screen.getByTestId('unban-modal-mock')).toBeInTheDocument());

    const callCountBefore = mockAdminGetUsers.mock.calls.length;

    await user.click(screen.getByTestId('mock-unban-success'));

    await waitFor(() => {
      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });

    await waitFor(() => {
      expect(mockAdminGetUsers.mock.calls.length).toBeGreaterThan(callCountBefore);
    });

    expect(messageSuccessSpy).toHaveBeenCalled();
    messageSuccessSpy.mockRestore();
  });
});

// ── I13: UnbanModal 取消 → unbanUserId 重置为 null，Modal 关闭 ────────────
describe('UsersPage — I13 (T-20010): UnbanModal 取消后关闭', () => {
  it('UnbanModal onClose 被调用后 unban-modal-mock 消失', async () => {
    mockAdminGetUserDetail.mockResolvedValue({
      id: 'user-1',
      phone: '+8613800138001',
      nickname: 'User1',
      avatar_url: null,
      coin_balance: 100,
      vip_level: 0,
      status: 'banned',
      created_at: '2025-01-01T00:00:00Z',
      recharge_records: [],
      consume_records: [],
      devices: [],
    });

    const user = userEvent.setup();
    renderWithRouter();

    const table = await screen.findByTestId('users-table');
    await waitFor(() => {
      const rows = table.querySelectorAll('tbody tr');
      expect(rows.length).toBeGreaterThan(0);
    });

    await user.click(screen.getAllByTestId('view-detail-btn')[0]);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
    const unbanBtn = await screen.findByTestId('unban-btn');
    await user.click(unbanBtn);
    await waitFor(() => expect(screen.getByTestId('unban-modal-mock')).toBeInTheDocument());

    await user.click(screen.getByTestId('mock-unban-close'));

    await waitFor(() => {
      expect(screen.queryByTestId('unban-modal-mock')).not.toBeInTheDocument();
    });
  });
});
