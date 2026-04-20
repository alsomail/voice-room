/**
 * RoomDetailModal — 房间详情弹窗（T-20005）
 *
 * Props：
 *   selectedRoomId  — 当前选中的房间 ID（null 时 Modal 关闭）
 *   onClose         — Modal 关闭回调（设置 selectedRoomId=null）
 *   onCloseRoom     — 强制关闭房间操作（失败时 re-throw，Modal 不关闭）
 *   closingId       — 当前正在关闭中的房间 ID（细粒度 loading）
 *
 * 内部结构：
 *   - 调用 useRoomDetail(selectedRoomId) 拉取房间详情
 *   - loading → <Spin data-testid="detail-loading" />
 *   - error   → <Alert data-testid="detail-error" />
 *   - detail  → <div data-testid="detail-basic-info"> 含基本字段 </div>
 *   - 始终渲染成员列表占位（data-testid="members-placeholder"）
 *   - 始终渲染聊天记录占位（data-testid="chat-placeholder"）
 *   - [强制关闭] 按钮：status=active 才可点，点击后 Modal.confirm 二次确认
 */

import { Modal, Button, Spin, Alert, Descriptions } from 'antd';
import { useTranslation } from 'react-i18next';
import { useRoomDetail } from './useRoomDetail';
import { RoomStatusTag } from './RoomStatusTag';

interface RoomDetailModalProps {
  selectedRoomId: string | null;
  onClose: () => void;
  onCloseRoom: (roomId: string) => Promise<void>;
  closingId: string | null;
}

/** 将 room_type 首字母大写，用于拼接 i18n key */
function capitalizeFirst(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

export function RoomDetailModal({
  selectedRoomId,
  onClose,
  onCloseRoom,
  closingId,
}: RoomDetailModalProps) {
  const { t } = useTranslation();
  const { detail, loading, error } = useRoomDetail(selectedRoomId);

  const handleForceClose = () => {
    if (!selectedRoomId) return;

    Modal.confirm({
      title: t('rooms.detail.forceCloseConfirmTitle'),
      content: t('rooms.detail.forceCloseConfirmContent'),
      onOk: async () => {
        // 若 onCloseRoom re-throw，Modal.confirm 会保持 dialog 打开
        await onCloseRoom(selectedRoomId);
        // 成功后关闭 Modal
        onClose();
      },
    });
  };

  const isForceCloseDisabled = detail?.status !== 'active' || closingId !== null;
  const isForceCloseLoading = closingId === selectedRoomId;

  return (
    <Modal
      open={selectedRoomId !== null}
      title={t('rooms.detail.title')}
      onCancel={onClose}
      destroyOnHidden={true}
      footer={
        <Button
          data-testid="close-room-btn"
          danger
          disabled={isForceCloseDisabled}
          loading={isForceCloseLoading}
          onClick={handleForceClose}
        >
          {t('rooms.detail.forceClose')}
        </Button>
      }
    >
      {/* 加载态 */}
      {loading && (
        <div data-testid="detail-loading" style={{ textAlign: 'center', padding: '24px' }}>
          <Spin />
        </div>
      )}

      {/* 错误态 */}
      {error && !loading && (
        <Alert
          data-testid="detail-error"
          type="error"
          message={t('rooms.detail.loadError')}
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      {/* 基本信息 */}
      {detail && !loading && (
        <div data-testid="detail-basic-info">
          <Descriptions column={1} size="small" style={{ marginBottom: 16 }}>
            <Descriptions.Item label={t('rooms.detail.title')}>
              {detail.title}
            </Descriptions.Item>
            <Descriptions.Item label={t('rooms.detail.roomType')}>
              {t(`rooms.detail.roomType${capitalizeFirst(detail.room_type)}`)}
            </Descriptions.Item>
            <Descriptions.Item label={t('rooms.colStatus')}>
              <RoomStatusTag status={detail.status} />
            </Descriptions.Item>
            <Descriptions.Item label={t('rooms.detail.owner')}>
              {detail.owner.nickname}
            </Descriptions.Item>
            <Descriptions.Item label={t('rooms.detail.memberCount')}>
              {detail.member_count} / {detail.max_members}
            </Descriptions.Item>
            <Descriptions.Item label={t('rooms.detail.createdAt')}>
              {detail.created_at}
            </Descriptions.Item>
          </Descriptions>
        </div>
      )}

      {/* 成员列表占位 */}
      <div data-testid="members-placeholder" style={{ marginBottom: 12 }}>
        <strong>{t('rooms.detail.membersSection')}</strong>
        <p style={{ color: '#999', marginTop: 4 }}>{t('rooms.detail.membersPlaceholder')}</p>
      </div>

      {/* 历史聊天记录占位 */}
      <div data-testid="chat-placeholder">
        <strong>{t('rooms.detail.chatSection')}</strong>
        <p style={{ color: '#999', marginTop: 4 }}>{t('rooms.detail.chatPlaceholder')}</p>
      </div>
    </Modal>
  );
}
