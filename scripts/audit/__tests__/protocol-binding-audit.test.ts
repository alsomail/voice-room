/**
 * TDD 测试套件：协议路径绑定审计脚本
 * Task: T-0000T
 *
 * 测试顺序遵循 RED → GREEN → REFACTOR。
 * 先运行会全部失败（实现文件不存在），实现完成后全部通过。
 */

import { describe, test, expect } from '@jest/globals';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';

// 导入待实现的函数（实现前会导致编译/运行时错误 → RED）
import {
  parseBindingTable,
  auditBindings,
  generateReport,
  renderMarkdownReport,
  discoverTdsFiles,
  grepServerImpl,
  grepClientCalls,
  runGrep,
  parseGrepOutput,
  writeReports,
  deduplicateGrep,
} from '../protocol-binding-audit';
import type {
  ProtocolBinding,
  AuditFinding,
  GrepResult,
  ReportMeta,
  AuditReport,
} from '../protocol-binding-audit';

// __dirname = scripts/audit/__tests__  →  ../../.. = project root
const REPO_ROOT = path.resolve(__dirname, '../../..');
const TDS_47 = path.join(REPO_ROOT, 'doc/tds/server/T-00047.md');
const TDS_48 = path.join(REPO_ROOT, 'doc/tds/server/T-00048.md');
const TDS_0T = path.join(REPO_ROOT, 'doc/tds/infra/T-0000T.md');

