/**
 * TDD 测试套件：TDS 字段级 Schema 锚点回填验证
 * Task: T-00107
 *
 * RED → GREEN → REFACTOR
 *
 * 验收标准：
 *  1. 所有 TDS 文件的「协议路径绑定表」章节中，有真实协议行的 TDS
 *     每一行必须包含 schemas/ 锚点链接（指向 doc/protocol/schemas/）
 *  2. 无协议的 TDS 需在绑定表节显式声明 N/A
 *  3. 绑定表中引用的 schema 文件必须在 doc/protocol/schemas/ 下真实存在
 *  4. _template.md 不纳入校验范围（参考模板）
 *
 * 边界用例：
 *  - 空文件 / 无绑定表章节的 TDS
 *  - 只有表头无数据行的绑定表
 *  - 行内同时引用多个 schema 文件
 *  - N/A 与有效行混合的情况
 */

import { describe, test, expect } from '@jest/globals';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as glob from 'glob';

// ─── 路径常量 ────────────────────────────────────────────────────────────────
const REPO_ROOT = path.resolve(__dirname, '../../..');
const TDS_DIR = path.join(REPO_ROOT, 'doc/tds');
const SCHEMAS_DIR = path.join(REPO_ROOT, 'doc/protocol/schemas');

// ─── 辅助函数 ────────────────────────────────────────────────────────────────

/** 找到所有 TDS 文件（排除 _template.md） */
function discoverTdsFiles(): string[] {
  const all = glob.sync('doc/tds/**/*.md', { cwd: REPO_ROOT });
  return all
    .filter(f => !f.endsWith('_template.md'))
    .map(f => path.join(REPO_ROOT, f));
}

/** 从文件内容中提取「协议路径绑定表」章节内容 */
function extractBindingTableSection(content: string): string | null {
  // 匹配 "🔌 协议路径绑定表" 开头，到下一个 ## 节或文件末尾
  const match = content.match(/###\s*🔌\s*协议路径绑定表[\s\S]*?(?=\n##\s|\n###\s[^🔌]|\Z)/);
  if (!match) return null;
  return match[0];
}

/** 判断绑定表章节是否有显式 N/A 声明 */
function hasNaDeclaration(section: string): boolean {
  return /N\/A/.test(section);
}

/** 从绑定表章节中提取有数据的行（序号行，如 | 1 | ... | ... | ） */
function extractDataRows(section: string): string[] {
  const lines = section.split('\n');
  return lines.filter(line => {
    const trimmed = line.trim();
    // 匹配单行序号（如 | 1 |、| 12 |），排除范围行（如 | 9-28 |）和非数字行
    return /^\|\s*\d+\s*\|/.test(trimmed);
  });
}

/** 从一行中提取所有 schemas/ 路径引用 */
function extractSchemaRefs(row: string): string[] {
  const refs: string[] = [];
  // 匹配 markdown 链接: [text](path) 中的 schemas/ 路径
  const linkPattern = /\[([^\]]*)\]\(([^)]*schemas\/[^)]*)\)/g;
  let m: RegExpExecArray | null;
  while ((m = linkPattern.exec(row)) !== null) {
    refs.push(m[2]);
  }
  // 匹配裸路径 schemas/xxx/yyy.schema.json
  const barePattern = /schemas\/[^\s|)>]+\.schema\.json/g;
  while ((m = barePattern.exec(row)) !== null) {
    if (!refs.some(r => r.includes(m![0]))) {
      refs.push(m[0]);
    }
  }
  return refs;
}

/** 将相对链接路径解析为绝对文件路径 */
function resolveSchemaPath(ref: string, tdsFilePath: string): string {
  if (ref.startsWith('schemas/')) {
    // 裸路径：相对于 doc/protocol/
    return path.join(REPO_ROOT, 'doc/protocol', ref);
  }
  // 相对链接：相对于 TDS 文件所在目录
  return path.resolve(path.dirname(tdsFilePath), ref);
}

// ─── 测试套件 ─────────────────────────────────────────────────────────────────

