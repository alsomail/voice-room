/**
 * 协议路径绑定审计脚本
 * Task: T-0000T
 *
 * 解析所有 doc/tds/**\/T-*.md 第二节「协议路径绑定表」（HTML 表格 / Markdown 表格双解析），
 * grep server/client 实现入口，三方比对，输出 P0 报告，非 0 退出码（当有 P0 错误时）。
 *
 * 运行时依赖：仅 Node.js 内置模块（fs, path, child_process）+ typescript（已有）
 */

import * as fs from 'node:fs';
import * as path from 'node:path';
import { execSync } from 'node:child_process';

// ─────────────────────────────────────────────────────────────────────────────
// 类型定义（全部导出，供测试 import）
// ─────────────────────────────────────────────────────────────────────────────

export interface ProtocolBinding {
  index: number;
  protocolType: string;      // e.g. "WS C→S", "HTTP REST"
  endpoint: string;          // e.g. "SendMessage", "/api/v1/chat-messages"
  clientFile: string;        // 实际文件路径（空字符串 = 无客户端）
  clientFunction: string;    // 调用函数名
  serverFile: string;        // 实际文件路径
  serverFunction: string;    // 处理函数名
  protocolAnchor: string;    // doc/protocol/ 锚点
  isPrimary: boolean;        // ⭐ 主路径标记
  sourceTds: string;         // 来源 TDS 文件路径
}

export interface GrepResult {
  file: string;
  line: number;
  content: string;
  matchedFunction?: string;
}

export interface AuditFinding {
  level: 'P0' | 'P1' | 'P2';
  type: 'MISSING_SERVER_IMPL' | 'MISSING_CLIENT_CALL' | 'MISSING_BINDING_TABLE' | 'FIELD_MISMATCH';
  message: string;
  tdsFile: string;
  binding?: ProtocolBinding;
  serverRef?: { file: string; line: number };   // file:lineNo
  clientRef?: { file: string; line: number };   // file:lineNo
  protocolRef?: string;                          // protocol/ 锚点
}

export interface ReportMeta {
  tdsFilesScanned: number;
  bindingsFound: number;
}

export interface AuditReport {
  generatedAt: string;
  tdsFilesScanned: number;
  bindingsFound: number;
  p0Errors: AuditFinding[];
  p1Warnings: AuditFinding[];
  p2Info: AuditFinding[];
  shouldExit: boolean;  // true when p0Errors.length > 0
}

// ─────────────────────────────────────────────────────────────────────────────
// 常量
// ─────────────────────────────────────────────────────────────────────────────

/** N/A 声明识别模式 */
const NA_PATTERNS: RegExp[] = [
  /N\/A.*本\s*Task.*无跨端协议/,
  /N\/A.*仅.*内部.*不动协议/,
  /N\/A.*本\s*Task.*为.*基础设施/,
  /N\/A.*纯.*测试/,
  /N\/A.*纯.*文档清理/,
  /N\/A.*不涉及.*跨端/,
];

/** 标准化的「无客户端」声明标识词 */
const NO_CLIENT_MARKERS = [
  '目前无客户端',
  '当前无客户端',
  '无客户端',
  'no client',
  'n/a',
];

// ─────────────────────────────────────────────────────────────────────────────
// 核心函数：解析绑定表
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 从 TDS 内容中解析「协议路径绑定表」。
 *
 * 支持：
 * - Markdown 表格（| # | 协议类型 | ...）
 * - HTML 表格（<table><tr><td>...）备用解析
 * - N/A 声明识别（跳过，返回 []）
 *
 * @param tdsContent TDS 文件内容
 * @param tdsFilePath TDS 文件路径（用于 sourceTds 字段）
 * @returns ProtocolBinding 数组
 */
