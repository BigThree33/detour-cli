use std::collections::BTreeSet;
use std::io::{self, Read};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::bail;
use serde::Deserialize;

use crate::cli::{CaptureFormat, SaveArgs};
use crate::models::{Evidence, Mistake, MistakeDocument, Severity};
use crate::storage::{write_documents, SavedDocument, StoredDocumentFormat};

/// `detour save` 保存 LLM 生成错题集后的结果。
#[derive(Debug, serde::Serialize)]
pub struct SaveResult {
    pub ok: bool,
    pub dry_run: bool,
    pub source: String,
    pub output_dir: String,
    pub document_format: String,
    pub document_count: usize,
    pub saved_documents: Vec<SavedDocument>,
    pub documents: Vec<MistakeDocument>,
}

/// LLM 可以传入的错题集 JSON 包装格式。
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LlmSaveInput {
    Envelope { documents: Vec<LlmDocument> },
    Documents(Vec<LlmDocument>),
}

/// LLM 生成的单篇错题集文档，允许 detour 补齐部分 metadata。
#[derive(Debug, Deserialize)]
struct LlmDocument {
    #[serde(default)]
    id: Option<String>,
    title: String,
    #[serde(default)]
    summary: Option<String>,
    mistakes: Vec<LlmMistake>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    source_session: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

/// LLM 生成的单条错题，允许 detour 补齐 id、tags 和 severity。
#[derive(Debug, Deserialize)]
struct LlmMistake {
    #[serde(default)]
    id: Option<String>,
    title: String,
    symptom: String,
    wrong_turn: String,
    root_cause: String,
    correction: String,
    prevention_rule: String,
    #[serde(default)]
    evidence: Vec<Evidence>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    severity: Option<Severity>,
}

/// 从 stdin 读取 LLM 生成的错题集 JSON，并保存到本地目录。
pub fn save_from_args(args: SaveArgs) -> anyhow::Result<SaveResult> {
    if !args.stdin {
        bail!("save requires --stdin");
    }

    let dry_run = args.dry_run;
    let output_dir = args
        .out
        .unwrap_or_else(|| PathBuf::from(".detour").join("mistakes"));
    let stored_format = stored_format_from_capture_format(args.format);
    let document_format = stored_format.name().to_string();
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let documents = parse_llm_documents(&input, args.max_docs)?;
    let saved_documents = if dry_run {
        Vec::new()
    } else {
        write_documents(&documents, &output_dir, stored_format)?
    };

    Ok(SaveResult {
        ok: true,
        dry_run,
        source: "stdin".to_string(),
        output_dir: output_dir.display().to_string(),
        document_format,
        document_count: documents.len(),
        saved_documents,
        documents,
    })
}

/// 解析 LLM 输出，并转换成 detour 内部模型。
fn parse_llm_documents(input: &str, max_docs: usize) -> anyhow::Result<Vec<MistakeDocument>> {
    if max_docs == 0 {
        return Ok(Vec::new());
    }

    let input: LlmSaveInput = serde_json::from_str(input)?;
    let documents = match input {
        LlmSaveInput::Envelope { documents } => documents,
        LlmSaveInput::Documents(documents) => documents,
    };

    documents
        .into_iter()
        .take(max_docs)
        .enumerate()
        .map(|(index, document)| normalize_document(index, document))
        .collect()
}

/// 补齐单篇文档的 id、时间和 tags。
fn normalize_document(index: usize, document: LlmDocument) -> anyhow::Result<MistakeDocument> {
    let mut tag_set = document.tags.into_iter().collect::<BTreeSet<_>>();
    let mistakes = document
        .mistakes
        .into_iter()
        .enumerate()
        .map(|(mistake_index, mistake)| normalize_mistake(mistake_index, mistake))
        .collect::<Vec<_>>();

    for mistake in &mistakes {
        for tag in &mistake.tags {
            tag_set.insert(tag.clone());
        }
    }

    if tag_set.is_empty() {
        tag_set.insert("llm-generated".to_string());
    }

    let title = document.title;
    let summary = document
        .summary
        .unwrap_or_else(|| format!("由 LLM 生成的 `{title}` 错题集。"));
    let mut normalized = MistakeDocument::new(
        document
            .id
            .unwrap_or_else(|| format!("doc-{}-{}", index + 1, slugify(&title))),
        title,
        summary,
        document.created_at.unwrap_or_else(generated_timestamp),
        mistakes,
    );

    normalized.source_session = document.source_session;
    normalized.tags = tag_set.into_iter().collect();

    Ok(normalized)
}

/// 补齐单条错题的 id、tags 和 severity。
fn normalize_mistake(index: usize, mut mistake: LlmMistake) -> Mistake {
    if mistake.tags.is_empty() {
        mistake.tags.push("llm-generated".to_string());
    }

    let mut normalized = Mistake::new(
        mistake
            .id
            .unwrap_or_else(|| format!("m{}-{}", index + 1, slugify(&mistake.title))),
        mistake.title,
        mistake.symptom,
        mistake.wrong_turn,
        mistake.root_cause,
        mistake.correction,
        mistake.prevention_rule,
        mistake.severity.unwrap_or(Severity::Medium),
    );

    normalized.evidence = mistake.evidence;
    normalized.tags = mistake.tags;
    normalized
}

/// 将 CLI 的文档格式转换为存储层格式。
fn stored_format_from_capture_format(format: CaptureFormat) -> StoredDocumentFormat {
    match format {
        CaptureFormat::Markdown => StoredDocumentFormat::Markdown,
        CaptureFormat::Json => StoredDocumentFormat::Json,
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

    let slug = slug.trim_matches('-');

    if slug.is_empty() {
        "mistakes".to_string()
    } else {
        slug.to_string()
    }
}

/// 不引入时间库，生成简单时间戳字符串。
fn generated_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    format!("unix:{seconds}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_llm_generated_documents() {
        let input = r#"{
          "documents": [
            {
              "title": "Claude Code 压缩前错题集",
              "summary": "由 Claude 生成的踩坑复盘。",
              "tags": ["claude-code", "precompact"],
              "mistakes": [
                {
                  "title": "不要把 hook 命令误认为 LLM 推理",
                  "symptom": "误以为 extractor 会把规则发给 Claude Code。",
                  "wrong_turn": "混淆了 shell hook 和 LLM 生成流程。",
                  "root_cause": "没有拆开 Claude Code 宿主和 Claude LLM 的职责。",
                  "correction": "让 Claude 先生成结构化错题 JSON，再交给 detour 保存。",
                  "prevention_rule": "需要模型生成内容时，detour 应提供 save/import 通道。",
                  "tags": ["llm-interface"],
                  "severity": "high"
                }
              ]
            }
          ]
        }"#;

        let documents = parse_llm_documents(input, 5).unwrap();

        assert_eq!(documents.len(), 1);
        assert!(documents[0].tags.contains(&"claude-code".to_string()));
        assert!(documents[0].tags.contains(&"llm-interface".to_string()));
        assert_eq!(documents[0].mistakes[0].severity, Severity::High);
    }
}
