/**
 * 治理日志 API（T-20014）
 *
 * 封装 GET /admin/governance/kicks 和 /mutes 调用，re-export 自 core/network/apiClient。
 * 独立文件方便测试时精准 mock，不影响其他 API。
 */

export type {
  KickLogItem,
  MuteLogItem,
  GovernanceListParams,
  MuteListParams,
  GovernanceListResponse,
} from '../../core/network/apiClient';

export { listKicks, listMutes, exportGovernanceLogsCsv } from '../../core/network/apiClient';
