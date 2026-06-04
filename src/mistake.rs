// 错误笔记模型
use serde::{Deserialize, Serialize};

/// A complete mistake notebook document rendered from one conversation.
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
    /// Create a mistake document with required metadata and empty optional fields.
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

/// One reusable lesson learned from a failed assumption, command, or workflow.
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
    /// Create a mistake item with required fields and empty evidence/tags.
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

/// Evidence attached to a mistake, such as a command result or file path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    pub kind: EvidenceKind,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Evidence {
    /// Create a command evidence item.
    pub fn command(value: impl Into<String>) -> Self {
        Self {
            kind: EvidenceKind::Command,
            value: value.into(),
            note: None,
        }
    }

    /// Create a file evidence item.
    pub fn file(value: impl Into<String>) -> Self {
        Self {
            kind: EvidenceKind::File,
            value: value.into(),
            note: None,
        }
    }
}

/// Evidence categories that help future search and rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Command,
    File,
    Output,
    Message,
    Other,
}

/// Severity of a mistake for sorting and future filtering.
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
