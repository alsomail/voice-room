/**
 * T-20003: StatCards 组件 — TDD 测试套件
 *
 * 验收用例：
 *   C01: 完整 stats → 4 张卡片显示正确数值
 *   C02: loading=true → 显示骨架屏，不显示数值
 *   C03: onlineUsers=null → 对应卡片显示 "--"
 *   C04: activeRooms=null → 对应卡片显示 "--"
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';

// ── i18n mock ─────────────────────────────────────────────────────────────────
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { changeLanguage: vi.fn(), language: 'zh' },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

import { StatCards, type DashboardStats } from './StatCards';

const FULL_STATS: DashboardStats = {
  totalRooms: 50,
  activeRooms: 12,
  onlineUsers: 340,
  dau: 1200,
  newUsersToday: 88,
  trend: [],
};

// ── C01: 完整 stats → 4 张卡片正确数值 ───────────────────────────────────────
describe('StatCards — C01: 正常渲染', () => {
  it('显示在线人数卡片', () => {
    render(<StatCards stats={FULL_STATS} loading={false} />);
    expect(screen.getByTestId('card-online-users')).toBeInTheDocument();
    expect(screen.getByTestId('card-online-users')).toHaveTextContent('340');
  });

  it('显示活跃房间数卡片', () => {
    render(<StatCards stats={FULL_STATS} loading={false} />);
    expect(screen.getByTestId('card-active-rooms')).toBeInTheDocument();
    expect(screen.getByTestId('card-active-rooms')).toHaveTextContent('12');
  });

  it('显示今日 DAU 卡片', () => {
    render(<StatCards stats={FULL_STATS} loading={false} />);
    expect(screen.getByTestId('card-dau')).toBeInTheDocument();
    expect(screen.getByTestId('card-dau')).toHaveTextContent('1200');
  });

  it('显示新增用户卡片', () => {
    render(<StatCards stats={FULL_STATS} loading={false} />);
    expect(screen.getByTestId('card-new-users')).toBeInTheDocument();
    expect(screen.getByTestId('card-new-users')).toHaveTextContent('88');
  });
});

// ── C02: loading=true → 骨架屏 ───────────────────────────────────────────────
describe('StatCards — C02: loading 状态', () => {
  it('loading=true 时渲染骨架屏', () => {
    render(<StatCards stats={FULL_STATS} loading={true} />);
    // Ant Design Skeleton 会渲染 ant-skeleton 类
    const skeletons = document.querySelectorAll('.ant-skeleton');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it('loading=true 时不渲染在线人数数值', () => {
    render(<StatCards stats={FULL_STATS} loading={true} />);
    expect(screen.queryByTestId('card-online-users')).not.toBeInTheDocument();
  });
});

// ── C03: onlineUsers=null → "--" ──────────────────────────────────────────────
describe('StatCards — C03: onlineUsers=null', () => {
  it('在线人数为 null 时卡片显示 "--"', () => {
    const stats = { ...FULL_STATS, onlineUsers: null };
    render(<StatCards stats={stats} loading={false} />);
    expect(screen.getByTestId('card-online-users')).toHaveTextContent('--');
  });
});

// ── C04: activeRooms=null → "--" ─────────────────────────────────────────────
describe('StatCards — C04: activeRooms=null', () => {
  it('活跃房间数为 null 时卡片显示 "--"', () => {
    const stats = { ...FULL_STATS, activeRooms: null };
    render(<StatCards stats={stats} loading={false} />);
    expect(screen.getByTestId('card-active-rooms')).toHaveTextContent('--');
  });
});
