/**
 * GovernanceLogsPage — 房间治理日志查询主页（T-20014）
 *
 * 路由：/rooms/governance（受 AuthGuard 保护）
 * 权限：roles ∩ [super_admin, operator, cs] 非空才能进入；finance 菜单隐藏
 *
 * 功能：
 *   - 双 Tab：踢人记录（KickLogsTab）/ 禁言记录（MuteLogsTab）
 *   - 共用筛选条（FiltersBar）：房间 ID / 目标用户 / 操作者 / 时间范围
 *   - mutes tab 额外：禁言类型筛选
 *   - 默认时间范围：最近 7 天
 *   - Tab 切换时重置筛选和分页
 *   - 目标用户点击 → 用户详情 Drawer
 *
 * testTag:
 *   governance-page, governance-page-title,
 *   governance-tab-kicks, governance-tab-mutes,
 *   governance-filter-room, governance-filter-target-user,
 *   governance-row-{id}, governance-user-link-{user_id},
 *   governance-user-drawer, governance-drawer-user-id
 */

import { useState, useCallback } from 'react';
import { Tabs, Drawer, Typography } from 'antd';
import { useTranslation } from 'react-i18next';
import type { GovernanceFilters } from './FiltersBar';
import { FiltersBar } from './FiltersBar';
import { KickLogsTab } from './KickLogsTab';
import { MuteLogsTab } from './MuteLogsTab';
import type { MuteListParams } from '../../services/api/governance';

const { Title } = Typography;

/** 构造默认 7 天筛选条件 */
function getDefaultFilters(): GovernanceFilters {
  const now = new Date();
  const sevenDaysAgo = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
  return {
    from: sevenDaysAgo.toISOString(),
    to: now.toISOString(),
  };
}

/**
 * 将 GovernanceFilters（含 mute_type 字段）映射为 MuteListParams（含 type 字段）
 *
 * [HIGH-1 修复] GovernanceFilters.mute_type 对应 MuteListParams.type，
 * 直接强转 `filters as MuteListParams` 不会重命名字段，
 * 运行时会向服务端发送 mute_type=mic（服务端期望 type=mic），导致筛选静默失效。
 */
// eslint-disable-next-line react-refresh/only-export-components
export function toMuteListParams(filters: GovernanceFilters): MuteListParams {
  const { mute_type, ...rest } = filters;
  return { ...rest, type: mute_type };
}

type ActiveTab = 'kicks' | 'mutes';

export function GovernanceLogsPage() {
  const { t } = useTranslation();

  const [activeTab, setActiveTab] = useState<ActiveTab>('kicks');
  const [filters, setFilters] = useState<GovernanceFilters>(getDefaultFilters);

  // 用户详情 Drawer 状态
  const [selectedUserId, setSelectedUserId] = useState<string | null>(null);

  const handleTabChange = useCallback((tab: string) => {
    setActiveTab(tab as ActiveTab);
    // Tab 切换时重置筛选（默认 7 天）
    setFilters(getDefaultFilters());
  }, []);

  const handleSearch = useCallback((newFilters: GovernanceFilters) => {
    setFilters(newFilters);
  }, []);

  const handleReset = useCallback(() => {
    setFilters(getDefaultFilters());
  }, []);

  const handleUserClick = useCallback((userId: string) => {
    setSelectedUserId(userId);
  }, []);

  const handleDrawerClose = useCallback(() => {
    setSelectedUserId(null);
  }, []);

  const tabItems = [
    {
      key: 'kicks',
      label: (
        <span data-testid="governance-tab-kicks">
          {t('governance.tabKicks')}
        </span>
      ),
      children: (
        <KickLogsTab
          filters={filters}
          onUserClick={handleUserClick}
        />
      ),
    },
    {
      key: 'mutes',
      label: (
        <span data-testid="governance-tab-mutes">
          {t('governance.tabMutes')}
        </span>
      ),
      children: (
        <MuteLogsTab
          filters={toMuteListParams(filters)}
          onUserClick={handleUserClick}
        />
      ),
    },
  ];

  return (
    <div
      data-testid="governance-page"
      style={{ padding: 24 }}
    >
      {/* 页面标题 */}
      <Title level={4} data-testid="governance-page-title" style={{ marginBottom: 16 }}>
        {t('governance.title')}
      </Title>

      {/* 共用筛选条 */}
      <FiltersBar
        activeTab={activeTab}
        filters={filters}
        onSearch={handleSearch}
        onReset={handleReset}
      />

      {/* 双 Tab（MEDIUM-1：destroyOnHidden 避免非活跃 Tab 发起多余 API 请求） */}
      <Tabs
        activeKey={activeTab}
        onChange={handleTabChange}
        items={tabItems}
        destroyOnHidden
      />

      {/* 用户详情 Drawer */}
      <Drawer
        title={t('governance.userDrawerTitle')}
        open={selectedUserId !== null}
        onClose={handleDrawerClose}
        size="large"
        data-testid="governance-user-drawer"
        destroyOnHidden
      >
        {selectedUserId && (
          <div>
            <span data-testid="governance-drawer-user-id">{selectedUserId}</span>
          </div>
        )}
      </Drawer>
    </div>
  );
}
