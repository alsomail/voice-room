/**
 * RoomStatusTag — 房间状态标签（T-20004）
 *
 * active → 绿色 Tag
 * closed → 默认灰色 Tag
 */

import { Tag } from 'antd';
import { useTranslation } from 'react-i18next';

interface RoomStatusTagProps {
  status: 'active' | 'closed';
}

export function RoomStatusTag({ status }: RoomStatusTagProps) {
  const { t } = useTranslation();
  if (status === 'active') {
    return (
      <Tag color="success" data-testid="status-tag-active">
        {t('rooms.statusActive')}
      </Tag>
    );
  }
  return (
    <Tag data-testid="status-tag-closed">
      {t('rooms.statusClosed')}
    </Tag>
  );
}
