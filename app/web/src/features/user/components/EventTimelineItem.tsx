/**
 * EventTimelineItem — 单条事件卡片（T-20013）
 *
 * 展示：
 *   - event_name（Tag，颜色按类别）
 *   - server_ts 格式化时间
 *   - 设备 / app_version / network_type（小字）
 *   - properties 折叠 JSON（关键字高亮）
 *
 * data-testid：
 *   - event-item-{id}
 *   - props-toggle-{id}
 *   - props-content-{id}  （仅展开时存在）
 */

import { useState } from 'react';
import { Tag, Typography, Button } from 'antd';
import { useTranslation } from 'react-i18next';
import type { EventItem } from '../../../services/api/events';
import { getEventColor } from '../events.dict';

const { Text, Paragraph } = Typography;

interface EventTimelineItemProps {
  event: EventItem;
  /** 关键字高亮（properties JSON 中高亮该词） */
  highlight?: string;
}

export function EventTimelineItem({ event, highlight }: EventTimelineItemProps) {
  const { t } = useTranslation();
  const [propsOpen, setPropsOpen] = useState(false);

  const color = getEventColor(event.event_name);

  const formattedTs = new Date(event.server_ts).toLocaleString();

  const propsJson = event.properties
    ? JSON.stringify(event.properties, null, 2)
    : null;

  /**
   * HTML 转义：防止 XSS 注入（Review R1 HIGH-1 修复）
   * 对 &、<、>、"、' 进行实体转义，确保原始内容不被浏览器解析为 HTML
   */
  const escapeHtml = (text: string): string =>
    text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#x27;');

  /**
   * 关键字高亮：先对文本做 HTML 转义，再用 <mark> 包裹匹配词
   * 对 highlight 关键字同样先 HTML 转义，确保正则能匹配转义后的文本
   */
  const highlightText = (text: string): string => {
    if (!highlight || !highlight.trim()) return escapeHtml(text);
    const escapedText = escapeHtml(text);
    // 对关键字先 HTML 转义，再对正则特殊字符转义
    const escapedKeyword = escapeHtml(highlight).replace(
      /[.*+?^${}()|[\]\\]/g,
      '\\$&',
    );
    return escapedText.replace(
      new RegExp(`(${escapedKeyword})`, 'gi'),
      '<mark>$1</mark>',
    );
  };

  return (
    <div
      data-testid={`event-item-${event.id}`}
      style={{
        borderLeft: `3px solid ${color}`,
        paddingLeft: 12,
        marginBottom: 16,
      }}
    >
      {/* 事件名 */}
      <div style={{ marginBottom: 4 }}>
        <Tag color={color}>{event.event_name}</Tag>
        <Text type="secondary" style={{ fontSize: 12 }}>
          {formattedTs}
        </Text>
      </div>

      {/* 设备信息 */}
      <div style={{ fontSize: 11, color: '#999', marginBottom: 4 }}>
        {[event.app_version, event.os_version, event.network_type]
          .filter(Boolean)
          .join(' · ')}
      </div>

      {/* Properties 折叠 */}
      {propsJson !== null && (
        <div>
          <Button
            type="link"
            size="small"
            data-testid={`props-toggle-${event.id}`}
            onClick={() => setPropsOpen((v) => !v)}
            style={{ padding: 0, fontSize: 11 }}
          >
            {propsOpen
              ? t('events.props.collapse')
              : t('events.props.expand')}
          </Button>

          {propsOpen && (
            <Paragraph
              data-testid={`props-content-${event.id}`}
              style={{
                background: '#f5f5f5',
                padding: 8,
                borderRadius: 4,
                fontSize: 11,
                fontFamily: 'monospace',
                marginTop: 4,
                marginBottom: 0,
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-all',
              }}
            >
              {highlight ? (
                <span
                  dangerouslySetInnerHTML={{
                    __html: highlightText(propsJson),
                  }}
                />
              ) : (
                propsJson
              )}
            </Paragraph>
          )}
        </div>
      )}
    </div>
  );
}
