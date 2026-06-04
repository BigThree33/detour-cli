use crate::models::{EvidenceKind, MistakeDocument};

/// 将错题集文档渲染成 Markdown 文本。
pub fn render_markdown(document: &MistakeDocument) -> String {
    let mut output = String::new();

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

/// 追加一行 Markdown 文本。
fn push_line(output: &mut String, line: impl AsRef<str>) {
    output.push_str(line.as_ref());
    output.push('\n');
}

/// 追加一个空行。
fn push_blank(output: &mut String) {
    output.push('\n');
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
        assert!(markdown.contains("## 错题 1"));
        assert!(markdown.contains("### 下次预防规则"));
    }
}
