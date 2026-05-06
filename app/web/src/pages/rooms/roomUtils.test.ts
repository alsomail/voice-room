/**
 * T-20011: roomUtils 纯函数单元测试
 *
 * 验收用例：
 *   U01–U08: getActivityStatus
 *   U09–U14: formatDuration
 */

import { describe, it, expect } from 'vitest';
import { getActivityStatus, formatDuration, filterByActivity } from './roomUtils';
import type { ActivityFilter } from './roomUtils';

// ── 辅助：构造最小 room 数据 ────────────────────────────────────────────────
function makeRoom(
  memberCount: number,
  status: 'active' | 'closed',
  createdAt: string,
) {
  return { member_count: memberCount, status, created_at: createdAt };
}

/** 生成 N 分钟前的 ISO 时间字符串 */
function minsAgo(mins: number, from = new Date()): string {
  return new Date(from.getTime() - mins * 60 * 1000).toISOString();
}

// ════════════════════════════════════════════════════════════════════════════
// getActivityStatus
// ════════════════════════════════════════════════════════════════════════════

const NOW = new Date('2025-06-01T12:00:00Z');

// ── U01: member_count=5, status='active' → 'active' ─────────────────────────
describe('getActivityStatus — U01: ≥5人 → active', () => {
  it('member_count=5, status=active → "active"', () => {
    const room = makeRoom(5, 'active', minsAgo(30, NOW));
    expect(getActivityStatus(room, NOW)).toBe('active');
  });
});

// ── U02: member_count=10, status='active' → 'active' ────────────────────────
describe('getActivityStatus — U02: >5人 → active', () => {
  it('member_count=10, status=active → "active"', () => {
    const room = makeRoom(10, 'active', minsAgo(30, NOW));
    expect(getActivityStatus(room, NOW)).toBe('active');
  });
});

// ── U03: member_count=0, status='active' → 'abnormal' ───────────────────────
describe('getActivityStatus — U03: 0人+active → abnormal', () => {
  it('member_count=0, status=active → "abnormal"', () => {
    const room = makeRoom(0, 'active', minsAgo(30, NOW));
    expect(getActivityStatus(room, NOW)).toBe('abnormal');
  });
});

// ── U04: member_count=0, status='closed' → 'normal' ─────────────────────────
describe('getActivityStatus — U04: 0人+closed → normal', () => {
  it('member_count=0, status=closed → "normal"（非活跃状态不算异常）', () => {
    const room = makeRoom(0, 'closed', minsAgo(30, NOW));
    expect(getActivityStatus(room, NOW)).toBe('normal');
  });
});

// ── U05: member_count=3, 90分钟前 → 'quiet' ─────────────────────────────────
describe('getActivityStatus — U05: 1-4人且>1h → quiet', () => {
  it('member_count=3, 90min前 → "quiet"', () => {
    const room = makeRoom(3, 'active', minsAgo(90, NOW));
    expect(getActivityStatus(room, NOW)).toBe('quiet');
  });
});

// ── U06: member_count=3, 30分钟前 → 'normal' ────────────────────────────────
describe('getActivityStatus — U06: 1-4人且≤1h → normal', () => {
  it('member_count=3, 30min前 → "normal"（未超1h）', () => {
    const room = makeRoom(3, 'active', minsAgo(30, NOW));
    expect(getActivityStatus(room, NOW)).toBe('normal');
  });
});

// ── U07: member_count=1, 61分钟前 → 'quiet' ─────────────────────────────────
describe('getActivityStatus — U07: 1人且61min → quiet', () => {
  it('member_count=1, 61min前 → "quiet"（边界+1）', () => {
    const room = makeRoom(1, 'active', minsAgo(61, NOW));
    expect(getActivityStatus(room, NOW)).toBe('quiet');
  });
});

// ── U08: member_count=4, 59分钟前 → 'normal' ────────────────────────────────
describe('getActivityStatus — U08: 4人且59min → normal', () => {
  it('member_count=4, 59min前 → "normal"（边界-1）', () => {
    const room = makeRoom(4, 'active', minsAgo(59, NOW));
    expect(getActivityStatus(room, NOW)).toBe('normal');
  });
});

// ════════════════════════════════════════════════════════════════════════════
// formatDuration
// ════════════════════════════════════════════════════════════════════════════

// ── U09: 0ms → '0m' ──────────────────────────────────────────────────────────
describe('formatDuration — U09: 0ms → "0m"', () => {
  it('createdAt === now → "0m"', () => {
    expect(formatDuration(NOW.toISOString(), NOW)).toBe('0m');
  });
});

