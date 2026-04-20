//! 敏感词过滤模块
//!
//! 提供 `filter_content` 函数，将文本中的敏感词替换为 `***`。
//! MVP 阶段使用静态词表；生产环境应从配置/数据库加载。

/// 敏感词列表（占位，生产环境应从配置/数据库加载）
const SENSITIVE_WORDS: &[&str] = &["badword", "spam"];

/// 将文本中的敏感词替换为 `***`
///
/// # 示例
/// ```
/// use voice_room_server::room::filter::filter_content;
/// assert_eq!(filter_content("hello badword world"), "hello *** world");
/// ```
pub fn filter_content(content: &str) -> String {
    let mut result = content.to_string();
    for word in SENSITIVE_WORDS {
        result = result.replace(word, "***");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_content_replaces_badword() {
        let result = filter_content("hello badword world");
        assert_eq!(result, "hello *** world");
        assert!(!result.contains("badword"));
    }

    #[test]
    fn test_filter_content_replaces_spam() {
        let result = filter_content("this is spam content");
        assert_eq!(result, "this is *** content");
    }

    #[test]
    fn test_filter_content_no_sensitive_words() {
        let result = filter_content("hello world");
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_filter_content_empty_string() {
        let result = filter_content("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_filter_content_multiple_occurrences() {
        let result = filter_content("badword and badword again");
        assert_eq!(result, "*** and *** again");
    }
}
