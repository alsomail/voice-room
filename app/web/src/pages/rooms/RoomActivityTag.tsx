/**
 * RoomActivityTag — 房间活跃状态标签（T-20011）
 *
 * level='active'   → success（绿）
 * level='quiet'    → warning（黄）
 * level='abnormal' → error（红）
 * level='normal'   → processing（蓝）
 */

import { useMemo } from 'react';
import { Tag } from 'antd';
import { useTranslation } from 'react-i18next';
import type { ActivityLevel } from './roomUtils';

interface RoomActivityTagProps {
  level: ActivityLevel;
  roomId: string;
}

const colorMap: Record<ActivityLevel, string> = {
  active: 'success',
  quiet: 'warning',
  abnormal: 'error',
  normal: 'processing',
};

export function RoomActivityTag({ level, roomId }: RoomActivityTagProps) {
  const { t } = useTranslation();

  // useMemo 避免每次渲染重建 labelMap（colorMap 已在组件外定义为常量）
  const labelMap = useMemo<Record<ActivityLevel, string>>(() => ({
    active: t('rooms.activityLevelActive'),
    quiet: t('rooms.activityLevelQuiet'),
    abnormal: t('rooms.activityLevelAbnormal'),
    normal: t('rooms.activityLevelNormal'),
  }), [t]);

  return (
    <Tag color={colorMap[level]} data-testid={`room-activity-tag-${roomId}`}>
      {labelMap[level]}
    </Tag>
  );
}
