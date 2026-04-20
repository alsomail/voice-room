/**
 * UserSearchForm — 用户搜索筛选表单（T-20006）
 *
 * 包含：手机号 / 用户ID / 昵称 / 状态 下拉 / 搜索 / 重置 按钮
 * layout="inline"，按搜索按钮触发提交（非即时/debounce）
 */

import { Form, Input, Select, Button, Space } from 'antd';
import { useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import type { UsersPageFilters } from './useUsersPage';

export interface UserSearchFormProps {
  /** 初始值（从 URL query 或 hook state 传入，用于 URL 状态恢复） */
  initialFilters?: UsersPageFilters;
  onSearch: (filters: UsersPageFilters) => void;
  onReset: () => void;
}

export function UserSearchForm({ initialFilters, onSearch, onReset }: UserSearchFormProps) {
  const { t } = useTranslation();
  const [form] = Form.useForm<{
    phone?: string;
    userId?: string;
    nickname?: string;
    status?: 'normal' | 'banned';
  }>();

  // 当初始值（来自 URL）变化时同步表单字段（URL 状态恢复）
  useEffect(() => {
    form.setFieldsValue({
      phone:    initialFilters?.phone    ?? '',
      userId:   initialFilters?.userId   ?? '',
      nickname: initialFilters?.nickname ?? '',
      status:   initialFilters?.status,
    });
  }, [initialFilters, form]);

  const handleSearch = () => {
    const values = form.getFieldsValue();
    onSearch({
      phone:    values.phone    || undefined,
      userId:   values.userId   || undefined,
      nickname: values.nickname || undefined,
      status:   values.status   || undefined,
    });
  };

  const handleReset = () => {
    form.setFieldsValue({ phone: '', userId: '', nickname: '', status: undefined });
    onReset();
  };

  const statusOptions = [
    { label: t('users.statusAll'),    value: '' },
    { label: t('users.statusNormal'), value: 'normal' },
    { label: t('users.statusBanned'), value: 'banned' },
  ];

  return (
    <Form form={form} layout="inline" style={{ marginBottom: 16 }}>
      <Form.Item name="phone">
        <Input
          placeholder={t('users.phonePlaceholder')}
          style={{ width: 160 }}
          allowClear
        />
      </Form.Item>

      <Form.Item name="userId">
        <Input
          placeholder={t('users.userIdPlaceholder')}
          style={{ width: 200 }}
          allowClear
        />
      </Form.Item>

      <Form.Item name="nickname">
        <Input
          placeholder={t('users.nicknamePlaceholder')}
          style={{ width: 160 }}
          allowClear
        />
      </Form.Item>

      {/* 状态筛选：外层 div 提供 testid 供测试定位 combobox */}
      <div data-testid="status-select">
        <Form.Item name="status">
          <Select
            placeholder={t('users.statusAll')}
            options={statusOptions}
            allowClear
            style={{ width: 120 }}
          />
        </Form.Item>
      </div>

      <Form.Item>
        <Space>
          <Button type="primary" onClick={handleSearch}>
            {t('users.search')}
          </Button>
          <Button onClick={handleReset}>
            {t('users.reset')}
          </Button>
        </Space>
      </Form.Item>
    </Form>
  );
}
