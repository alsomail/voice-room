/**
 * useEventNames — 行为流 Tab event_name 多选下拉枚举（缺陷 8 / R1 批 3）
 *
 * 行为：
 *   1. 组件挂载时调用 GET /admin/events/names（最近 30 天）；
 *   2. 模块级缓存 5 分钟 TTL，避免重复抽屉打开/Tab 切换重复打网络；
 *   3. 接口失败时降级到 `events.dict.ts::ANALYTICS_EVENTS` 硬编码列表，
 *      并在控制台 `console.warn` 提示，保证 UI 可用；
 *   4. 在测试环境下可通过 `__resetEventNamesCache` 重置缓存。
 *
 * 注意：项目当前未引入 `@tanstack/react-query`，本 hook 用 useState/useEffect
 * 实现等价的"加载-缓存-降级"语义；如未来引入 React Query，可一键替换。
 */

import { useEffect, useRef, useState } from 'react';
import { listEventNames } from '../../services/api/events';
import { ANALYTICS_EVENTS } from './events.dict';

/** 缓存条目 */
interface CacheEntry {
  items: string[];
  expiresAt: number;
}

const CACHE_TTL_MS = 5 * 60 * 1000;
let cache: CacheEntry | null = null;
/** 进行中的请求（同一时刻多组件挂载时复用，避免抖动） */
let inflight: Promise<string[]> | null = null;

/** 测试辅助：重置模块级缓存（仅供单元测试调用） */
export function __resetEventNamesCache(): void {
  cache = null;
  inflight = null;
}

/** 钩子返回值 */
export interface UseEventNamesResult {
  /** 渲染用的事件名列表（成功为后端枚举；失败降级硬编码） */
  items: string[];
  /** 是否正在首次加载（已有缓存时为 false） */
  loading: boolean;
  /** 接口是否失败 — 失败时 items 来自硬编码字典 */
  failed: boolean;
}

async function fetchEventNamesWithCache(signal?: AbortSignal): Promise<string[]> {
  const now = Date.now();
  if (cache && cache.expiresAt > now) {
    return cache.items;
  }
  if (inflight) {
    return inflight;
  }
  inflight = (async () => {
    try {
      const resp = await listEventNames(30, signal);
      const items = Array.isArray(resp.items) ? [...resp.items].sort() : [];
      cache = { items, expiresAt: Date.now() + CACHE_TTL_MS };
      return items;
    } finally {
      inflight = null;
    }
  })();
  return inflight;
}

/**
 * 取后端 event_name 字典；失败降级到本地硬编码字典并 warn。
 *
 * 卸载/重新渲染由 AbortController 控制，避免对已卸载组件 setState。
 */
export function useEventNames(): UseEventNamesResult {
  // 若已有缓存，初始 items 直接走缓存（避免 SSR/快速 mount 时闪烁）
  const initial = cache?.items ?? null;
  const [items, setItems] = useState<string[]>(initial ?? ANALYTICS_EVENTS);
  const [loading, setLoading] = useState<boolean>(initial === null);
  const [failed, setFailed] = useState<boolean>(false);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    // 已有缓存：直接使用，不再请求
    if (cache && cache.expiresAt > Date.now()) {
      setItems(cache.items);
      setLoading(false);
      setFailed(false);
      return;
    }

    abortRef.current?.abort();
    const ctrl = new AbortController();
    abortRef.current = ctrl;

    setLoading(true);
    fetchEventNamesWithCache(ctrl.signal)
      .then((names) => {
        if (ctrl.signal.aborted) return;
        setItems(names.length > 0 ? names : ANALYTICS_EVENTS);
        setFailed(false);
      })
      .catch((err: unknown) => {
        if (ctrl.signal.aborted) return;
        // AbortError 不视为失败
        if (err instanceof Error && err.name === 'AbortError') return;
        // 接口失败：降级硬编码 + warn（不抛给 UI，保证可用）
        // eslint-disable-next-line no-console
        console.warn(
          '[useEventNames] failed to fetch /admin/events/names, falling back to local dict',
          err,
        );
        setItems(ANALYTICS_EVENTS);
        setFailed(true);
      })
      .finally(() => {
        if (!ctrl.signal.aborted) setLoading(false);
      });

    return () => {
      ctrl.abort();
    };
  }, []);

  return { items, loading, failed };
}