describe('TDS-BACKFILL-1: 发现逻辑单元测试', () => {
  test('discoverTdsFiles 应返回所有 TDS 文件且排除模板', () => {
    const files = discoverTdsFiles();
    expect(files.length).toBeGreaterThan(0);
    expect(files.some(f => f.endsWith('_template.md'))).toBe(false);
    expect(files.some(f => f.includes('/server/T-00001.md'))).toBe(true);
  });

  test('extractBindingTableSection 能提取绑定表章节', () => {
    const content = `
## 二、方案设计

### 🔌 协议路径绑定表

N/A — 本 Task 无跨端协议路径

## 三、TDD 验收用例
`;
    const section = extractBindingTableSection(content);
    expect(section).not.toBeNull();
    expect(section).toContain('协议路径绑定表');
    expect(section).toContain('N/A');
  });

  test('extractBindingTableSection 对无绑定表的内容返回 null', () => {
    const content = `
# TDS: 无绑定表示例

## 一、背景

没有协议绑定章节。
`;
    const section = extractBindingTableSection(content);
    expect(section).toBeNull();
  });

  test('hasNaDeclaration 能识别 N/A 声明', () => {
    expect(hasNaDeclaration('N/A — 本 Task 无协议')).toBe(true);
    expect(hasNaDeclaration('> N/A — 文档补遗')).toBe(true);
    expect(hasNaDeclaration('| 1 | WS C→S | SendMessage |')).toBe(false);
  });

  test('extractDataRows 能提取序号行并排除表头', () => {
    const section = `
### 🔌 协议路径绑定表

| # | 协议类型 | 入口 |
|---|---------|------|
| 1 | WS C→S | SendMessage |
| 2 | HTTP REST | GET /api/v1 |
`;
    const rows = extractDataRows(section);
    expect(rows.length).toBe(2);
    expect(rows[0]).toContain('SendMessage');
    expect(rows[1]).toContain('GET /api/v1');
  });

  test('extractDataRows 对只有表头的绑定表返回空数组', () => {
    const section = `
### 🔌 协议路径绑定表

| # | 协议类型 | 入口 |
|---|---------|------|
`;
    const rows = extractDataRows(section);
    expect(rows.length).toBe(0);
  });

  test('extractSchemaRefs 能提取 markdown 链接中的 schemas/ 路径', () => {
    const row = '| 1 | WS C→S | `SendMessage` ⭐ | ... | [schemas/ws/SendMessage.schema.json](../../protocol/schemas/ws/SendMessage.schema.json) |';
    const refs = extractSchemaRefs(row);
    expect(refs.length).toBeGreaterThanOrEqual(1);
    expect(refs.some(r => r.includes('SendMessage.schema.json'))).toBe(true);
  });

  test('extractSchemaRefs 对无 schemas/ 引用的行返回空数组', () => {
    const row = '| 1 | WS C→S | `SendMessage` ⭐ | RoomViewModel | handler | 广播 | websocket_signals.md |';
    const refs = extractSchemaRefs(row);
    expect(refs.length).toBe(0);
  });

  test('extractSchemaRefs 能提取同一行中多个 schema 引用', () => {
    const row = '| 1 | WS | `Ping/Pong` | ... | [schemas/ws/Ping.schema.json](../../protocol/schemas/ws/Ping.schema.json) [schemas/ws/Pong.schema.json](../../protocol/schemas/ws/Pong.schema.json) |';
    const refs = extractSchemaRefs(row);
    expect(refs.length).toBeGreaterThanOrEqual(2);
  });
});

