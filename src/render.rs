use crate::models::{EvidenceKind, MistakeDocument};

/// 将错题集文档渲染成 Markdown 文本。
pub fn render_markdown(document: &MistakeDocument) -> String {
    let mut output = String::new();

    push_frontmatter(&mut output, document);
    push_line(&mut output, format!("# {}", document.title));
    push_blank(&mut output);
    push_line(&mut output, format!("- ID: {}", document.id));
    push_line(&mut output, format!("- 生成时间: {}", document.created_at));

    if let Some(source_session) = &document.source_session {
        push_line(&mut output, format!("- 来源会话: {}", source_session));
    }

    if !document.tags.is_empty() {
        push_line(&mut output, format!("- 标签: {}", document.tags.join(", ")));
    }

    push_blank(&mut output);
    push_line(&mut output, "## 摘要");
    push_blank(&mut output);
    push_line(&mut output, &document.summary);

    for (index, mistake) in document.mistakes.iter().enumerate() {
        push_blank(&mut output);
        push_line(
            &mut output,
            format!("## 错题 {}：{}", index + 1, mistake.title),
        );
        push_blank(&mut output);
        push_line(&mut output, "### 错误现象");
        push_blank(&mut output);
        push_line(&mut output, &mistake.symptom);
        push_blank(&mut output);
        push_line(&mut output, "### 错误路径");
        push_blank(&mut output);
        push_line(&mut output, &mistake.wrong_turn);
        push_blank(&mut output);
        push_line(&mut output, "### 根因");
        push_blank(&mut output);
        push_line(&mut output, &mistake.root_cause);
        push_blank(&mut output);
        push_line(&mut output, "### 修正方式");
        push_blank(&mut output);
        push_line(&mut output, &mistake.correction);
        push_blank(&mut output);
        push_line(&mut output, "### 下次预防规则");
        push_blank(&mut output);
        push_line(&mut output, &mistake.prevention_rule);

        if !mistake.evidence.is_empty() {
            push_blank(&mut output);
            push_line(&mut output, "### 证据");
            push_blank(&mut output);

            for evidence in &mistake.evidence {
                push_line(
                    &mut output,
                    format!(
                        "- {}: {}",
                        evidence_kind_name(&evidence.kind),
                        evidence.value
                    ),
                );
            }
        }

        if !mistake.tags.is_empty() {
            push_blank(&mut output);
            push_line(&mut output, "### 标签");
            push_blank(&mut output);
            push_line(&mut output, mistake.tags.join(", "));
        }
    }

    output
}

/// 追加文档级 frontmatter，作为 Markdown 文件自身的 metadata。
fn push_frontmatter(output: &mut String, document: &MistakeDocument) {
    push_line(output, "---");
    push_line(output, "detour_metadata_version: 1");
    push_line(output, "generator: detour-cli");
    push_line(output, "metadata_layers:");
    push_line(output, "  - filesystem");
    push_line(output, "  - markdown_frontmatter");
    push_line(output, format!("id: {}", yaml_string(&document.id)));
    push_line(output, format!("title: {}", yaml_string(&document.title)));
    push_line(
        output,
        format!("created_at: {}", yaml_string(&document.created_at)),
    );

    if let Some(source_session) = &document.source_session {
        push_line(
            output,
            format!("source_session: {}", yaml_string(source_session)),
        );
    }

    if document.tags.is_empty() {
        push_line(output, "tags: []");
    } else {
        push_line(output, "tags:");

        for tag in &document.tags {
            push_line(output, format!("  - {}", yaml_string(tag)));
        }
    }

    push_line(output, "---");
    push_blank(output);
}

/// 追加一行 Markdown 文本。
fn push_line(output: &mut String, line: impl AsRef<str>) {
    output.push_str(line.as_ref());
    output.push('\n');
}

/// 追加一个空行。
fn push_blank(output: &mut String) {
    output.push('\n');
}

/// 把普通字符串转成安全的 YAML 双引号字符串。
fn yaml_string(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// 将证据类型转换成 Markdown 中显示的名称。
fn evidence_kind_name(kind: &EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::Command => "命令",
        EvidenceKind::File => "文件",
        EvidenceKind::Output => "输出",
        EvidenceKind::Message => "消息",
        EvidenceKind::Other => "其他",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Mistake, MistakeDocument, Severity};

    #[test]
    fn renders_markdown_sections() {
        let mistake = Mistake::new(
            "m1",
            "Missing JSON output",
            "The LLM could not parse CLI output.",
            "Returned only human-readable logs.",
            "The command contract was not strict enough.",
            "Add --json output.",
            "LLM-facing commands should return JSON.",
            Severity::Medium,
        );
        let document = MistakeDocument::new(
            "doc1",
            "LLM Interface Lessons",
            "Lessons about tool contracts.",
            "unix:1",
            vec![mistake],
        );

        let markdown = render_markdown(&document);

        assert!(markdown.contains("# LLM Interface Lessons"));
        assert!(markdown.starts_with("---\n"));
        assert!(markdown.contains("metadata_layers:"));
        assert!(markdown.contains("tags: []"));
        assert!(markdown.contains("## 错题 1"));
        assert!(markdown.contains("### 下次预防规则"));
    }

    #[test]
    fn renders_document_tags_in_frontmatter() {
        let mistake = Mistake::new(
            "m1",
            "Hook failed",
            "The hook failed before compact.",
            "Assumed hook output would be ignored.",
            "Claude Code hook output can affect context.",
            "Return structured JSON with additional context.",
            "PreCompact hooks should inject compact guidance.",
            Severity::Medium,
        );
        let mut document = MistakeDocument::new(
            "doc-claude-code-hook",
            "Claude Code Hook 错题集",
            "Lessons about Claude Code hooks.",
            "unix:1",
            vec![mistake],
        );
        document.tags.push("claude-code".to_string());
        document.tags.push("precompact".to_string());

        let markdown = render_markdown(&document);

        assert!(markdown.contains("tags:\n  - \"claude-code\"\n  - \"precompact\""));
    }
}
