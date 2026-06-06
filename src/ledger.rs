use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use anyhow::{bail, Context};
use serde::Serialize;

/// 默认错题集存储目录。
pub const DEFAULT_MISTAKES_DIR: &str = ".detour/mistakes";

/// 本地错题集文件的列表项。
#[derive(Debug, Clone, Serialize)]
pub struct LedgerDocument {
    pub id: String,
    pub title: String,
    pub path: String,
    pub modified_at: String,
    pub size_bytes: u64,
}

/// 单篇错题集的完整读取结果。
#[derive(Debug, Clone, Serialize)]
pub struct ShownDocument {
    pub id: String,
    pub title: String,
    pub path: String,
    pub content: String,
}

/// 搜索命中的错题集结果。
#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub id: String,
    pub title: String,
    pub path: String,
    pub snippet: String,
}

/// 从错题集中提取出的预防规则。
#[derive(Debug, Clone, Serialize)]
pub struct RuleItem {
    pub document_id: String,
    pub title: String,
    pub path: String,
    pub rule: String,
}

/// 列出本地错题集文档。
pub fn list_documents(root: &Path) -> anyhow::Result<Vec<LedgerDocument>> {
    let mistakes_dir = mistakes_dir(root);

    if !mistakes_dir.exists() {
        return Ok(Vec::new());
    }

    let mut documents = Vec::new();

    for entry in fs::read_dir(&mistakes_dir)
        .with_context(|| format!("failed to read {}", mistakes_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if !is_supported_document(&path) {
            continue;
        }

        documents.push(read_document_metadata(&path)?);
    }

    documents.sort_by(|left, right| right.modified_at.cmp(&left.modified_at));
    Ok(documents)
}

/// 读取指定 ID 或路径对应的错题集文档。
pub fn show_document(root: &Path, id_or_path: &str) -> anyhow::Result<ShownDocument> {
    let path = resolve_document_path(root, id_or_path)?;
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read mistake document {}", path.display()))?;
    let title = extract_title(&content).unwrap_or_else(|| fallback_title(&path));

    Ok(ShownDocument {
        id: document_id(&path),
        title,
        path: path.display().to_string(),
        content,
    })
}

/// 在本地错题集中搜索关键词。
pub fn search_documents(root: &Path, query: &str) -> anyhow::Result<Vec<SearchHit>> {
    let query = query.trim();

    if query.is_empty() {
        bail!("search query cannot be empty");
    }

    let query_lower = query.to_lowercase();
    let mut hits = Vec::new();

    for document in list_documents(root)? {
        let content = fs::read_to_string(&document.path)
            .with_context(|| format!("failed to read mistake document {}", document.path))?;

        if !content.to_lowercase().contains(&query_lower) {
            continue;
        }

        hits.push(SearchHit {
            id: document.id,
            title: document.title,
            path: document.path,
            snippet: find_snippet(&content, query),
        });
    }

    Ok(hits)
}

/// 返回最近的本地错题集文档。
pub fn recent_documents(root: &Path, limit: usize) -> anyhow::Result<Vec<LedgerDocument>> {
    let mut documents = list_documents(root)?;
    documents.truncate(limit);
    Ok(documents)
}

/// 从最近的错题集中提取预防规则。
pub fn recent_rules(root: &Path, limit: usize) -> anyhow::Result<Vec<RuleItem>> {
    let mut rules = Vec::new();

    for document in list_documents(root)? {
        let content = fs::read_to_string(&document.path)
            .with_context(|| format!("failed to read mistake document {}", document.path))?;

        for rule in extract_prevention_rules(&content) {
            rules.push(RuleItem {
                document_id: document.id.clone(),
                title: document.title.clone(),
                path: document.path.clone(),
                rule,
            });

            if rules.len() >= limit {
                return Ok(rules);
            }
        }
    }

    Ok(rules)
}

/// 返回项目根目录下的错题集目录。
fn mistakes_dir(root: &Path) -> PathBuf {
    root.join(DEFAULT_MISTAKES_DIR)
}

/// 判断路径是否是 detour 支持读取的文档。
fn is_supported_document(path: &Path) -> bool {
    path.is_file()
        && matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("md") | Some("json")
        )
}