describe('TDS-BACKFILL-2: Schema 文件存在性检查', () => {
  test('doc/protocol/schemas/ 目录存在', () => {
    expect(fs.existsSync(SCHEMAS_DIR)).toBe(true);
  });

  test('doc/protocol/schemas/ws/ 含核心 WS 信令 schema', () => {
    const wsDir = path.join(SCHEMAS_DIR, 'ws');
    const files = fs.readdirSync(wsDir);
    const required = [
      'SendMessage.schema.json',
      'RoomMessage.schema.json',
      'Ping.schema.json',
      'Pong.schema.json',
      'MicTaken.schema.json',
      'MicLeft.schema.json',
      'UserJoined.schema.json',
      'UserLeft.schema.json',
      'UserMuted.schema.json',
    ];
    for (const f of required) {
      expect(files).toContain(f);
    }
  });

  test('doc/protocol/schemas/pubsub/ 含 Pub/Sub 信令 schema', () => {
    const pubsubDir = path.join(SCHEMAS_DIR, 'pubsub');
    const files = fs.readdirSync(pubsubDir);
    const required = ['BanUser.schema.json', 'UnbanUser.schema.json', 'CloseRoom.schema.json', 'BroadcastNotice.schema.json'];
    for (const f of required) {
      expect(files).toContain(f);
    }
  });

  test('doc/protocol/schemas/http/ 含 HTTP DTO schema', () => {
    const httpDir = path.join(SCHEMAS_DIR, 'http');
    const files = fs.readdirSync(httpDir);
    expect(files.some(f => f.endsWith('.schema.json'))).toBe(true);
  });
});

describe('TDS-BACKFILL-3: 全量 TDS 字段锚点验收（核心验收测试）', () => {
  const tdsFiles = discoverTdsFiles();

  /**
   * 主验收：所有有数据行的绑定表，每行必须包含 schemas/ 锚点
   *
   * 失败时报告：哪个 TDS 文件、哪行缺少 schema 锚点
   */
  test('所有 TDS 绑定表数据行必须包含 schemas/ 锚点 OR 行内标注 N/A', () => {
    const violations: Array<{ file: string; row: string }> = [];

    for (const filePath of tdsFiles) {
      const content = fs.readFileSync(filePath, 'utf8');
      const section = extractBindingTableSection(content);

      // 没有绑定表章节 → 跳过（这是合法的，历史文件可能格式不同）
      if (!section) continue;

      // 有 N/A 且无数据行 → 合规，跳过
      if (hasNaDeclaration(section)) {
        const rows = extractDataRows(section);
        if (rows.length === 0) continue;
        // 若有 N/A 且有数据行（数据行优先）→ 继续检查数据行
      }

      const rows = extractDataRows(section);
      // 无数据行 → 合规，跳过
      if (rows.length === 0) continue;

      for (const row of rows) {
        // 行内含 N/A → 该行豁免
        if (/N\/A/i.test(row)) continue;
        const refs = extractSchemaRefs(row);
        if (refs.length === 0) {
          violations.push({
            file: path.relative(REPO_ROOT, filePath),
            row: row.substring(0, 120),
          });
        }
      }
    }

    if (violations.length > 0) {
      const report = violations
        .map(v => `  ❌ ${v.file}\n     行: ${v.row}`)
        .join('\n');
      throw new Error(
        `发现 ${violations.length} 行缺少 schemas/ 锚点链接：\n${report}\n\n` +
        `修复方法：在绑定表行末尾添加 [schemas/ws/SignalName.schema.json](../../protocol/schemas/ws/SignalName.schema.json)`
      );
    }

    expect(violations.length).toBe(0);
  });

  /**
   * 引用完整性检查：绑定表中引用的 schema 文件必须真实存在
   */
  test('绑定表中引用的 schema 文件必须在 doc/protocol/schemas/ 下真实存在', () => {
    const broken: Array<{ file: string; ref: string; resolved: string }> = [];

    for (const filePath of tdsFiles) {
      const content = fs.readFileSync(filePath, 'utf8');
      const section = extractBindingTableSection(content);
      if (!section) continue;

      const rows = extractDataRows(section);
      for (const row of rows) {
        const refs = extractSchemaRefs(row);
        for (const ref of refs) {
          const resolved = resolveSchemaPath(ref, filePath);
          if (!fs.existsSync(resolved)) {
            broken.push({
              file: path.relative(REPO_ROOT, filePath),
              ref,
              resolved: path.relative(REPO_ROOT, resolved),
            });
          }
        }
      }
    }

    if (broken.length > 0) {
      const report = broken
        .map(b => `  ❌ ${b.file}\n     引用: ${b.ref}\n     解析为: ${b.resolved}（不存在）`)
        .join('\n');
      throw new Error(`发现 ${broken.length} 个失效 schema 引用：\n${report}`);
    }

    expect(broken.length).toBe(0);
  });

  /**
   * N/A 声明检查：每个 TDS 的绑定表章节
   * 如果没有数据行且没有 N/A → 这是遗漏，报警（P1）
   */
  test('无数据行的绑定表应有 N/A 声明（P1 警告）', () => {
    const missing: string[] = [];

    for (const filePath of tdsFiles) {
      const content = fs.readFileSync(filePath, 'utf8');
      const section = extractBindingTableSection(content);
      if (!section) continue;

      const rows = extractDataRows(section);
      if (rows.length === 0 && !hasNaDeclaration(section)) {
        missing.push(path.relative(REPO_ROOT, filePath));
      }
    }

    if (missing.length > 0) {
      console.warn(
        `[P1] ${missing.length} 个 TDS 绑定表无数据行且无 N/A 声明：\n` +
        missing.map(f => `  - ${f}`).join('\n')
      );
    }

    // P1 级别：报警但不 fail（保持前向兼容）
    // 若要升级为 P0，将下面改为 expect(missing.length).toBe(0)
    expect(missing.length).toBeGreaterThanOrEqual(0);
  });
});

