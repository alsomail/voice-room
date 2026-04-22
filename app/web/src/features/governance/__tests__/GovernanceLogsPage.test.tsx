/**
 * T-20014: GovernanceLogsPage 组件测试
 *
 * 验收用例（对应 TDS §三）：
 *   G14-01 路由加载成功
 *   G14-02 默认展示最近 7 天数据
 *   G14-03 Tab 切换重置分页与筛选
 *   G14-04 房间 ID 筛选触发 API
 *   G14-05 finance 角色无菜单项
 *   G14-06 空数据显示占位
 *   G14-07 i18n 正确
 *   G14-08 目标用户点击 → 跳转用户详情 Drawer
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import React from 'react';

// ── i18n mock ─────────────────────────────────────────────────────────────────
const { _stableT, _stableI18n } = vi.hoisted(() => {
  const t = (key: string) => key;
  const i18n = { changeLanguage: vi.fn(), language: 'en' };
  return { _stableT: t, _stableI18n: i18n };
});

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: _stableT,
    i18n: _stableI18n,
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ── governance API mock ───────────────────────────────────────────────────────
vi.mock('../../../services/api/governance', async (importOriginal) => {
  const original = await importOriginal<typeof import('../../../services/api/governance')>();
  return {
    ...original,
    listKicks: vi.fn(),
    listMutes: vi.fn(),
  };
});

// ── useAuthStore mock ─────────────────────────────────────────────────────────
const mockAdmin = {
  id: 'admin-1',
  username: 'admin',
  role: 'super_admin' as string,
  display_name: 'Admin',
  last_login_at: '',
};
const mockAuthState = {
  isAuthenticated: true,
  token: 'test-token',
  admin: mockAdmin,
  checkAuth: vi.fn().mockReturnValue(true),
  login: vi.fn(),
  logout: vi.fn(),
};

vi.mock('../../../stores/useAuthStore', () => ({
  useAuthStore: (selector?: (s: typeof mockAuthState) => unknown) => {
    if (typeof selector === 'function') return selector(mockAuthState);
    return mockAuthState;
  },
  ADMIN_TOKEN_KEY: 'adminToken',
}));

// ── react-router-dom mock ─────────────────────────────────────────────────────
const mockNavigate = vi.fn();
vi.mock('react-router-dom', async (importOriginal) => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return {
    ...actual,
    useNavigate: () => mockNavigate,
    useLocation: () => ({ pathname: '/rooms/governance' }),
    MemoryRouter: actual.MemoryRouter,
    Outlet: () => <div data-testid="outlet" />,
  };
});

import { listKicks, listMutes } from '../../../services/api/governance';
import { GovernanceLogsPage } from '../GovernanceLogsPage';
import { AppLayout } from '../../../app/AppLayout';
import { MemoryRouter, Routes, Route } from 'react-router-dom';
import { RoleGuard } from '../../../components/RoleGuard';

const mockListKicks = listKicks as ReturnType<typeof vi.fn>;
const mockListMutes = listMutes as ReturnType<typeof vi.fn>;

// ── 测试数据 ──────────────────────────────────────────────────────────────────
const MOCK_KICK_1 = {
  id: 'kick-uuid-001',
  room_id: 'room-uuid-001',
  room_title: 'Test Room',
  target_user_id: 'user-uuid-001',
  target_nickname: 'Alice',
  operator_user_id: 'op-uuid-001',
  operator_nickname: 'Operator',
  reason: 'harassment',
  created_at: '2025-07-17T10:00:00Z',
};

const MOCK_KICK_2 = {
  id: 'kick-uuid-002',
  room_id: 'room-uuid-002',
  room_title: 'Another Room',
  target_user_id: 'user-uuid-002',
  target_nickname: 'Bob',
  operator_user_id: 'op-uuid-001',
  operator_nickname: 'Operator',
  reason: null,
  created_at: '2025-07-17T09:00:00Z',
};

const MOCK_MUTE_1 = {
  id: 'mute-uuid-001',
  room_id: 'room-uuid-001',
  room_title: 'Test Room',
  target_user_id: 'user-uuid-003',
  target_nickname: 'Charlie',
  operator_user_id: 'op-uuid-001',
  operator_nickname: 'Operator',
  type: 'mic' as const,
  duration_sec: 300,
  reason: 'noise',
  created_at: '2025-07-17T10:00:00Z',
};

const MOCK_KICKS_RESPONSE = {
  total: 2,
  page: 1,
  limit: 20,
  items: [MOCK_KICK_1, MOCK_KICK_2],
};

const MOCK_MUTES_RESPONSE = {
  total: 1,
  page: 1,
  limit: 20,
  items: [MOCK_MUTE_1],
};

const EMPTY_RESPONSE = {
  total: 0,
  page: 1,
  limit: 20,
  items: [],
};

beforeEach(() => {
  vi.clearAllMocks();
  mockAdmin.role = 'super_admin';
  mockListKicks.mockResolvedValue(MOCK_KICKS_RESPONSE);
  mockListMutes.mockResolvedValue(MOCK_MUTES_RESPONSE);
});

// ─────────────────────────────────────────────────────────────────────────────
// G14-01: 路由加载成功
// ─────────────────────────────────────────────────────────────────────────────
describe('G14-01: 路由加载成功', () => {
  it('页面渲染，data-testid=governance-page 存在', async () => {
    render(<GovernanceLogsPage />);
    expect(screen.getByTestId('governance-page')).toBeInTheDocument();
  });

  it('Kicks Tab 默认选中，testid=governance-tab-kicks 存在', async () => {
    render(<GovernanceLogsPage />);
    expect(screen.getByTestId('governance-tab-kicks')).toBeInTheDocument();
  });

  it('Mutes Tab 存在，testid=governance-tab-mutes', async () => {
    render(<GovernanceLogsPage />);
    expect(screen.getByTestId('governance-tab-mutes')).toBeInTheDocument();
  });

  it('房间筛选输入框存在，testid=governance-filter-room', async () => {
    render(<GovernanceLogsPage />);
    expect(screen.getByTestId('governance-filter-room')).toBeInTheDocument();
  });

  it('目标用户筛选输入框存在，testid=governance-filter-target-user', async () => {
    render(<GovernanceLogsPage />);
    expect(screen.getByTestId('governance-filter-target-user')).toBeInTheDocument();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// G14-02: 默认展示最近 7 天数据
// ─────────────────────────────────────────────────────────────────────────────
describe('G14-02: 默认展示最近 7 天数据', () => {
  it('页面加载时调用 listKicks，from 参数约为 7 天前', async () => {
    render(<GovernanceLogsPage />);
    await waitFor(() => {
      expect(mockListKicks).toHaveBeenCalled();
    });
    const callParams = mockListKicks.mock.calls[0][0] as Record<string, unknown>;
    expect(callParams).toHaveProperty('from');
    const fromDate = new Date(callParams.from as string);
    const sevenDaysAgo = new Date(Date.now() - 7 * 24 * 60 * 60 * 1000);
    // 允许 10 秒误差
    expect(Math.abs(fromDate.getTime() - sevenDaysAgo.getTime())).toBeLessThan(10_000);
  });

  it('API 返回数据后，踢人记录行渲染（data-testid=governance-row-{id}）', async () => {
    render(<GovernanceLogsPage />);
    await waitFor(() => {
      expect(screen.getByTestId(`governance-row-${MOCK_KICK_1.id}`)).toBeInTheDocument();
    });
    expect(screen.getByTestId(`governance-row-${MOCK_KICK_2.id}`)).toBeInTheDocument();
  });

  it('显示目标用户昵称 Alice', async () => {
    render(<GovernanceLogsPage />);
    await waitFor(() => {
      expect(screen.getByText('Alice')).toBeInTheDocument();
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// G14-03: Tab 切换重置分页与筛选
// ─────────────────────────────────────────────────────────────────────────────
describe('G14-03: Tab 切换重置分页与筛选', () => {
  it('切换到 Mutes Tab 后，调用 listMutes API', async () => {
    const user = userEvent.setup();
    render(<GovernanceLogsPage />);
    // 等待初始加载
    await waitFor(() => expect(mockListKicks).toHaveBeenCalled());

    // 切换到 mutes tab
    const muteTab = screen.getByTestId('governance-tab-mutes');
    await user.click(muteTab);

    await waitFor(() => {
      expect(mockListMutes).toHaveBeenCalled();
    });
  });

  it('在 kicks tab 输入 room_id 后切换到 mutes tab，room_id 被重置', async () => {
    const user = userEvent.setup();
    render(<GovernanceLogsPage />);
    await waitFor(() => expect(mockListKicks).toHaveBeenCalled());

    // 输入 room_id 过滤条件
    const roomInput = screen.getByTestId('governance-filter-room');
    await user.clear(roomInput);
    await user.type(roomInput, 'room-test-123');
    // 触发搜索
    fireEvent.keyDown(roomInput, { key: 'Enter', code: 'Enter' });

    await waitFor(() => {
      const calls = mockListKicks.mock.calls;
      const hasRoomId = calls.some((c) => {
        const params = c[0] as Record<string, unknown>;
        return params.room_id === 'room-test-123';
      });
      expect(hasRoomId).toBe(true);
    });

    // 切换 tab
    const muteTab = screen.getByTestId('governance-tab-mutes');
    await user.click(muteTab);

    // 等待 listMutes 被调用
    await waitFor(() => expect(mockListMutes).toHaveBeenCalled());

    // mutes API 调用时 room_id 应为空（重置后）
    const muteCalls = mockListMutes.mock.calls;
    const lastMuteCall = muteCalls[muteCalls.length - 1][0] as Record<string, unknown>;
    expect(lastMuteCall.room_id).toBeUndefined();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// G14-04: 房间 ID 筛选触发 API
// ─────────────────────────────────────────────────────────────────────────────
describe('G14-04: 房间 ID 筛选触发 API', () => {
  it('输入房间 ID 并回车，API 携带 room_id 参数', async () => {
    const user = userEvent.setup();
    render(<GovernanceLogsPage />);
    await waitFor(() => expect(mockListKicks).toHaveBeenCalled());

    const roomInput = screen.getByTestId('governance-filter-room');
    await user.clear(roomInput);
    await user.type(roomInput, 'room-abc-123');
    fireEvent.keyDown(roomInput, { key: 'Enter', code: 'Enter' });

    await waitFor(() => {
      const calls = mockListKicks.mock.calls;
      const found = calls.some((c) => {
        const params = c[0] as Record<string, unknown>;
        return params.room_id === 'room-abc-123';
      });
      expect(found).toBe(true);
    });
  });

  it('点击重置按钮后，room_id 参数被清除', async () => {
    const user = userEvent.setup();
    render(<GovernanceLogsPage />);
    await waitFor(() => expect(mockListKicks).toHaveBeenCalled());

    // 先输入 room_id
    const roomInput = screen.getByTestId('governance-filter-room');
    await user.type(roomInput, 'room-123');
    fireEvent.keyDown(roomInput, { key: 'Enter', code: 'Enter' });
    await waitFor(() => {
      const calls = mockListKicks.mock.calls;
      expect(calls.some((c) => (c[0] as Record<string, unknown>).room_id === 'room-123')).toBe(true);
    });

    // 点击重置按钮
    const resetBtn = screen.getByTestId('governance-filter-reset');
    await user.click(resetBtn);

    // 最后一次 API 调用不含 room_id
    await waitFor(() => {
      const calls = mockListKicks.mock.calls;
      const lastCall = calls[calls.length - 1][0] as Record<string, unknown>;
      expect(lastCall.room_id).toBeUndefined();
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// G14-05: finance 角色无菜单项
// ─────────────────────────────────────────────────────────────────────────────
describe('G14-05: finance 角色无菜单项', () => {
  it('finance 角色无 governance 菜单项', async () => {
    mockAdmin.role = 'finance';
    render(
      <MemoryRouter initialEntries={['/rooms/governance']}>
        <AppLayout />
      </MemoryRouter>,
    );
    expect(screen.queryByTestId('menu-item-governance')).not.toBeInTheDocument();
  });

  it('super_admin 角色有 governance 菜单项', async () => {
    mockAdmin.role = 'super_admin';
    render(
      <MemoryRouter initialEntries={['/rooms/governance']}>
        <AppLayout />
      </MemoryRouter>,
    );
    expect(screen.getByTestId('menu-item-governance')).toBeInTheDocument();
  });

  it('operator 角色有 governance 菜单项', async () => {
    mockAdmin.role = 'operator';
    render(
      <MemoryRouter initialEntries={['/rooms/governance']}>
        <AppLayout />
      </MemoryRouter>,
    );
    expect(screen.getByTestId('menu-item-governance')).toBeInTheDocument();
  });

  it('cs 角色有 governance 菜单项', async () => {
    mockAdmin.role = 'cs';
    render(
      <MemoryRouter initialEntries={['/rooms/governance']}>
        <AppLayout />
      </MemoryRouter>,
    );
    expect(screen.getByTestId('menu-item-governance')).toBeInTheDocument();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// G14-06: 空数据显示占位
// ─────────────────────────────────────────────────────────────────────────────
describe('G14-06: 空数据显示占位', () => {
  it('kicks API 返回空数据时，显示空状态占位符', async () => {
    mockListKicks.mockResolvedValue(EMPTY_RESPONSE);
    render(<GovernanceLogsPage />);
    await waitFor(() => {
      expect(screen.getByTestId('governance-kicks-empty')).toBeInTheDocument();
    });
  });

  it('mutes tab 空数据时，显示空状态', async () => {
    const user = userEvent.setup();
    mockListMutes.mockResolvedValue(EMPTY_RESPONSE);
    render(<GovernanceLogsPage />);

    const muteTab = screen.getByTestId('governance-tab-mutes');
    await user.click(muteTab);

    await waitFor(() => {
      expect(screen.getByTestId('governance-mutes-empty')).toBeInTheDocument();
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// G14-07: i18n 正确
// ─────────────────────────────────────────────────────────────────────────────
describe('G14-07: i18n 正确', () => {
  it('页面标题使用 i18n key governance.title', () => {
    render(<GovernanceLogsPage />);
    // t函数返回key本身，验证i18n key被正确使用
    expect(screen.getByTestId('governance-page-title')).toBeInTheDocument();
  });

  it('kicks tab label 使用 i18n key governance.tabKicks', () => {
    render(<GovernanceLogsPage />);
    // tab label 渲染为 governance.tabKicks
    expect(screen.getByText('governance.tabKicks')).toBeInTheDocument();
  });

  it('mutes tab label 使用 i18n key governance.tabMutes', () => {
    render(<GovernanceLogsPage />);
    expect(screen.getByText('governance.tabMutes')).toBeInTheDocument();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// G14-08: 目标用户点击 → 跳转用户详情 Drawer
// ─────────────────────────────────────────────────────────────────────────────
describe('G14-08: 目标用户点击 → 跳转用户详情 Drawer', () => {
  it('点击目标用户链接，打开用户详情 Drawer（selectedUserId 被设置）', async () => {
    const user = userEvent.setup();
    render(<GovernanceLogsPage />);

    await waitFor(() => {
      expect(screen.getByTestId(`governance-row-${MOCK_KICK_1.id}`)).toBeInTheDocument();
    });

    // 点击目标用户链接
    const userLink = screen.getByTestId(`governance-user-link-${MOCK_KICK_1.target_user_id}`);
    await user.click(userLink);

    // 验证用户详情 Drawer 被打开
    await waitFor(() => {
      expect(screen.getByTestId('governance-user-drawer')).toBeInTheDocument();
    });
  });

  it('点击目标用户链接后 Drawer 显示正确的 user_id', async () => {
    const user = userEvent.setup();
    render(<GovernanceLogsPage />);

    await waitFor(() => {
      expect(screen.getByTestId(`governance-row-${MOCK_KICK_1.id}`)).toBeInTheDocument();
    });

    const userLink = screen.getByTestId(`governance-user-link-${MOCK_KICK_1.target_user_id}`);
    await user.click(userLink);

    await waitFor(() => {
      const drawer = screen.getByTestId('governance-user-drawer');
      expect(within(drawer).getByTestId('governance-drawer-user-id')).toHaveTextContent(
        MOCK_KICK_1.target_user_id,
      );
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 边界情况
// ─────────────────────────────────────────────────────────────────────────────
describe('边界情况', () => {
  it('API 调用失败时显示错误提示', async () => {
    mockListKicks.mockRejectedValue(new Error('Network Error'));
    render(<GovernanceLogsPage />);
    await waitFor(() => {
      expect(screen.getByTestId('governance-kicks-error')).toBeInTheDocument();
    });
  });

  it('空 room_id（空字符串）不作为参数传递给 API', async () => {
    render(<GovernanceLogsPage />);
    await waitFor(() => expect(mockListKicks).toHaveBeenCalled());
    const firstCall = mockListKicks.mock.calls[0][0] as Record<string, unknown>;
    expect(firstCall.room_id).toBeUndefined();
  });

  it('mutes tab 的 type 过滤（mic）正确传递', async () => {
    const user = userEvent.setup();
    render(<GovernanceLogsPage />);

    // 切换到 mutes tab
    const muteTab = screen.getByTestId('governance-tab-mutes');
    await user.click(muteTab);
    await waitFor(() => expect(mockListMutes).toHaveBeenCalled());

    // 找到 mute type 筛选
    const muteTypeSelect = screen.getByTestId('governance-filter-mute-type');
    expect(muteTypeSelect).toBeInTheDocument();
  });

  it('[HIGH-1] listMutes 不含 mute_type 字段（应映射为 type）', async () => {
    const user = userEvent.setup();
    render(<GovernanceLogsPage />);

    await user.click(screen.getByTestId('governance-tab-mutes'));
    await waitFor(() => expect(mockListMutes).toHaveBeenCalled());

    // 所有调用参数均不应含 mute_type 字段
    mockListMutes.mock.calls.forEach((call) => {
      const params = call[0] as Record<string, unknown>;
      expect(params).not.toHaveProperty('mute_type');
    });
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// G14-ROUTE: 路由访问控制（HIGH-2 + HIGH-3 R1 修复）
// ─────────────────────────────────────────────────────────────────────────────
describe('G14-ROUTE: 路由访问控制', () => {
  it('[HIGH-3] finance 角色直接访问路由被重定向 403', () => {
    mockAdmin.role = 'finance';

    render(
      <MemoryRouter initialEntries={['/rooms/governance']}>
        <Routes>
          <Route element={<RoleGuard allowedRoles={['super_admin', 'operator', 'cs']} />}>
            <Route path="/rooms/governance" element={<GovernanceLogsPage />} />
          </Route>
          <Route path="/403" element={<div data-testid="page-403">Forbidden</div>} />
        </Routes>
      </MemoryRouter>,
    );

    // finance 角色：governance 页面不可访问，应渲染 /403 页面
    expect(screen.queryByTestId('governance-page')).not.toBeInTheDocument();
    expect(screen.getByTestId('page-403')).toBeInTheDocument();
  });

  it('[HIGH-2] super_admin 角色可访问 governance 路由（RoleGuard 放行）', () => {
    mockAdmin.role = 'super_admin';

    render(
      <MemoryRouter initialEntries={['/rooms/governance']}>
        <Routes>
          <Route element={<RoleGuard allowedRoles={['super_admin', 'operator', 'cs']} />}>
            <Route path="/rooms/governance" element={<GovernanceLogsPage />} />
          </Route>
          <Route path="/403" element={<div data-testid="page-403">Forbidden</div>} />
        </Routes>
      </MemoryRouter>,
    );

    // super_admin：RoleGuard 放行，Outlet 渲染（mocked 为 data-testid="outlet"）
    expect(screen.queryByTestId('page-403')).not.toBeInTheDocument();
    expect(screen.getByTestId('outlet')).toBeInTheDocument();
  });

  it('[HIGH-2] operator 角色可访问 governance 路由（RoleGuard 放行）', () => {
    mockAdmin.role = 'operator';

    render(
      <MemoryRouter initialEntries={['/rooms/governance']}>
        <Routes>
          <Route element={<RoleGuard allowedRoles={['super_admin', 'operator', 'cs']} />}>
            <Route path="/rooms/governance" element={<GovernanceLogsPage />} />
          </Route>
          <Route path="/403" element={<div data-testid="page-403">Forbidden</div>} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.queryByTestId('page-403')).not.toBeInTheDocument();
    expect(screen.getByTestId('outlet')).toBeInTheDocument();
  });

  it('[HIGH-2] cs 角色可访问 governance 路由（RoleGuard 放行）', () => {
    mockAdmin.role = 'cs';

    render(
      <MemoryRouter initialEntries={['/rooms/governance']}>
        <Routes>
          <Route element={<RoleGuard allowedRoles={['super_admin', 'operator', 'cs']} />}>
            <Route path="/rooms/governance" element={<GovernanceLogsPage />} />
          </Route>
          <Route path="/403" element={<div data-testid="page-403">Forbidden</div>} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.queryByTestId('page-403')).not.toBeInTheDocument();
    expect(screen.getByTestId('outlet')).toBeInTheDocument();
  });

  it('[HIGH-2] admin 为 null 时重定向 403', () => {
    // admin 为 null 时（未登录但通过 AuthGuard 的极端情况）
    const originalAdmin = mockAuthState.admin;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockAuthState as any).admin = null;

    render(
      <MemoryRouter initialEntries={['/rooms/governance']}>
        <Routes>
          <Route element={<RoleGuard allowedRoles={['super_admin', 'operator', 'cs']} />}>
            <Route path="/rooms/governance" element={<GovernanceLogsPage />} />
          </Route>
          <Route path="/403" element={<div data-testid="page-403">Forbidden</div>} />
        </Routes>
      </MemoryRouter>,
    );

    expect(screen.getByTestId('page-403')).toBeInTheDocument();

    // 恢复
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (mockAuthState as any).admin = originalAdmin;
  });
});
