/**
 * UsersTable — 用户列表表格组件（T-20006）
 *
 * 包含：
 *   - 工具栏：刷新按钮
 *   - Ant Design Table：用户 ID / 手机号 / 昵称 / 头像 / 金币余额 / VIP / 状态 / 注册时间 / 操作
 *   - 操作列："查看详情"（disabled，T-20007 占位）
 */

import { useMemo } from 'react';
import { Table, Button, Space, Avatar } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { UserStatusTag } from './UserStatusTag';
import type { AdminUserItem } from '../../core/network/apiClient';
import type { ColumnsType } from 'antd/es/table';

export interface UsersTableProps {
  items: AdminUserItem[];
  total: number;
  page: number;
  pageSize: number;
  loading: boolean;
  onPageChange: (page: number, pageSize: number) => void;
  onRefresh: () => void;
  onViewDetail?: (userId: string) => void;
}

export function UsersTable({
  items,
  total,
  page,
  pageSize,
  loading,
  onPageChange,
  onRefresh,
  onViewDetail,
}: UsersTableProps) {
  const { t } = useTranslation();

  const columns = useMemo<ColumnsType<AdminUserItem>>(() => [
    {
      title: t('users.colId'),
      dataIndex: 'id',
      key: 'id',
      width: 220,
      ellipsis: true,
    },
    {
      title: t('users.colPhone'),
      dataIndex: 'phone',
      key: 'phone',
      width: 140,
    },
    {
      title: t('users.colNickname'),
      dataIndex: 'nickname',
      key: 'nickname',
    },
    {
      title: t('users.colAvatar'),
      dataIndex: 'avatar',
      key: 'avatar',
      render: (avatar: string | undefined) => (
        <Avatar src={avatar} size={32} />
      ),
    },
    {
      title: t('users.colCoinBalance'),
      dataIndex: 'coin_balance',
      key: 'coin_balance',
      align: 'right',
    },
    {
      title: t('users.colVipLevel'),
      dataIndex: 'vip_level',
      key: 'vip_level',
    },
    {
      title: t('users.colStatus'),
      dataIndex: 'status',
      key: 'status',
      render: (status: AdminUserItem['status']) => <UserStatusTag status={status} />,
    },
    {
      title: t('users.colCreatedAt'),
      dataIndex: 'created_at',
      key: 'created_at',
      render: (val: string) => new Date(val).toLocaleString(),
    },
    {
      title: t('users.colActions'),
      key: 'actions',
      render: (_: unknown, record: AdminUserItem) => (
        <Button
          size="small"
          disabled={!onViewDetail}
          data-testid="view-detail-btn"
          onClick={() => onViewDetail?.(record.id)}
        >
          {t('users.viewDetail')}
        </Button>
      ),
    },
  ], [t, onViewDetail]);

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
          {t('users.refresh')}
        </Button>
      </Space>

      {/* 表格 */}
      <Table<AdminUserItem>
        data-testid="users-table"
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
