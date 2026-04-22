/**
 * FiltersBar — 治理日志共用筛选条（T-20014）
 *
 * 包含：
 *   - 房间 ID 输入（data-testid="governance-filter-room"）
 *   - 目标用户 ID 输入（data-testid="governance-filter-target-user"）
 *   - 操作者 ID 输入
 *   - 时间范围（DatePicker.RangePicker）
 *   - mute 专属：禁言类型下拉（data-testid="governance-filter-mute-type"）
 *   - 重置按钮（data-testid="governance-filter-reset"）
 *   - 搜索按钮（data-testid="governance-filter-search"）
 *
 * 触发时机：按 Enter 或点击搜索按钮提交筛选；重置按钮清除所有条件并立即触发查询。
 */

import { Form, Input, Select, Button, Space, DatePicker } from 'antd';
import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import type { Dayjs } from 'dayjs';

export interface GovernanceFilters {
  room_id?: string;
  target_user_id?: string;
  operator_user_id?: string;
  from?: string;
  to?: string;
  mute_type?: 'mic' | 'chat';
}

export interface FiltersBarProps {
  /** 当前激活的 tab */
  activeTab: 'kicks' | 'mutes';
  /** 当前筛选值 */
  filters: GovernanceFilters;
  /** 筛选值变化 + 触发查询 */
  onSearch: (filters: GovernanceFilters) => void;
  /** 重置所有筛选 */
  onReset: () => void;
}

export function FiltersBar({ activeTab, filters, onSearch, onReset }: FiltersBarProps) {
  const { t } = useTranslation();

  const [form] = Form.useForm<{
    room_id?: string;
    target_user_id?: string;
    operator_user_id?: string;
    dateRange?: [Dayjs, Dayjs] | undefined;
    mute_type?: 'mic' | 'chat';
  }>();

  // 外部 filters 变化时同步表单（Tab 切换后重置）
  useEffect(() => {
    form.setFieldsValue({
      room_id: filters.room_id ?? '',
      target_user_id: filters.target_user_id ?? '',
      operator_user_id: filters.operator_user_id ?? '',
      mute_type: filters.mute_type,
      dateRange: undefined, // 时间范围不从 state 恢复，操作者重选
    });
  }, [filters, form]);

  const handleSearch = () => {
    const values = form.getFieldsValue();
    const dateRange = values.dateRange;
    onSearch({
      room_id: values.room_id || undefined,
      target_user_id: values.target_user_id || undefined,
      operator_user_id: values.operator_user_id || undefined,
      from: dateRange?.[0]?.toISOString() ?? undefined,
      to: dateRange?.[1]?.toISOString() ?? undefined,
      mute_type: values.mute_type || undefined,
    });
  };

  const handleReset = () => {
    form.setFieldsValue({
      room_id: '',
      target_user_id: '',
      operator_user_id: '',
      dateRange: undefined,
      mute_type: undefined,
    });
    onReset();
  };

  const muteTypeOptions = [
    { label: t('governance.filterMuteTypeAll'), value: '' },
    { label: t('governance.filterMuteTypeMic'), value: 'mic' },
    { label: t('governance.filterMuteTypeChat'), value: 'chat' },
  ];

  return (
    <Form form={form} layout="inline" style={{ marginBottom: 16 }}>
      {/* 房间 ID */}
      <Form.Item name="room_id">
        <Input
          data-testid="governance-filter-room"
          placeholder={t('governance.filterRoomPlaceholder')}
          style={{ width: 220 }}
          allowClear
          onPressEnter={handleSearch}
        />
      </Form.Item>

      {/* 目标用户 ID */}
      <Form.Item name="target_user_id">
        <Input
          data-testid="governance-filter-target-user"
          placeholder={t('governance.filterTargetUserPlaceholder')}
          style={{ width: 220 }}
          allowClear
          onPressEnter={handleSearch}
        />
      </Form.Item>

      {/* 操作者 ID */}
      <Form.Item name="operator_user_id">
        <Input
          data-testid="governance-filter-operator"
          placeholder={t('governance.filterOperatorPlaceholder')}
          style={{ width: 200 }}
          allowClear
          onPressEnter={handleSearch}
        />
      </Form.Item>

      {/* 时间范围 */}
      <Form.Item name="dateRange">
        <DatePicker.RangePicker
          data-testid="governance-filter-date-range"
          placeholder={[t('governance.filterStartDate'), t('governance.filterEndDate')]}
          style={{ width: 280 }}
        />
      </Form.Item>

      {/* mute 专属：禁言类型 */}
      {activeTab === 'mutes' && (
        <div data-testid="governance-filter-mute-type">
          <Form.Item name="mute_type">
            <Select
              placeholder={t('governance.filterMuteTypeAll')}
              options={muteTypeOptions}
              allowClear
              style={{ width: 120 }}
            />
          </Form.Item>
        </div>
      )}

      <Form.Item>
        <Space>
          <Button
            type="primary"
            data-testid="governance-filter-search"
            onClick={handleSearch}
          >
            {t('governance.search')}
          </Button>
          <Button
            data-testid="governance-filter-reset"
            onClick={handleReset}
          >
            {t('governance.reset')}
          </Button>
        </Space>
      </Form.Item>
    </Form>
  );
}
