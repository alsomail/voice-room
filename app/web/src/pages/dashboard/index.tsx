/**
 * DashboardPage — 数据看板首页（T-20003）
 *
 * 展示实时统计卡片（在线人数 / 活跃房间 / DAU / 新增用户）+ ECharts 趋势图
 * 支持手动刷新和 30 秒自动刷新（由 useDashboardStats Hook 管理）
 */

import { Button, Alert, Space, Typography } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useDashboardStats } from './useDashboardStats';
import { StatCards } from './StatCards';
import { TrendChart } from './TrendChart';

const { Title } = Typography;

export function DashboardPage() {
  const { t } = useTranslation();
  const { stats, loading, error, refresh, lastUpdatedAt } = useDashboardStats();

  return (
    <div data-testid="dashboard-page" style={{ padding: '24px' }}>
      {/* 标题栏 */}
      <Space style={{ marginBottom: 16, width: '100%', justifyContent: 'space-between' }}>
        <Title level={4} style={{ margin: 0 }}>
          {t('dashboard.title')}
        </Title>
        <Space>
          {lastUpdatedAt && (
            <span style={{ color: '#8c8c8c', fontSize: 12 }}>
              {t('dashboard.lastUpdated')}: {lastUpdatedAt.toLocaleTimeString()}
            </span>
          )}
          <Button
            data-testid="btn-refresh"
            icon={<ReloadOutlined />}
            loading={loading}
            onClick={refresh}
          >
            {t('dashboard.refresh')}
          </Button>
        </Space>
      </Space>

      {/* 错误提示 */}
      {error && (
        <Alert
          data-testid="dashboard-error"
          type="error"
          title={t('dashboard.errorTitle')}
          description={error.message}
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      {/* 统计卡片 */}
      <StatCards stats={stats} loading={loading} />

      {/* 趋势图 */}
      <div style={{ marginTop: 24, background: '#fff', padding: 24, borderRadius: 8 }}>
        <Title level={5} style={{ marginBottom: 16 }}>
          {t('dashboard.trendTitle')}
        </Title>
        <TrendChart trend={stats.trend} loading={loading} />
      </div>
    </div>
  );
}
