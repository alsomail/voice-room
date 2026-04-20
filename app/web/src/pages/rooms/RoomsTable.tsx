/**
 * RoomsTable — 房间列表表格组件（T-20004）
 *
 * 包含：
 *   - 工具栏：Input.Search（keyword）+ Select（status）+ 刷新按钮
 *   - Ant Design Table：列定义 + Popconfirm 关闭操作
 *   - 行点击：onRowClick(roomId)
 */

import { Table, Button, Input, Select, Space, Popconfirm } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { RoomStatusTag } from './RoomStatusTag';
import type { AdminRoomItem } from '../../core/network/apiClient';
import type { RoomsPageFilters } from './useRoomsPage';
import type { ColumnsType } from 'antd/es/table';

export interface RoomsTableProps {
  items: AdminRoomItem[];
  total: number;
  page: number;
  pageSize: number;
  filters: RoomsPageFilters;
  loading: boolean;
  closingId: string | null;
  onPageChange: (page: number, pageSize: number) => void;
  onFiltersChange: (patch: Partial<RoomsPageFilters>) => void;
  onCloseRoom: (roomId: string) => void;
  onRowClick: (roomId: string) => void;
  onRefresh: () => void;
}

export function RoomsTable({
  items,
  total,
  page,
  pageSize,
  filters,
  loading,
  closingId,
  onPageChange,
  onFiltersChange,
  onCloseRoom,
  onRowClick,
  onRefresh,
}: RoomsTableProps) {
  const { t } = useTranslation();

  const statusOptions = [
    { label: t('rooms.statusAll'), value: '' },
    { label: t('rooms.statusActive'), value: 'active' },
    { label: t('rooms.statusClosed'), value: 'closed' },
  ];

  const typeLabel = (type: AdminRoomItem['room_type']) => {
    const map: Record<string, string> = {
      normal: t('rooms.typeNormal'),
      password: t('rooms.typePassword'),
      paid: t('rooms.typePaid'),
    };
    return map[type] ?? type;
  };

  const columns: ColumnsType<AdminRoomItem> = [
    {
      title: t('rooms.colRoomId'),
      dataIndex: 'room_id',
      key: 'room_id',
      width: 180,
      ellipsis: true,
    },
    {
      title: t('rooms.colTitle'),
      dataIndex: 'title',
      key: 'title',
    },
    {
      title: t('rooms.colType'),
      dataIndex: 'room_type',
      key: 'room_type',
      render: (type: AdminRoomItem['room_type']) => typeLabel(type),
    },
    {
      title: t('rooms.colStatus'),
      dataIndex: 'status',
      key: 'status',
      render: (status: AdminRoomItem['status']) => <RoomStatusTag status={status} />,
    },
    {
      title: t('rooms.colMembers'),
      key: 'members',
      render: (_: unknown, record: AdminRoomItem) =>
        `${record.member_count}/${record.max_members}`,
    },
    {
      title: t('rooms.colOwner'),
      dataIndex: 'owner_nickname',
      key: 'owner_nickname',
    },
    {
      title: t('rooms.colCreatedAt'),
      dataIndex: 'created_at',
      key: 'created_at',
      render: (val: string) => new Date(val).toLocaleString(),
    },
    {
      title: t('rooms.colActions'),
      key: 'actions',
      render: (_: unknown, record: AdminRoomItem) => (
        // stopPropagation 防止行 onClick 触发
        <div onClick={(e) => e.stopPropagation()}>
          <Popconfirm
            title={t('rooms.confirmClose')}
            okText={t('rooms.confirmCloseOk')}
            cancelText={t('rooms.confirmCloseCancel')}
            onConfirm={() => onCloseRoom(record.room_id)}
            disabled={record.status === 'closed'}
          >
            <Button
              data-testid={`close-btn-${record.room_id}`}
              size="small"
              danger
              disabled={record.status === 'closed'}
              loading={closingId === record.room_id}
            >
              {t('rooms.closeRoom')}
            </Button>
          </Popconfirm>
        </div>
      ),
    },
  ];

  return (
    <div aria-busy={loading}>
      {/* 工具栏 */}
      <Space style={{ marginBottom: 16, width: '100%', justifyContent: 'space-between' }}>
        <Space>
          <Input.Search
            placeholder={t('rooms.search')}
            value={filters.keyword ?? ''}
            onChange={(e) =>
              onFiltersChange({ keyword: e.target.value || undefined })
            }
            style={{ width: 240 }}
            allowClear
          />
          <div data-testid="status-filter">
            <Select
              value={filters.status ?? ''}
              onChange={(value: string) =>
                onFiltersChange({ status: (value as 'active' | 'closed') || undefined })
              }
              options={statusOptions}
              style={{ width: 120 }}
            />
          </div>
        </Space>
        <Button icon={<ReloadOutlined />} onClick={onRefresh} loading={loading}>
          {t('rooms.refresh')}
        </Button>
      </Space>

      {/* 表格 */}
      <Table<AdminRoomItem>
        data-testid="rooms-table"
        rowKey="room_id"
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
        onRow={(record) => ({
          onClick: () => onRowClick(record.room_id),
          style: { cursor: 'pointer' },
        })}
      />
    </div>
  );
}
