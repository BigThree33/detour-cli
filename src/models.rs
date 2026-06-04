use serde::{Deserialize, Serialize};

// One message or event captured from an AI-assisted conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationEvent {
    pub role: String,
    pub content: String,
    pub timestamp: Option<String>,
    pub source: Option<String>,
}

impl ConversationEvent {
    // Build a minimal conversation event when only role and content are known.
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            timestamp: None,
            source: None,
        }
    }

    // Return true when the event has no useful message content.
    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }
}

// A normalized conversation input that can come from JSONL, JSON, Markdown, or stdin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationInput {
    pub session_id: Option<String>,
    pub project: Option<String>,
    pub events: Vec<ConversationEvent>,
}

impl ConversationInput {
    // Build a conversation input from already-parsed events.
    pub fn from_events(events: Vec<ConversationEvent>) -> Self {
        Self {
            session_id: None,
            project: None,
            events,
        }
    }

    // Return true when there are no meaningful conversation events.
    pub fn is_empty(&self) -> bool {
        self.events.iter().all(ConversationEvent::is_empty)
    }
}

// Severity level for a captured mistake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
}

impl Default for Severity {
    // Use medium as the default because most captured mistakes are worth remembering.
    fn default() -> Self {
        Self::Medium
    }
}

// One reusable lesson learned from a wrong turn, failed command, or missed constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mistake {
    pub id: String,
    pub title: String,
    pub symptom: String,
    pub wrong_turn: String,
    pub root_cause: String,
    pub correction: String,
    pub prevention_rule: String,
    pub evidence: Vec<String>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub severity: Severity,
}

impl Mistake {
    // Return true when the mistake has the minimum fields needed to be useful later.
    pub fn has_required_fields(&self) -> bool {
        !self.title.trim().is_empty()
            && !self.symptom.trim().is_empty()
            && !self.root_cause.trim().is_empty()
            && !self.correction.trim().is_empty()
            && !self.prevention_rule.trim().is_empty()
    }
}

// A Markdown-ready document that groups related mistakes under one topic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MistakeDoc {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub mistakes: Vec<Mistake>,
    pub created_at: Option<String>,
    pub source_session: Option<String>,
    pub tags: Vec<String>,
}

impl MistakeDoc {
    // Return the number of valid mistakes that are ready to be rendered or saved.
    pub fn valid_mistake_count(&self) -> usize {
        self.mistakes
            .iter()
            .filter(|mistake| mistake.has_required_fields())
            .count()
    }

    // Return true when the document has a title and at least one useful mistake.
    pub fn is_renderable(&self) -> bool {
        !self.title.trim().is_empty() && self.valid_mistake_count() > 0
    }
}

// The structured output produced by a mistake generator before Markdown rendering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MistakeCollection {
    pub documents: Vec<MistakeDoc>,
}

impl MistakeCollection {
    // Return true when there are no documents that can be rendered.
    pub fn is_empty(&self) -> bool {
        self.documents.iter().all(|document| !document.is_renderable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversation_event_round_trips_as_json() {
        let event = ConversationEvent::new("user", "please capture this lesson");

        let json = serde_json::to_string(&event).expect("serialize event");
        let parsed: ConversationEvent = serde_json::from_str(&json).expect("deserialize event");

        assert_eq!(parsed, event);
    }

    #[test]
    fn mistake_requires_prevention_rule() {
        let mistake = Mistake {
            id: "m1".to_string(),
            title: "Missing hook check".to_string(),
            symptom: "The design assumed a hook without checking support.".to_string(),
            wrong_turn: "Started from wrapper design only.".to_string(),
            root_cause: "The integration surface was not verified first.".to_string(),
            correction: "Prefer Claude Code PreCompact when available.".to_string(),
            prevention_rule: String::new(),
            evidence: vec![],
            tags: vec!["claude-code".to_string()],
            severity: Severity::High,
        };

        assert!(!mistake.has_required_fields());
    }

    #[test]
    fn mistake_doc_counts_only_valid_mistakes() {
        let valid_mistake = Mistake {
            id: "m1".to_string(),
            title: "Keep CLI output structured".to_string(),
            symptom: "LLM cannot reliably parse long logs.".to_string(),
            wrong_turn: "Returned only human-readable text.".to_string(),
            root_cause: "The CLI contract was not machine-friendly.".to_string(),
            correction: "Add --json for LLM-facing commands.".to_string(),
            prevention_rule: "Any LLM-facing command must support JSON output.".to_string(),
            evidence: vec!["detour capture --json".to_string()],
            tags: vec!["llm-interface".to_string()],
            severity: Severity::Medium,
        };

        let invalid_mistake = Mistake {
            prevention_rule: String::new(),
            ..valid_mistake.clone()
        };

        let document = MistakeDoc {
            id: "doc1".to_string(),
            title: "LLM Interface Lessons".to_string(),
            summary: "Lessons about CLI output contracts.".to_string(),
            mistakes: vec![valid_mistake, invalid_mistake],
            created_at: None,
            source_session: None,
            tags: vec!["llm-interface".to_string()],
        };

        assert_eq!(document.valid_mistake_count(), 1);
        assert!(document.is_renderable());
    }
}
