/**
 * 埋点事件字典（T-20013）
 *
 * 前端维护事件分类和列表，避免频繁查询后端。
 * 对应 T-10015 analytics 模块中的事件规范。
 */

export const EVENT_CATEGORIES = {
  auth: [
    'login_request',
    'login_success',
    'login_fail',
    'logout',
  ],
  gift: [
    'gift_panel_open',
    'gift_select',
    'gift_send_click',
    'gift_send_success',
    'gift_send_fail',
  ],
  wallet: [
    'wallet_view',
    'balance_update',
    'insufficient_balance',
    'recharge_start',
    'recharge_success',
    'recharge_fail',
  ],
  room: [
    'room_enter',
    'room_leave',
    'mic_take',
    'mic_leave',
    'room_create',
    'room_close',
  ],
  user: [
    'profile_view',
    'profile_update',
    'follow',
    'unfollow',
  ],
} as const;

/** 所有事件名列表（去重，按字母排序） */
export const ANALYTICS_EVENTS: string[] = Array.from(
  new Set(Object.values(EVENT_CATEGORIES).flat()),
).sort();

/** 事件类别颜色映射 */
export const EVENT_CATEGORY_COLOR: Record<keyof typeof EVENT_CATEGORIES, string> = {
  auth: '#1677ff',
  gift: '#ff69b4',
  wallet: '#52c41a',
  room: '#fa8c16',
  user: '#722ed1',
};

/** 根据事件名获取所属类别颜色 */
export function getEventColor(eventName: string): string {
  for (const [category, events] of Object.entries(EVENT_CATEGORIES)) {
    if ((events as readonly string[]).includes(eventName)) {
      return EVENT_CATEGORY_COLOR[category as keyof typeof EVENT_CATEGORIES];
    }
  }
  return '#666'; // 默认灰色（未知事件）
}
