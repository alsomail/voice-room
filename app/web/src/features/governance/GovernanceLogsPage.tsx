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
import { Tabs, Drawer, Typography, Button, message as antdMessage } from 'antd';
import { DownloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import type { GovernanceFilters } from './FiltersBar';
import { FiltersBar } from './FiltersBar';
import { KickLogsTab } from './KickLogsTab';
import { MuteLogsTab } from './MuteLogsTab';
import {
  exportGovernanceLogsCsv,
  type MuteListParams,
} from '../../services/api/governance';

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

  const [exporting, setExporting] = useState(false);

  /**
   * 导出 CSV — R1 P1-6 / T-20014 #4
   *
   * 1. 透传当前筛选条件（含 activeTab → type=kick|mute|mic|chat 维度）；
   * 2. fetch 后用 Blob + URL.createObjectURL 触发下载；
   * 3. filename 来自后端 Content-Disposition（`governance-logs-YYYYMMDD.csv`）。
   */
  const handleExportCsv = useCallback(async () => {
    setExporting(true);
    try {
      const exportParams: GovernanceFilters & { type?: string } = { ...filters };
      if (activeTab === 'kicks') {
        exportParams.type = 'kick';
      } else if (filters.mute_type) {
        // mutes tab + 已选了 mic / chat 子类型时，type 取细粒度过滤
        exportParams.type = filters.mute_type;
      } else {
        exportParams.type = 'mute';
      }
      // governance 后端不识别 mute_type 字段（用 type 维度统一表达）
      delete exportParams.mute_type;

      const { blob, filename } = await exportGovernanceLogsCsv(exportParams);
      const objectUrl = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = objectUrl;
      link.download = filename;
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      // 100ms 后释放，避免某些浏览器尚未读完
      setTimeout(() => URL.revokeObjectURL(objectUrl), 100);
    } catch (e) {
      const msg = e instanceof Error ? e.message : 'export failed';
      antdMessage.error(msg);
    } finally {
      setExporting(false);
    }
  }, [filters, activeTab]);

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
      {/* 页面标题 + 导出按钮 */}
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 16,
        }}
      >
        <Title level={4} data-testid="governance-page-title" style={{ margin: 0 }}>
          {t('governance.title')}
        </Title>
        <Button
          data-testid="governance-export-csv"
          icon={<DownloadOutlined />}
          onClick={handleExportCsv}
          loading={exporting}
        >
          {t('governance.exportCsv')}
        </Button>
      </div>

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
