use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use serde::Serialize;

use crate::cli::ClaudeHookMode;
use crate::skills::{default_skill_path, write_skill, SkillWriteResult};

/// Claude Code 项目级集成状态。
#[derive(Debug, Clone, Serialize)]
pub struct ClaudeStatus {
    pub project_root: String,
    pub skill_path: String,
    pub skill_installed: bool,
    pub capture_command_path: String,
    pub capture_command_installed: bool,
    pub rules_command_path: String,
    pub rules_command_installed: bool,
}

/// Claude Code 安装操作的写入结果。
#[derive(Debug, Clone, Serialize)]
pub struct ClaudeInstallResult {
    pub mode: String,
    pub files: Vec<ClaudeWrittenFile>,
}

/// Claude Code 卸载操作的删除结果。
#[derive(Debug, Clone, Serialize)]
pub struct ClaudeUninstallResult {
    pub mode: String,
    pub files: Vec<ClaudeRemovedFile>,
}

/// Claude Code 集成写入的单个文件。
#[derive(Debug, Clone, Serialize)]
pub struct ClaudeWrittenFile {
    pub path: String,
    pub overwritten: bool,
}

/// Claude Code 集成删除的单个文件。
#[derive(Debug, Clone, Serialize)]
pub struct ClaudeRemovedFile {
    pub path: String,
    pub removed: bool,
}

/// 检测当前项目中的 Claude Code detour 集成状态。
pub fn detect_claude_project(project_root: &Path) -> ClaudeStatus {
    let skill_path = project_root.join(default_skill_path(crate::cli::SkillTarget::Claude));
    let capture_command_path = capture_command_path(project_root);
    let rules_command_path = rules_command_path(project_root);

    ClaudeStatus {
        project_root: project_root.display().to_string(),
        skill_installed: skill_path.exists(),
        skill_path: skill_path.display().to_string(),
        capture_command_installed: capture_command_path.exists(),
        capture_command_path: capture_command_path.display().to_string(),
        rules_command_installed: rules_command_path.exists(),
        rules_command_path: rules_command_path.display().to_string(),
    }
}

/// 安装 Claude Code 项目级集成文件。
pub fn install_claude_project(
    project_root: &Path,
    mode: ClaudeHookMode,
    global: bool,
    force: bool,
) -> anyhow::Result<ClaudeInstallResult> {
    if global {
        bail!("global Claude Code install is not supported yet; use project-level install");
    }

    match mode {
        ClaudeHookMode::SlashCommand => install_slash_commands(project_root, force),
        ClaudeHookMode::SkillOnly => install_skill(project_root, force),
        ClaudeHookMode::PreCompact => {
            bail!("PreCompact hook install is not implemented yet; this will be stage 9")
        }
        ClaudeHookMode::Wrapper => {
            bail!("wrapper install is not implemented yet")
        }
    }
}

/// 移除 Claude Code 项目级集成文件。
pub fn uninstall_claude_project(
    project_root: &Path,
    mode: ClaudeHookMode,
    global: bool,
) -> anyhow::Result<ClaudeUninstallResult> {
    if global {
        bail!("global Claude Code uninstall is not supported yet; use project-level uninstall");
    }

    match mode {
        ClaudeHookMode::SlashCommand => uninstall_slash_commands(project_root),
        ClaudeHookMode::SkillOnly => uninstall_skill(project_root),
        ClaudeHookMode::PreCompact => {
            bail!("PreCompact hook uninstall is not implemented yet; this will be stage 9")
        }
        ClaudeHookMode::Wrapper => {
            bail!("wrapper uninstall is not implemented yet")
        }
    }
}

/// 打印后续 PreCompact hook 阶段可使用的配置片段。
pub fn precompact_config_snippet() -> serde_json::Value {
    serde_json::json!({
        "hooks": {
            "PreCompact": [
                {
                    "matcher": "*",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "detour hook claude run-precompact --json"
                        }
                    ]
                }
            ]
        }
    })
}

/// 返回 PreCompact hook 占位运行结果。
pub fn run_precompact_placeholder() -> serde_json::Value {
    serde_json::json!({
        "ok": true,
        "implemented": false,
        "message": "PreCompact hook execution will be implemented in stage 9. Use /detour-capture or detour-capture skill for now."
    })
}

/// 安装 Claude Code slash command 文件。
fn install_slash_commands(project_root: &Path, force: bool) -> anyhow::Result<ClaudeInstallResult> {
    let files = vec![
        write_file(
            capture_command_path(project_root),
            claude_capture_command_content(),
            force,
        )?,
        write_file(
            rules_command_path(project_root),
            claude_rules_command_content(),
            force,
        )?,
    ];

    Ok(ClaudeInstallResult {
        mode: "slash-command".to_string(),
        files,
    })
}

/// 移除 Claude Code slash command 文件。
fn uninstall_slash_commands(project_root: &Path) -> anyhow::Result<ClaudeUninstallResult> {
    let files = vec![
        remove_file(capture_command_path(project_root))?,
        remove_file(rules_command_path(project_root))?,
    ];

    Ok(ClaudeUninstallResult {
        mode: "slash-command".to_string(),
        files,
    })
}