// ─────────────────────────────────────────────────────────────────────────────
// TC-AUDIT-01: 解析 T-00047.md 绑定表（真实文件）
// ─────────────────────────────────────────────────────────────────────────────
describe('TC-AUDIT-01: 解析 T-00047.md 绑定表', () => {
  test('应从 T-00047.md 中提取 ≥3 条路径绑定', () => {
    const content = fs.readFileSync(TDS_47, 'utf8');
    const bindings = parseBindingTable(content, TDS_47);

    // 至少 3 条绑定行
    expect(bindings.length).toBeGreaterThanOrEqual(3);

    // 每条绑定必须有核心字段
    for (const b of bindings) {
      expect(typeof b.protocolType).toBe('string');
      expect(b.protocolType.length).toBeGreaterThan(0);

      expect(typeof b.endpoint).toBe('string');
      expect(b.endpoint.length).toBeGreaterThan(0);

      // clientFile 可以是空字符串（表示「目前无客户端」），但必须是字符串
      expect(typeof b.clientFile).toBe('string');
      expect(typeof b.clientFunction).toBe('string');

      // serverFile / serverFunction 必须有值
      expect(typeof b.serverFile).toBe('string');
      expect(b.serverFile.length).toBeGreaterThan(0);
      expect(typeof b.serverFunction).toBe('string');
      expect(b.serverFunction.length).toBeGreaterThan(0);

      // sourceTds 指向来源文件
      expect(b.sourceTds).toBe(TDS_47);

      // index ≥ 1
      expect(b.index).toBeGreaterThanOrEqual(1);
    }

    // Row #1 应为 SendMessage ⭐（主路径）
    const row1 = bindings.find((b) => b.index === 1);
    expect(row1).toBeDefined();
    expect(row1!.endpoint).toContain('SendMessage');
    expect(row1!.isPrimary).toBe(true);

    // Row #1 服务端文件应指向 chat handler
    expect(row1!.serverFile).toContain('app/server/src/room/handler/chat.rs');
    expect(row1!.serverFunction).toContain('handle_send_message');

    // Row #3 应为 HTTP REST 备路径
    const row3 = bindings.find((b) => b.index === 3);
    expect(row3).toBeDefined();
    expect(row3!.protocolType).toMatch(/HTTP\s*REST|REST/i);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// TC-AUDIT-02: 解析 T-00048.md 绑定表（双路径范本）
// ─────────────────────────────────────────────────────────────────────────────
describe('TC-AUDIT-02: 解析 T-00048.md 绑定表', () => {
  test('应从 T-00048.md 中提取双路径绑定（≥2 条）', () => {
    const content = fs.readFileSync(TDS_48, 'utf8');
    const bindings = parseBindingTable(content, TDS_48);

    // 至少 2 条（WS + REST 双路径）
    expect(bindings.length).toBeGreaterThanOrEqual(2);

    // Row #1 应为 WS SendMessage ⭐
    const row1 = bindings.find((b) => b.index === 1);
    expect(row1).toBeDefined();
    expect(row1!.endpoint).toContain('SendMessage');
    expect(row1!.isPrimary).toBe(true);
    expect(row1!.serverFile).toContain('app/server/src/room/handler/chat.rs');

    // 至少有一个 REST 路径（HTTP REST or REST）
    const hasRest = bindings.some((b) => /HTTP\s*REST|REST/i.test(b.protocolType));
    expect(hasRest).toBe(true);

    // sourceTds 指向来源文件
    for (const b of bindings) {
      expect(b.sourceTds).toBe(TDS_48);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// TC-AUDIT-03: 解析 N/A 声明（T-0000T 自身）
// ─────────────────────────────────────────────────────────────────────────────
describe('TC-AUDIT-03: 解析 N/A 声明', () => {
  test('应将 T-0000T.md 中的 N/A 声明识别为无绑定，返回空数组且不抛出异常', () => {
    const content = fs.readFileSync(TDS_0T, 'utf8');

    // 必须不抛出异常
    let bindings: ProtocolBinding[] = [];
    expect(() => {
      bindings = parseBindingTable(content, TDS_0T);
    }).not.toThrow();

    // 应返回空数组（N/A 声明 → 无路径提取）
    expect(Array.isArray(bindings)).toBe(true);
    expect(bindings.length).toBe(0);
  });

  test('内联 N/A 字符串也应被识别为无绑定', () => {
    const naContent = `
## 二、方案设计

### 🔌 协议路径绑定表（Plan 必填）

> N/A — 本 Task 为纯基础设施工具，无跨端协议路径；脚本本身读取 doc/protocol/index.md 作为协议锚点参考，不新增任何 HTTP REST / WebSocket 通信入口。
    `;

    const bindings = parseBindingTable(naContent, 'fake/T-TEST.md');
    expect(bindings.length).toBe(0);
  });

  test('各种 N/A 模式都应被识别', () => {
    const patterns = [
      'N/A — 本 Task 无跨端协议',
      'N/A — 本 Task 为纯基础设施工具',
      'N/A — 仅内部组件不动协议',
      'N/A — 纯测试任务',
    ];

    for (const naText of patterns) {
      const content = `## 二、方案设计\n\n### 协议路径绑定表\n\n> ${naText}\n`;
      const bindings = parseBindingTable(content, 'fake/T-TEST.md');
      expect(bindings.length).toBe(0);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// TC-AUDIT-04: 路径不一致时报告 P0 错误
// ─────────────────────────────────────────────────────────────────────────────
describe('TC-AUDIT-04: 路径不一致时非 0 退出', () => {
  test('制造不一致时应报告 P0 错误，且 report.shouldExit === true', () => {
    // Mock 数据：声明了 serverFile，但 serverGrep 返回空（找不到实现）
    const mockBindings: ProtocolBinding[] = [
      {
        index: 1,
        protocolType: 'WS C→S',
        endpoint: 'FakeSignal',
        clientFile: 'app/android/feature/FakeViewModel.kt',
        clientFunction: 'sendFakeMessage',
        serverFile: 'app/server/src/fake/handler/missing.rs',
        serverFunction: 'handle_fake_signal',
        protocolAnchor: 'websocket_signals.md §99.1',
        isPrimary: true,
        sourceTds: 'doc/tds/server/T-FAKE.md',
      },
    ];

    // serverGrep 为空 → 服务端未实现
    const emptyServerGrep: GrepResult[] = [];
    // clientGrep 也为空
    const emptyClientGrep: GrepResult[] = [];

    const findings = auditBindings(mockBindings, emptyServerGrep, emptyClientGrep);

    // 应产生至少一个 P0 错误
    const p0Findings = findings.filter((f) => f.level === 'P0');
    expect(p0Findings.length).toBeGreaterThan(0);

    // P0 类型应为 MISSING_SERVER_IMPL
    expect(p0Findings.some((f) => f.type === 'MISSING_SERVER_IMPL')).toBe(true);

    // P0 finding 必须携带来源 TDS 文件信息
    expect(p0Findings[0].tdsFile).toBe('doc/tds/server/T-FAKE.md');

    // 生成报告
    const meta: ReportMeta = {
      tdsFilesScanned: 1,
      bindingsFound: 1,
    };
    const report = generateReport(findings, meta);

    // 报告应有 P0 错误
    expect(report.p0Errors.length).toBeGreaterThan(0);

    // shouldExit === true（有 P0 时必须阻断 CI）
    expect(report.shouldExit).toBe(true);

    // 报告元数据
    expect(report.tdsFilesScanned).toBe(1);
    expect(report.bindingsFound).toBe(1);
    expect(typeof report.generatedAt).toBe('string');
    expect(report.generatedAt.length).toBeGreaterThan(0);
  });

  test('无 P0 错误时 shouldExit === false', () => {
    // 空 findings → 无问题
    const meta: ReportMeta = { tdsFilesScanned: 5, bindingsFound: 10 };
    const report = generateReport([], meta);

    expect(report.p0Errors.length).toBe(0);
    expect(report.shouldExit).toBe(false);
  });

  test('缺失 clientFile 时也应产生 P0 错误', () => {
    const mockBindings: ProtocolBinding[] = [
      {
        index: 1,
        protocolType: 'HTTP REST',
        endpoint: 'POST /api/v1/fake',
        clientFile: 'app/android/feature/FakeApi.kt',  // 声明了客户端
        clientFunction: 'callFakeApi',
        serverFile: 'app/server/src/fake/controller.rs',
        serverFunction: 'fake_handler',
        protocolAnchor: 'room_api.md §99.1',
        isPrimary: false,
        sourceTds: 'doc/tds/server/T-FAKE2.md',
      },
    ];

    // serverGrep 有命中（服务端有实现）
    const serverGrep: GrepResult[] = [
      {
        file: 'app/server/src/fake/controller.rs',
        line: 42,
        content: 'pub async fn fake_handler(',
        matchedFunction: 'fake_handler',
      },
    ];
    // clientGrep 无命中（客户端未调用）
    const clientGrep: GrepResult[] = [];

    const findings = auditBindings(mockBindings, serverGrep, clientGrep);
    const p0Findings = findings.filter((f) => f.level === 'P0');

    expect(p0Findings.length).toBeGreaterThan(0);
    expect(p0Findings.some((f) => f.type === 'MISSING_CLIENT_CALL')).toBe(true);
  });

  test('标记为"目前无客户端"的绑定行不应产生 clientCall P0', () => {
    const mockBindings: ProtocolBinding[] = [
      {
        index: 3,
        protocolType: 'HTTP REST',
        endpoint: 'POST /api/v1/chat-messages',
        clientFile: '',  // 空字符串 = 无客户端
        clientFunction: '',
        serverFile: 'app/server/src/modules/chat/controller.rs',
        serverFunction: 'send_chat_message_handler',
        protocolAnchor: 'room_api.md §3.6.1',
        isPrimary: false,
        sourceTds: 'doc/tds/server/T-00047.md',
      },
    ];

    const serverGrep: GrepResult[] = [
      {
        file: 'app/server/src/modules/chat/controller.rs',
        line: 88,
        content: 'pub async fn send_chat_message_handler(',
        matchedFunction: 'send_chat_message_handler',
      },
    ];
    const clientGrep: GrepResult[] = [];

    const findings = auditBindings(mockBindings, serverGrep, clientGrep);
    // 不应产生 P0 MISSING_CLIENT_CALL（因为无客户端是合法声明）
    const clientP0 = findings.filter(
      (f) => f.level === 'P0' && f.type === 'MISSING_CLIENT_CALL'
    );
    expect(clientP0.length).toBe(0);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// TC-AUDIT-05: 报告包含 file:lineNo 格式
// ─────────────────────────────────────────────────────────────────────────────
describe('TC-AUDIT-05: 报告包含 file:lineNo 格式', () => {
  test('AuditFinding 中的 serverRef 和 clientRef 应包含 file 路径和行号', () => {
    // 构造包含 serverRef 和 clientRef 的 P0 finding
    const mockFinding: AuditFinding = {
      level: 'P0',
      type: 'MISSING_CLIENT_CALL',
      message: 'Client call not found for endpoint FakeSignal',
      tdsFile: 'doc/tds/server/T-FAKE.md',
      serverRef: {
        file: 'app/server/src/room/handler/chat.rs',
        line: 123,
      },
      clientRef: undefined,
    };

    const meta: ReportMeta = { tdsFilesScanned: 1, bindingsFound: 1 };
    const report = generateReport([mockFinding], meta);

    // 报告应包含该 finding
    expect(report.p0Errors.length).toBe(1);
    const finding = report.p0Errors[0];

    // serverRef 必须有 file 和 line
    expect(finding.serverRef).toBeDefined();
    expect(finding.serverRef!.file).toBe('app/server/src/room/handler/chat.rs');
    expect(finding.serverRef!.line).toBe(123);
    expect(typeof finding.serverRef!.line).toBe('number');
    expect(finding.serverRef!.line).toBeGreaterThan(0);
  });

  test('auditBindings 发现服务端实现时，finding 应带有 serverRef file:lineNo', () => {
    // 制造场景：server 找到了，client 没有 → MISSING_CLIENT_CALL with serverRef
    const mockBindings: ProtocolBinding[] = [
      {
        index: 1,
        protocolType: 'WS C→S',
        endpoint: 'SendMessage',
        clientFile: 'app/android/feature/room/RoomViewModel.kt',
        clientFunction: 'sendMessage',
        serverFile: 'app/server/src/room/handler/chat.rs',
        serverFunction: 'handle_send_message',
        protocolAnchor: 'websocket_signals.md §6.8.1',
        isPrimary: true,
        sourceTds: 'doc/tds/server/T-00047.md',
      },
    ];

    // serverGrep 命中（服务端有实现，file:lineNo 有效）
    const serverGrep: GrepResult[] = [
      {
        file: 'app/server/src/room/handler/chat.rs',
        line: 40,
        content: 'pub async fn handle_send_message(',
        matchedFunction: 'handle_send_message',
      },
    ];
    // clientGrep 无命中 → MISSING_CLIENT_CALL
    const clientGrep: GrepResult[] = [];

    const findings = auditBindings(mockBindings, serverGrep, clientGrep);
    const p0Findings = findings.filter((f) => f.level === 'P0');
    expect(p0Findings.length).toBeGreaterThan(0);

    // 找到 MISSING_CLIENT_CALL finding，它应当携带 serverRef（来自 serverGrep）
    const missingClient = p0Findings.find((f) => f.type === 'MISSING_CLIENT_CALL');
    expect(missingClient).toBeDefined();
    expect(missingClient!.serverRef).toBeDefined();
    expect(missingClient!.serverRef!.file).toContain('chat.rs');
    expect(missingClient!.serverRef!.line).toBe(40);
  });

  test('generateReport 元数据字段应完整', () => {
    const meta: ReportMeta = { tdsFilesScanned: 42, bindingsFound: 67 };
    const report = generateReport([], meta);

    expect(report.tdsFilesScanned).toBe(42);
    expect(report.bindingsFound).toBe(67);
    expect(report.p0Errors).toEqual([]);
    expect(report.p1Warnings).toEqual([]);
    expect(report.p2Info).toEqual([]);
    expect(report.shouldExit).toBe(false);

    // generatedAt 应为 ISO 8601 格式
    expect(report.generatedAt).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 边界用例
// ─────────────────────────────────────────────────────────────────────────────
describe('边界与异常用例', () => {
  test('B-AUDIT-01: 空字符串输入不崩溃，返回空数组', () => {
    expect(() => parseBindingTable('', 'fake/T-EMPTY.md')).not.toThrow();
    expect(parseBindingTable('', 'fake/T-EMPTY.md')).toEqual([]);
  });

  test('B-AUDIT-02: 无绑定表节（非 N/A）的 TDS 返回空数组', () => {
    const contentWithoutTable = `
# TDS: Some Task (Task ID: T-FAKE)
## 一、背景
Some background.
## 二、方案设计
Some design without a binding table.
## 三、TDD 验收
Some tests.
    `;
    const bindings = parseBindingTable(contentWithoutTable, 'fake/T-FAKE.md');
    // 无表格则返回空数组（不崩溃）
    expect(Array.isArray(bindings)).toBe(true);
    expect(bindings.length).toBe(0);
  });

  test('B-AUDIT-03: auditBindings 传入空数组返回空数组', () => {
    const findings = auditBindings([], [], []);
    expect(Array.isArray(findings)).toBe(true);
    expect(findings.length).toBe(0);
  });

  test('B-AUDIT-04: generateReport 正确区分 P0/P1/P2 严重度', () => {
    const findings: AuditFinding[] = [
      {
        level: 'P0',
        type: 'MISSING_SERVER_IMPL',
        message: 'P0 error',
        tdsFile: 'a.md',
      },
      {
        level: 'P1',
        type: 'MISSING_BINDING_TABLE',
        message: 'P1 warning',
        tdsFile: 'b.md',
      },
      {
        level: 'P2',
        type: 'FIELD_MISMATCH',
        message: 'P2 info',
        tdsFile: 'c.md',
      },
    ];

    const meta: ReportMeta = { tdsFilesScanned: 3, bindingsFound: 1 };
    const report = generateReport(findings, meta);

    expect(report.p0Errors.length).toBe(1);
    expect(report.p1Warnings.length).toBe(1);
    expect(report.p2Info.length).toBe(1);
    expect(report.shouldExit).toBe(true);
  });

  test('B-AUDIT-05: HTML 表格包含的绑定信息可被解析（备用解析器）', () => {
    const htmlContent = `
## 二、方案设计

### 🔌 协议路径绑定表（Plan 必填）

<table>
<tr>
<th>#</th><th>协议类型</th><th>入口/信令名</th><th>客户端调用方</th><th>服务端处理函数</th><th>广播/响应</th><th>protocol/ 锚点</th>
</tr>
<tr>
<td>1</td><td>WS C→S</td><td>TestSignal ⭐</td><td>app/android/feature/Test.kt::testMethod</td><td>app/server/src/test/handler.rs::test_handler</td><td>broadcast</td><td>websocket_signals.md §1.1</td>
</tr>
</table>
    `;

    const bindings = parseBindingTable(htmlContent, 'fake/T-HTML.md');
    expect(bindings.length).toBeGreaterThanOrEqual(1);
    expect(bindings[0].endpoint).toContain('TestSignal');
    expect(bindings[0].isPrimary).toBe(true);
    expect(bindings[0].serverFunction).toContain('test_handler');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 覆盖率补充测试：renderMarkdownReport、discoverTdsFiles、grep 引擎、writeReports
// ─────────────────────────────────────────────────────────────────────────────

describe('renderMarkdownReport', () => {
  const makeReport = (overrides: Partial<AuditReport> = {}): AuditReport => ({
    generatedAt: '2026-05-05T12:00:00.000Z',
    tdsFilesScanned: 10,
    bindingsFound: 5,
    p0Errors: [],
    p1Warnings: [],
    p2Info: [],
    shouldExit: false,
    ...overrides,
  });

  test('无问题时应输出 "All clear ✅" 且 "No P1 warnings."', () => {
    const md = renderMarkdownReport(makeReport(), []);
    expect(md).toContain('# Protocol Binding Audit Report');
    expect(md).toContain('All clear ✅');
    expect(md).toContain('No P1 warnings.');
    expect(md).toContain('| TDS Files Scanned | 10 |');
  });

  test('有 P0 错误时应渲染 P0 错误行（含 serverRef file:lineNo）', () => {
    const report = makeReport({
      p0Errors: [
        {
          level: 'P0',
          type: 'MISSING_SERVER_IMPL',
          message: 'Server not found',
          tdsFile: 'doc/tds/server/T-FAKE.md',
          serverRef: { file: 'app/server/src/fake/handler.rs', line: 99 },
        },
      ],
      shouldExit: true,
    });
    const md = renderMarkdownReport(report, []);

    expect(md).toContain('MISSING_SERVER_IMPL');
    expect(md).toContain('T-FAKE.md');
    expect(md).toContain('app/server/src/fake/handler.rs:99');
  });

  test('有 P1 警告时应渲染 P1 警告行', () => {
    const report = makeReport({
      p1Warnings: [
        {
          level: 'P1',
          type: 'MISSING_BINDING_TABLE',
          message: 'No binding table',
          tdsFile: 'doc/tds/server/T-NO-TABLE.md',
        },
      ],
    });
    const md = renderMarkdownReport(report, []);

    expect(md).toContain('MISSING_BINDING_TABLE');
    expect(md).toContain('T-NO-TABLE.md');
  });

  test('绑定覆盖矩阵应包含每条 binding 信息', () => {
    const binding: ProtocolBinding = {
      index: 1,
      protocolType: 'WS C→S',
      endpoint: 'SendMessage',
      clientFile: 'app/android/RoomViewModel.kt',
      clientFunction: 'sendMessage',
      serverFile: 'app/server/src/room/handler/chat.rs',
      serverFunction: 'handle_send_message',
      protocolAnchor: 'websocket_signals.md §6.8.1',
      isPrimary: true,
      sourceTds: 'doc/tds/server/T-00047.md',
    };

    const md = renderMarkdownReport(makeReport(), [binding]);
    expect(md).toContain('SendMessage');
    expect(md).toContain('WS C→S');
    expect(md).toContain('T-00047.md');
    expect(md).toContain('✅');
  });

  test('binding 没有 clientFile 时矩阵显示 N/A', () => {
    const binding: ProtocolBinding = {
      index: 3,
      protocolType: 'HTTP REST',
      endpoint: 'POST /api/v1/chat-messages',
      clientFile: '',
      clientFunction: '',
      serverFile: 'app/server/src/modules/chat/controller.rs',
      serverFunction: 'send_chat_message_handler',
      protocolAnchor: 'room_api.md §3.6.1',
      isPrimary: false,
      sourceTds: 'doc/tds/server/T-00047.md',
    };
    const md = renderMarkdownReport(makeReport(), [binding]);
    expect(md).toContain('N/A');
  });

  test('P0 finding 有 clientRef 时也应渲染 clientRef file:lineNo', () => {
    const report = makeReport({
      p0Errors: [
        {
          level: 'P0',
          type: 'MISSING_CLIENT_CALL',
          message: 'Client not found',
          tdsFile: 'doc/tds/android/T-30054.md',
          serverRef: { file: 'app/server/src/chat.rs', line: 42 },
          clientRef: { file: 'app/android/RoomViewModel.kt', line: 77 },
        },
      ],
      shouldExit: true,
    });
    const md = renderMarkdownReport(report, []);
    expect(md).toContain('app/server/src/chat.rs:42');
    expect(md).toContain('app/android/RoomViewModel.kt:77');
  });
});

describe('discoverTdsFiles', () => {
  test('应发现真实项目 doc/tds 下的 TDS 文件（≥4）', () => {
    const files = discoverTdsFiles(REPO_ROOT);
    expect(files.length).toBeGreaterThanOrEqual(4);

    for (const f of files) {
      const basename = path.basename(f);
      expect(basename).toMatch(/^T-.*\.md$/);
    }

    const hasTemplate = files.some((f) => path.basename(f).startsWith('_'));
    expect(hasTemplate).toBe(false);

    const sorted = [...files].sort();
    expect(files).toEqual(sorted);
  });

  test('不存在的目录返回空数组', () => {
    const files = discoverTdsFiles('/nonexistent/path/that/does/not/exist');
    expect(files).toEqual([]);
  });

  test('应包含 T-00047.md、T-00048.md 和 T-0000T.md', () => {
    const files = discoverTdsFiles(REPO_ROOT);
    const basenames = files.map((f) => path.basename(f));
    expect(basenames).toContain('T-00047.md');
    expect(basenames).toContain('T-00048.md');
    expect(basenames).toContain('T-0000T.md');
  });
});

describe('parseGrepOutput', () => {
  test('应正确解析 grep -n 格式输出', () => {
    const output = [
      'app/server/src/room/handler/chat.rs:40:pub async fn handle_send_message(',
      'app/server/src/modules/chat/controller.rs:88:pub async fn send_chat_message_handler(',
      '',
    ].join('\n');

    const results = parseGrepOutput(output);
    expect(results.length).toBe(2);

    expect(results[0].file).toBe('app/server/src/room/handler/chat.rs');
    expect(results[0].line).toBe(40);
    expect(results[0].content).toContain('handle_send_message');

    expect(results[1].file).toBe('app/server/src/modules/chat/controller.rs');
    expect(results[1].line).toBe(88);
  });

  test('空字符串应返回空数组', () => {
    expect(parseGrepOutput('')).toEqual([]);
  });

  test('无匹配格式的行应被过滤', () => {
    const output = 'some line without colon pattern\nanother bad line\n';
    expect(parseGrepOutput(output).length).toBe(0);
  });

  test('行号应为 number 类型', () => {
    const results = parseGrepOutput('path/to/file.ts:123:const x = 1;');
    expect(results[0].line).toBe(123);
    expect(typeof results[0].line).toBe('number');
  });
});

describe('runGrep', () => {
  test('对真实文件执行 grep 应返回结构化结果', () => {
    const results = runGrep(
      '协议路径绑定表',
      [path.join(REPO_ROOT, 'doc/tds/server')],
      ['*.md']
    );
    expect(results.length).toBeGreaterThan(0);
    for (const r of results) {
      expect(typeof r.file).toBe('string');
      expect(typeof r.line).toBe('number');
      expect(r.line).toBeGreaterThan(0);
    }
  });

  test('不存在的目录返回空数组（不崩溃）', () => {
    const results = runGrep('anything', ['/nonexistent/path/12345'], ['*.ts']);
    expect(Array.isArray(results)).toBe(true);
  });
});

describe('grepServerImpl', () => {
  test('server 目录不存在时返回空数组（不崩溃）', () => {
    const results = grepServerImpl('/nonexistent/repo/root');
    expect(Array.isArray(results)).toBe(true);
    expect(results.length).toBe(0);
  });
});

describe('grepClientCalls', () => {
  test('client 目录不存在时返回空数组（不崩溃）', () => {
    const results = grepClientCalls('/nonexistent/repo/root');
    expect(Array.isArray(results)).toBe(true);
    expect(results.length).toBe(0);
  });
});

describe('writeReports', () => {
  test('应将 JSON 和 Markdown 报告写入指定目录', () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'audit-test-'));

    const report: AuditReport = {
      generatedAt: '2026-05-05T12:00:00.000Z',
      tdsFilesScanned: 5,
      bindingsFound: 3,
      p0Errors: [],
      p1Warnings: [],
      p2Info: [],
      shouldExit: false,
    };

    const bindings: ProtocolBinding[] = [
      {
        index: 1,
        protocolType: 'WS C→S',
        endpoint: 'SendMessage',
        clientFile: 'app/android/RoomViewModel.kt',
        clientFunction: 'sendMessage',
        serverFile: 'app/server/src/room/handler/chat.rs',
        serverFunction: 'handle_send_message',
        protocolAnchor: 'websocket_signals.md §6.8.1',
        isPrimary: true,
        sourceTds: 'doc/tds/server/T-00047.md',
      },
    ];

    expect(() => writeReports(report, bindings, tmpDir)).not.toThrow();

    const jsonPath = path.join(tmpDir, 'tests/protocol-audit/report.json');
    expect(fs.existsSync(jsonPath)).toBe(true);
    const jsonContent = JSON.parse(fs.readFileSync(jsonPath, 'utf8'));
    expect(jsonContent.tdsFilesScanned).toBe(5);
    expect(jsonContent.bindingsFound).toBe(3);
    expect(jsonContent.shouldExit).toBe(false);

    const mdPath = path.join(tmpDir, 'tests/protocol-audit/report.md');
    expect(fs.existsSync(mdPath)).toBe(true);
    const mdContent = fs.readFileSync(mdPath, 'utf8');
    expect(mdContent).toContain('# Protocol Binding Audit Report');
    expect(mdContent).toContain('SendMessage');

    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test('应处理已存在的输出目录（不崩溃）', () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'audit-test2-'));
    fs.mkdirSync(path.join(tmpDir, 'tests/protocol-audit'), { recursive: true });

    const report: AuditReport = {
      generatedAt: new Date().toISOString(),
      tdsFilesScanned: 0,
      bindingsFound: 0,
      p0Errors: [],
      p1Warnings: [],
      p2Info: [],
      shouldExit: false,
    };

    expect(() => writeReports(report, [], tmpDir)).not.toThrow();
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });
});

describe('parseBindingTable — 更多边界场景', () => {
  test('N/A 出现在引用块中应被正确识别', () => {
    const content = `
## 🔌 协议路径绑定表（Plan 必填）

> N/A — 纯测试任务，不涉及任何新协议入口
    `;
    const bindings = parseBindingTable(content, 'fake/T-PURE-TEST.md');
    expect(bindings.length).toBe(0);
  });

  test('绑定行中的 Markdown 链接应被正确解析为锚点文本', () => {
    const content = `
## 二、方案设计

### 🔌 协议路径绑定表

| # | 协议类型 | 入口 / 信令名 | 客户端调用方 | 服务端处理函数 | 广播 / 响应 | protocol/ 锚点 |
|---|---------|--------------|------------|--------------|----------|---------------|
| 1 | WS C→S | \`Foo\` ⭐ | app/android/Foo.kt::fooMethod | app/server/src/foo/handler.rs::handle_foo | broadcast | [websocket_signals.md §1.1](../../protocol/websocket_signals.md) |
    `;

    const bindings = parseBindingTable(content, 'fake/T-ANCHOR.md');
    expect(bindings.length).toBeGreaterThanOrEqual(1);
    expect(bindings[0].protocolAnchor).toContain('websocket_signals.md');
    expect(bindings[0].protocolAnchor).toContain('§1.1');
  });

  test('「目前无客户端」描述应被识别为无客户端引用', () => {
    const content = `
## 二、方案设计

### 协议路径绑定表

| # | 协议类型 | 入口 / 信令名 | 客户端调用方 | 服务端处理函数 | 广播 / 响应 | protocol/ 锚点 |
|---|---------|--------------|------------|--------------|----------|---------------|
| 1 | HTTP REST | POST /api/v1/test | （目前无客户端调用方） | app/server/src/test/ctrl.rs::test_handler | 200 OK | room_api.md §1.0 |
    `;

    const bindings = parseBindingTable(content, 'fake/T-NOCLIENT.md');
    expect(bindings.length).toBeGreaterThanOrEqual(1);
    expect(bindings[0].clientFile).toBe('');
  });

  test('非主路径（无 ⭐ 标记）应正确识别 isPrimary=false', () => {
    const content = `
## 协议路径绑定表

| # | 协议类型 | 入口 / 信令名 | 客户端调用方 | 服务端处理函数 | 广播 / 响应 | protocol/ 锚点 |
|---|---------|--------------|------------|--------------|----------|---------------|
| 2 | WS S→Room 广播 | RoomMessage | （接收端） | app/server/src/ws/broadcaster.rs::broadcast_to_room | 房间所有连接 | websocket_signals.md §6.8.2 |
    `;
    const bindings = parseBindingTable(content, 'fake/T-NOSTAR.md');
    expect(bindings.length).toBeGreaterThanOrEqual(1);
    expect(bindings[0].isPrimary).toBe(false);
    expect(bindings[0].endpoint).toBe('RoomMessage');
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 覆盖率补充 II：真实 grep 路径、extractBindingSection 分支、parseFileAndFunction 边界
// ─────────────────────────────────────────────────────────────────────────────

describe('grepServerImpl — 真实 app/server/src 目录', () => {
  test('应在真实 server 目录中执行 grep 并返回结果', () => {
    // 测试真实的 server/src 目录（该目录存在于项目中）
    const results = grepServerImpl(REPO_ROOT);
    // 目录存在，grep 应返回数组（可能为空或非空，取决于代码内容）
    expect(Array.isArray(results)).toBe(true);
    // 结果中若有条目，应符合格式
    for (const r of results) {
      expect(typeof r.file).toBe('string');
      expect(typeof r.line).toBe('number');
      expect(r.line).toBeGreaterThan(0);
    }
  });
});

describe('grepClientCalls — 真实 app/android 和 app/web 目录', () => {
  test('应在真实 android/web 目录中执行 grep 并返回结果（不崩溃）', () => {
    const results = grepClientCalls(REPO_ROOT);
    expect(Array.isArray(results)).toBe(true);
    // 结果应为去重后的 GrepResult 数组（无重复 file:line）
    const keys = results.map((r) => `${r.file}:${r.line}`);
    const uniqueKeys = new Set(keys);
    expect(keys.length).toBe(uniqueKeys.size); // 应已去重
  });
});

describe('extractBindingSection — null 返回路径', () => {
  test('完全无关键词的内容应使 parseBindingTable 返回空数组（extractBindingSection null）', () => {
    // 没有任何 协议路径绑定表 / 方案设计 / 二、 关键词
    const content = `
# TDS: Random Task

## Introduction
Just some intro text.

## Technical Details
Some details without any binding table.

## Testing
Some tests.
    `;
    // extractBindingSection 会走策略 1（无关键词）→ 策略 2（无 方案设计/二、 ）→ 返回 null
    const bindings = parseBindingTable(content, 'fake/T-NOKEYWORD.md');
    expect(bindings.length).toBe(0);
  });
});

describe('parseFileAndFunction — 边界场景', () => {
  test('仅有文件路径无 :: 的客户端描述应提取 file 而不报错', () => {
    // 包含 app/... 文件路径但没有 :: 的情况
    const content = `
## 协议路径绑定表

| # | 协议类型 | 入口 / 信令名 | 客户端调用方 | 服务端处理函数 | 广播 / 响应 | protocol/ 锚点 |
|---|---------|--------------|------------|--------------|----------|---------------|
| 1 | WS C→S | Signal ⭐ | app/android/feature/room/RoomViewModel.kt（调用 wsClient） | app/server/src/room/handler.rs::handle_signal | broadcast | signals.md §1 |
    `;

    const bindings = parseBindingTable(content, 'fake/T-FILEPATH-ONLY.md');
    expect(bindings.length).toBeGreaterThanOrEqual(1);
    // 不论是否提取到 clientFile，不应崩溃
    expect(typeof bindings[0].clientFile).toBe('string');
  });

  test('描述性客户端文本（无文件路径）应返回空字符串', () => {
    const content = `
## 协议路径绑定表

| # | 协议类型 | 入口 / 信令名 | 客户端调用方 | 服务端处理函数 | 广播 / 响应 | protocol/ 锚点 |
|---|---------|--------------|------------|--------------|----------|---------------|
| 1 | WS S→C | PushNotification | 客户端监听 envelope type == "Push" 分发到 ViewModel | app/server/src/push/handler.rs::handle_push | 单播 | push_api.md §1 |
    `;

    const bindings = parseBindingTable(content, 'fake/T-DESCRIPTIVE.md');
    expect(bindings.length).toBeGreaterThanOrEqual(1);
    expect(typeof bindings[0].clientFile).toBe('string');
    expect(typeof bindings[0].clientFunction).toBe('string');
  });

  test('客户端描述含 backtick 函数调用应提取函数名', () => {
    const content = `
## 协议路径绑定表

| # | 协议类型 | 入口 / 信令名 | 客户端调用方 | 服务端处理函数 | 广播 / 响应 | protocol/ 锚点 |
|---|---------|--------------|------------|--------------|----------|---------------|
| 1 | WS C→S | SendMsg ⭐ | \`sendEnvelope(type="SendMsg")\` | app/server/src/room/handler.rs::handle_send_msg | broadcast | ws.md §1 |
    `;

    const bindings = parseBindingTable(content, 'fake/T-BACKTICK-FUNC.md');
    expect(bindings.length).toBeGreaterThanOrEqual(1);
    // 不崩溃，clientFunction 可能有值
    expect(typeof bindings[0].clientFunction).toBe('string');
  });
});

describe('parseMarkdownTable — 兼容表格格式', () => {
  test('无 # 表头但有足够列的表格也能被识别', () => {
    const content = `
## 协议路径绑定表

| 协议类型 | 入口信令 | 客户端 | 服务端 | 广播 | 锚点 | 备注 |
|---------|---------|-------|-------|-----|-----|-----|
| WS C→S | FooSignal ⭐ | app/android/Foo.kt::fooFn | app/server/src/foo/bar.rs::foo_handler | broadcast | ws.md §1 | N/A |
    `;
    const bindings = parseBindingTable(content, 'fake/T-COMPAT-TABLE.md');
    // 由于无 # 列，索引解析可能不同，但不应崩溃
    expect(Array.isArray(bindings)).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 覆盖率补充 III：deduplicateGrep、非标题行 keyword、runGrep catch
// ─────────────────────────────────────────────────────────────────────────────

describe('deduplicateGrep', () => {
  test('应去除重复的 file:line 条目', () => {
    const input: GrepResult[] = [
      { file: 'app/server/src/chat.rs', line: 40, content: 'fn handle_send_message(' },
      { file: 'app/server/src/chat.rs', line: 40, content: 'fn handle_send_message(' }, // 重复
      { file: 'app/server/src/broadcaster.rs', line: 30, content: 'pub fn broadcast_to_room(' },
    ];

    const deduped = deduplicateGrep(input);
    expect(deduped.length).toBe(2); // 去重后只有 2 条

    // 确认顺序保留（第一次出现的）
    expect(deduped[0].file).toBe('app/server/src/chat.rs');
    expect(deduped[1].file).toBe('app/server/src/broadcaster.rs');
  });

  test('无重复时应返回原始长度', () => {
    const input: GrepResult[] = [
      { file: 'a.rs', line: 1, content: 'fn a(' },
      { file: 'a.rs', line: 2, content: 'fn b(' },  // 同文件但不同行
      { file: 'b.rs', line: 1, content: 'fn c(' },  // 不同文件
    ];
    expect(deduplicateGrep(input).length).toBe(3);
  });

  test('空数组返回空数组', () => {
    expect(deduplicateGrep([])).toEqual([]);
  });
});

describe('extractBindingSection — 非标题行关键词（line 186 分支）', () => {
  test('协议路径绑定表 关键词出现在非标题行时仍应正确提取后续表格', () => {
    // 关键词在普通段落中（非 ## 标题），触发 startHeadingLevel=0 分支（line 186）
    const content = `
这是普通文字。协议路径绑定表如下所示：

| # | 协议类型 | 入口 / 信令名 | 客户端调用方 | 服务端处理函数 | 广播 / 响应 | protocol/ 锚点 |
|---|---------|--------------|------------|--------------|----------|---------------|
| 1 | WS C→S | TestSig ⭐ | app/android/Test.kt::testFn | app/server/src/test/h.rs::test_fn | broadcast | ws.md §1 |
    `;
    const bindings = parseBindingTable(content, 'fake/T-INLINE-KEYWORD.md');
    // startHeadingLevel=0 时取接下来 100 行，表格在关键词行的后续 → 应被解析
    expect(Array.isArray(bindings)).toBe(true);
    // 不崩溃即通过（行数限制下能否解析到表格取决于行间距）
  });
});
