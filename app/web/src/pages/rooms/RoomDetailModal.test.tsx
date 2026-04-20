/**
 * T-20005: RoomDetailModal 组件测试
 *
 * 验收用例：
 *   C01: selectedRoomId=null → Modal 不可见
 *   C02: selectedRoomId='uuid-1' → Modal open，useRoomDetail 以 'uuid-1' 调用
 *   C03: loading=true → data-testid="detail-loading" 可见
 *   C04: detail 加载成功 → 展示各字段（title, owner.nickname, member_count）
 *   C05: status=active → 关闭按钮不 disabled
 *   C06: status=closed → 关闭按钮 disabled
 *   C07: error → data-testid="detail-error" 可见
 *   C08: 点击 Modal 关闭 → onClose 被调用
 *   C09: 点击 [强制关闭] → Modal.confirm 被调用
 *   C10: 确认关闭 → onCloseRoom 调用，成功后 onClose 调用
 *   C11: 取消关闭 → onCloseRoom/onClose 不调用
 *   C12: members-placeholder 有占位文字
 *   C13: chat-placeholder 有占位文字
 *   C14: closingId === selectedRoomId → 按钮 loading
 *   C15: selectedRoomId 切换 → destroyOnClose 保证旧数据清除
 */

import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';
import { Modal } from 'antd';

// ── i18n mock ─────────────────────────────────────────────────────────────
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { changeLanguage: vi.fn(), language: 'zh' },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ── useRoomDetail mock ─────────────────────────────────────────────────────
vi.mock('./useRoomDetail', () => ({
  useRoomDetail: vi.fn(),
}));

import { useRoomDetail } from './useRoomDetail';
import { RoomDetailModal } from './RoomDetailModal';
import type { AdminRoomDetail } from '../../core/network/apiClient';

const mockUseRoomDetail = useRoomDetail as ReturnType<typeof vi.fn>;

// ── 测试数据 ──────────────────────────────────────────────────────────────
const mockDetail: AdminRoomDetail = {
  room_id: 'uuid-1',
  title: 'Test Room Title',
  status: 'active',
  room_type: 'normal',
  member_count: 3,
  max_members: 20,
  owner: {
    user_id: 'user-1',
    nickname: 'TestOwner',
    avatar: null,
  },
  mic_slots: [],
  created_at: '2025-01-01T00:00:00Z',
  updated_at: '2025-01-01T00:00:00Z',
};

// ── Props ──────────────────────────────────────────────────────────────────
function makeProps(overrides: Partial<{
  selectedRoomId: string | null;
  onClose: () => void;
  onCloseRoom: (roomId: string) => Promise<void>;
  closingId: string | null;
}> = {}) {
  return {
    selectedRoomId: 'uuid-1',
    onClose: vi.fn(),
    onCloseRoom: vi.fn().mockResolvedValue(undefined),
    closingId: null,
    ...overrides,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  // 默认：加载成功状态
  mockUseRoomDetail.mockReturnValue({ detail: mockDetail, loading: false, error: null });
});

// ── C01: selectedRoomId=null → Modal 不可见 ───────────────────────────────
describe('RoomDetailModal — C01: selectedRoomId=null 不显示', () => {
  it('selectedRoomId=null 时 Modal 不在 DOM 中', () => {
    mockUseRoomDetail.mockReturnValue({ detail: null, loading: false, error: null });
    render(<RoomDetailModal {...makeProps({ selectedRoomId: null })} />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});

// ── C02: selectedRoomId='uuid-1' → Modal open，useRoomDetail 调用 ─────────
describe('RoomDetailModal — C02: selectedRoomId 非 null 打开 Modal', () => {
  it('selectedRoomId="uuid-1" 时 Modal 可见，useRoomDetail 以 "uuid-1" 调用', async () => {
    render(<RoomDetailModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());
    expect(mockUseRoomDetail).toHaveBeenCalledWith('uuid-1');
  });
});

// ── C03: loading=true → detail-loading 可见 ──────────────────────────────
describe('RoomDetailModal — C03: loading 态', () => {
  it('useRoomDetail 返回 loading=true 时 detail-loading 可见', async () => {
    mockUseRoomDetail.mockReturnValue({ detail: null, loading: true, error: null });
    render(<RoomDetailModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByTestId('detail-loading')).toBeInTheDocument());
  });
});

// ── C04: detail 加载成功 → 展示各字段 ────────────────────────────────────
describe('RoomDetailModal — C04: detail 字段展示', () => {
  it('detail 加载后 detail-basic-info 展示 title / owner.nickname / member_count', async () => {
    render(<RoomDetailModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByTestId('detail-basic-info')).toBeInTheDocument());

    const basicInfo = screen.getByTestId('detail-basic-info');
    expect(basicInfo).toHaveTextContent('Test Room Title');
    expect(basicInfo).toHaveTextContent('TestOwner');
    expect(basicInfo).toHaveTextContent('3');
  });
});

// ── C05: status=active → 关闭按钮不 disabled ─────────────────────────────
describe('RoomDetailModal — C05: status=active 关闭按钮可用', () => {
  it('detail.status=active 时 close-room-btn 不 disabled', async () => {
    render(<RoomDetailModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByTestId('close-room-btn')).toBeInTheDocument());
    expect(screen.getByTestId('close-room-btn')).not.toBeDisabled();
  });
});

