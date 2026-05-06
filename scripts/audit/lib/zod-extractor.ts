/**
 * Zod schema 字段提取器
 * 解析 TypeScript z.object({...}) 中的字段名
 * T-00106
 */
import * as fs from 'fs';

/**
 * 提取 z.object({...}) 中的顶层 key
 * 手动扫描花括号平衡，仅取第一层字段
 */
export function extractZodObjectFields(source: string): string[] {
  const fields: string[] = [];

  const objectStart = /z\.object\s*\(\s*\{/g;
  let startMatch: RegExpExecArray | null;

  while ((startMatch = objectStart.exec(source)) !== null) {
    // 找到 { 的位置后开始扫描
    const openBrace = source.indexOf('{', startMatch.index + startMatch[0].indexOf('{'));
    let depth = 0;
    let i = openBrace;
    let bodyEnd = openBrace;

    while (i < source.length) {
      if (source[i] === '{') depth++;
      else if (source[i] === '}') {
        depth--;
        if (depth === 0) { bodyEnd = i; break; }
      }
      i++;
    }

    // 只提取第一层的 key（不进入嵌套对象）
    const objectBody = source.slice(openBrace + 1, bodyEnd);
    extractTopLevelKeys(objectBody, fields);
  }

  return [...new Set(fields)];
}

/**
 * 从对象体字符串中提取顶层 key（跳过嵌套对象/数组内容）
 */
function extractTopLevelKeys(body: string, fields: string[]): void {
  // 逐行匹配：行首的 identifier: 或 "identifier": 模式
  // 仅在顶层（depth=0，即没有未闭合的 {} 或 []）
  let depth = 0;
  let inString = false;
  let escape = false;
  let lineBuffer = '';

  for (let i = 0; i < body.length; i++) {
    const ch = body[i];

    if (escape) { escape = false; lineBuffer += ch; continue; }
    if (ch === '\\' && inString) { escape = true; lineBuffer += ch; continue; }
    if (ch === '"') { inString = !inString; lineBuffer += ch; continue; }
    if (inString) { lineBuffer += ch; continue; }

    if (ch === '{' || ch === '[') { depth++; lineBuffer += ch; continue; }
    if (ch === '}' || ch === ']') { depth--; lineBuffer += ch; continue; }

    if (ch === '\n' || ch === ',') {
      if (depth === 0) {
        // 尝试从 lineBuffer 提取 key
        const trimmed = lineBuffer.trim();
        // 匹配 identifier: 或 "identifier":
        const unquotedKey = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_]*)\s*:/);
        const quotedKey = trimmed.match(/^"([^"]+)"\s*:/);
        if (unquotedKey) fields.push(unquotedKey[1]);
        else if (quotedKey) fields.push(quotedKey[1]);
      }
      lineBuffer = '';
      continue;
    }

    lineBuffer += ch;
  }

  // 处理最后一行
  if (depth === 0 && lineBuffer.trim()) {
    const trimmed = lineBuffer.trim();
    const unquotedKey = trimmed.match(/^([a-zA-Z_][a-zA-Z0-9_]*)\s*:/);
    const quotedKey = trimmed.match(/^"([^"]+)"\s*:/);
    if (unquotedKey) fields.push(unquotedKey[1]);
    else if (quotedKey) fields.push(quotedKey[1]);
  }
}

/**
 * 从文件路径列表中提取 Web 字段集合
 * 返回 Map<字段名, 来源文件列表>
 */
export function extractWebFields(filePaths: string[]): Map<string, string[]> {
  const result = new Map<string, string[]>();

  for (const filePath of filePaths) {
    try {
      const source = fs.readFileSync(filePath, 'utf-8');
      const lines = source.split('\n');
      const fields = extractZodObjectFields(source);

      for (const field of fields) {
        const lineNo = lines.findIndex(l => l.includes(field)) + 1;
        if (!result.has(field)) result.set(field, []);
        result.get(field)!.push(`${filePath}:${lineNo}`);
      }
    } catch {
      // 忽略读取失败的文件
    }
  }

  return result;
}
