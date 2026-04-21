/**
 * RoomsTable — 房间列表表格组件（T-20004 + T-20011）
 *
 * 包含：
 *   - 工具栏：Input.Search（keyword）+ Select（status）+ Select（activityFilter）+ 刷新按钮
 *   - Ant Design Table：列定义 + Popconfirm 关闭操作
 *   - 行点击：onRowClick(roomId)
 *   - T-20011 新增：活跃状态列 / 持续时长列 / 异常行高亮
 */

import { useMemo } from 'react';
import { Table, Button, Input, Select, Space, Popconfirm } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { RoomStatusTag } from './RoomStatusTag';
import { RoomActivityTag } from './RoomActivityTag';
import { getActivityStatus, formatDuration, type ActivityFilter } from './roomUtils';
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
  /** T-20011: 活跃度筛选（默认 'all'，不破坏现有调用方） */
  activityFilter?: ActivityFilter;
  onPageChange: (page: number, pageSize: number) => void;
  onFiltersChange: (patch: Partial<RoomsPageFilters>) => void;
  onCloseRoom: (roomId: string) => void;
  onRowClick: (roomId: string) => void;
  onRefresh: () => void;
  /** T-20011: 活跃度筛选变更回调（默认 noop，不破坏现有调用方） */
  onActivityFilterChange?: (filter: ActivityFilter) => void;
}

// eslint-disable-next-line @typescript-eslint/no-empty-function
const noop = () => {};

export function RoomsTable({
  items,
  total,
  page,
  pageSize,
  filters,
  loading,
  closingId,
  activityFilter = 'all',
  onPageChange,
  onFiltersChange,
  onCloseRoom,
  onRowClick,
  onRefresh,
  onActivityFilterChange = noop,
}: RoomsTableProps) {
  const { t } = useTranslation();

  const statusOptions = [
    { label: t('rooms.statusAll'), value: '' },
    { label: t('rooms.statusActive'), value: 'active' },
    { label: t('rooms.statusClosed'), value: 'closed' },
  ];

  // T-20011: 活跃度筛选选项（useMemo 避免重复创建，与 UsersTable 保持一致）
  const activityOptions = useMemo(() => [
    { label: t('rooms.activityAll'), value: 'all' },
    { label: t('rooms.activityLevelActive'), value: 'active' },
    { label: t('rooms.activityLevelAbnormal'), value: 'abnormal' },
    { label: t('rooms.activityLevelQuiet'), value: 'quiet' },
    { label: t('rooms.activityLevelNormal'), value: 'normal' },
  ], [t]);

  // useMemo 缓存 columns 避免重复创建（与 UsersTable 保持一致）
  const columns = useMemo<ColumnsType<AdminRoomItem>>(() => {
    const typeLabel = (type: AdminRoomItem['room_type']): string => {
      const map: Record<string, string> = {
        normal: t('rooms.typeNormal'),
        password: t('rooms.typePassword'),
        paid: t('rooms.typePaid'),
      };
      return map[type] ?? type;
    };

    return [
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
      // T-20011: 活跃状态列
      {
        title: t('rooms.columnActivityStatus'),
        key: 'activity_status',
        render: (_: unknown, record: AdminRoomItem) => (
          <RoomActivityTag
            level={getActivityStatus(record)}
            roomId={record.room_id}
          />
        ),
      },
      // T-20011: 持续时长列
      {
        title: t('rooms.columnDuration'),
        key: 'duration',
        render: (_: unknown, record: AdminRoomItem) => (
          <span data-testid={`room-duration-${record.room_id}`}>
            {formatDuration(record.created_at)}
          </span>
        ),
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
  }, [t, closingId, onCloseRoom]);

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
          {/* T-20011: 活跃度筛选 */}
          <div data-testid="activity-filter">
            <Select
              value={activityFilter}
              onChange={(value: string) =>
                onActivityFilterChange(value as ActivityFilter)
              }
              options={activityOptions}
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
          style: {
            cursor: 'pointer',
            // T-20011: 异常房间行高亮背景
            ...(getActivityStatus(record) === 'abnormal'
              ? { background: 'rgba(231, 76, 60, 0.1)' }
              : {}),
          },
        })}
      />
    </div>
  );
}
