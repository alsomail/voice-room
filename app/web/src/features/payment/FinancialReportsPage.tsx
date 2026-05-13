/**
 * FinancialReportsPage — 财务报表页 (T-20033)
 *
 * 路由：/payments/reports（RoleGuard: super_admin/operator/finance）
 */

import { useState, useRef, useEffect } from 'react';
import { Table, Space, Select, DatePicker, Button, Statistic, Row, Col, Card, Typography } from 'antd';
import type { TableColumnsType } from 'antd';
import { SearchOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useFinancialReportsPage } from './useFinancialReportsPage';
import type { ReportSeriesItem } from '../../api/payment';
import dayjs from 'dayjs';

const { Title } = Typography;

export function FinancialReportsPage() {
  const { t } = useTranslation();
  const { report, loading, error, fetch } = useFinancialReportsPage();
  const [granularity, setGranularity] = useState<'day' | 'month'>('day');
  const [dateRange, setDateRange] = useState<[dayjs.Dayjs, dayjs.Dayjs]>([
    dayjs().subtract(30, 'day'),
    dayjs(),
  ]);
  const hasQueried = useRef(false);

  const handleSearch = () => {
    hasQueried.current = true;
    fetch(
      {
        granularity,
        from: dateRange[0].format('YYYY-MM-DD'),
        to: dateRange[1].format('YYYY-MM-DD'),
      },
    );
  };

  // Auto-search on first render
  useEffect(() => {
    handleSearch();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const columns: TableColumnsType<ReportSeriesItem> = [
    {
      title: t('payment.reports.colDate'),
      dataIndex: 'date',
      key: 'date',
      width: 120,
    },
    {
      title: t('payment.reports.colGmv'),
      dataIndex: 'gmv_usd',
      key: 'gmv',
      width: 140,
      align: 'right',
      render: (v: string) => `$${v}`,
    },
    {
      title: t('payment.reports.colOrderCount'),
      dataIndex: 'order_count',
      key: 'orders',
      width: 100,
      align: 'right',
    },
    {
      title: t('payment.reports.colRefundCount'),
      dataIndex: 'refund_count',
      key: 'refunds',
      width: 100,
      align: 'right',
    },
    {
      title: t('payment.reports.colRefundAmount'),
      dataIndex: 'refund_amount_usd',
      key: 'refund_amount',
      width: 150,
      align: 'right',
      render: (v: string) => <span style={{ color: v.startsWith('-') ? 'red' : undefined }}>${v}</span>,
    },
    {
      title: t('payment.reports.colAvgTicket'),
      dataIndex: 'avg_ticket_usd',
      key: 'avg_ticket',
      width: 140,
      align: 'right',
      render: (v: string) => `$${v}`,
    },
  ];

  return (
    <div data-testid="financial-reports-page">
      <Title level={3}>{t('payment.reports.title')}</Title>

      {/* Filters */}
      <Space style={{ marginBottom: 16 }}>
        <Select
          value={granularity}
          onChange={(v) => setGranularity(v as 'day' | 'month')}
          style={{ width: 120 }}
          options={[
            { value: 'day', label: t('payment.reports.granularityDay') },
            { value: 'month', label: t('payment.reports.granularityMonth') },
          ]}
        />
        <DatePicker.RangePicker
          value={dateRange}
          onChange={(v) => { if (v && v[0] && v[1]) setDateRange([v[0], v[1]]); }}
          allowClear={false}
        />
        <Button type="primary" icon={<SearchOutlined />} onClick={handleSearch}>
          {t('payment.reports.search')}
        </Button>
      </Space>

      {/* Totals */}
      {report && (
        <Row gutter={16} style={{ marginBottom: 16 }}>
          <Col span={4}>
            <Card size="small">
              <Statistic title={t('payment.reports.totalGmv')} value={report.totals.gmv_usd} prefix="$" />
            </Card>
          </Col>
          <Col span={4}>
            <Card size="small">
              <Statistic title={t('payment.reports.totalOrders')} value={report.totals.order_count} />
            </Card>
          </Col>
          <Col span={4}>
            <Card size="small">
              <Statistic title={t('payment.reports.totalRefunds')} value={report.totals.refund_count} />
            </Card>
          </Col>
          <Col span={4}>
            <Card size="small">
              <Statistic
                title={t('payment.reports.totalRefundAmount')}
                value={report.totals.refund_amount_usd}
                prefix="$"
                valueStyle={{ color: report.totals.refund_amount_usd.startsWith('-') ? 'red' : undefined }}
              />
            </Card>
          </Col>
          <Col span={4}>
            <Card size="small">
              <Statistic title={t('payment.reports.totalAvgTicket')} value={report.totals.avg_ticket_usd} prefix="$" />
            </Card>
          </Col>
        </Row>
      )}

      {/* Series Table */}
      <Table<ReportSeriesItem>
        data-testid="report-table"
        rowKey="date"
        columns={columns}
        dataSource={report?.series ?? []}
        loading={loading}
        pagination={false}
        locale={{ emptyText: error ? t('payment.reports.errorLoad') : t('payment.reports.noData') }}
      />
    </div>
  );
}
