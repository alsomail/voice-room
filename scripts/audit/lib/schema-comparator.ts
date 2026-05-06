/**
 * Schema 比较器
 * 比对提取出的字段集合与 JSON Schema 定义
 * T-00106
 */
import * as fs from 'fs';

export interface FieldMismatch {
  schemaFile: string;
  fieldName: string;
  issue: 'camelCase' | 'undefined_field' | 'missing_field';
  sourceFile: string;
  sourceLine?: number;
  severity: 'P0' | 'P1';
}

/**
 * 检查字段名是否为 camelCase（P0 违规）
 * 规则：小写字母开头 + 包含大写字母（排除 PascalCase）
 */
export function isCamelCase(fieldName: string): boolean {
  // 必须包含大写字母
  if (!/[A-Z]/.test(fieldName)) return false;
  // 以大写字母开头属于 PascalCase，不算 camelCase 违规
  if (/^[A-Z]/.test(fieldName)) return false;
  return true;
}

/**
 * 加载 JSON Schema 并提取其中定义的字段名集合
 */
export function loadSchemaFields(schemaFilePath: string): Set<string> {
  const fields = new Set<string>();

  try {
    const content = fs.readFileSync(schemaFilePath, 'utf-8');
    const schema = JSON.parse(content);

    if (schema.properties) {
      for (const key of Object.keys(schema.properties)) {
        fields.add(key);
      }
    }

    // 递归提取 definitions/$defs 中的字段名
    const defs = schema.definitions || schema.$defs || {};
    for (const def of Object.values(defs) as Record<string, unknown>[]) {
      const typedDef = def as { properties?: Record<string, unknown> };
      if (typedDef.properties) {
        for (const key of Object.keys(typedDef.properties)) {
          fields.add(key);
        }
      }
    }
  } catch {
    // 忽略解析失败
  }

  return fields;
}

/**
 * 比对字段集合与 schema，返回 mismatch 列表
 * - camelCase 字段 → P0
 * - schema 中未定义的字段 → P1
 */
export function compareWithSchema(
  extractedFields: Map<string, string[]>,
  schemaFields: Set<string>,
  schemaFile: string
): FieldMismatch[] {
  const mismatches: FieldMismatch[] = [];

  for (const [fieldName, sources] of extractedFields) {
    const sourceRef = sources[0] || 'unknown';
    const sourceLineMatch = sourceRef.match(/:(\d+)$/);
    const sourceLine = sourceLineMatch ? parseInt(sourceLineMatch[1]) : undefined;
    const cleanSourceFile = sourceRef.replace(/:(\d+)$/, '');

    if (isCamelCase(fieldName)) {
      mismatches.push({
        schemaFile,
        fieldName,
        issue: 'camelCase',
        sourceFile: cleanSourceFile,
        sourceLine,
        severity: 'P0',
      });
    } else if (!schemaFields.has(fieldName)) {
      mismatches.push({
        schemaFile,
        fieldName,
        issue: 'undefined_field',
        sourceFile: cleanSourceFile,
        sourceLine,
        severity: 'P1',
      });
    }
  }

  return mismatches;
}
