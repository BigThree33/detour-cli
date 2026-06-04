use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use serde::Serialize;

use crate::cli::{CaptureArgs, CaptureFormat};
use crate::extractor::{extract_mistake_documents, ExtractOptions};
use crate::models::{parse_conversation, ConversationFormat, MistakeDocument};
use crate::storage::{write_documents, SavedDocument, StoredDocumentFormat};

/// `detour capture` 完成本地规则提取后的结果。
#[derive(Debug, Serialize)]
pub struct CaptureResult {
    pub ok: bool,
    pub dry_run: bool,
    pub source: String,
    pub output_dir: String,
    pub document_format: String,
    pub document_count: usize,
    pub saved_documents: Vec<SavedDocument>,
    pub documents: Vec<MistakeDocument>,
}

/// 读取输入、解析会话、提取错题集，并按需写入本地目录。
pub fn capture_from_args(args: CaptureArgs) -> anyhow::Result<CaptureResult> {
    if args.from.is_some() && args.stdin {
        bail!("use either --from <PATH> or --stdin, not both");
    }

    if args.from.is_none() && !args.stdin {
        bail!("capture requires --from <PATH> or --stdin");
    }

    let dry_run = args.dry_run;
    let max_docs = args.max_docs;
    let output_dir = args
        .out
        .unwrap_or_else(|| PathBuf::from(".detour").join("mistakes"));
    let stored_format = stored_format_from_capture_format(args.format);
    let document_format = stored_format.name().to_string();
    let (source, input, format) = read_input(args.from.as_deref(), args.stdin)?;
    let events = parse_conversation(&input, format)?;
    let documents = extract_mistake_documents(&events, ExtractOptions { max_docs });
    let saved_documents = if dry_run {
        Vec::new()
    } else {
        write_documents(&documents, &output_dir, stored_format)?
    };

    Ok(CaptureResult {
        ok: true,
        dry_run,
        source,
        output_dir: display_path(output_dir),
        document_format,
        document_count: documents.len(),
        saved_documents,
        documents,
    })
}

/// 从文件或 stdin 读取 capture 输入。
fn read_input(
    from: Option<&Path>,
    use_stdin: bool,
) -> anyhow::Result<(String, String, ConversationFormat)> {
    if let Some(path) = from {
        let input = fs::read_to_string(path)
            .with_context(|| format!("failed to read capture input from {}", path.display()))?;
        let format =
            detect_format_from_path(path).unwrap_or_else(|| detect_format_from_text(&input));
        return Ok((display_path(path), input, format));
    }

    if use_stdin {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .context("failed to read capture input from stdin")?;
        let format = detect_format_from_text(&input);
        return Ok(("stdin".to_string(), input, format));
    }

    bail!("capture requires --from <PATH> or --stdin")
}

/// 根据文件扩展名识别会话输入格式。
fn detect_format_from_path(path: &Path) -> Option<ConversationFormat> {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("json") => Some(ConversationFormat::Json),
        Some("jsonl") => Some(ConversationFormat::JsonLines),
        Some("ndjson") => Some(ConversationFormat::JsonLines),
        _ => None,
    }
}

/// 根据原始文本内容识别会话输入格式。
fn detect_format_from_text(input: &str) -> ConversationFormat {
    let trimmed = input.trim();

    if trimmed.starts_with('[') {
        ConversationFormat::Json
    } else if trimmed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .all(|line| {
            let line = line.trim();
            line.starts_with('{') && line.ends_with('}')
        })
    {
        ConversationFormat::JsonLines
    } else {
        ConversationFormat::Text
    }
}

/// 将路径转换成可展示字符串。
fn display_path(path: impl Into<PathBuf>) -> String {
    path.into().display().to_string()
}

/// 将 CLI 的文档格式转换为存储层格式。
fn stored_format_from_capture_format(format: CaptureFormat) -> StoredDocumentFormat {
    match format {
        CaptureFormat::Markdown => StoredDocumentFormat::Markdown,
        CaptureFormat::Json => StoredDocumentFormat::Json,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_json_array() {
        assert_eq!(
            detect_format_from_text(r#"[{"role":"user","content":"ok"}]"#),
            ConversationFormat::Json
        );
    }

    #[test]
    fn detects_json_lines() {
        assert_eq!(
            detect_format_from_text(r#"{"role":"user","content":"ok"}"#),
            ConversationFormat::JsonLines
        );
    }

    #[test]
    fn detects_text() {
        assert_eq!(
            detect_format_from_text("plain transcript"),
            ConversationFormat::Text
        );
    }
}