// ── U10: 45min → '45m' ───────────────────────────────────────────────────────
describe('formatDuration — U10: 45min → "45m"', () => {
  it('45分钟前 → "45m"', () => {
    const createdAt = new Date(NOW.getTime() - 45 * 60 * 1000).toISOString();
    expect(formatDuration(createdAt, NOW)).toBe('45m');
  });
});

// ── U11: 2h35m → '2h 35m' ────────────────────────────────────────────────────
describe('formatDuration — U11: 2h35m → "2h 35m"', () => {
  it('2小时35分钟前 → "2h 35m"', () => {
    const createdAt = new Date(NOW.getTime() - (2 * 60 + 35) * 60 * 1000).toISOString();
    expect(formatDuration(createdAt, NOW)).toBe('2h 35m');
  });
});

// ── U12: 3d2h → '3d 2h' ──────────────────────────────────────────────────────
describe('formatDuration — U12: 3d2h → "3d 2h"', () => {
  it('3天2小时前 → "3d 2h"', () => {
    const createdAt = new Date(NOW.getTime() - (3 * 24 * 60 + 2 * 60) * 60 * 1000).toISOString();
    expect(formatDuration(createdAt, NOW)).toBe('3d 2h');
  });
});

// ── U13: 未来时间（负数 diff）→ '0m' ─────────────────────────────────────────
describe('formatDuration — U13: 未来时间 → "0m"', () => {
  it('createdAt 在 now 之后 → "0m"', () => {
    const futureCreatedAt = new Date(NOW.getTime() + 60 * 1000).toISOString();
    expect(formatDuration(futureCreatedAt, NOW)).toBe('0m');
  });
});

// ── U14: 恰好 1min → '1m' ────────────────────────────────────────────────────
describe('formatDuration — U14: 恰好1min → "1m"', () => {
  it('恰好1分钟前 → "1m"', () => {
    const createdAt = new Date(NOW.getTime() - 60 * 1000).toISOString();
    expect(formatDuration(createdAt, NOW)).toBe('1m');
  });
});

// ════════════════════════════════════════════════════════════════════════════
// filterByActivity
// ════════════════════════════════════════════════════════════════════════════

describe('filterByActivity — 辅助函数覆盖', () => {
  const activeRoom = {
    id: 'r1', room_id: 'r1', title: 'R1', room_type: 'normal' as const,
    member_count: 5, max_members: 20, status: 'active' as const,
    owner_id: 'u1', owner_nickname: 'O1', owner_avatar: null,
    created_at: minsAgo(30, NOW),
  };
  const abnormalRoom = {
    id: 'r2', room_id: 'r2', title: 'R2', room_type: 'normal' as const,
    member_count: 0, max_members: 20, status: 'active' as const,
    owner_id: 'u2', owner_nickname: 'O2', owner_avatar: null,
    created_at: minsAgo(30, NOW),
  };
  const quietRoom = {
    id: 'r3', room_id: 'r3', title: 'R3', room_type: 'normal' as const,
    member_count: 2, max_members: 20, status: 'active' as const,
    owner_id: 'u3', owner_nickname: 'O3', owner_avatar: null,
    created_at: minsAgo(90, NOW),
  };
  const normalRoom = {
    id: 'r4', room_id: 'r4', title: 'R4', room_type: 'normal' as const,
    member_count: 3, max_members: 20, status: 'active' as const,
    owner_id: 'u4', owner_nickname: 'O4', owner_avatar: null,
    created_at: minsAgo(30, NOW),
  };
  const rooms = [activeRoom, abnormalRoom, quietRoom, normalRoom];

  it('"all" 返回全部', () => {
    expect(filterByActivity(rooms, 'all', NOW)).toHaveLength(4);
  });

  it('"active" 只返回 member_count≥5 的房间', () => {
    const result = filterByActivity(rooms, 'active', NOW);
    expect(result).toHaveLength(1);
    expect(result[0].room_id).toBe('r1');
  });

  it('"abnormal" 只返回 0人+active 的房间', () => {
    const result = filterByActivity(rooms, 'abnormal', NOW);
    expect(result).toHaveLength(1);
    expect(result[0].room_id).toBe('r2');
  });

  it('"quiet" 只返回冷清的房间', () => {
    const result = filterByActivity(rooms, 'quiet', NOW);
    expect(result).toHaveLength(1);
    expect(result[0].room_id).toBe('r3');
  });

  it('空数组返回空数组', () => {
    expect(filterByActivity([], 'active' as ActivityFilter, NOW)).toEqual([]);
  });
});