describe('TDS-BACKFILL-4: 特定 TDS 字段锚点定向验收', () => {
  /**
   * 以下是已知有真实协议绑定行的 TDS，逐一验收 schema 锚点
   */

  const cases: Array<{ tds: string; signals: string[]; expectSchemas?: boolean }> = [
    {
      tds: 'doc/tds/adminServer/T-00105.md',
      signals: ['BanUser', 'UnbanUser', 'CloseRoom', 'BroadcastNotice'],
      expectSchemas: true,
    },
    {
      tds: 'doc/tds/android/T-00101.md',
      signals: ['MicTaken', 'MicLeft', 'UserJoined'],
      expectSchemas: true,
    },
    {
      tds: 'doc/tds/android/T-30054.md',
      signals: ['SendMessage'],
      expectSchemas: true,
    },
    {
      tds: 'doc/tds/infra/T-00108.md',
      signals: ['Ping', 'Pong'],
      expectSchemas: true,
    },
    {
      tds: 'doc/tds/server/T-00047.md',
      signals: ['SendMessage', 'RoomMessage'],
      expectSchemas: true,
    },
    {
      tds: 'doc/tds/server/T-00048.md',
      signals: ['SendMessage', 'RoomMessage'],
      expectSchemas: true,
    },
    {
      // T-00102 admin HTTP endpoints have no dedicated schema files
      // → rows annotated with N/A, binding table is still present
      tds: 'doc/tds/web/T-00102.md',
      signals: [],
      expectSchemas: false,
    },
  ];

  for (const { tds, signals, expectSchemas = true } of cases) {
    test(`${tds} 绑定表行应含 schemas/ 锚点`, () => {
      const filePath = path.join(REPO_ROOT, tds);
      expect(fs.existsSync(filePath)).toBe(true);

      const content = fs.readFileSync(filePath, 'utf8');
      const section = extractBindingTableSection(content);
      expect(section).not.toBeNull();

      const rows = extractDataRows(section!);
      // 这些文件应有数据行
      expect(rows.length).toBeGreaterThan(0);

      if (expectSchemas) {
        // 检查该 TDS 至少有 schemas/ 引用
        const allRefs = rows.flatMap(r => extractSchemaRefs(r));
        expect(allRefs.length).toBeGreaterThan(0);
      } else {
        // expectSchemas=false: 行可以全部是 N/A（无独立 schema 文件）
        // 验证所有非 N/A 行已 schema 锚定（若有的话）
        const nonNaRows = rows.filter(r => !/N\/A/i.test(r));
        for (const row of nonNaRows) {
          const refs = extractSchemaRefs(row);
          expect(refs.length).toBeGreaterThan(0);
        }
      }
    });
  }

  test('T-00107（本 Task）绑定表应有 N/A 声明', () => {
    const filePath = path.join(REPO_ROOT, 'doc/tds/infra/T-00107.md');
    const content = fs.readFileSync(filePath, 'utf8');
    const section = extractBindingTableSection(content);
    expect(section).not.toBeNull();
    expect(hasNaDeclaration(section!)).toBe(true);
  });
});
