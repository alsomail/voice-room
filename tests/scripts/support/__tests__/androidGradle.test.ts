/**
 * 缺陷 2 修复（batch-e2e-foundation-01 第 1 轮）静态守护：
 *   Android staging / prod productFlavors 不得字面硬编码 URL，
 *   必须通过 `resolveConfigValue(localProperties, ..., "VOICE_ROOM_*", default)` 通道，
 *   保证根 .env → envLoader.writeProcessEnv → process.env → gradlew → BuildConfig 单一事实源链路。
 *
 *   默认值（与商店域名一致）允许保留以确保 0 回归，但每个 flavor 必须存在
 *   resolveConfigValue 调用作为 env 注入通道。
 */
import { test, expect } from '@playwright/test';
import * as fs from 'node:fs';
import * as path from 'node:path';

const REPO_ROOT = path.resolve(__dirname, '../../../..');
const GRADLE = path.join(REPO_ROOT, 'app/android/app/build.gradle.kts');

function readGradle(): string {
  return fs.readFileSync(GRADLE, 'utf8');
}

function extractFlavor(text: string, name: string): string {
  // 匹配 `create("<name>") { ... }`，括号匹配（限粒度，足够静态体检）
  const idx = text.indexOf(`create("${name}")`);
  expect(idx, `flavor ${name} not found`).toBeGreaterThan(-1);
  const open = text.indexOf('{', idx);
  let depth = 0;
  for (let i = open; i < text.length; i++) {
    if (text[i] === '{') depth++;
    else if (text[i] === '}') {
      depth--;
      if (depth === 0) return text.slice(open, i + 1);
    }
  }
  throw new Error('unbalanced braces');
}

test('U-A1 staging flavor 必须用 resolveConfigValue 接 VOICE_ROOM_API_BASE_URL（不得字面硬编码）', () => {
  const block = extractFlavor(readGradle(), 'staging');
  expect(block).toContain('resolveConfigValue');
  expect(block).toContain('VOICE_ROOM_API_BASE_URL');
  expect(block).toContain('VOICE_ROOM_WS_URL');
  expect(block).toContain('VOICE_ROOM_ANALYTICS_ENDPOINT');
  // 不得在 buildConfigField 行直接写整段字面 URL（必须通过变量插值 $apiBaseUrl）
  const directLiteral = /buildConfigField\(\s*"String"\s*,\s*"API_BASE_URL"\s*,\s*"\\"https:\/\//;
  expect(directLiteral.test(block), 'staging API_BASE_URL 仍是字面 URL 硬编码').toBe(false);
});

test('U-A2 prod flavor 必须用 resolveConfigValue 接 VOICE_ROOM_API_BASE_URL（不得字面硬编码）', () => {
  const block = extractFlavor(readGradle(), 'prod');
  expect(block).toContain('resolveConfigValue');
  expect(block).toContain('VOICE_ROOM_API_BASE_URL');
  expect(block).toContain('VOICE_ROOM_WS_URL');
  expect(block).toContain('VOICE_ROOM_ANALYTICS_ENDPOINT');
  const directLiteral = /buildConfigField\(\s*"String"\s*,\s*"API_BASE_URL"\s*,\s*"\\"https:\/\//;
  expect(directLiteral.test(block), 'prod API_BASE_URL 仍是字面 URL 硬编码').toBe(false);
});

test('U-A3 envLoader.writeProcessEnv 写入 VOICE_ROOM_* 桥接键', () => {
  const src = fs.readFileSync(path.join(REPO_ROOT, 'tests/scripts/support/envLoader.ts'), 'utf8');
  expect(src).toContain('VOICE_ROOM_API_BASE_URL');
  expect(src).toContain('VOICE_ROOM_WS_URL');
  // 确保位于 writeProcessEnv 函数内
  const fnIdx = src.indexOf('export function writeProcessEnv');
  expect(fnIdx).toBeGreaterThan(-1);
  const after = src.slice(fnIdx);
  expect(after.indexOf('VOICE_ROOM_API_BASE_URL')).toBeGreaterThan(-1);
});
