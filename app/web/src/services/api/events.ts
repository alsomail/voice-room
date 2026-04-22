/**
 * 用户事件流 API（T-20013）
 *
 * 封装 GET /admin/users/:id/events 调用，re-export 自 core/network/apiClient。
 * 独立文件方便测试时精准 mock，不影响其他 API。
 */

export type {
  EventItem,
  EventListParams,
  EventListResponse,
} from '../../core/network/apiClient';

export { listUserEvents } from '../../core/network/apiClient';
