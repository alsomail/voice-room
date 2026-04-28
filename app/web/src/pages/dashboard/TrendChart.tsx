/**
 * TrendChart — 趋势折线图组件（T-20003）
 *
 * - trend.length === 0 → 显示 <Empty data-testid="trend-empty" />
 * - trend 有数据 → <ReactECharts data-testid="trend-chart" />
 * - loading=true → 显示 <Skeleton />
 */

import { useMemo } from 'react';
import { Skeleton, Empty } from 'antd';
import ReactECharts from 'echarts-for-react';
import { useTranslation } from 'react-i18next';
import type { TrendPoint } from './useDashboardStats';

// 重新导出供测试文件使用
export type { TrendPoint };

export interface TrendChartProps {
  trend: TrendPoint[];
  loading: boolean;
}

/** 图表容器高度（px） */
const CHART_HEIGHT_PX = 320;

export function TrendChart({ trend, loading }: TrendChartProps) {
  const { t } = useTranslation();

  // [H-01] useMemo 缓存 option 对象，避免 trend/t 未变时触发 ECharts 重绘
  // [BUG-FIX] guard against undefined trend (API may not return trend field)
  const safeTrend = trend ?? [];
  const option = useMemo(
    () => ({
      tooltip: { trigger: 'axis' },
      legend: { data: [t('dashboard.dau'), t('dashboard.newUsersToday')] },
      xAxis: {
        type: 'category',
        data: safeTrend.map((p) => p.date),
      },
      yAxis: { type: 'value' },
      series: [
        {
          name: t('dashboard.dau'),
          type: 'line',
          smooth: true,
          data: safeTrend.map((p) => p.dau),
        },
        {
          name: t('dashboard.newUsersToday'),
          type: 'line',
          smooth: true,
          data: safeTrend.map((p) => p.new_users),
        },
      ],
    }),
    [safeTrend, t],
  );

  if (loading) {
    return <Skeleton active paragraph={{ rows: 4 }} />;
  }

  if (safeTrend.length === 0) {
    return (
      <Empty
        data-testid="trend-empty"
        description={t('dashboard.noTrendData')}
      />
    );
  }

  return (
    <ReactECharts
      data-testid="trend-chart"
      option={option}
      style={{ height: CHART_HEIGHT_PX }}
    />
  );
}