/// 读取单篇错题集文件的元数据。
fn read_document_metadata(path: &Path) -> anyhow::Result<LedgerDocument> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("failed to read metadata for {}", path.display()))?;
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read mistake document {}", path.display()))?;

    Ok(LedgerDocument {
        id: document_id(path),
        title: extract_title(&content).unwrap_or_else(|| fallback_title(path)),
        path: path.display().to_string(),
        modified_at: modified_at(&metadata),
        size_bytes: metadata.len(),
    })
}

/// 根据 ID 或路径解析出实际文档路径。
fn resolve_document_path(root: &Path, id_or_path: &str) -> anyhow::Result<PathBuf> {
    let direct_path = PathBuf::from(id_or_path);

    if direct_path.exists() {
        return Ok(direct_path);
    }

    let mistakes_dir = mistakes_dir(root);
    let candidates = [
        mistakes_dir.join(id_or_path),
        mistakes_dir.join(format!("{}.md", id_or_path)),
        mistakes_dir.join(format!("{}.json", id_or_path)),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    bail!("mistake document not found: {}", id_or_path)
}

/// 从文件路径生成文档 ID。
fn document_id(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// 从 Markdown 正文中提取标题。
fn extract_title(content: &str) -> Option<String> {
    content
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
        .map(ToOwned::to_owned)
}

/// 使用文件名作为兜底标题。
fn fallback_title(path: &Path) -> String {
    document_id(path).replace('-', " ")
}

/// 将修改时间转换成简单 unix 时间戳。
fn modified_at(metadata: &fs::Metadata) -> String {
    let seconds = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    format!("unix:{}", seconds)
}

/// 为搜索命中生成一行简短片段。
fn find_snippet(content: &str, query: &str) -> String {
    let query_lower = query.to_lowercase();

    content
        .lines()
        .find(|line| line.to_lowercase().contains(&query_lower))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| truncate(line, 180))
        .unwrap_or_else(|| truncate(content.trim(), 180))
}

/// 从 Markdown 中提取“下次预防规则”小节。
fn extract_prevention_rules(content: &str) -> Vec<String> {
    let mut rules = Vec::new();
    let mut in_rule_section = false;
    let mut current_rule = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "### 下次预防规则" || trimmed == "### Prevention Rule" {
            if !current_rule.is_empty() {
                rules.push(current_rule.join(" "));
                current_rule.clear();
            }
            in_rule_section = true;
            continue;
        }

        if in_rule_section && trimmed.starts_with('#') {
            if !current_rule.is_empty() {
                rules.push(current_rule.join(" "));
                current_rule.clear();
            }
            in_rule_section = false;
            continue;
        }

        if in_rule_section && !trimmed.is_empty() {
            current_rule.push(trimmed.trim_start_matches("- ").to_string());
        }
    }

    if !current_rule.is_empty() {
        rules.push(current_rule.join(" "));
    }

    rules
}

/// 在不切坏 UTF-8 字节的前提下截断文本。
fn truncate(content: &str, max_chars: usize) -> String {
    let mut chars = content.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();

    if chars.next().is_some() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_shows_searches_and_extracts_rules() {
        let root = std::env::temp_dir().join(format!("detour-ledger-test-{}", std::process::id()));
        let mistakes_dir = mistakes_dir(&root);
        fs::create_dir_all(&mistakes_dir).expect("mistakes dir should be created");
        let path = mistakes_dir.join("doc-rust-cli.md");
        fs::write(
            &path,
            "# Rust CLI 错题集\n\n## 摘要\n\n测试文档。\n\n### 下次预防规则\n\n先用 help 输出验证命令结构。\n",
        )
        .expect("fixture should be written");

        let documents = list_documents(&root).expect("documents should list");
        assert_eq!(documents.len(), 1);

        let shown = show_document(&root, "doc-rust-cli").expect("document should show");
        assert!(shown.content.contains("Rust CLI 错题集"));

        let hits = search_documents(&root, "help").expect("search should work");
        assert_eq!(hits.len(), 1);

        let rules = recent_rules(&root, 5).expect("rules should extract");
        assert_eq!(rules[0].rule, "先用 help 输出验证命令结构。");

        let _ = fs::remove_dir_all(root);
    }
}
