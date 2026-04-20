/**
 * T-20003: TrendChart 组件 — TDD 测试套件
 *
 * 验收用例：
 *   T01: trend=[] → 渲染 Empty，data-testid="trend-empty"
 *   T02: trend 有数据 → data-testid="trend-chart" 存在，不渲染 Empty
 *   T03: loading=true → 渲染 Skeleton，不渲染图表
 *   T04: trend 引用未变时重新渲染 → option 对象引用保持不变（useMemo 缓存）
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';

// ── i18n mock ─────────────────────────────────────────────────────────────────
// 使用模块级常量确保 t 函数引用稳定，让 useMemo([trend, t]) 能正确命中缓存
const mockT = (key: string) => key;

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: mockT,
    i18n: { changeLanguage: vi.fn(), language: 'zh' },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

// ── ECharts mock（jsdom 无法渲染 canvas）────────────────────────────────────────
// 用模块级数组捕获每次渲染时传入的 option 引用，供 T04 断言使用
const capturedOptions: unknown[] = [];

vi.mock('echarts-for-react', () => ({
  default: ({
    'data-testid': testId,
    option,
  }: {
    'data-testid'?: string;
    option?: unknown;
  }) => {
    capturedOptions.push(option);
    return <div data-testid={testId ?? 'echarts-mock'} />;
  },
}));

import { TrendChart, type TrendPoint } from './TrendChart';

const TREND_DATA: TrendPoint[] = [
  { date: '2025-05-10', dau: 1100, new_users: 80 },
  { date: '2025-05-11', dau: 1200, new_users: 88 },
];

// ── T01: trend=[] → Empty ─────────────────────────────────────────────────────
describe('TrendChart — T01: 空数据', () => {
  it('trend=[] 时渲染 data-testid="trend-empty"', () => {
    render(<TrendChart trend={[]} loading={false} />);
    expect(screen.getByTestId('trend-empty')).toBeInTheDocument();
  });

  it('trend=[] 时不渲染图表', () => {
    render(<TrendChart trend={[]} loading={false} />);
    expect(screen.queryByTestId('trend-chart')).not.toBeInTheDocument();
  });
});

// ── T02: trend 有数据 → 渲染图表 ──────────────────────────────────────────────
describe('TrendChart — T02: 有数据', () => {
  it('trend 有数据时渲染 data-testid="trend-chart"', () => {
    render(<TrendChart trend={TREND_DATA} loading={false} />);
    expect(screen.getByTestId('trend-chart')).toBeInTheDocument();
  });

  it('trend 有数据时不渲染 Empty', () => {
    render(<TrendChart trend={TREND_DATA} loading={false} />);
    expect(screen.queryByTestId('trend-empty')).not.toBeInTheDocument();
  });
});

// ── T03: loading=true → Skeleton ──────────────────────────────────────────────
describe('TrendChart — T03: loading 状态', () => {
  it('loading=true 时渲染骨架屏', () => {
    render(<TrendChart trend={[]} loading={true} />);
    const skeletons = document.querySelectorAll('.ant-skeleton');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it('loading=true 时不渲染图表', () => {
    render(<TrendChart trend={TREND_DATA} loading={true} />);
    expect(screen.queryByTestId('trend-chart')).not.toBeInTheDocument();
  });
});

// ── T04: useMemo 缓存验证 ─────────────────────────────────────────────────────
// 验证当 trend 数据引用未发生变化时，option 对象引用不重新创建（useMemo 效果）
describe('TrendChart — T04: useMemo 缓存', () => {
  it('trend 引用未变时重新渲染 option 对象引用保持不变', () => {
    // 清空捕获数组，隔离本用例
    capturedOptions.length = 0;

    const { rerender } = render(<TrendChart trend={TREND_DATA} loading={false} />);
    // 用同一个 TREND_DATA 引用重新渲染（依赖未变）
    rerender(<TrendChart trend={TREND_DATA} loading={false} />);

    // ECharts mock 应该被调用了两次（initial render + rerender）
    expect(capturedOptions).toHaveLength(2);
    // option 对象引用必须相同（useMemo 命中缓存，工厂函数未重新执行）
    expect(capturedOptions[0]).toBe(capturedOptions[1]);
  });

  it('trend 引用变化时 option 对象引用更新', () => {
    capturedOptions.length = 0;

    const TREND_V2: typeof TREND_DATA = [
      { date: '2025-05-12', dau: 1300, new_users: 90 },
    ];

    const { rerender } = render(<TrendChart trend={TREND_DATA} loading={false} />);
    // 用不同引用重新渲染（依赖变化）
    rerender(<TrendChart trend={TREND_V2} loading={false} />);

    expect(capturedOptions).toHaveLength(2);
    // option 对象引用必须不同（useMemo 重新计算）
    expect(capturedOptions[0]).not.toBe(capturedOptions[1]);
  });
});
