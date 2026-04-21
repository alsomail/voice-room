/**
 * T-20011: RoomActivityTag 组件测试
 *
 * 验收用例：
 *   C01: level='active' → color='success'
 *   C02: level='quiet' → color='warning'
 *   C03: level='abnormal' → color='error'
 *   C04: level='normal' → color='processing'
 *   C05: roomId='r1' → data-testid="room-activity-tag-r1"
 *   C06: level='active', zh 环境 → 文字"活跃"
 *   C07: level='abnormal', zh 环境 → 文字"异常"
 *   C08: level='normal', zh 环境 → 文字"正常"
 */

import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom';

// ── i18n mock：模拟 zh 环境，返回真实中文翻译 ──────────────────────────────
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        'rooms.activityLevelActive': '活跃',
        'rooms.activityLevelQuiet': '冷清',
        'rooms.activityLevelAbnormal': '异常',
        'rooms.activityLevelNormal': '正常',
      };
      return translations[key] ?? key;
    },
    i18n: { changeLanguage: vi.fn(), language: 'zh' },
  }),
  initReactI18next: { type: '3rdParty', init: vi.fn() },
}));

import { RoomActivityTag } from './RoomActivityTag';

// ── C01: level='active' → color='success' ────────────────────────────────────
describe('RoomActivityTag — C01: active → success 颜色', () => {
  it('level=active 时 Tag 带有 success 色彩类', () => {
    render(<RoomActivityTag level="active" roomId="r1" />);
    const tag = screen.getByTestId('room-activity-tag-r1');
    // Ant Design Tag color="success" → class 包含 ant-tag-success
    expect(tag.className).toContain('ant-tag-success');
  });
});

// ── C02: level='quiet' → color='warning' ─────────────────────────────────────
describe('RoomActivityTag — C02: quiet → warning 颜色', () => {
  it('level=quiet 时 Tag 带有 warning 色彩类', () => {
    render(<RoomActivityTag level="quiet" roomId="r2" />);
    const tag = screen.getByTestId('room-activity-tag-r2');
    expect(tag.className).toContain('ant-tag-warning');
  });
});

// ── C03: level='abnormal' → color='error' ────────────────────────────────────
describe('RoomActivityTag — C03: abnormal → error 颜色', () => {
  it('level=abnormal 时 Tag 带有 error 色彩类', () => {
    render(<RoomActivityTag level="abnormal" roomId="r3" />);
    const tag = screen.getByTestId('room-activity-tag-r3');
    expect(tag.className).toContain('ant-tag-error');
  });
});

// ── C04: level='normal' → color='processing' ─────────────────────────────────
describe('RoomActivityTag — C04: normal → processing 颜色', () => {
  it('level=normal 时 Tag 带有 processing 色彩类', () => {
    render(<RoomActivityTag level="normal" roomId="r4" />);
    const tag = screen.getByTestId('room-activity-tag-r4');
    expect(tag.className).toContain('ant-tag-processing');
  });
});

// ── C05: roomId='r1' → data-testid="room-activity-tag-r1" ────────────────────
describe('RoomActivityTag — C05: testid 格式', () => {
  it('roomId 拼接到 data-testid 上', () => {
    render(<RoomActivityTag level="active" roomId="r1" />);
    expect(screen.getByTestId('room-activity-tag-r1')).toBeInTheDocument();
  });
});

// ── C06: level='active', zh 环境 → 文字"活跃" ────────────────────────────────
describe('RoomActivityTag — C06: active 中文文本', () => {
  it('zh 环境下 active Tag 显示"活跃"', () => {
    render(<RoomActivityTag level="active" roomId="r1" />);
    expect(screen.getByTestId('room-activity-tag-r1')).toHaveTextContent('活跃');
  });
});

// ── C07: level='abnormal', zh 环境 → 文字"异常" ──────────────────────────────
describe('RoomActivityTag — C07: abnormal 中文文本', () => {
  it('zh 环境下 abnormal Tag 显示"异常"', () => {
    render(<RoomActivityTag level="abnormal" roomId="r5" />);
    expect(screen.getByTestId('room-activity-tag-r5')).toHaveTextContent('异常');
  });
});

// ── C08: level='normal', zh 环境 → 文字"正常" ────────────────────────────────
describe('RoomActivityTag — C08: normal 中文文本', () => {
  it('zh 环境下 normal Tag 显示"正常"', () => {
    render(<RoomActivityTag level="normal" roomId="r6" />);
    expect(screen.getByTestId('room-activity-tag-r6')).toHaveTextContent('正常');
  });
});
