// 错误笔记模型
use serde::{Deserialize, Serialize};

/// 从一次会话中生成的一篇完整错题集文档。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MistakeDocument {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub mistakes: Vec<Mistake>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_session: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl MistakeDocument {
    /// 使用必要元数据创建错题集文档，并初始化空的可选字段。
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        summary: impl Into<String>,
        created_at: impl Into<String>,
        mistakes: Vec<Mistake>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            summary: summary.into(),
            mistakes,
            created_at: created_at.into(),
            source_session: None,
            tags: Vec::new(),
        }
    }
}

/// 从错误假设、失败命令或工作流问题中提炼出的一条可复用错题。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mistake {
    pub id: String,
    pub title: String,
    pub symptom: String,
    pub wrong_turn: String,
    pub root_cause: String,
    pub correction: String,
    pub prevention_rule: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub severity: Severity,
}

impl Mistake {
    /// 使用必要字段创建一条错题，并初始化空证据和标签。
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        symptom: impl Into<String>,
        wrong_turn: impl Into<String>,
        root_cause: impl Into<String>,
        correction: impl Into<String>,
        prevention_rule: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            symptom: symptom.into(),
            wrong_turn: wrong_turn.into(),
            root_cause: root_cause.into(),
            correction: correction.into(),
            prevention_rule: prevention_rule.into(),
            evidence: Vec::new(),
            tags: Vec::new(),
            severity,
        }
    }
}

/// 附加在错题上的证据，例如命令结果或文件路径。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: EvidenceKind,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Evidence {
    /// 创建命令证据。
    pub fn command(value: impl Into<String>) -> Self {
        Self {
            kind: EvidenceKind::Command,
            value: value.into(),
            note: None,
        }
    }

    /// 创建文件证据。
    pub fn file(value: impl Into<String>) -> Self {
        Self {
            kind: EvidenceKind::File,
            value: value.into(),
            note: None,
        }
    }

    /// 创建消息证据。
    pub fn message(value: impl Into<String>) -> Self {
        Self {
            kind: EvidenceKind::Message,
            value: value.into(),
            note: None,
        }
    }
}

/// 帮助后续搜索和渲染的证据类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Command,
    File,
    Output,
    Message,
    Other,
}

/// 用于排序和后续过滤的错题严重程度。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_mistake_document() {
        let mut mistake = Mistake::new(
            "m1",
            "Wrong shell assumption",
            "cargo was not found in one shell",
            "Assumed every terminal had the same PATH",
            "VS Code inherited an old PATH",
            "Restart VS Code or update PATH",
            "When a desktop terminal works but VS Code does not, check PATH refresh first",
            Severity::Medium,
        );
        mistake.evidence.push(Evidence::command("cargo --version"));

        let document = MistakeDocument::new(
            "doc1",
            "Rust install mistakes",
            "Lessons from setting up Rust on Windows.",
            "2026-06-04T00:00:00Z",
            vec![mistake],
        );

        let json = serde_json::to_string(&document).expect("document should serialize");

        assert!(json.contains("Rust install mistakes"));
        assert!(json.contains("prevention_rule"));
        assert!(json.contains("medium"));
    }
}
