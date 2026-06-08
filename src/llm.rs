use serde_json::{json, Value};

use crate::cli::SkillTarget;

/// 返回 `detour capture --stdin` 支持的会话事件 JSON schema。
pub fn capture_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "DetourConversationEvents",
        "description": "传给 `detour capture --stdin --json` 的会话事件数组。JSONL 也可用，但本 schema 描述的是 JSON 数组形式。",
        "type": "array",
        "minItems": 1,
        "items": {
            "type": "object",
            "required": ["role", "content"],
            "additionalProperties": false,
            "properties": {
                "role": {
                    "type": "string",
                    "enum": ["user", "assistant", "tool", "system", "other"],
                    "description": "会话事件来源角色。"
                },
                "content": {
                    "type": "string",
                    "minLength": 1,
                    "description": "会话事件正文。应包含错误、修复、命令输出或关键上下文。"
                },
                "timestamp": {
                    "type": ["string", "null"],
                    "description": "可选时间戳。"
                },
                "source": {
                    "type": ["string", "null"],
                    "description": "可选来源，例如 claude、codex、shell、terminal。"
                }
            }
        },
        "examples": [
            [
                {
                    "role": "user",
                    "content": "VS Code says cargo is not recognized after Rust install."
                },
                {
                    "role": "assistant",
                    "content": "This is likely a PATH refresh issue. Restart VS Code and check cargo --version.",
                    "source": "codex"
                },
                {
                    "role": "tool",
                    "content": "error: cargo was not found",
                    "source": "terminal"
                }
            ]
        ]
    })
}

/// 返回给 LLM 使用 detour 的操作说明。
pub fn usage_prompt(target: SkillTarget) -> String {
    let target_name = match target {
        SkillTarget::Codex => "Codex",
        SkillTarget::Claude => "Claude Code",
    };

    format!(
        r#"# Detour 使用说明（{target_name}）

你可以使用本地 CLI `detour` 把当前 AI 会话中踩过的坑保存成错题集。

重要：如果你是 {target_name} 的 LLM，请由你先分析会话并生成错题集 JSON，再调用 `detour save` 保存。`detour capture` 是没有 LLM 参与的本地规则兜底。

## 什么时候使用

- 上下文即将压缩、总结或切换之前。
- 一次调试经历了多次失败、错误假设或返工。
- 发现了环境、shell、权限、依赖、API、hook、模型调用契约等约束。
- 用户要求保存经验、复盘问题或避免下次重复踩坑。

## 首选：由 LLM 生成错题集，再保存

优先运行：

```bash
detour save --stdin --json
```

如果当前还在 detour 项目源码中开发，也可以运行：

```bash
cargo run -- save --stdin --json
```

stdin 输入必须是你生成的错题集 JSON：

```json
{{
  "documents": [
    {{
      "title": "Claude Code Hook 错题集",
      "summary": "本文件记录 Claude Code hook 集成时踩过的坑。",
      "tags": ["claude-code", "precompact"],
      "mistakes": [
        {{
          "title": "不要把 hook 命令误认为 LLM 推理",
          "symptom": "误以为 hook 外部命令会自动调用 Claude 生成内容。",
          "wrong_turn": "混淆了 Claude Code hook 和 Claude LLM 生成流程。",
          "root_cause": "没有拆开宿主生命周期、外部 CLI 和 LLM 生成职责。",
          "correction": "由 LLM 先生成结构化错题 JSON，再调用 detour save 保存。",
          "prevention_rule": "需要模型生成内容时，先让模型生成 JSON，再让 CLI 负责校验和持久化。",
          "tags": ["llm-interface", "claude-code"],
          "severity": "high",
          "evidence": [
            {{
              "kind": "message",
              "value": "用户明确要求让大模型生成错题集。"
            }}
          ]
        }}
      ]
    }}
  ]
}}
```

## 输入质量要求

- 不要编造不存在的错误或证据。
- 优先保留错误现象、错误假设、根因、修正方式、下次预防规则相关内容。
- 工具输出、命令报错、文件路径、用户偏好和项目约束都应尽量保留。
- 会话很长时，优先由你先压缩成高质量错题集 JSON，再交给 detour 保存。
- document 和 mistake 都必须带 tags；detour 会把 tags 写入 Markdown frontmatter。

## 兜底：本地规则生成

如果你无法生成结构化错题集 JSON，才使用：

```bash
detour capture --stdin --json
```

这条命令会用 detour 本地规则分析输入，质量低于 LLM 生成。

## 保存后如何读取

查看最近文档：

```bash
detour recent --limit 5 --json
```

搜索历史错题：

```bash
detour search <keyword> --json
```

读取预防规则：

```bash
detour rules --limit 20 --json
```

## 输出处理

如果 `detour save --stdin --json` 返回 `ok: true`，请向用户报告保存路径。

如果命令失败：

1. 检查 `detour` 是否在 PATH 中。
2. 如果当前在源码项目里，改用 `cargo run -- save --stdin --json`。
3. 检查 stdin 是否是合法错题集 JSON。
4. 最多重试一次，不要反复执行同一个失败命令。
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_describes_conversation_events() {
        let schema = capture_schema();

        assert_eq!(schema["title"], "DetourConversationEvents");
        assert_eq!(schema["items"]["required"][0], "role");
    }

    #[test]
    fn prompt_mentions_capture_command() {
        let prompt = usage_prompt(SkillTarget::Codex);

        assert!(prompt.contains("detour capture --stdin --json"));
        assert!(prompt.contains("detour rules --limit 20 --json"));
    }
}
