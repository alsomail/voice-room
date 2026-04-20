/**
 * LogSearchForm — 操作日志搜索筛选表单（T-20009）
 *
 * 包含：操作人ID（Input）/ 操作类型（Select）/ 时间范围（DatePicker.RangePicker）
 *       搜索 / 重置 按钮
 * layout="inline"，按搜索按钮触发提交（非即时/debounce）
 */

import { Form, Input, Select, Button, Space, DatePicker } from 'antd';
import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import type { LogsPageFilters } from './useLogsPage';
import type { Dayjs } from 'dayjs';

export interface LogSearchFormProps {
  /** 初始值（从 URL query 或 hook state 传入，用于 URL 状态恢复） */
  initialFilters?: LogsPageFilters;
  onSearch: (filters: LogsPageFilters) => void;
  onReset: () => void;
}

export function LogSearchForm({ initialFilters, onSearch, onReset }: LogSearchFormProps) {
  const { t } = useTranslation();
  const [form] = Form.useForm<{
    adminId?: string;
    action?: string;
    dateRange?: [Dayjs, Dayjs] | undefined;
  }>();

  // 当初始值（来自 URL）变化时同步表单字段（URL 状态恢复）
  useEffect(() => {
    form.setFieldsValue({
      adminId: initialFilters?.adminId ?? '',
      action:  initialFilters?.action,
      // dateRange 恢复较复杂，暂不从 URL 恢复，运营可重新选择
      dateRange: undefined,
    });
  }, [initialFilters, form]);

  const handleSearch = () => {
    const values = form.getFieldsValue();
    const dateRange = values.dateRange;
    onSearch({
      adminId:   values.adminId   || undefined,
      action:    values.action    || undefined,
      startDate: dateRange?.[0]?.toISOString() ?? undefined,
      endDate:   dateRange?.[1]?.toISOString() ?? undefined,
    });
  };

  const handleReset = () => {
    form.setFieldsValue({ adminId: '', action: undefined, dateRange: undefined });
    onReset();
  };

  const actionOptions = [
    { label: t('logs.actionAll'),       value: '' },
    { label: t('logs.actionBanUser'),   value: 'ban_user' },
    { label: t('logs.actionUnbanUser'), value: 'unban_user' },
    { label: t('logs.actionCloseRoom'), value: 'close_room' },
  ];

  return (
    <Form form={form} layout="inline" style={{ marginBottom: 16 }}>
      <Form.Item name="adminId">
        <Input
          placeholder={t('logs.adminIdPlaceholder')}
          style={{ width: 240 }}
          allowClear
        />
      </Form.Item>

      {/* 操作类型筛选：外层 div 提供 testid 供测试定位 combobox */}
      <div data-testid="action-select">
        <Form.Item name="action">
          <Select
            placeholder={t('logs.actionAll')}
            options={actionOptions}
            allowClear
            style={{ width: 140 }}
          />
        </Form.Item>
      </div>

      <Form.Item name="dateRange">
        <DatePicker.RangePicker
          placeholder={[t('logs.startDatePlaceholder'), t('logs.endDatePlaceholder')]}
          style={{ width: 260 }}
        />
      </Form.Item>

      <Form.Item>
        <Space>
          <Button type="primary" onClick={handleSearch}>
            {t('logs.search')}
          </Button>
          <Button onClick={handleReset}>
            {t('logs.reset')}
          </Button>
        </Space>
      </Form.Item>
    </Form>
  );
}
