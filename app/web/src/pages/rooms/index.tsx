/**
 * RoomsPage — 房间管理页面（T-20004 + T-20005 + T-20011）
 *
 * 入口组件：集成 useRoomsPage Hook + RoomsTable 组件 + RoomDetailModal 组件
 */

import { Alert, Typography } from 'antd';
import { useTranslation } from 'react-i18next';
import { useRoomsPage } from './useRoomsPage';
import { RoomsTable } from './RoomsTable';
import { RoomDetailModal } from './RoomDetailModal';

export function RoomsPage() {
  const { t } = useTranslation();
  const {
    filteredItems,
    total,
    loading,
    error,
    page,
    pageSize,
    filters,
    activityFilter,
    closingId,
    selectedRoomId,
    setPage,
    setFilters,
    setActivityFilter,
    closeRoom,
    refresh,
    setSelectedRoomId,
  } = useRoomsPage();

  return (
    <div data-testid="rooms-page" style={{ padding: '24px' }}>
      <Typography.Title level={4} style={{ marginBottom: 16 }}>
        {t('rooms.title')}
      </Typography.Title>

      {/* 错误提示（fetch 失败 或 close 失败） */}
      {error && (
        <Alert
          data-testid="rooms-error"
          type="error"
          description={error.message}
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      <RoomsTable
        items={filteredItems}
        total={total}
        page={page}
        pageSize={pageSize}
        filters={filters}
        loading={loading}
        closingId={closingId}
        activityFilter={activityFilter}
        onPageChange={setPage}
        onFiltersChange={setFilters}
        onActivityFilterChange={setActivityFilter}
        onCloseRoom={(roomId) => void closeRoom(roomId).catch(() => {})}
        onRowClick={setSelectedRoomId}
        onRefresh={refresh}
      />

      {/* T-20005：房间详情弹窗 */}
      <RoomDetailModal
        selectedRoomId={selectedRoomId}
        onClose={() => setSelectedRoomId(null)}
        onCloseRoom={closeRoom}
        closingId={closingId}
      />

      {/* T-20005 占位：供集成测试验证 selectedRoomId（向后兼容 I06 旧测试） */}
      {selectedRoomId && (
        <span data-testid="selected-room-id" style={{ display: 'none' }}>
          {selectedRoomId}
        </span>
      )}
    </div>
  );
}
