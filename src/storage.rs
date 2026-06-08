use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::Serialize;

use crate::models::MistakeDocument;
use crate::render::render_markdown;

/// 实际写入磁盘的文档格式。
#[derive(Debug, Clone, Copy)]
pub enum StoredDocumentFormat {
    Markdown,
    Json,
}

impl StoredDocumentFormat {
    /// 返回当前格式对应的文件扩展名。
    pub fn extension(self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Json => "json",
        }
    }

    /// 返回当前格式对应的短名称。
    pub fn name(self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Json => "json",
        }
    }
}

/// 写入成功后的文档元数据。
#[derive(Debug, Clone, Serialize)]
pub struct SavedDocument {
    pub id: String,
    pub title: String,
    pub path: String,
    pub mistake_count: usize,
    pub tags: Vec<String>,
}

/// 将错题集文档写入指定目录，并返回保存结果。
pub fn write_documents(
    documents: &[MistakeDocument],
    output_dir: &Path,
    format: StoredDocumentFormat,
) -> anyhow::Result<Vec<SavedDocument>> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create output directory {}", output_dir.display()))?;

    let mut saved_documents = Vec::new();

    for document in documents {
        let path = unique_document_path(output_dir, document, format);
        let content = render_document(document, format)?;
        fs::write(&path, content)
            .with_context(|| format!("failed to write mistake document {}", path.display()))?;

        saved_documents.push(SavedDocument {
            id: document.id.clone(),
            title: document.title.clone(),
            path: path.display().to_string(),
            mistake_count: document.mistakes.len(),
            tags: document.tags.clone(),
        });
    }

    Ok(saved_documents)
}

/// 根据目标格式渲染单篇文档。
fn render_document(
    document: &MistakeDocument,
    format: StoredDocumentFormat,
) -> anyhow::Result<String> {
    match format {
        StoredDocumentFormat::Markdown => Ok(render_markdown(document)),
        StoredDocumentFormat::Json => Ok(serde_json::to_string_pretty(document)?),
    }
}

/// 为文档生成不会覆盖已有文件的路径。
fn unique_document_path(
    output_dir: &Path,
    document: &MistakeDocument,
    format: StoredDocumentFormat,
) -> PathBuf {
    let base_name = if document.id.trim().is_empty() {
        slugify(&document.title)
    } else {
        slugify(&document.id)
    };
    let extension = format.extension();
    let mut path = output_dir.join(format!("{}.{}", base_name, extension));

    if !path.exists() {
        return path;
    }

    for index in 2.. {
        path = output_dir.join(format!("{}-{}.{}", base_name, index, extension));

        if !path.exists() {
            return path;
        }
    }

    unreachable!("unique path loop should always return");
}

/// 将标题转换为简单、安全的文件名片段。
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
        "mistake-document".to_string()
    } else {
        slug.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Mistake, Severity};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn writes_markdown_document() {
        let output_dir = std::env::temp_dir().join(format!(
            "detour-storage-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be valid")
                .as_nanos()
        ));
        let mistake = Mistake::new(
            "m1",
            "Missing file ignore",
            "Generated files appeared in git status.",
            "Forgot to update .gitignore.",
            "Generated output needs explicit ignore rules.",
            "Add generated directories to .gitignore.",
            "Check git status after writing generated files.",
            Severity::Medium,
        );
        let document = MistakeDocument::new(
            "doc-filesystem-lessons",
            "Filesystem Lessons",
            "Lessons about generated files.",
            "unix:1",
            vec![mistake],
        );

        let saved =
            write_documents(&[document], &output_dir, StoredDocumentFormat::Markdown).unwrap();

        assert_eq!(saved.len(), 1);
        assert!(Path::new(&saved[0].path).exists());

        fs::remove_dir_all(output_dir).expect("temp output directory should be removable");
    }
}
