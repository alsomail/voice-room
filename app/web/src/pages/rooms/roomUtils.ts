/**
 * roomUtils — 活水房间监控纯函数工具库（T-20011）
 *
 * 职责：
 *   - `ActivityLevel`：房间活跃等级类型
 *   - `ActivityFilter`：活跃度筛选类型
 *   - `getActivityStatus`：计算单个房间活跃状态（注入 now 支持测试）
 *   - `formatDuration`：格式化房间存在时长（注入 now 支持测试）
 *   - `filterByActivity`：按活跃度筛选房间列表
 */

import type { AdminRoomItem } from '../../core/network/apiClient';

/** 房间活跃等级 */
export type ActivityLevel = 'active' | 'quiet' | 'abnormal' | 'normal';

/** 活跃度筛选条件 */
export type ActivityFilter = 'all' | 'active' | 'quiet' | 'abnormal';

/**
 * 计算房间活跃状态
 *
 * 规则优先级（从高到低）：
 *   1. member_count >= 5                        → 'active'
 *   2. member_count === 0 && status === 'active' → 'abnormal'
 *   3. member_count 1-4 && 存在时长 > 1h         → 'quiet'
 *   4. 其余                                     → 'normal'
 *
 * @param room  - 房间数据（只需 member_count / status / created_at 字段）
 * @param now   - 当前时间（默认 new Date()，测试时注入固定时间）
 */
export function getActivityStatus(
  room: Pick<AdminRoomItem, 'member_count' | 'status' | 'created_at'>,
  now: Date = new Date(),
): ActivityLevel {
  const { member_count, status, created_at } = room;

  if (member_count >= 5) return 'active';

  if (member_count === 0 && status === 'active') return 'abnormal';

  if (member_count >= 1 && member_count <= 4) {
    const durationMs = now.getTime() - new Date(created_at).getTime();
    if (durationMs > 60 * 60 * 1000) return 'quiet'; // > 1h
  }

  return 'normal';
}

/**
 * 格式化房间持续时长
 *
 * 格式规则：
 *   - diffMs < 0          → '0m'（未来时间兜底）
 *   - < 1min              → '0m'
 *   - < 1h                → '35m'
 *   - < 24h               → '2h 35m'
 *   - ≥ 24h               → '3d 2h'
 *
 * @param createdAt - 房间创建时间（ISO 字符串）
 * @param now       - 当前时间（默认 new Date()，测试时注入固定时间）
 */
export function formatDuration(createdAt: string, now: Date = new Date()): string {
  const diffMs = now.getTime() - new Date(createdAt).getTime();

  if (diffMs < 0) return '0m';

  const totalMinutes = Math.floor(diffMs / 60_000);
  const days = Math.floor(totalMinutes / 1440);
  const hours = Math.floor((totalMinutes % 1440) / 60);
  const minutes = totalMinutes % 60;

  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

/**
 * 根据活跃度筛选房间列表（纯前端过滤，不发 API）
 *
 * @param items  - 原始房间列表
 * @param filter - 活跃度筛选条件（'all' 返回全量）
 * @param now    - 当前时间（默认 new Date()，测试时注入固定时间）
 */
export function filterByActivity(
  items: AdminRoomItem[],
  filter: ActivityFilter,
  now: Date = new Date(),
): AdminRoomItem[] {
  if (filter === 'all') return items;
  return items.filter((room) => getActivityStatus(room, now) === filter);
}
