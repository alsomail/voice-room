/**
 * EventStreamTab — 用户详情页"行为流"Tab（T-20013）
 *
 * 功能：
 *   - 时间筛选：Radio.Group [1h, 24h, 7d, 30d, custom]
 *   - 自定义时间窗：DatePicker.RangePicker（限 30 天）
 *   - 事件名多选：Select mode=multiple（选项来自 ANALYTICS_EVENTS）
 *   - 时间线列表：倒序，EventTimelineItem
 *   - 分页：Ant Design Pagination，默认 20/页
 *   - CSV 导出：最多 1000 条，文件名 user_{id}_events_{ts}.csv
 *
 * data-testid：
 *   event-stream-tab, event-time-range, event-name-select,
 *   btn-export-csv, events-loading, events-empty, events-error,
 *   custom-range-picker, range-error
 *
 * 导出：
 *   EventStreamTab       — 主组件
 *   validateCustomRange  — 纯函数（供单元测试直接调用）
 */

import { useState, useEffect, useRef, useMemo } from 'react';
import {
  Radio,
  Select,
  DatePicker,
  Alert,
  Button,
  Pagination,
  Spin,
  Empty,
  Space,
  Typography,
  message,
} from 'antd';
import { DownloadOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import type { Dayjs } from 'dayjs';
import type { EventListParams, EventListResponse } from '../../services/api/events';
import { listUserEvents } from '../../services/api/events';
import { ANALYTICS_EVENTS } from './events.dict';
import { EventTimelineItem } from './components/EventTimelineItem';
import { downloadCsv, generateEventCsvFilename, objectsToCsv } from '../../lib/csv';

const { Text } = Typography;

/** 时间范围预设类型 */
type TimeRangePreset = '1h' | '24h' | '7d' | '30d' | 'custom';

/** 各预设对应小时数 */
const PRESET_HOURS: Record<Exclude<TimeRangePreset, 'custom'>, number> = {
  '1h': 1,
  '24h': 24,
  '7d': 24 * 7,
  '30d': 24 * 30,
};

/** 最大自定义时间窗（天） */
const MAX_WINDOW_DAYS = 30;

/**
 * 验证自定义时间范围是否合法（≤ 30 天）
 * 导出为纯函数，方便单元测试
 */
export function validateCustomRange(from: string, to: string): boolean {
  const diff = new Date(to).getTime() - new Date(from).getTime();
  return diff <= MAX_WINDOW_DAYS * 24 * 60 * 60 * 1000;
}

// ─────────────────────────────────────────────────────────────────────────────

interface EventStreamTabProps {
  userId: string;
}

export function EventStreamTab({ userId }: EventStreamTabProps) {
  const { t } = useTranslation();

  // ── 筛选状态 ──────────────────────────────────────────────────────────────
  const [timeRange, setTimeRange] = useState<TimeRangePreset>('24h');
  const [customFrom, setCustomFrom] = useState<string | null>(null);
  const [customTo, setCustomTo] = useState<string | null>(null);
  const [selectedEvents, setSelectedEvents] = useState<string[]>([]);
  const [page, setPage] = useState(1);

  // ── UI 状态 ───────────────────────────────────────────────────────────────
  const [data, setData] = useState<EventListResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [apiError, setApiError] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);

  // AbortController 防竞态（主数据加载）
  const abortRef = useRef<AbortController | null>(null);
  // MEDIUM-1（Review R1）：CSV 导出专用 AbortController，组件卸载或导出完成时取消
  const exportAbortRef = useRef<AbortController | null>(null);

  // ── 范围错误（派生状态，不放在 effect deps 中避免无限循环）──────────────────
  const hasRangeError = useMemo(() => {
    if (timeRange === 'custom' && customFrom && customTo) {
      return !validateCustomRange(customFrom, customTo);
    }
    return false;
  }, [timeRange, customFrom, customTo]);

  // ── 主数据加载 ────────────────────────────────────────────────────────────
  // 注意：依赖列表中不包含 t / buildParams 等不稳定引用，避免无限渲染循环
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => {
    // 自定义范围超 30 天：不发请求
    if (timeRange === 'custom' && customFrom && customTo) {
      if (!validateCustomRange(customFrom, customTo)) return;
    }

    // 自定义范围尚未选择：不发请求
    if (timeRange === 'custom' && (!customFrom || !customTo)) return;

    // ── 计算 from / to ──
    const now = new Date();
    let from: string;
    let to: string = now.toISOString();

    if (timeRange === 'custom') {
      // 上面已判空，安全断言
      from = customFrom!;
      to = customTo!;
    } else {
      const hours = PRESET_HOURS[timeRange];
      from = new Date(now.getTime() - hours * 60 * 60 * 1000).toISOString();
    }

    const params: EventListParams = { from, to, page, limit: 20 };
    if (selectedEvents.length > 0) {
      params.event_name = selectedEvents.join(',');
    }

    // 取消上一次未完成请求
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;

    setLoading(true);
    setApiError(null);

    listUserEvents(userId, params, controller.signal)
      .then((res) => {
        if (!controller.signal.aborted) {
          setData(res);
        }
      })
      .catch((err: Error) => {
        if (!controller.signal.aborted) {
          setApiError(err.message);
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      });

    return () => {
      controller.abort();
      // 组件卸载时同时取消进行中的 CSV 导出请求
      exportAbortRef.current?.abort();
    };
  }, [userId, timeRange, customFrom, customTo, selectedEvents, page]); // 仅对业务依赖项响应

  // ── 自定义时间窗回调 ──────────────────────────────────────────────────────
  const handleRangeChange = (
    _: [Dayjs | null, Dayjs | null] | null,
    strings: [string, string],
  ) => {
    if (!strings[0] || !strings[1]) return;
    const from = new Date(strings[0]).toISOString();
    const to = new Date(strings[1]).toISOString();
    setCustomFrom(from);
    setCustomTo(to);
    setPage(1);
  };

  // ── CSV 导出 ───────────────────────────────────────────────────────────────
  const handleExportCsv = async () => {
    if (hasRangeError) return;

    // 重新计算当前筛选参数
    const now = new Date();
    let from: string;
    let to: string = now.toISOString();

    if (timeRange === 'custom') {
      if (!customFrom || !customTo) return;
      from = customFrom;
      to = customTo;
    } else {
      const hours = PRESET_HOURS[timeRange];
      from = new Date(now.getTime() - hours * 60 * 60 * 1000).toISOString();
    }

    // HIGH-2（Review R1）：limit 从 20 改为 100，减少 API 调用次数（50次→10次）
    const baseParams: EventListParams = { from, to, limit: 100 };
    if (selectedEvents.length > 0) {
      baseParams.event_name = selectedEvents.join(',');
    }

    // MEDIUM-1（Review R1）：创建独立 AbortController，导出完成/组件卸载时取消请求
    exportAbortRef.current?.abort();
    const exportController = new AbortController();
    exportAbortRef.current = exportController;

    setExporting(true);
    try {
      const allItems: EventListResponse['items'] = [];
      let fetchPage = 1;

      while (allItems.length < 1000) {
        if (exportController.signal.aborted) break;
        const res = await listUserEvents(
          userId,
          { ...baseParams, page: fetchPage },
          exportController.signal,
        );
        allItems.push(...res.items);
        if (allItems.length >= res.total || res.items.length < (baseParams.limit ?? 100)) break;
        fetchPage++;
      }

      if (exportController.signal.aborted) return;

      const truncated = allItems.slice(0, 1000);
      if (allItems.length > 1000) {
        void message.warning(t('events.csvTruncated'));
      }

      const rows = truncated.map((item) => ({
        id: item.id,
        event_name: item.event_name,
        server_ts: item.server_ts,
        client_ts: item.client_ts ?? '',
        session_id: item.session_id ?? '',
        device_id: item.device_id ?? '',
        app_version: item.app_version ?? '',
        os_version: item.os_version ?? '',
        network_type: item.network_type ?? '',
        locale: item.locale ?? '',
        properties: JSON.stringify(item.properties ?? {}),
      }));

      const csv = objectsToCsv(rows);
      const filename = generateEventCsvFilename(userId, Date.now());
      downloadCsv(csv, filename);
    } catch (err) {
      // AbortError 不提示用户（主动取消，非真实错误）
      if (err instanceof Error && err.name === 'AbortError') return;
      void message.error(t('events.csvError'));
    } finally {
      if (!exportController.signal.aborted) {
        setExporting(false);
      }
    }
  };

  // ── 渲染 ─────────────────────────────────────────────────────────────────
  return (
    <div data-testid="event-stream-tab">
      {/* 工具栏 */}
      <Space wrap style={{ marginBottom: 16 }}>
        {/* 时间范围 */}
        <Radio.Group
          data-testid="event-time-range"
          value={timeRange}
          onChange={(e) => {
            setTimeRange(e.target.value as TimeRangePreset);
            setPage(1);
          }}
          optionType="button"
          buttonStyle="solid"
          size="small"
        >
          <Radio.Button value="1h">1h</Radio.Button>
          <Radio.Button value="24h">24h</Radio.Button>
          <Radio.Button value="7d">7d</Radio.Button>
          <Radio.Button value="30d">30d</Radio.Button>
          <Radio.Button value="custom">{t('events.custom')}</Radio.Button>
        </Radio.Group>

        {/* 自定义时间范围选择器 */}
        {timeRange === 'custom' && (
          <DatePicker.RangePicker
            data-testid="custom-range-picker"
            showTime
            size="small"
            onChange={handleRangeChange as (dates: unknown, strings: [string, string]) => void}
          />
        )}

        {/* 事件名多选 */}
        <Select
          data-testid="event-name-select"
          mode="multiple"
          allowClear
          placeholder={t('events.selectEvents')}
          options={ANALYTICS_EVENTS.map((e) => ({ value: e, label: e }))}
          value={selectedEvents}
          onChange={(values: string[]) => {
            setSelectedEvents(values);
            setPage(1);
          }}
          style={{ minWidth: 220 }}
          size="small"
          maxTagCount="responsive"
        />

        {/* 导出 CSV */}
        <Button
          data-testid="btn-export-csv"
          icon={<DownloadOutlined />}
          size="small"
          loading={exporting}
          onClick={handleExportCsv}
        >
          {t('events.exportCsv')}
        </Button>
      </Space>

      {/* 时间范围错误提示 */}
      {hasRangeError && (
        <Alert
          data-testid="range-error"
          type="error"
          message={t('events.rangeError')}
          showIcon
          style={{ marginBottom: 12 }}
        />
      )}

      {/* 加载中 */}
      {loading && (
        <div data-testid="events-loading" style={{ textAlign: 'center', padding: 24 }}>
          <Spin />
        </div>
      )}

      {/* API 错误 */}
      {apiError && !loading && (
        <Alert
          data-testid="events-error"
          type="error"
          message={t('events.loadError')}
          description={apiError}
          showIcon
          style={{ marginBottom: 12 }}
        />
      )}

      {/* 空状态 */}
      {!loading && !apiError && data && data.items.length === 0 && (
        <Empty
          data-testid="events-empty"
          description={<Text type="secondary">{t('events.empty')}</Text>}
        />
      )}

      {/* 事件时间线 */}
      {!loading && !apiError && data && data.items.length > 0 && (
        <div>
          {data.items.map((item) => (
            <EventTimelineItem key={item.id} event={item} />
          ))}
        </div>
      )}

      {/* 分页 */}
      {data && data.total > 0 && (
        <div style={{ textAlign: 'right', marginTop: 16 }}>
          <Pagination
            total={data.total}
            pageSize={20}
            current={page}
            onChange={(p) => setPage(p)}
            size="small"
            showTotal={(total) => `${t('events.total')}: ${total}`}
          />
        </div>
      )}
    </div>
  );
}
