/**
 * Rust AST 字段提取器
 * 解析 serde_json::json!({...}) 宏和 #[serde(rename = "...")] 字段
 * T-00106
 */
import * as fs from 'fs';

/**
 * 提取 json!({...}) 宏中的顶层键
 * 仅提取直接在宏花括号内的字符串键（不递归嵌套对象）
 */
export function extractRustJsonMacroFields(source: string): string[] {
  const fields: string[] = [];

  // 找到 json!( 的开始位置，然后手动扫描平衡括号来提取宏内容
  const macroStart = /json!\s*\(/g;
  let startMatch: RegExpExecArray | null;

  while ((startMatch = macroStart.exec(source)) !== null) {
    // 从 `(` 后面开始扫描，找到对应的闭合 `)`
    let depth = 1;
    let i = startMatch.index + startMatch[0].length;
    const bodyStart = i;

    while (i < source.length && depth > 0) {
      if (source[i] === '(') depth++;
      else if (source[i] === ')') depth--;
      i++;
    }

    const macroBody = source.slice(bodyStart, i - 1);

    // 在宏体内找第一层花括号 { ... }
    const braceStart = macroBody.indexOf('{');
    if (braceStart === -1) continue;

    // 手动扫描花括号内容，只提取顶层键
    let braceDepth = 0;
    let inString = false;
    let escape = false;
    let topLevelContent = '';

    for (let j = braceStart; j < macroBody.length; j++) {
      const ch = macroBody[j];

      if (escape) { escape = false; continue; }
      if (ch === '\\' && inString) { escape = true; continue; }

      if (ch === '"' && braceDepth === 1) {
        inString = !inString;
        topLevelContent += ch;
        continue;
      }
      if (inString) { topLevelContent += ch; continue; }

      if (ch === '{') {
        braceDepth++;
        if (braceDepth > 1) continue; // 嵌套对象，跳过
      } else if (ch === '}') {
        braceDepth--;
        if (braceDepth === 0) break;
      }

      if (braceDepth === 1) topLevelContent += ch;
    }

    // 从顶层内容中提取 "key": 模式
    const keyPattern = /"([^"]+)"\s*:/g;
    let keyMatch: RegExpExecArray | null;
    while ((keyMatch = keyPattern.exec(topLevelContent)) !== null) {
      fields.push(keyMatch[1]);
    }
  }

  return [...new Set(fields)];
}

/**
 * 提取 struct 中 #[serde(rename = "field_name")] 的字段名
 */
export function extractRustSerdeFields(source: string): string[] {
  const fields: string[] = [];
  const pattern = /#\[serde\([^)]*rename\s*=\s*"([^"]+)"[^)]*\)\]/g;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(source)) !== null) {
    fields.push(match[1]);
  }

  return [...new Set(fields)];
}

/**
 * 从文件路径列表中提取 Rust 字段集合
 * 返回 Map<字段名, 来源文件列表>
 */
export function extractServerFields(filePaths: string[]): Map<string, string[]> {
  const result = new Map<string, string[]>();

  for (const filePath of filePaths) {
    try {
      const source = fs.readFileSync(filePath, 'utf-8');
      const lines = source.split('\n');

      const jsonFields = extractRustJsonMacroFields(source);
      for (const field of jsonFields) {
        const lineNo = lines.findIndex(l => l.includes(`"${field}"`)) + 1;
        if (!result.has(field)) result.set(field, []);
        result.get(field)!.push(`${filePath}:${lineNo}`);
      }

      const serdeFields = extractRustSerdeFields(source);
      for (const field of serdeFields) {
        const lineNo = lines.findIndex(l => l.includes(`rename = "${field}"`)) + 1;
        if (!result.has(field)) result.set(field, []);
        result.get(field)!.push(`${filePath}:${lineNo}`);
      }
    } catch {
      // 忽略读取失败的文件
    }
  }

  return result;
}