// ── C06: status=closed → 关闭按钮 disabled ───────────────────────────────
describe('RoomDetailModal — C06: status=closed 关闭按钮禁用', () => {
  it('detail.status=closed 时 close-room-btn disabled', async () => {
    mockUseRoomDetail.mockReturnValue({
      detail: { ...mockDetail, status: 'closed' },
      loading: false,
      error: null,
    });
    render(<RoomDetailModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByTestId('close-room-btn')).toBeInTheDocument());
    expect(screen.getByTestId('close-room-btn')).toBeDisabled();
  });
});

// ── C07: error → detail-error 可见 ───────────────────────────────────────
describe('RoomDetailModal — C07: error 态', () => {
  it('useRoomDetail 返回 error 时 detail-error 可见', async () => {
    mockUseRoomDetail.mockReturnValue({
      detail: null,
      loading: false,
      error: new Error('Load failed'),
    });
    render(<RoomDetailModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByTestId('detail-error')).toBeInTheDocument());
  });
});

// ── C08: 点击 Modal 关闭 → onClose 被调用 ────────────────────────────────
describe('RoomDetailModal — C08: onClose 调用', () => {
  it('点击 Modal 右上角关闭按钮后 onClose 被调用', async () => {
    const user = userEvent.setup();
    const props = makeProps();
    render(<RoomDetailModal {...props} />);

    await waitFor(() => expect(screen.getByRole('dialog')).toBeInTheDocument());

    // Ant Design Modal 关闭按钮：使用 .ant-modal-close 选择器避免多元素匹配
    const closeBtn = document.querySelector('.ant-modal-close') as HTMLElement;
    expect(closeBtn).toBeTruthy();
    await user.click(closeBtn);

    expect(props.onClose).toHaveBeenCalled();
  });
});

// ── C09: 点击 [强制关闭] → Modal.confirm 被调用 ───────────────────────────
describe('RoomDetailModal — C09: Modal.confirm 被触发', () => {
  it('点击 close-room-btn 后 Modal.confirm 被调用', async () => {
    const user = userEvent.setup();
    const confirmSpy = vi.spyOn(Modal, 'confirm').mockReturnValue({
      destroy: vi.fn(),
      update: vi.fn(),
    } as ReturnType<typeof Modal.confirm>);

    render(<RoomDetailModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByTestId('close-room-btn')).toBeInTheDocument());

    await user.click(screen.getByTestId('close-room-btn'));

    expect(confirmSpy).toHaveBeenCalled();
    confirmSpy.mockRestore();
  });
});

// ── C10: 确认关闭 → onCloseRoom 调用，成功后 onClose 调用 ──────────────────
describe('RoomDetailModal — C10: 确认关闭执行操作', () => {
  it('Modal.confirm onOk 回调触发 → onCloseRoom("uuid-1") 且成功后 onClose 调用', async () => {
    const user = userEvent.setup();
    const props = makeProps();

    let capturedOnOk: (() => Promise<void> | void) | undefined;
    const confirmSpy = vi.spyOn(Modal, 'confirm').mockImplementation((config) => {
      capturedOnOk = config?.onOk as (() => Promise<void>) | undefined;
      return { destroy: vi.fn(), update: vi.fn() } as ReturnType<typeof Modal.confirm>;
    });

    render(<RoomDetailModal {...props} />);
    await waitFor(() => expect(screen.getByTestId('close-room-btn')).toBeInTheDocument());

    await user.click(screen.getByTestId('close-room-btn'));
    expect(capturedOnOk).toBeDefined();

    // 触发确认回调
    await capturedOnOk!();

    expect(props.onCloseRoom).toHaveBeenCalledWith('uuid-1');
    expect(props.onClose).toHaveBeenCalled();

    confirmSpy.mockRestore();
  });
});

