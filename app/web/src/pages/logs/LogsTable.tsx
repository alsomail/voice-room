/**
 * LogsTable — 操作日志列表表格组件（T-20009）
 *
 * 包含：
 *   - 工具栏：刷新按钮
 *   - Ant Design Table：日志 ID / 操作人 ID / 操作类型(Tag) / 目标类型 / 目标 ID /
 *                       IP 地址 / 详情 / 操作时间
 *   - 只读，无操作列
 */

import { Table, Button, Space, Tag } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import type { AdminLogItem } from '../../core/network/apiClient';
import type { ColumnsType } from 'antd/es/table';

/** action → Tag 颜色映射 */
const ACTION_COLOR: Record<string, string> = {
  ban_user:   'red',
  unban_user: 'green',
  close_room: 'orange',
};

export interface LogsTableProps {
  items: AdminLogItem[];
  total: number;
  page: number;
  pageSize: number;
  loading: boolean;
  onPageChange: (page: number, pageSize: number) => void;
  onRefresh: () => void;
}

export function LogsTable({
  items,
  total,
  page,
  pageSize,
  loading,
  onPageChange,
  onRefresh,
}: LogsTableProps) {
  const { t } = useTranslation();

  const columns: ColumnsType<AdminLogItem> = [
    {
      title: t('logs.colId'),
      dataIndex: 'id',
      key: 'id',
      width: 220,
      ellipsis: true,
    },
    {
      title: t('logs.colAdminId'),
      dataIndex: 'admin_id',
      key: 'admin_id',
      width: 220,
      ellipsis: true,
    },
    {
      title: t('logs.colAction'),
      dataIndex: 'action',
      key: 'action',
      render: (action: string) => (
        <Tag
          color={ACTION_COLOR[action] ?? 'default'}
          data-testid={`action-tag-${action}`}
        >
          {action}
        </Tag>
      ),
    },
    {
      title: t('logs.colTargetType'),
      dataIndex: 'target_type',
      key: 'target_type',
      render: (val?: string) => val ?? '-',
    },
    {
      title: t('logs.colTargetId'),
      dataIndex: 'target_id',
      key: 'target_id',
      width: 220,
      ellipsis: true,
      render: (val?: string) => val ?? '-',
    },
    {
      title: t('logs.colIpAddress'),
      dataIndex: 'ip_address',
      key: 'ip_address',
      render: (val?: string) => val ?? '-',
    },
    {
      title: t('logs.colDetail'),
      dataIndex: 'detail',
      key: 'detail',
      width: 200,
      ellipsis: { showTitle: true },
      render: (val?: Record<string, unknown>) =>
        val != null ? JSON.stringify(val) : '-',
    },
    {
      title: t('logs.colCreatedAt'),
      dataIndex: 'created_at',
      key: 'created_at',
      width: 180,
      render: (val: string) => new Date(val).toLocaleString(),
    },
  ];

  return (
    <div aria-busy={loading}>
      {/* 工具栏 */}
      <Space style={{ marginBottom: 16, width: '100%', justifyContent: 'flex-end' }}>
        <Button
          icon={<ReloadOutlined />}
          onClick={onRefresh}
          loading={loading}
          data-testid="refresh-btn"
        >
          {t('logs.refresh')}
        </Button>
      </Space>

      {/* 表格 */}
      <Table<AdminLogItem>
        data-testid="logs-table"
        rowKey="id"
        columns={columns}
        dataSource={items}
        loading={loading}
        pagination={{
          current: page,
          pageSize,
          total,
          showSizeChanger: true,
          onChange: onPageChange,
        }}
      />
    </div>
  );
}
