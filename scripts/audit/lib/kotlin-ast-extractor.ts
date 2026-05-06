/**
 * Kotlin AST 字段提取器
 * 解析 @SerializedName("field_name") 注解
 * T-00106
 */
import * as fs from 'fs';

/**
 * 提取 @SerializedName("...") 中的字段名
 */
export function extractKotlinSerializedNameFields(source: string): string[] {
  const fields: string[] = [];
  const pattern = /@SerializedName\s*\(\s*"([^"]+)"\s*\)/g;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(source)) !== null) {
    fields.push(match[1]);
  }

  return [...new Set(fields)];
}

/**
 * 从文件路径列表中提取 Android 字段集合
 * 返回 Map<字段名, 来源文件列表>
 */
export function extractAndroidFields(filePaths: string[]): Map<string, string[]> {
  const result = new Map<string, string[]>();

  for (const filePath of filePaths) {
    try {
      const source = fs.readFileSync(filePath, 'utf-8');
      const lines = source.split('\n');
      const fields = extractKotlinSerializedNameFields(source);

      for (const field of fields) {
        const lineNo = lines.findIndex(l => l.includes(`"${field}"`)) + 1;
        if (!result.has(field)) result.set(field, []);
        result.get(field)!.push(`${filePath}:${lineNo}`);
      }
    } catch {
      // 忽略读取失败的文件
    }
  }

  return result;
}
