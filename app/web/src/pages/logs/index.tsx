/**
 * LogsPage — 操作日志页面（T-20009）
 *
 * 入口组件：集成 useLogsPage Hook + LogSearchForm 组件 + LogsTable 组件
 */

import { Alert, Typography } from 'antd';
import { useTranslation } from 'react-i18next';
import { useLogsPage } from './useLogsPage';
import { LogSearchForm } from './LogSearchForm';
import { LogsTable } from './LogsTable';

export function LogsPage() {
  const { t } = useTranslation();
  const {
    items,
    total,
    loading,
    error,
    page,
    pageSize,
    filters,
    setPage,
    setFilters,
    refresh,
  } = useLogsPage();

  const handleReset = () => {
    setFilters({});
  };

  return (
    <div data-testid="logs-page" style={{ padding: '24px' }}>
      <Typography.Title level={4} style={{ marginBottom: 16 }}>
        {t('logs.title')}
      </Typography.Title>

      {/* 错误提示 */}
      {error && (
        <Alert
          data-testid="logs-error"
          type="error"
          description={error.message}
          showIcon
          style={{ marginBottom: 16 }}
        />
      )}

      {/* 搜索表单 */}
      <LogSearchForm
        initialFilters={filters}
        onSearch={setFilters}
        onReset={handleReset}
      />

      {/* 日志列表 */}
      <LogsTable
        items={items}
        total={total}
        page={page}
        pageSize={pageSize}
        loading={loading}
        onPageChange={setPage}
        onRefresh={refresh}
      />
    </div>
  );
}
