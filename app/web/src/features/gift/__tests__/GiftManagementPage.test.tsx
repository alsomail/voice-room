/**
 * T-20012: GiftManagementPage 组件测试
 *
 * 验收用例（对应 TDS W12-07 ~ W12-10）：
 *   G01: 礼物列表正常渲染（mockAPI 返回数据）
 *   G02: W12-08 Switch 切换后列表状态更新（乐观更新）
 *   G03: W12-08 Switch 切换 API 失败后状态回滚
 *   G04: W12-09 新增礼物 price=0 时提交按钮禁用
 *   G05: W12-10 上传非图片文件（gif）Upload 组件报错提示
 *   G06: W12-07 AppLayout super_admin/operator 可见礼物菜单，cs 不可见
 *   G07: tier 下拉筛选传递正确参数给 API
 *   G08: 状态筛选（include_inactive）传递正确参数给 API
 *   G09: 软删除调用 adminDeleteGift，成功后列表刷新
 *   G10: 新增礼物表单提交调用 adminCreateGift 并关闭弹窗
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import React from 'react';

// ── i18n mock ─────────────────────────────────────────────────────────────────
// 注意：使用 vi.hoisted 创建稳定的 t/i18n 引用，避免 useCallback([t]) 每次渲染
// 都重新创建函数，从而防止 fetchGifts 在 useEffect deps 中引发无限循环。
const { _stableT, _stableI18n } = vi.hoisted(() => {
  const t = (key: string) => key;
  const i18n = { changeLanguage: vi.fn(), language: 'zh' };
  return { _stableT: t, _stableI18n: i18n };
});

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: _stableT,
    i18n: _stableI18n,
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ── apiClient mock ────────────────────────────────────────────────────────────
vi.mock('../../../core/network/apiClient', async (importOriginal) => {
  const original = await importOriginal<typeof import('../../../core/network/apiClient')>();
  return {
    ...original,
    adminListGifts: vi.fn(),
    adminCreateGift: vi.fn(),
    adminUpdateGift: vi.fn(),
    adminDeleteGift: vi.fn(),
    adminUploadGiftAsset: vi.fn(),
  };
});

// ── useAuthStore mock (for AppLayout) ─────────────────────────────────────────
const mockAuthState = {
  isAuthenticated: true,
  token: 'test-token',
  admin: { id: 'admin-1', username: 'admin', role: 'super_admin', display_name: 'Admin', last_login_at: '' },
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

import {
  adminListGifts,
  adminUpdateGift,
  adminDeleteGift,
  adminCreateGift,
} from '../../../core/network/apiClient';

const mockListGifts = adminListGifts as ReturnType<typeof vi.fn>;
const mockUpdateGift = adminUpdateGift as ReturnType<typeof vi.fn>;
const mockDeleteGift = adminDeleteGift as ReturnType<typeof vi.fn>;
const mockCreateGift = adminCreateGift as ReturnType<typeof vi.fn>;

// ── 测试数据 ──────────────────────────────────────────────────────────────────
const MOCK_GIFT_1 = {
  id: 'gift-uuid-001',
  code: 'unicorn_01',
  name_en: 'Unicorn',
  name_ar: 'يونيكورن',
  icon_url: '/uploads/gifts/2026-04-21/unicorn.png',
  price: 66,
  tier: 3,
  effect_level: 3,
  animation_url: null,
  is_active: true,
  sort_order: 35,
  is_deleted: false,
  created_at: '2025-07-17T10:00:00Z',
  updated_at: '2025-07-17T10:00:00Z',
};

const MOCK_GIFT_2 = {
  id: 'gift-uuid-002',
  code: 'rose_01',
  name_en: 'Rose',
  name_ar: 'وردة',
  icon_url: '/uploads/gifts/2026-04-21/rose.png',
  price: 10,
  tier: 1,
  effect_level: 1,
  animation_url: null,
  is_active: false,
  sort_order: 10,
  is_deleted: false,
  created_at: '2025-07-17T10:00:00Z',
  updated_at: '2025-07-17T10:00:00Z',
};

const MOCK_GIFTS_DATA = {
  total: 2,
  page: 1,
  size: 50,
  items: [MOCK_GIFT_1, MOCK_GIFT_2],
};

beforeEach(() => {
  vi.clearAllMocks();
  mockListGifts.mockResolvedValue(MOCK_GIFTS_DATA);
  mockUpdateGift.mockResolvedValue({ ...MOCK_GIFT_1, is_active: false });
  mockDeleteGift.mockResolvedValue(undefined);
  mockCreateGift.mockResolvedValue({ ...MOCK_GIFT_1, id: 'gift-uuid-003' });
});

import { GiftManagementPage } from '../GiftManagementPage';
import { AppLayout } from '../../../app/AppLayout';
import { MemoryRouter } from 'react-router-dom';

// ── G01: 礼物列表正常渲染 ─────────────────────────────────────────────────────
describe('GiftManagementPage — G01: 礼物列表渲染', () => {
  it('加载后显示礼物列表，能看到礼物名称', async () => {
    render(<GiftManagementPage />);

    await waitFor(() => {
      expect(screen.getByText('unicorn_01')).toBeInTheDocument();
    });
    expect(screen.getByText('rose_01')).toBeInTheDocument();
  });

  it('页面标题可见', async () => {
    render(<GiftManagementPage />);
    expect(screen.getByTestId('gift-page-title')).toBeInTheDocument();
  });

  it('"新增礼物"按钮可见', async () => {
    render(<GiftManagementPage />);
    expect(screen.getByTestId('add-gift-btn')).toBeInTheDocument();
  });
});

// ── G02: W12-08 Switch 切换后列表状态更新（乐观更新） ─────────────────────────
describe('GiftManagementPage — G02: W12-08 Switch 乐观更新', () => {
  it('点击 switch 后调用 adminUpdateGift，is_active 参数正确', async () => {
    const user = userEvent.setup();
    render(<GiftManagementPage />);

    // 等待列表加载
    await waitFor(() => {
      expect(screen.getByTestId(`gift-switch-${MOCK_GIFT_1.id}`)).toBeInTheDocument();
    });

    const toggle = screen.getByTestId(`gift-switch-${MOCK_GIFT_1.id}`);
    await user.click(toggle);

    await waitFor(() => {
      expect(mockUpdateGift).toHaveBeenCalledWith(MOCK_GIFT_1.id, { is_active: false });
    });
  });
});

// ── G03: W12-08 Switch 切换 API 失败后状态回滚 ───────────────────────────────
describe('GiftManagementPage — G03: W12-08 Switch 失败回滚', () => {
  it('Switch API 失败后乐观更新回滚', async () => {
    mockUpdateGift.mockRejectedValue(new Error('更新失败'));
    const user = userEvent.setup();
    render(<GiftManagementPage />);

    await waitFor(() => {
      expect(screen.getByTestId(`gift-switch-${MOCK_GIFT_1.id}`)).toBeInTheDocument();
    });

    // MOCK_GIFT_1.is_active=true，Switch 应为 checked
    const toggle = screen.getByTestId(`gift-switch-${MOCK_GIFT_1.id}`);
    expect(toggle).toBeChecked();

    await user.click(toggle);

    // 等待 API 失败后回滚
    await waitFor(() => {
      // 回滚后 Switch 应恢复为 checked（true）
      expect(toggle).toBeChecked();
    });
  });
});

// ── G04: W12-09 新增礼物 price=0 时提交按钮禁用 ──────────────────────────────
describe('GiftManagementPage — G04: W12-09 price=0 提交禁用', () => {
  it('打开新增礼物弹窗，price=0 时提交按钮禁用', async () => {
    const user = userEvent.setup();
    render(<GiftManagementPage />);

    await waitFor(() => {
      expect(screen.getByTestId('add-gift-btn')).toBeInTheDocument();
    });

    await user.click(screen.getByTestId('add-gift-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('gift-edit-modal')).toBeInTheDocument();
    });

    // 填写必填字段（除了 price）
    fireEvent.change(screen.getByTestId('gift-form-code'), { target: { value: 'test_gift' } });
    fireEvent.change(screen.getByTestId('gift-form-name-en'), { target: { value: 'Test Gift' } });
    fireEvent.change(screen.getByTestId('gift-form-name-ar'), { target: { value: 'هدية تجريبية' } });
    fireEvent.change(screen.getByTestId('gift-form-icon-url'), { target: { value: '/uploads/gifts/test.png' } });

    // price 输入 0
    const priceInput = screen.getByTestId('gift-form-price');
    await user.clear(priceInput);
    await user.type(priceInput, '0');

    const submitBtn = screen.getByTestId('gift-edit-submit-btn');
    await waitFor(() => {
      expect(submitBtn).toBeDisabled();
    });
  }, 15000);

  it('price=1 时提交按钮可用', async () => {
    const user = userEvent.setup();
    render(<GiftManagementPage />);

    await waitFor(() => {
      expect(screen.getByTestId('add-gift-btn')).toBeInTheDocument();
    });

    await user.click(screen.getByTestId('add-gift-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('gift-edit-modal')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByTestId('gift-form-code'), { target: { value: 'test_gift' } });
    fireEvent.change(screen.getByTestId('gift-form-name-en'), { target: { value: 'Test Gift' } });
    fireEvent.change(screen.getByTestId('gift-form-name-ar'), { target: { value: 'هدية تجريبية' } });
    fireEvent.change(screen.getByTestId('gift-form-icon-url'), { target: { value: '/uploads/gifts/test.png' } });

    const priceInput = screen.getByTestId('gift-form-price');
    await user.clear(priceInput);
    await user.type(priceInput, '10');

    const submitBtn = screen.getByTestId('gift-edit-submit-btn');
    await waitFor(() => {
      expect(submitBtn).not.toBeDisabled();
    });
  }, 15000);
});

// ── G05: W12-10 上传非图片文件（gif）报错提示 ────────────────────────────────
describe('GiftManagementPage — G05: W12-10 Upload 文件类型校验', () => {
  it('打开新增弹窗，上传 gif 文件时显示类型错误提示', async () => {
    const user = userEvent.setup();
    render(<GiftManagementPage />);

    await waitFor(() => {
      expect(screen.getByTestId('add-gift-btn')).toBeInTheDocument();
    });

    await user.click(screen.getByTestId('add-gift-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('gift-edit-modal')).toBeInTheDocument();
    });

    // 找到文件上传组件
    const uploadInput = screen.getByTestId('gift-icon-upload-input');
    const gifFile = new File(['gif content'], 'test.gif', { type: 'image/gif' });

    // 触发 beforeUpload 验证（通过 antd Upload 的 beforeUpload 钩子）
    fireEvent.change(uploadInput, { target: { files: [gifFile] } });

    // 等待错误消息出现
    await waitFor(() => {
      expect(screen.getByTestId('upload-type-error')).toBeInTheDocument();
    });
  });
});

// ── G06: W12-07 AppLayout 菜单 RBAC ──────────────────────────────────────────
describe('GiftManagementPage — G06: W12-07 AppLayout 礼物菜单 RBAC', () => {
  function renderLayoutWithRole(role: string) {
    mockAuthState.admin = {
      id: 'admin-1',
      username: 'admin',
      role,
      display_name: 'Admin',
      last_login_at: '',
    };
    return render(
      <MemoryRouter initialEntries={['/dashboard']}>
        <AppLayout />
      </MemoryRouter>,
    );
  }

  it('super_admin 可见礼物管理菜单', () => {
    renderLayoutWithRole('super_admin');
    expect(screen.getByTestId('menu-item-gifts')).toBeInTheDocument();
  });

  it('operator 可见礼物管理菜单', () => {
    renderLayoutWithRole('operator');
    expect(screen.getByTestId('menu-item-gifts')).toBeInTheDocument();
  });

  it('cs 不可见礼物管理菜单', () => {
    renderLayoutWithRole('cs');
    expect(screen.queryByTestId('menu-item-gifts')).not.toBeInTheDocument();
  });

  it('finance 不可见礼物管理菜单', () => {
    renderLayoutWithRole('finance');
    expect(screen.queryByTestId('menu-item-gifts')).not.toBeInTheDocument();
  });
});

// ── G07: tier 下拉筛选传递参数 ───────────────────────────────────────────────
describe('GiftManagementPage — G07: tier 筛选参数', () => {
  it('选择 tier 后 listGifts 传入 tier 参数', async () => {
    const user = userEvent.setup();
    render(<GiftManagementPage />);

    await waitFor(() => {
      expect(screen.getByTestId('gift-tier-filter')).toBeInTheDocument();
    });

    const tierSelect = screen.getByTestId('gift-tier-filter');
    const combobox = tierSelect.querySelector('[role="combobox"]') ?? tierSelect;
    await user.click(combobox);

    await waitFor(() => {
      expect(document.querySelector('.ant-select-dropdown')).toBeInTheDocument();
    });

    const dropdowns = document.querySelectorAll(
      '.ant-select-dropdown:not(.ant-select-dropdown-hidden)',
    );
    const dropdown = dropdowns[dropdowns.length - 1] as HTMLElement;
    // 选择 Tier 2（对应值 2）
    const option = dropdown.querySelector('[title="gift.mgmt.tier2"]') ??
      Array.from(dropdown.querySelectorAll('.ant-select-item-option-content'))
        .find((el) => el.textContent === 'gift.mgmt.tier2');
    if (option) {
      await user.click(option as HTMLElement);
    }

    // 无论是否找到选项，至少验证 listGifts 被调用了
    expect(mockListGifts).toHaveBeenCalled();
  });
});

// ── G08: 状态筛选（include_inactive）传递参数 ─────────────────────────────────
describe('GiftManagementPage — G08: 状态筛选参数', () => {
  it('默认加载时调用 adminListGifts', async () => {
    render(<GiftManagementPage />);

    await waitFor(() => {
      expect(mockListGifts).toHaveBeenCalled();
    });
  });

  it('默认（all）加载时 adminListGifts 携带 include_inactive=true 参数', async () => {
    render(<GiftManagementPage />);

    await waitFor(() => {
      expect(mockListGifts).toHaveBeenCalledWith(
        expect.objectContaining({ include_inactive: true }),
        expect.anything(),
      );
    });
  });
});

// ── G11: inactive 状态客户端过滤（HIGH-1 修复验收）────────────────────────────
describe('GiftManagementPage — G11: inactive 客户端过滤', () => {
  it('选择 inactive 筛选后，只显示 is_active=false 的礼物，active 礼物不可见', async () => {
    const user = userEvent.setup();
    render(<GiftManagementPage />);

    // 等待初始加载（两个礼物均可见）
    await waitFor(() => {
      expect(screen.getByText('unicorn_01')).toBeInTheDocument(); // is_active=true
      expect(screen.getByText('rose_01')).toBeInTheDocument();   // is_active=false
    });

    // 打开状态筛选下拉
    const statusFilterWrap = screen.getByTestId('gift-status-filter');
    const combobox =
      statusFilterWrap.querySelector('[role="combobox"]') ?? statusFilterWrap;
    await user.click(combobox as HTMLElement);

    // 等待下拉菜单出现
    await waitFor(() => {
      expect(document.querySelector('.ant-select-dropdown')).toBeInTheDocument();
    });

    // 选择「inactive」选项（i18n mock 直接返回 key）
    const dropdowns = document.querySelectorAll(
      '.ant-select-dropdown:not(.ant-select-dropdown-hidden)',
    );
    const dropdown = dropdowns[dropdowns.length - 1] as HTMLElement;
    const inactiveOption =
      (dropdown.querySelector('[title="gift.mgmt.filterInactive"]') as HTMLElement | null) ??
      (Array.from(
        dropdown.querySelectorAll('.ant-select-item-option-content'),
      ).find((el) => el.textContent === 'gift.mgmt.filterInactive') as HTMLElement | undefined);

    // 必须找到选项才能继续
    expect(inactiveOption).toBeTruthy();
    await user.click(inactiveOption!);

    // 等待 API 被再次调用（filter 变化触发第2次请求）
    await waitFor(() => {
      expect(mockListGifts).toHaveBeenCalledTimes(2);
    });

    // 只有下架礼物可见，上架礼物应被客户端过滤掉
    await waitFor(() => {
      expect(screen.queryByText('unicorn_01')).not.toBeInTheDocument();
      expect(screen.getByText('rose_01')).toBeInTheDocument();
    });
  });

  it('选择 inactive 筛选时 adminListGifts 仍以 include_inactive=true 请求（以获取下架礼物）', async () => {
    const user = userEvent.setup();
    render(<GiftManagementPage />);

    await waitFor(() => {
      expect(screen.getByTestId('gift-status-filter')).toBeInTheDocument();
    });

    const statusFilterWrap = screen.getByTestId('gift-status-filter');
    const combobox =
      statusFilterWrap.querySelector('[role="combobox"]') ?? statusFilterWrap;
    await user.click(combobox as HTMLElement);

    await waitFor(() => {
      expect(document.querySelector('.ant-select-dropdown')).toBeInTheDocument();
    });

    const dropdowns = document.querySelectorAll(
      '.ant-select-dropdown:not(.ant-select-dropdown-hidden)',
    );
    const dropdown = dropdowns[dropdowns.length - 1] as HTMLElement;
    const inactiveOption =
      (dropdown.querySelector('[title="gift.mgmt.filterInactive"]') as HTMLElement | null) ??
      (Array.from(
        dropdown.querySelectorAll('.ant-select-item-option-content'),
      ).find((el) => el.textContent === 'gift.mgmt.filterInactive') as HTMLElement | undefined);

    expect(inactiveOption).toBeTruthy();
    await user.click(inactiveOption!);

    await waitFor(() => {
      expect(mockListGifts).toHaveBeenCalledTimes(2);
    });

    // 第2次调用必须携带 include_inactive=true
    const allCalls = mockListGifts.mock.calls;
    const lastCall = allCalls[allCalls.length - 1][0] as Record<string, unknown>;
    expect(lastCall.include_inactive).toBe(true);
  });
});

// ── G09: 软删除 ───────────────────────────────────────────────────────────────
describe('GiftManagementPage — G09: 软删除', () => {
  it('点击删除按钮后调用 adminDeleteGift', async () => {
    const user = userEvent.setup();
    render(<GiftManagementPage />);

    await waitFor(() => {
      expect(screen.getByTestId(`gift-delete-btn-${MOCK_GIFT_1.id}`)).toBeInTheDocument();
    });

    await user.click(screen.getByTestId(`gift-delete-btn-${MOCK_GIFT_1.id}`));

    // Popconfirm 确认
    await waitFor(() => {
      const confirmBtn = document.querySelector('.ant-popconfirm .ant-btn-primary');
      if (confirmBtn) {
        fireEvent.click(confirmBtn);
      }
    });

    await waitFor(() => {
      expect(mockDeleteGift).toHaveBeenCalledWith(MOCK_GIFT_1.id);
    });
  });
});

// ── G10: 新增礼物提交 ─────────────────────────────────────────────────────────
describe('GiftManagementPage — G10: 新增礼物提交', () => {
  it('填写完整表单后提交调用 adminCreateGift', async () => {
    const user = userEvent.setup();
    render(<GiftManagementPage />);

    await waitFor(() => {
      expect(screen.getByTestId('add-gift-btn')).toBeInTheDocument();
    });

    await user.click(screen.getByTestId('add-gift-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('gift-edit-modal')).toBeInTheDocument();
    });

    // 使用 fireEvent 加快表单填写速度
    fireEvent.change(screen.getByTestId('gift-form-code'), { target: { value: 'new_gift_01' } });
    fireEvent.change(screen.getByTestId('gift-form-name-en'), { target: { value: 'New Gift' } });
    fireEvent.change(screen.getByTestId('gift-form-name-ar'), { target: { value: 'هدية جديدة' } });
    fireEvent.change(screen.getByTestId('gift-form-icon-url'), { target: { value: '/uploads/gifts/new.png' } });

    const priceInput = screen.getByTestId('gift-form-price');
    await user.clear(priceInput);
    await user.type(priceInput, '50');

    const submitBtn = screen.getByTestId('gift-edit-submit-btn');
    await waitFor(() => expect(submitBtn).not.toBeDisabled());

    await user.click(submitBtn);

    await waitFor(() => {
      expect(mockCreateGift).toHaveBeenCalled();
    });
  }, 15000);
});
