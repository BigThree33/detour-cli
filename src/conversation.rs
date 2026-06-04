// 输入会话模型
use serde::{Deserialize, Serialize};

/// One message or tool event from an AI-assisted conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationEvent {
    pub role: ConversationRole,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl ConversationEvent {
    /// Create a plain conversation event with only role and content.
    pub fn new(role: ConversationRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            timestamp: None,
            source: None,
        }
    }
}

/// Roles that can appear in a captured conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationRole {
    User,
    Assistant,
    Tool,
    System,
    Other,
}

/// Supported input formats for conversation material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationFormat {
    Json,
    JsonLines,
    Text,
}

/// Parse conversation material into normalized events.
pub fn parse_conversation(
    input: &str,
    format: ConversationFormat,
) -> anyhow::Result<Vec<ConversationEvent>> {
    match format {
        ConversationFormat::Json => parse_json(input),
        ConversationFormat::JsonLines => parse_json_lines(input),
        ConversationFormat::Text => Ok(parse_text(input)),
    }
}

/// Parse a JSON array of conversation events.
pub fn parse_json(input: &str) -> anyhow::Result<Vec<ConversationEvent>> {
    let events = serde_json::from_str(input)?;
    Ok(events)
}

/// Parse newline-delimited JSON conversation events.
pub fn parse_json_lines(input: &str) -> anyhow::Result<Vec<ConversationEvent>> {
    let mut events = Vec::new();

    for (index, line) in input.lines().enumerate() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        let event = serde_json::from_str(trimmed)
            .map_err(|error| anyhow::anyhow!("invalid JSONL at line {}: {}", index + 1, error))?;
        events.push(event);
    }

    Ok(events)
}

/// Wrap plain text as one conversation event.
pub fn parse_text(input: &str) -> Vec<ConversationEvent> {
    vec![ConversationEvent::new(
        ConversationRole::Other,
        input.trim(),
    )]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_lines() {
        let input = r#"{"role":"user","content":"fix the bug"}
{"role":"assistant","content":"the command failed","source":"claude"}"#;

        let events = parse_json_lines(input).expect("jsonl should parse");

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].role, ConversationRole::User);
        assert_eq!(events[1].source.as_deref(), Some("claude"));
    }

    #[test]
    fn reports_json_lines_line_number() {
        let input = r#"{"role":"user","content":"ok"}
not json"#;

        let error = parse_json_lines(input).expect_err("bad jsonl should fail");

        assert!(error.to_string().contains("line 2"));
    }

    #[test]
    fn wraps_plain_text() {
        let events = parse_text("  a raw transcript  ");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].role, ConversationRole::Other);
        assert_eq!(events[0].content, "a raw transcript");
    }
}
