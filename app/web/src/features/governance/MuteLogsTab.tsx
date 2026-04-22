/**
 * MuteLogsTab — 禁言记录 Tab（T-20014）
 *
 * 展示禁言日志列表：
 *   - 时间 / 房间 / 目标用户（可点击）/ 操作者 / 类型 / 时长 / 原因
 *   - 分页
 *   - data-testid="governance-row-{id}" 用于测试定位
 */

import { useState, useEffect, useRef } from 'react';
import { Table, Alert, Empty, Tag } from 'antd';
import type { TableColumnsType } from 'antd';
import { useTranslation } from 'react-i18next';
import type { MuteLogItem, MuteListParams } from '../../services/api/governance';
import { listMutes } from '../../services/api/governance';

const PAGE_SIZE = 20;

export interface MuteLogsTabProps {
  filters: MuteListParams;
  onUserClick: (userId: string) => void;
}

export function MuteLogsTab({ filters, onUserClick }: MuteLogsTabProps) {
  const { t } = useTranslation();

  const [items, setItems] = useState<MuteLogItem[]>([]);
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

    listMutes(
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

  const formatDuration = (sec: number | null): string => {
    if (sec === null) return t('governance.durationPermanent');
    return t('governance.durationSeconds').replace('{sec}', String(sec));
  };

  const columns: TableColumnsType<MuteLogItem> = [
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
      title: t('governance.colMuteType'),
      dataIndex: 'type',
      key: 'type',
      render: (type: 'mic' | 'chat') => (
        <Tag color={type === 'mic' ? 'orange' : 'blue'}>
          {type === 'mic' ? t('governance.muteTypeMic') : t('governance.muteTypeChat')}
        </Tag>
      ),
    },
    {
      title: t('governance.colDuration'),
      dataIndex: 'duration_sec',
      key: 'duration_sec',
      render: (sec: number | null) => formatDuration(sec),
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
        data-testid="governance-mutes-error"
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
        data-testid="governance-mutes-empty"
        description={t('governance.empty')}
      />
    );
  }

  return (
    <Table<MuteLogItem>
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
