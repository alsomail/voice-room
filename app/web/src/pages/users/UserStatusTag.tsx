/**
 * UserStatusTag — 用户状态标签（T-20006）
 *
 * normal → 绿色 Tag（success）
 * banned → 红色 Tag（error）
 */

import { Tag } from 'antd';
import { useTranslation } from 'react-i18next';

interface UserStatusTagProps {
  status: 'normal' | 'banned';
}

export function UserStatusTag({ status }: UserStatusTagProps) {
  const { t } = useTranslation();

  if (status === 'normal') {
    return (
      <Tag color="success" data-testid="status-tag-normal">
        {t('users.statusNormal')}
      </Tag>
    );
  }

  return (
    <Tag color="error" data-testid="status-tag-banned">
      {t('users.statusBanned')}
    </Tag>
  );
}