export function parseBindingTable(tdsContent: string, tdsFilePath: string): ProtocolBinding[] {
  if (!tdsContent || tdsContent.trim().length === 0) {
    return [];
  }

  // ① 提取「协议路径绑定表」章节内容
  const section = extractBindingSection(tdsContent);
  if (!section) {
    return [];
  }

  // ② 检查 N/A 声明
  if (isNaDeclaration(section)) {
    return [];
  }

  // ③ 先尝试 Markdown 表格解析
  const mdBindings = parseMarkdownTable(section, tdsFilePath);
  if (mdBindings.length > 0) {
    return mdBindings;
  }

  // ④ 降级：HTML 表格备用解析器
  const htmlBindings = parseHtmlTable(section, tdsFilePath);
  return htmlBindings;
}

/**
 * 提取「协议路径绑定表」章节。
 * 从匹配行到下一个同级/更高级标题之间的内容。
 */
function extractBindingSection(content: string): string | null {
  // 尝试找到包含「协议路径绑定表」的标题或段落
  const bindingTableKeyword = /协议路径绑定表/;

  // 策略 1：找到包含关键词的标题行
  const lines = content.split('\n');
  let startIdx = -1;
  let startHeadingLevel = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    // 检查是否为标题行（## 协议路径绑定表 或 ### 协议路径绑定表）
    const headingMatch = line.match(/^(#{1,6})\s.*协议路径绑定表/);
    if (headingMatch) {
      startIdx = i;
      startHeadingLevel = headingMatch[1].length;
      break;
    }
    // 检查是否为非标题行中包含关键词（如 > 引用块中的段落包含关键词）
    if (bindingTableKeyword.test(line) && startIdx === -1) {
      startIdx = i;
      startHeadingLevel = 0; // 非标题行
    }
  }

  // 策略 2：若没找到「协议路径绑定表」关键词，查找「方案设计」或「二、」章节
  if (startIdx === -1) {
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      if (/^#{1,3}.*(?:方案设计|二[、.。])/.test(line)) {
        startIdx = i;
        startHeadingLevel = (line.match(/^(#{1,3})/) || ['', '##'])[1].length;
        break;
      }
    }
  }

  if (startIdx === -1) {
    return null;
  }

  // 提取从 startIdx 到下一个同级/更高级标题之间的内容
  let endIdx = lines.length;
  if (startHeadingLevel > 0) {
    for (let i = startIdx + 1; i < lines.length; i++) {
      const headingMatch = lines[i].match(/^(#{1,6})\s/);
      if (headingMatch && headingMatch[1].length <= startHeadingLevel) {
        endIdx = i;
        break;
      }
    }
  } else {
    // 非标题行起点：取接下来 100 行
    endIdx = Math.min(startIdx + 100, lines.length);
  }

  return lines.slice(startIdx, endIdx).join('\n');
}

/**
 * 检查内容中是否包含 N/A 声明
 */
function isNaDeclaration(content: string): boolean {
  return NA_PATTERNS.some((pattern) => pattern.test(content));
}

/**
 * 解析 Markdown 表格，提取绑定行
 */
function parseMarkdownTable(section: string, tdsFilePath: string): ProtocolBinding[] {
  const lines = section.split('\n');
  const bindings: ProtocolBinding[] = [];

  // 找到表头行（包含 # | 协议类型 或 #|协议）
  let tableStartIdx = -1;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    // 表头行特征：以 | 开头且包含 # 和协议
    if (line.startsWith('|') && /[#＃]/.test(line) && /协议/.test(line)) {
      tableStartIdx = i;
      break;
    }
    // 兼容：以 | 开头且有足够多的 | 分隔符（表格行）
    if (line.startsWith('|') && (line.match(/\|/g) || []).length >= 5) {
      // 检查后续行是否有分隔符行
      if (i + 1 < lines.length && /^\|[\s\-:|]+\|/.test(lines[i + 1].trim())) {
        tableStartIdx = i;
        break;
      }
    }
  }

  if (tableStartIdx === -1) {
    return [];
  }

  // 跳过表头行和分隔行
  let dataStartIdx = tableStartIdx + 1;
  // 跳过分隔行（| --- | --- |）
  if (
    dataStartIdx < lines.length &&
    /^\|[\s\-:|]+\|/.test(lines[dataStartIdx].trim())
  ) {
    dataStartIdx++;
  }

  // 解析数据行
  for (let i = dataStartIdx; i < lines.length; i++) {
    const line = lines[i].trim();
    if (!line.startsWith('|')) break; // 表格结束
    if (/^\|[\s\-:|]+\|/.test(line)) continue; // 跳过分隔行

    const cells = splitTableRow(line);
    if (cells.length < 5) continue; // 需要至少 5 列

    const binding = parseBindingRow(cells, i, tdsFilePath);
    if (binding) {
      bindings.push(binding);
    }
  }

  return bindings;
}

/**
 * 拆分 Markdown 表格行为单元格数组
 * 处理单元格内的 `backtick`、**bold**、链接 [text](url) 等
 */
function splitTableRow(line: string): string[] {
  // 移除首尾的 |
  const trimmed = line.replace(/^\||\|$/g, '');
  // 按 | 分割（但要注意单元格内部不应有未转义的 |）
  const cells = trimmed.split('|').map((c) => c.trim());
  return cells;
}

/**
 * 从表格行单元格解析出 ProtocolBinding
 * 表头列顺序：# | 协议类型 | 入口/信令名 | 客户端调用方 | 服务端处理函数 | 广播/响应 | protocol/ 锚点
 */
function parseBindingRow(cells: string[], _rowIdx: number, tdsFilePath: string): ProtocolBinding | null {
  if (cells.length < 5) return null;

  // 列 0: # (索引/序号)
  const indexStr = cleanText(cells[0]);
  const index = parseInt(indexStr, 10);
  if (isNaN(index)) return null;

  // 列 1: 协议类型
  const protocolType = cleanText(cells[1]);
  if (!protocolType) return null;

  // 列 2: 入口/信令名
  const endpointRaw = cleanText(cells[2]);
  const isPrimary = endpointRaw.includes('⭐');
  const endpoint = endpointRaw.replace('⭐', '').trim();

  // 列 3: 客户端调用方
  const clientRaw = cleanText(cells[3]);
  const { file: clientFile, func: clientFunction } = parseFileAndFunction(clientRaw);

  // 列 4: 服务端处理函数
  const serverRaw = cleanText(cells[4]);
  const { file: serverFile, func: serverFunction } = parseFileAndFunction(serverRaw);

  if (!serverFile && !serverFunction) return null;

  // 列 6（若存在）: protocol/ 锚点
  const protocolAnchor = cells.length > 6 ? cleanText(cells[6]) : (cells.length > 5 ? cleanText(cells[5]) : '');

  return {
    index,
    protocolType,
    endpoint,
    clientFile,
    clientFunction,
    serverFile,
    serverFunction,
    protocolAnchor: extractAnchorText(protocolAnchor),
    isPrimary,
    sourceTds: tdsFilePath,
  };
}

/**
 * 从字符串中提取文件路径和函数名
 * 支持格式：
 * - `app/server/src/...::function_name`
 * - `app/server/src/...::function_name`（行 ~N）
 * - 纯文字描述（无法提取文件路径时返回空字符串）
 */
function parseFileAndFunction(raw: string): { file: string; func: string } {
  if (!raw) return { file: '', func: '' };

  // 检查是否为「无客户端」标记
  const lowerRaw = raw.toLowerCase();
  if (NO_CLIENT_MARKERS.some((marker) => lowerRaw.includes(marker.toLowerCase()))) {
    return { file: '', func: '' };
  }

  // 尝试匹配 file::function 格式（可能包含括号、行号等后缀）
  // 模式：路径类字符串（包含 / 和 .）::函数名
  const colonMatch = raw.match(/(app\/[^\s:（(「]+)::([\w_]+)/);
  if (colonMatch) {
    return {
      file: colonMatch[1].replace(/`/g, '').trim(),
      func: colonMatch[2].trim(),
    };
  }

  // 备用：只有文件路径（无 :: 分隔符）
  const fileMatch = raw.match(/(app\/[^\s「（(]+\.[a-z]+)/i);
  if (fileMatch) {
    return {
      file: fileMatch[1].replace(/`/g, '').trim(),
      func: '',
    };
  }

  // 尝试提取函数名（如 wsClient.sendEnvelope 等描述性文本）
  const funcMatch = raw.match(/`([\w.]+\([^)]*\))`/) || raw.match(/(\w+\()/) ;
  if (funcMatch) {
    return {
      file: '',
      func: funcMatch[1].replace(/[`(]/g, '').trim(),
    };
  }

  return { file: '', func: '' };
}

/**
 * 从锚点文本中提取纯文本（去除 Markdown 链接格式）
 * [websocket_signals.md §6.8.1](../../protocol/...) → websocket_signals.md §6.8.1
 */
function extractAnchorText(anchor: string): string {
  // [text](url) → text（注意：cleanText 先调用会预处理链接格式，此分支为防御性兜底）
  /* istanbul ignore next */
  const linkMatch = anchor.match(/\[([^\]]+)\]\([^)]+\)/);
  /* istanbul ignore next */
  if (linkMatch) {
    return linkMatch[1].trim();
  }
  return anchor.trim();
}

/**
 * 清理单元格文本：去除 Markdown 标记（backtick、**、_、行号注释等）
 */
function cleanText(raw: string): string {
  return raw
    .replace(/`([^`]*)`/g, '$1')  // 去除 backtick
    .replace(/\*\*([^*]*)\*\*/g, '$1')  // 去除 bold
    .replace(/\[([^\]]+)\]\([^)]+\)/g, '$1')  // Markdown 链接 → 文本
    .replace(/（行\s*~?\d+[-\d]*）/g, '')  // 去除行号注释（行 ~463-480）
    .replace(/（[^）]*）/g, ' ')  // 去除其他括号注释（宽松版）
    .replace(/\s+/g, ' ')
    .trim();
}

// ─────────────────────────────────────────────────────────────────────────────
// HTML 表格备用解析器
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 解析 HTML 表格格式的绑定表（备用）
 */
function parseHtmlTable(section: string, tdsFilePath: string): ProtocolBinding[] {
  const HTML_TABLE_REGEX = /<table[\s\S]*?<\/table>/gi;
  const TR_REGEX = /<tr[\s\S]*?<\/tr>/gi;
  const TD_REGEX = /<t[dh][^>]*>([\s\S]*?)<\/t[dh]>/gi;

  const bindings: ProtocolBinding[] = [];
  const tableMatches = section.match(HTML_TABLE_REGEX);
  if (!tableMatches) return [];

  for (const tableHtml of tableMatches) {
    const rows = tableHtml.match(TR_REGEX);
    if (!rows || rows.length < 2) continue; // 需要至少 1 数据行（跳过表头）

    // 跳过第一行（表头）
    for (let rowIdx = 1; rowIdx < rows.length; rowIdx++) {
      const row = rows[rowIdx];
      const cells: string[] = [];
      let tdMatch: RegExpExecArray | null;
      TD_REGEX.lastIndex = 0;
      while ((tdMatch = TD_REGEX.exec(row)) !== null) {
        cells.push(stripHtml(tdMatch[1]).trim());
      }

      if (cells.length < 5) continue;

      const binding = parseBindingRow(cells, rowIdx, tdsFilePath);
      if (binding) {
        bindings.push(binding);
      }
    }
  }

  return bindings;
}

/**
 * 去除 HTML 标签，还原纯文本
 */
function stripHtml(html: string): string {
  return html
    .replace(/<[^>]+>/g, '')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&amp;/g, '&')
    .replace(/&nbsp;/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

// ─────────────────────────────────────────────────────────────────────────────
// 核心函数：三角对账
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 对每条 ProtocolBinding 与 grep 结果进行三角对账，输出 AuditFinding[]
 *
 * 校验规则：
 * ① server 实现存在性：grep 结果中必须有对应函数名命中 → P0 MISSING_SERVER_IMPL
 * ② client 调用存在性：若 clientFile 非空，grep 结果必须有命中 → P0 MISSING_CLIENT_CALL
 */
export function auditBindings(
  bindings: ProtocolBinding[],
  serverGrep: GrepResult[],
  clientGrep: GrepResult[]
): AuditFinding[] {
  const findings: AuditFinding[] = [];

  for (const binding of bindings) {
    // ① 校验 server 实现
    const serverMatch = findGrepMatch(serverGrep, binding.serverFunction, binding.serverFile);

    if (!serverMatch) {
      findings.push({
        level: 'P0',
        type: 'MISSING_SERVER_IMPL',
        message: `Server implementation not found for endpoint "${binding.endpoint}" ` +
          `(expected: ${binding.serverFile}::${binding.serverFunction})`,
        tdsFile: binding.sourceTds,
        binding,
      });
    }

    // ② 校验 client 调用（仅当 clientFile 非空时）
    const hasClientRef = binding.clientFile.trim().length > 0;
    if (hasClientRef) {
      const clientMatch = findGrepMatch(clientGrep, binding.clientFunction, binding.clientFile);

      if (!clientMatch) {
        const finding: AuditFinding = {
          level: 'P0',
          type: 'MISSING_CLIENT_CALL',
          message: `Client call not found for endpoint "${binding.endpoint}" ` +
            `(expected: ${binding.clientFile}::${binding.clientFunction})`,
          tdsFile: binding.sourceTds,
          binding,
        };
        // 若 server 找到了，附加 serverRef file:lineNo
        if (serverMatch) {
          finding.serverRef = {
            file: serverMatch.file,
            line: serverMatch.line,
          };
        }
        findings.push(finding);
      }
    }
  }

  return findings;
}

/**
 * 在 grep 结果中查找匹配的条目
 * 匹配策略：函数名或文件路径命中
 */
function findGrepMatch(
  grepResults: GrepResult[],
  functionName: string,
  filePath: string
): GrepResult | undefined {
  if (!functionName && !filePath) return undefined;

  return grepResults.find((r) => {
    const fileMatch = filePath ? r.file.includes(filePath) || filePath.includes(r.file) : false;
    const funcMatch = functionName ? r.content.includes(functionName) ||
      (r.matchedFunction !== undefined && r.matchedFunction.includes(functionName)) : false;
    return fileMatch || funcMatch;
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// 核心函数：生成报告
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 从 AuditFinding[] 生成结构化 AuditReport
 */
export function generateReport(findings: AuditFinding[], meta: ReportMeta): AuditReport {
  const p0Errors = findings.filter((f) => f.level === 'P0');
  const p1Warnings = findings.filter((f) => f.level === 'P1');
  const p2Info = findings.filter((f) => f.level === 'P2');

  return {
    generatedAt: new Date().toISOString(),
    tdsFilesScanned: meta.tdsFilesScanned,
    bindingsFound: meta.bindingsFound,
    p0Errors,
    p1Warnings,
    p2Info,
    shouldExit: p0Errors.length > 0,
  };
}

/**
 * 将 AuditReport 渲染为 Markdown 格式
 */
export function renderMarkdownReport(report: AuditReport, bindings: ProtocolBinding[]): string {
  const lines: string[] = [
    '# Protocol Binding Audit Report',
    `Generated: ${report.generatedAt}`,
    '',
    '## Summary',
    '',
    '| Metric | Value |',
    '|--------|-------|',
    `| TDS Files Scanned | ${report.tdsFilesScanned} |`,
    `| Total Binding Rows | ${report.bindingsFound} |`,
    `| **P0 Issues** | **${report.p0Errors.length}** |`,
    `| P1 Issues | ${report.p1Warnings.length} |`,
    `| P2 Info | ${report.p2Info.length} |`,
    '',
  ];

  // P0 section
  lines.push('## ⛔ P0 Issues (Blocks CI)');
  lines.push('');
  if (report.p0Errors.length === 0) {
    lines.push('All clear ✅');
  } else {
    for (const issue of report.p0Errors) {
      const serverRefStr = issue.serverRef
        ? ` (server: \`${issue.serverRef.file}:${issue.serverRef.line}\`)`
        : '';
      const clientRefStr = issue.clientRef
        ? ` (client: \`${issue.clientRef.file}:${issue.clientRef.line}\`)`
        : '';
      lines.push(`- \`${issue.tdsFile}\` → **${issue.type}**: ${issue.message}${serverRefStr}${clientRefStr}`);
    }
  }
  lines.push('');

  // P1 section
  lines.push('## ⚠️ P1 Issues');
  lines.push('');
  if (report.p1Warnings.length === 0) {
    lines.push('No P1 warnings.');
  } else {
    for (const issue of report.p1Warnings) {
      lines.push(`- \`${issue.tdsFile}\` → ${issue.type}: ${issue.message}`);
    }
  }
  lines.push('');

  // Binding Coverage Matrix
  lines.push('## Binding Coverage Matrix');
  lines.push('');
  lines.push('| TDS File | Endpoint | Protocol | Server | Client |');
  lines.push('|----------|----------|----------|--------|--------|');
  for (const b of bindings) {
    const tdsFile = path.basename(b.sourceTds);
    lines.push(`| ${tdsFile} | ${b.endpoint} | ${b.protocolType} | ${b.serverFile ? '✅' : '❌'} | ${b.clientFile ? '✅' : 'N/A'} |`);
  }

  return lines.join('\n');
}

// ─────────────────────────────────────────────────────────────────────────────
// Grep 引擎（运行时使用，测试中可通过 mock 绕过）
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 执行 grep 命令，返回结构化结果
 */
export function runGrep(pattern: string, dirs: string[], includes: string[]): GrepResult[] {
  const includeArgs = includes.map((i) => `--include="${i}"`).join(' ');
  const dirArgs = dirs.join(' ');

  const cmd = `grep -rEn "${pattern}" ${dirArgs} ${includeArgs} 2>/dev/null || true`;

  try {
    const output = execSync(cmd, { encoding: 'utf8', maxBuffer: 10 * 1024 * 1024 });
    return parseGrepOutput(output);
  } catch {
    return [];
  }
}

/**
 * 解析 grep -n 输出（file:line:content 格式）
 * @internal 也可导出供测试使用
 */
export function parseGrepOutput(output: string): GrepResult[] {
  const results: GrepResult[] = [];
  for (const line of output.split('\n')) {
    const match = line.match(/^([^:]+):(\d+):(.*)$/);
    if (match) {
      results.push({
        file: match[1],
        line: parseInt(match[2], 10),
        content: match[3],
      });
    }
  }
  return results;
}

/**
 * 执行 server 端 grep（路由注册 + WS 信令分发）
 */
export function grepServerImpl(repoRoot: string): GrepResult[] {
  const serverDirs = [path.join(repoRoot, 'app/server/src')];
  // 过滤到实际存在的目录
  const existingDirs = serverDirs.filter((d) => fs.existsSync(d));
  if (existingDirs.length === 0) return [];

  const results: GrepResult[] = [];

  // Router::route 注册（HTTP 路由）
  results.push(
    ...runGrep(
      'Router::route|\\.route(|pub async fn|pub fn',
      existingDirs,
      ['*.rs']
    )
  );

  // WS 信令分发
  results.push(
    ...runGrep(
      'match.*envelope|r#type|SendMessage|RoomMessage|JoinRoom|LeaveRoom',
      existingDirs,
      ['*.rs']
    )
  );

  return deduplicateGrep(results);
}

/**
 * 执行 client 端 grep（Android + Web）
 */
export function grepClientCalls(repoRoot: string): GrepResult[] {
  const results: GrepResult[] = [];

  // Android WS 发送
  const androidDir = path.join(repoRoot, 'app/android/app/src/main');
  if (fs.existsSync(androidDir)) {
    results.push(
      ...runGrep(
        'wsClient\\.send|wsClient\\.sendEnvelope|wsClient\\.sendMessage|\\.sendEnvelope\\(',
        [androidDir],
        ['*.kt']
      )
    );
    // Retrofit HTTP 调用
    results.push(
      ...runGrep(
        '@(GET|POST|PUT|DELETE|PATCH)\\(',
        [androidDir],
        ['*.kt']
      )
    );
  }

  // Web apiClient 调用
  const webDir = path.join(repoRoot, 'app/web/src');
  if (fs.existsSync(webDir)) {
    results.push(
      ...runGrep(
        'apiClient\\.(get|post|put|delete|patch|request)\\(',
        [webDir],
        ['*.ts', '*.tsx']
      )
    );
  }

  return deduplicateGrep(results);
}

/**
 * 去重 grep 结果（按 file:line 去重）
 */
export function deduplicateGrep(results: GrepResult[]): GrepResult[] {
  const seen = new Set<string>();
  return results.filter((r) => {
    const key = `${r.file}:${r.line}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

// ─────────────────────────────────────────────────────────────────────────────
// TDS 文件发现
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 发现所有 TDS 文件（doc/tds 目录下的 T-开头 .md 文件）
 */
export function discoverTdsFiles(repoRoot: string): string[] {
  const tdsRoot = path.join(repoRoot, 'doc/tds');
  if (!fs.existsSync(tdsRoot)) return [];

  const files: string[] = [];
  walkDir(tdsRoot, (filePath) => {
    const basename = path.basename(filePath);
    if (basename.match(/^T-[^.]+\.md$/) && !basename.startsWith('_')) {
      files.push(filePath);
    }
  });

  return files.sort();
}

/**
 * 递归遍历目录，对每个文件调用回调
 */
function walkDir(dir: string, callback: (filePath: string) => void): void {
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walkDir(fullPath, callback);
    } else if (entry.isFile()) {
      callback(fullPath);
    }
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// 报告输出
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 将报告写入 tests/protocol-audit/ 目录
 */
export function writeReports(
  report: AuditReport,
  bindings: ProtocolBinding[],
  repoRoot: string
): void {
  const outputDir = path.join(repoRoot, 'tests/protocol-audit');
  fs.mkdirSync(outputDir, { recursive: true });

  // JSON 报告
  const jsonPath = path.join(outputDir, 'report.json');
  fs.writeFileSync(jsonPath, JSON.stringify(report, null, 2), 'utf8');

  // Markdown 报告
  const mdPath = path.join(outputDir, 'report.md');
  fs.writeFileSync(mdPath, renderMarkdownReport(report, bindings), 'utf8');

  console.log(`📄 JSON report: ${jsonPath}`);
  console.log(`📝 Markdown report: ${mdPath}`);
}

// ─────────────────────────────────────────────────────────────────────────────
// main() 入口
// ─────────────────────────────────────────────────────────────────────────────

/**
 * 主入口函数
 *
 * 流程：
 * 1. 发现所有 TDS 文件
 * 2. 解析所有绑定表
 * 3. grep server 实现入口
 * 4. grep client 调用入口
 * 5. 三角对账
 * 6. 生成报告（JSON + Markdown）
 * 7. P0 存在则 exit(1)
 */
/* istanbul ignore next */
export async function main(): Promise<void> {
  const isDryRun = process.argv.includes('--dry-run');
  const repoRoot = path.resolve(__dirname, '../..');

  console.log('🔍 Protocol Binding Audit');
  console.log(`   Root: ${repoRoot}`);
  if (isDryRun) {
    console.log('   Mode: --dry-run (no exit code enforcement)');
  }
  console.log('');

  // Step 1: 发现 TDS 文件
  const tdsFiles = discoverTdsFiles(repoRoot);
  console.log(`📂 Found ${tdsFiles.length} TDS files`);

  // Step 2: 解析绑定表
  const allBindings: ProtocolBinding[] = [];
  const naFiles: string[] = [];
  const missingTableFiles: string[] = [];

  for (const tdsFile of tdsFiles) {
    const content = fs.readFileSync(tdsFile, 'utf8');
    const bindings = parseBindingTable(content, tdsFile);

    if (bindings.length === 0) {
      // 区分 N/A 和真正缺失
      const section = extractBindingSection(content);
      if (section && isNaDeclaration(section)) {
        naFiles.push(tdsFile);
      } else {
        missingTableFiles.push(tdsFile);
      }
    } else {
      allBindings.push(...bindings);
    }
  }

  console.log(`✅ Bindings found: ${allBindings.length}`);
  console.log(`📋 N/A declarations: ${naFiles.length}`);
  console.log(`⚠️  Missing tables: ${missingTableFiles.length}`);
  console.log('');

  // Step 3: grep server 实现
  console.log('🔎 Grepping server implementations...');
  const serverGrep = grepServerImpl(repoRoot);
  console.log(`   Found ${serverGrep.length} server grep results`);

  // Step 4: grep client 调用
  console.log('🔎 Grepping client calls...');
  const clientGrep = grepClientCalls(repoRoot);
  console.log(`   Found ${clientGrep.length} client grep results`);
  console.log('');

  // Step 5: 三角对账
  console.log('⚖️  Running triangle audit...');
  let allFindings = auditBindings(allBindings, serverGrep, clientGrep);

  // 补充 P1：缺失绑定表（非 N/A）
  for (const missingFile of missingTableFiles) {
    allFindings.push({
      level: 'P1',
      type: 'MISSING_BINDING_TABLE',
      message: `TDS file has no binding table and no N/A declaration`,
      tdsFile: path.relative(repoRoot, missingFile),
    });
  }

  // Step 6: 生成报告
  const meta: ReportMeta = {
    tdsFilesScanned: tdsFiles.length,
    bindingsFound: allBindings.length,
  };
  const report = generateReport(allFindings, meta);

  if (!isDryRun) {
    writeReports(report, allBindings, repoRoot);
  }

  // 打印摘要
  console.log('');
  console.log('📊 Audit Summary:');
  console.log(`   P0 Errors:   ${report.p0Errors.length}`);
  console.log(`   P1 Warnings: ${report.p1Warnings.length}`);
  console.log(`   P2 Info:     ${report.p2Info.length}`);

  if (report.p0Errors.length > 0) {
    console.error('');
    console.error('⛔ P0 Issues:');
    for (const issue of report.p0Errors) {
      const serverRef = issue.serverRef ? ` [${issue.serverRef.file}:${issue.serverRef.line}]` : '';
      console.error(`   [P0] ${issue.type} in ${issue.tdsFile}: ${issue.message}${serverRef}`);
    }
    console.error('');
    console.error(`❌ Audit FAILED: ${report.p0Errors.length} P0 issue(s) found. Merge blocked.`);
    if (!isDryRun) {
      process.exit(1);
    }
  } else {
    console.log('');
    console.log(`✅ Audit PASSED: No P0 issues. ${report.p1Warnings.length} P1 warning(s).`);
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// CLI 入口（直接运行时）
// ─────────────────────────────────────────────────────────────────────────────
/* istanbul ignore next */
if (require.main === module) {
  main().catch((err) => {
    console.error('Fatal error:', err);
    process.exit(1);
  });
}
