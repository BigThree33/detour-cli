use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{ConversationEvent, Evidence, Mistake, MistakeDocument, Severity};

/// 控制规则版错题提取的参数。
#[derive(Debug, Clone, Copy)]
pub struct ExtractOptions {
    pub max_docs: usize,
}

impl Default for ExtractOptions {
    /// 构造本地规则提取的默认参数。
    fn default() -> Self {
        Self { max_docs: 5 }
    }
}

/// 使用本地规则从会话事件中提取错题集文档。
pub fn extract_mistake_documents(
    events: &[ConversationEvent],
    options: ExtractOptions,
) -> Vec<MistakeDocument> {
    if options.max_docs == 0 {
        return Vec::new();
    }

    let mut grouped: BTreeMap<String, Vec<Mistake>> = BTreeMap::new();

    for (index, event) in events.iter().enumerate() {
        if !looks_like_mistake_candidate(&event.content) {
            continue;
        }

        let tag = classify_tag(&event.content);
        let title = mistake_title(&tag, &event.content);
        let mut mistake = Mistake::new(
            format!("m{}", index + 1),
            title,
            summarize_content(&event.content),
            wrong_turn_for(&tag, &event.content),
            root_cause_for(&tag),
            correction_for(&tag),
            prevention_rule_for(&tag),
            severity_for(&event.content),
        );

        mistake.tags.push(tag.clone());
        mistake.evidence = evidence_window(events, index);

        grouped.entry(tag).or_default().push(mistake);
    }

    grouped
        .into_iter()
        .take(options.max_docs)
        .map(|(tag, mistakes)| {
            let title = document_title(&tag);
            let mut document = MistakeDocument::new(
                format!("doc-{}", slugify(&title)),
                title.clone(),
                format!("基于本地规则提取，并按 `{}` 主题聚合的错题集。", tag),
                generated_timestamp(),
                mistakes,
            );
            document.tags.push(tag);
            document
        })
        .collect()
}

/// 判断消息是否包含失败或修正相关关键词。
fn looks_like_mistake_candidate(content: &str) -> bool {
    let lower = content.to_lowercase();
    let keywords = [
        "error",
        "failed",
        "failure",
        "panic",
        "not found",
        "not recognized",
        "permission",
        "sandbox",
        "wrong",
        "bug",
        "失败",
        "报错",
        "错误",
        "异常",
        "崩溃",
        "找不到",
        "不识别",
        "权限",
        "沙箱",
    ];

    keywords.iter().any(|keyword| lower.contains(keyword))
}

/// 将消息分类为稳定的错题主题标签。
fn classify_tag(content: &str) -> String {
    let lower = content.to_lowercase();

    if contains_any(&lower, &["claude", "hook", "precompact", "compact"]) {
        "claude-code".to_string()
    } else if contains_any(&lower, &["cargo", "rust", "clap", "serde"]) {
        "rust-cli".to_string()
    } else if contains_any(&lower, &["path", "powershell", "cmd", "terminal", "cargo"]) {
        "shell-env".to_string()
    } else if contains_any(&lower, &["json", "schema", "llm", "mcp", "stdin"]) {
        "llm-interface".to_string()
    } else if contains_any(&lower, &["file", "folder", "directory", "gitignore"]) {
        "filesystem".to_string()
    } else {
        "general".to_string()
    }
}

/// 判断文本中是否出现任意关键词。
fn contains_any(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| text.contains(keyword))
}

/// 根据主题和消息内容生成短错题标题。
fn mistake_title(tag: &str, content: &str) -> String {
    format!("{}: {}", document_title(tag), summarize_content(content))
}

/// 根据主题标签返回可读文档标题。
fn document_title(tag: &str) -> String {
    match tag {
        "claude-code" => "Claude Code Hook 错题集".to_string(),
        "rust-cli" => "Rust CLI 错题集".to_string(),
        "shell-env" => "Shell 环境错题集".to_string(),
        "llm-interface" => "LLM 接口错题集".to_string(),
        "filesystem" => "文件系统错题集".to_string(),
        _ => "通用会话错题集".to_string(),
    }
}

/// 描述规则引擎识别到的错误路径。
fn wrong_turn_for(tag: &str, content: &str) -> String {
    format!(
        "会话在 `{}` 主题下遇到了问题，相关消息是：{}",
        tag,
        summarize_content(content)
    )
}

/// 根据主题标签推断根因。
fn root_cause_for(tag: &str) -> String {
    match tag {
        "claude-code" => "在依赖 Claude Code 集成行为或 hook 生命周期前，没有先验证实际支持情况。",
        "rust-cli" => "CLI 实现或 Rust 工具链行为需要拆成更小步骤逐步验证。",
        "shell-env" => "当前终端环境和预期环境不一致。",
        "llm-interface" => "LLM 与 CLI 之间缺少更严格的机器可读契约。",
        "filesystem" => "文件或目录行为需要更明确的路径和忽略规则处理。",
        _ => "会话中出现了值得沉淀为预防规则的问题。",
    }
    .to_string()
}