// ── C11: 取消关闭 → onCloseRoom/onClose 不调用 ────────────────────────────
describe('RoomDetailModal — C11: 取消关闭不执行操作', () => {
  it('不触发 onOk 时 onCloseRoom 和 onClose 均不调用', async () => {
    const user = userEvent.setup();
    const props = makeProps();

    const confirmSpy = vi.spyOn(Modal, 'confirm').mockReturnValue({
      destroy: vi.fn(),
      update: vi.fn(),
    } as ReturnType<typeof Modal.confirm>);

    render(<RoomDetailModal {...props} />);
    await waitFor(() => expect(screen.getByTestId('close-room-btn')).toBeInTheDocument());

    await user.click(screen.getByTestId('close-room-btn'));

    // 未调用 onOk，simulate 用户取消
    expect(props.onCloseRoom).not.toHaveBeenCalled();
    expect(props.onClose).not.toHaveBeenCalled();

    confirmSpy.mockRestore();
  });
});

// ── C12: members-placeholder 有占位文字 ──────────────────────────────────
describe('RoomDetailModal — C12: members-placeholder', () => {
  it('members-placeholder 区域包含占位文字', async () => {
    render(<RoomDetailModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByTestId('members-placeholder')).toBeInTheDocument());
    // t(key) => key，所以占位文字为 key 本身
    expect(screen.getByTestId('members-placeholder')).toHaveTextContent(
      'rooms.detail.membersPlaceholder',
    );
  });
});

// ── C13: chat-placeholder 有占位文字 ─────────────────────────────────────
describe('RoomDetailModal — C13: chat-placeholder', () => {
  it('chat-placeholder 区域包含占位文字', async () => {
    render(<RoomDetailModal {...makeProps()} />);
    await waitFor(() => expect(screen.getByTestId('chat-placeholder')).toBeInTheDocument());
    expect(screen.getByTestId('chat-placeholder')).toHaveTextContent(
      'rooms.detail.chatPlaceholder',
    );
  });
});

// ── C14: closingId === selectedRoomId → 按钮 loading ─────────────────────
describe('RoomDetailModal — C14: closingId 匹配时按钮 loading', () => {
  it('closingId="uuid-1" 且 selectedRoomId="uuid-1" 时按钮处于 loading 状态', async () => {
    render(<RoomDetailModal {...makeProps({ closingId: 'uuid-1' })} />);
    await waitFor(() => expect(screen.getByTestId('close-room-btn')).toBeInTheDocument());

    const btn = screen.getByTestId('close-room-btn');
    const hasLoading =
      btn.getAttribute('aria-busy') === 'true' ||
      btn.querySelector('[class*="loading"]') !== null ||
      btn.classList.contains('ant-btn-loading');
    expect(hasLoading).toBe(true);
  });
});

// ── C15: selectedRoomId 切换 → destroyOnHidden 保证旧数据清除 ──────────────
describe('RoomDetailModal — C15: destroyOnHidden 旧数据清除', () => {
  it('selectedRoomId 切换时旧 detail 数据不再显示', async () => {
    // 初始：uuid-1 的 detail 已加载
    mockUseRoomDetail.mockReturnValue({ detail: mockDetail, loading: false, error: null });

    const { rerender } = render(<RoomDetailModal {...makeProps({ selectedRoomId: 'uuid-1' })} />);
    await waitFor(() =>
      expect(screen.getByTestId('detail-basic-info')).toHaveTextContent('Test Room Title'),
    );

    // 切换到 uuid-2 的 loading 状态（新数据未到）
    mockUseRoomDetail.mockReturnValue({ detail: null, loading: true, error: null });

    rerender(
      <RoomDetailModal
        {...makeProps({ selectedRoomId: 'uuid-2', closingId: null })}
      />,
    );

    // 旧 title 不应出现
    expect(screen.queryByText('Test Room Title')).not.toBeInTheDocument();
    // 加载中标志可见
    await waitFor(() => expect(screen.getByTestId('detail-loading')).toBeInTheDocument());
  });
});
