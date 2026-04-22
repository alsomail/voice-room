/**
 * KickLogsTab — 踢人记录 Tab（T-20014）
 *
 * 展示踢人日志列表：
 *   - 时间 / 房间 / 目标用户（可点击）/ 操作者 / 原因
 *   - 分页
 *   - data-testid="governance-row-{id}" 用于测试定位
 *   - 目标用户链接 data-testid="governance-user-link-{user_id}"
 */

import { useState, useEffect, useRef } from 'react';
import { Table, Alert, Empty } from 'antd';
import type { TableColumnsType } from 'antd';
import { useTranslation } from 'react-i18next';
import type { KickLogItem, GovernanceListParams } from '../../services/api/governance';
import { listKicks } from '../../services/api/governance';

const PAGE_SIZE = 20;

export interface KickLogsTabProps {
  filters: GovernanceListParams;
  onUserClick: (userId: string) => void;
}

export function KickLogsTab({ filters, onUserClick }: KickLogsTabProps) {
  const { t } = useTranslation();

  const [items, setItems] = useState<KickLogItem[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const abortRef = useRef<AbortController | null>(null);

  // 当 filters 变化时重置到第1页
  useEffect(() => {
    setPage(1);
  }, [filters]);

  useEffect(() => {
    abortRef.current?.abort();
    const ctrl = new AbortController();
    abortRef.current = ctrl;

    setLoading(true);
    setError(null);

    listKicks(
      { ...filters, page, limit: PAGE_SIZE },
      ctrl.signal,
    )
      .then((data) => {
        if (ctrl.signal.aborted) return;
        setItems(data.items);
        setTotal(data.total);
      })
      .catch((err: Error) => {
        if (ctrl.signal.aborted) return;
        setError(err.message);
      })
      .finally(() => {
        if (!ctrl.signal.aborted) setLoading(false);
      });

    return () => ctrl.abort();
  }, [filters, page]);

  const columns: TableColumnsType<KickLogItem> = [
    {
      title: t('governance.colCreatedAt'),
      dataIndex: 'created_at',
      key: 'created_at',
      render: (val: string) => new Date(val).toLocaleString(),
    },
    {
      title: t('governance.colRoomId'),
      dataIndex: 'room_title',
      key: 'room_title',
      render: (title: string, record) => (
        <span title={record.room_id}>{title}</span>
      ),
    },
    {
      title: t('governance.colTargetUser'),
      dataIndex: 'target_nickname',
      key: 'target_user',
      render: (nickname: string, record) => (
        <a
          data-testid={`governance-user-link-${record.target_user_id}`}
          onClick={(e) => {
            e.preventDefault();
            onUserClick(record.target_user_id);
          }}
          style={{ cursor: 'pointer' }}
        >
          {nickname}
        </a>
      ),
    },
    {
      title: t('governance.colOperator'),
      dataIndex: 'operator_nickname',
      key: 'operator',
    },
    {
      title: t('governance.colReason'),
      dataIndex: 'reason',
      key: 'reason',
      render: (val: string | null) => val ?? '—',
    },
  ];

  if (error) {
    return (
      <Alert
        data-testid="governance-kicks-error"
        type="error"
        title={t('governance.errorLoad')}
        description={error}
        showIcon
      />
    );
  }

  if (!loading && items.length === 0) {
    return (
      <Empty
        data-testid="governance-kicks-empty"
        description={t('governance.empty')}
      />
    );
  }

  return (
    <Table<KickLogItem>
      rowKey="id"
      loading={loading}
      dataSource={items}
      columns={columns}
      pagination={{
        current: page,
        pageSize: PAGE_SIZE,
        total,
        onChange: (p) => setPage(p),
        showSizeChanger: false,
      }}
      onRow={(record) => ({
        'data-testid': `governance-row-${record.id}`,
      })}
    />
  );
}
