use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context};
use serde::Serialize;

use crate::cli::SkillTarget;
use crate::llm::usage_prompt;

const SKILL_NAME: &str = "detour-capture";

/// skill 文件写入后的结果。
#[derive(Debug, Clone, Serialize)]
pub struct SkillWriteResult {
    pub target: String,
    pub path: String,
    pub overwritten: bool,
}

/// 返回指定目标环境的默认 skill 文件路径。
pub fn default_skill_path(target: SkillTarget) -> PathBuf {
    match target {
        SkillTarget::Codex => PathBuf::from(".detour")
            .join("skills")
            .join(SKILL_NAME)
            .join("SKILL.md"),
        SkillTarget::Claude => PathBuf::from(".claude")
            .join("skills")
            .join(SKILL_NAME)
            .join("SKILL.md"),
    }
}

/// 返回指定目标环境的 skill 内容。
pub fn skill_content(target: SkillTarget) -> String {
    match target {
        SkillTarget::Codex => codex_skill_content(),
        SkillTarget::Claude => claude_skill_content(),
    }
}

/// 创建或安装 skill 文件。
pub fn write_skill(
    target: SkillTarget,
    dir: Option<PathBuf>,
    force: bool,
) -> anyhow::Result<SkillWriteResult> {
    let path = resolve_skill_path(target, dir);
    let overwritten = path.exists();

    if overwritten && !force {
        bail!(
            "skill file already exists: {}. Pass --force to overwrite.",
            path.display()
        );
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create skill directory {}", parent.display()))?;
    }

    fs::write(&path, skill_content(target))
        .with_context(|| format!("failed to write skill file {}", path.display()))?;

    Ok(SkillWriteResult {
        target: skill_target_name(target).to_string(),
        path: path.display().to_string(),
        overwritten,
    })
}

/// 根据用户传入目录解析实际 SKILL.md 路径。
fn resolve_skill_path(target: SkillTarget, dir: Option<PathBuf>) -> PathBuf {
    let Some(dir) = dir else {
        return default_skill_path(target);
    };

    if dir.extension().and_then(|extension| extension.to_str()) == Some("md") {
        return dir;
    }

    if dir.file_name().and_then(|name| name.to_str()) == Some(SKILL_NAME) {
        return dir.join("SKILL.md");
    }

    dir.join(SKILL_NAME).join("SKILL.md")
}

/// 返回 Codex 目标的 skill 内容。
fn codex_skill_content() -> String {
    format!(
        r#"# detour-capture

Use this skill before context compaction, after repeated debugging failures, or when the user asks to preserve lessons learned from an AI-assisted coding session.

## Workflow

{}
"#,
        usage_prompt(SkillTarget::Codex)
    )
}

/// 返回 Claude Code 目标的 skill 内容。
fn claude_skill_content() -> String {
    format!(
        r#"---
name: detour-capture
description: Capture AI-session mistakes into detour before context compaction or after debugging.
argument-hint: [optional topic or reason]
---

# detour-capture

Use this skill when the current Claude Code session has accumulated mistakes, wrong assumptions, command failures, environment constraints, or lessons that should survive context compaction.

## Workflow

{}
"#,
        usage_prompt(SkillTarget::Claude)
    )
}

/// 返回目标环境名称。
fn skill_target_name(target: SkillTarget) -> &'static str {
    match target {
        SkillTarget::Codex => "codex",
        SkillTarget::Claude => "claude",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_skill_contains_frontmatter_and_command() {
        let content = skill_content(SkillTarget::Claude);

        assert!(content.contains("name: detour-capture"));
        assert!(content.contains("detour capture --stdin --json"));
    }

    #[test]
    fn writes_skill_to_custom_directory() {
        let root = std::env::temp_dir().join(format!("detour-skill-test-{}", std::process::id()));
        let result = write_skill(SkillTarget::Codex, Some(root.clone()), true).unwrap();

        assert!(PathBuf::from(&result.path).exists());

        let _ = fs::remove_dir_all(root);
    }
}