/// 根据主题标签推断修正方式。
fn correction_for(tag: &str) -> String {
    match tag {
        "claude-code" => "安装或依赖 hook 前，先确认当前 Claude Code 可用的集成入口。",
        "rust-cli" => "用 `cargo run -- ...`、`cargo fmt`、`cargo test` 分层验证 CLI。",
        "shell-env" => "先检查当前 shell、PATH、权限和终端重启状态，再判断是不是代码问题。",
        "llm-interface" => "面向模型的命令优先提供明确 JSON schema、`--stdin` 和 `--json` 输出。",
        "filesystem" => "新增、移动或生成文件前，先检查路径和忽略规则。",
        _ => "继续之前，先记录错误现象、根因、修正方式和下次预防规则。",
    }
    .to_string()
}

/// 根据主题标签推断下次预防规则。
fn prevention_rule_for(tag: &str) -> String {
    match tag {
        "claude-code" => {
            "接入 Claude Code 时，先检测支持的 hooks，再决定是否使用 wrapper 或 watcher 兜底。"
        }
        "rust-cli" => "做 Rust CLI 时，先用 help 输出验证每一层命令结构，再添加行为。",
        "shell-env" => "一个终端可用、另一个终端失败时，优先检查 PATH 刷新和 shell 差异。",
        "llm-interface" => "所有面向 LLM 的命令都应该有稳定 JSON 输入和 JSON 输出。",
        "filesystem" => "改动生成文件前，先检查 Git 状态和忽略规则。",
        _ => "在上下文压缩前，把重复失败转成明确的下次预防规则。",
    }
    .to_string()
}

/// 根据消息内容判断错题严重程度。
fn severity_for(content: &str) -> Severity {
    let lower = content.to_lowercase();

    if contains_any(
        &lower,
        &[
            "panic", "security", "secret", "delete", "rm -rf", "崩溃", "密钥",
        ],
    ) {
        Severity::High
    } else {
        Severity::Medium
    }
}

/// 收集附近消息作为错题证据。
fn evidence_window(events: &[ConversationEvent], index: usize) -> Vec<Evidence> {
    let start = index.saturating_sub(1);
    let end = (index + 2).min(events.len());

    events[start..end]
        .iter()
        .map(|event| {
            Evidence::message(format!(
                "{:?}: {}",
                event.role,
                summarize_content(&event.content)
            ))
        })
        .collect()
}

/// 压缩空白并裁剪长文本，用于标题和证据。
fn summarize_content(content: &str) -> String {
    let compact = content.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate(&compact, 160)
}

/// 在不截断 UTF-8 字节的前提下裁剪文本。
fn truncate(content: &str, max_chars: usize) -> String {
    let mut chars = content.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();

    if chars.next().is_some() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

/// 将标题转换成简单 ASCII ID 后缀。
fn slugify(input: &str) -> String {
    let mut slug = String::new();

    for character in input.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }

    slug.trim_matches('-').to_string()
}

/// 不引入时间库，生成简单时间戳字符串。
fn generated_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    format!("unix:{}", seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ConversationRole;

    #[test]
    fn extracts_rule_based_mistake_documents() {
        let events = vec![
            ConversationEvent::new(ConversationRole::User, "cargo is not recognized in VS Code"),
            ConversationEvent::new(
                ConversationRole::Assistant,
                "This is likely a PATH refresh issue after Rust install.",
            ),
        ];

        let documents = extract_mistake_documents(&events, ExtractOptions { max_docs: 5 });

        assert_eq!(documents.len(), 1);
        assert_eq!(documents[0].tags, vec!["rust-cli"]);
        assert_eq!(documents[0].mistakes.len(), 1);
    }

    #[test]
    fn respects_max_docs() {
        let events = vec![
            ConversationEvent::new(ConversationRole::Tool, "Claude hook failed before compact"),
            ConversationEvent::new(ConversationRole::Tool, "JSON schema error from stdin"),
        ];

        let documents = extract_mistake_documents(&events, ExtractOptions { max_docs: 1 });

        assert_eq!(documents.len(), 1);
    }

    #[test]
    fn ignores_clean_conversations() {
        let events = vec![ConversationEvent::new(
            ConversationRole::Assistant,
            "Everything completed successfully.",
        )];

        let documents = extract_mistake_documents(&events, ExtractOptions { max_docs: 5 });

        assert!(documents.is_empty());
    }
}
