/**
 * StatCards — 数据看板统计卡片组（T-20003）
 *
 * 展示 4 张统计卡片：在线人数 / 活跃房间 / 今日 DAU / 今日新增用户
 * - loading=true 时显示骨架屏
 * - 对应字段为 null 时显示 "--"
 */

import { Row, Col, Card, Statistic, Skeleton } from 'antd';
import { useTranslation } from 'react-i18next';
import type { DashboardStats } from './useDashboardStats';

// 重新导出供测试文件使用
export type { DashboardStats };

export interface StatCardsProps {
  stats: DashboardStats;
  loading: boolean;
}

function formatValue(value: number | null): string {
  // 返回字符串以禁用 Ant Design Statistic 的千分符格式化，便于测试断言
  return value === null ? '--' : String(value);
}

export function StatCards({ stats, loading }: StatCardsProps) {
  const { t } = useTranslation();

  if (loading) {
    return (
      <Row gutter={[16, 16]}>
        {[0, 1, 2, 3].map((i) => (
          <Col key={i} xs={24} sm={12} lg={6}>
            <Card>
              <Skeleton active paragraph={{ rows: 1 }} />
            </Card>
          </Col>
        ))}
      </Row>
    );
  }

  return (
    <Row gutter={[16, 16]}>
      <Col xs={24} sm={12} lg={6}>
        <Card data-testid="card-online-users">
          <Statistic
            title={t('dashboard.onlineUsers')}
            value={formatValue(stats.onlineUsers)}
            groupSeparator=""
          />
        </Card>
      </Col>

      <Col xs={24} sm={12} lg={6}>
        <Card data-testid="card-active-rooms">
          <Statistic
            title={t('dashboard.activeRooms')}
            value={formatValue(stats.activeRooms)}
            groupSeparator=""
          />
        </Card>
      </Col>

      <Col xs={24} sm={12} lg={6}>
        <Card data-testid="card-dau">
          <Statistic
            title={t('dashboard.dau')}
            value={formatValue(stats.dau)}
            groupSeparator=""
          />
        </Card>
      </Col>

      <Col xs={24} sm={12} lg={6}>
        <Card data-testid="card-new-users">
          <Statistic
            title={t('dashboard.newUsersToday')}
            value={formatValue(stats.newUsersToday)}
            groupSeparator=""
          />
        </Card>
      </Col>
    </Row>
  );
}