/// 安装 Claude Code skill 文件。
fn install_skill(project_root: &Path, force: bool) -> anyhow::Result<ClaudeInstallResult> {
    let skill_dir = project_root.join(".claude").join("skills");
    let SkillWriteResult {
        path, overwritten, ..
    } = write_skill(crate::cli::SkillTarget::Claude, Some(skill_dir), force)?;

    Ok(ClaudeInstallResult {
        mode: "skill-only".to_string(),
        files: vec![ClaudeWrittenFile { path, overwritten }],
    })
}

/// 移除 Claude Code skill 文件。
fn uninstall_skill(project_root: &Path) -> anyhow::Result<ClaudeUninstallResult> {
    let path = project_root.join(default_skill_path(crate::cli::SkillTarget::Claude));

    Ok(ClaudeUninstallResult {
        mode: "skill-only".to_string(),
        files: vec![remove_file(path)?],
    })
}

/// 写入文件，并根据 force 控制是否允许覆盖。
fn write_file(path: PathBuf, content: String, force: bool) -> anyhow::Result<ClaudeWrittenFile> {
    let overwritten = path.exists();

    if overwritten && !force {
        bail!(
            "Claude Code file already exists: {}. Pass --force to overwrite.",
            path.display()
        );
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create Claude Code directory {}",
                parent.display()
            )
        })?;
    }

    fs::write(&path, content)
        .with_context(|| format!("failed to write Claude Code file {}", path.display()))?;

    Ok(ClaudeWrittenFile {
        path: path.display().to_string(),
        overwritten,
    })
}

/// 删除文件；文件不存在时视为已处于卸载状态。
fn remove_file(path: PathBuf) -> anyhow::Result<ClaudeRemovedFile> {
    let removed = path.exists();

    if removed {
        fs::remove_file(&path)
            .with_context(|| format!("failed to remove Claude Code file {}", path.display()))?;
    }

    Ok(ClaudeRemovedFile {
        path: path.display().to_string(),
        removed,
    })
}

/// 返回 capture slash command 的路径。
fn capture_command_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".claude")
        .join("commands")
        .join("detour-capture.md")
}

/// 返回 rules slash command 的路径。
fn rules_command_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".claude")
        .join("commands")
        .join("detour-rules.md")
}

/// 返回 `/detour-capture` 的 Claude Code command 内容。
fn claude_capture_command_content() -> String {
    r#"# detour-capture

请复盘当前 Claude Code 会话中已经踩过的坑，并调用 detour 保存错题集。

你必须：

1. 找出本会话中已经出现的错误假设、命令失败、环境约束、权限问题、路径问题、API 误解或用户偏好遗漏。
2. 不要编造不存在的错误或证据。
3. 优先保留错误现象、错误路径、根因、修正方式和下次预防规则。
4. 使用 shell 运行：

```bash
detour capture --stdin --json
```

如果 `detour` 不在 PATH 中，且当前目录是 detour 源码项目，则改用：

```bash
cargo run -- capture --stdin --json
```

stdin 输入应使用 JSON 数组或 JSONL，至少包含关键 user、assistant、tool 消息。

保存成功后，请告诉用户：

- 生成了几篇错题集。
- 每篇错题集的路径。
- 最重要的下次预防规则。
"#
    .to_string()
}

/// 返回 `/detour-rules` 的 Claude Code command 内容。
fn claude_rules_command_content() -> String {
    r#"# detour-rules

请读取 detour 最近保存的错题集预防规则，并把最相关的规则用于当前任务。

优先运行：

```bash
detour rules --limit 20 --json
```

如果 `detour` 不在 PATH 中，且当前目录是 detour 源码项目，则改用：

```bash
cargo run -- rules --limit 20 --json
```

读取后请：

1. 简短总结最相关的 3 到 5 条规则。
2. 说明这些规则如何影响当前任务。
3. 不要把全部历史内容原样复制给用户。
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_slash_commands() {
        let root = std::env::temp_dir().join(format!("detour-claude-test-{}", std::process::id()));
        let result =
            install_claude_project(&root, ClaudeHookMode::SlashCommand, false, true).unwrap();

        assert_eq!(result.files.len(), 2);
        assert!(capture_command_path(&root).exists());
        assert!(rules_command_path(&root).exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn detects_project_files() {
        let root =
            std::env::temp_dir().join(format!("detour-claude-status-test-{}", std::process::id()));
        install_claude_project(&root, ClaudeHookMode::SlashCommand, false, true).unwrap();

        let status = detect_claude_project(&root);

        assert!(status.capture_command_installed);
        assert!(status.rules_command_installed);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn uninstalls_slash_commands() {
        let root = std::env::temp_dir().join(format!(
            "detour-claude-uninstall-test-{}",
            std::process::id()
        ));
        install_claude_project(&root, ClaudeHookMode::SlashCommand, false, true).unwrap();

        let result = uninstall_claude_project(&root, ClaudeHookMode::SlashCommand, false).unwrap();

        assert_eq!(result.files.len(), 2);
        assert!(!capture_command_path(&root).exists());
        assert!(!rules_command_path(&root).exists());

        let _ = fs::remove_dir_all(root);
    }
}
