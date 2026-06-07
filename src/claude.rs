use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cli::ClaudeHookMode;
use crate::extractor::{extract_mistake_documents, ExtractOptions};
use crate::models::{ConversationEvent, ConversationRole};
use crate::skills::{default_skill_path, write_skill, SkillWriteResult};
use crate::storage::{write_documents, SavedDocument, StoredDocumentFormat};

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

/// Claude Code PreCompact hook 的运行结果。
#[derive(Debug, Clone, Serialize)]
pub struct ClaudePreCompactResult {
    pub ok: bool,
    pub implemented: bool,
    pub hook_event_name: Option<String>,
    pub trigger: Option<String>,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    pub cwd: Option<String>,
    pub source: String,
    pub output_dir: String,
    pub document_count: usize,
    pub saved_documents: Vec<SavedDocument>,
    pub warnings: Vec<String>,
}

/// Claude Code hook 通过 stdin 传入的关键字段。
#[derive(Debug, Clone, Deserialize)]
struct ClaudeHookInput {
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    transcript_path: Option<PathBuf>,
    #[serde(default)]
    cwd: Option<PathBuf>,
    #[serde(default)]
    hook_event_name: Option<String>,
    #[serde(default)]
    trigger: Option<String>,
    #[serde(default)]
    custom_instructions: Option<String>,
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
                    "matcher": "manual",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "detour hook claude run-precompact --max-docs 5 --json"
                        }
                    ]
                },
                {
                    "matcher": "auto",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "detour hook claude run-precompact --max-docs 5 --json"
                        }
                    ]
                }
            ]
        }
    })
}

/// 执行 Claude Code PreCompact hook，把 transcript 转成错题集。
pub fn run_precompact(
    hook_stdin: &str,
    max_docs: usize,
    out: Option<PathBuf>,
    dry_run: bool,
) -> ClaudePreCompactResult {
    let hook_input = match serde_json::from_str::<ClaudeHookInput>(hook_stdin.trim()) {
        Ok(input) => input,
        Err(error) => {
            return precompact_result(
                false,
                None,
                None,
                None,
                None,
                None,
                "stdin".to_string(),
                display_path(default_output_dir(Path::new("."), out.as_deref())),
                Vec::new(),
                vec![format!(
                    "failed to parse Claude Code hook stdin JSON: {error}"
                )],
            );
        }
    };

    let cwd = hook_input.cwd.unwrap_or_else(|| PathBuf::from("."));
    let output_dir = default_output_dir(&cwd, out.as_deref());
    let mut warnings = Vec::new();

    if hook_input.hook_event_name.as_deref() != Some("PreCompact") {
        warnings.push(format!(
            "expected hook_event_name PreCompact, got {}",
            hook_input.hook_event_name.as_deref().unwrap_or("<missing>")
        ));
    }

    if let Some(instructions) = hook_input.custom_instructions.as_deref() {
        if !instructions.trim().is_empty() {
            warnings.push(format!(
                "custom compact instructions were present and preserved only as metadata: {}",
                truncate(instructions, 120)
            ));
        }
    }

    let Some(raw_transcript_path) = hook_input.transcript_path.as_ref() else {
        warnings.push("Claude Code hook input did not include transcript_path".to_string());
        return precompact_result(
            false,
            hook_input.hook_event_name,
            hook_input.trigger,
            hook_input.session_id,
            None,
            Some(display_path(&cwd)),
            "stdin".to_string(),
            display_path(output_dir),
            Vec::new(),
            warnings,
        );
    };

    let transcript_path = resolve_hook_path(&cwd, raw_transcript_path);
    let transcript_text = match fs::read_to_string(&transcript_path) {
        Ok(text) => text,
        Err(error) => {
            warnings.push(format!(
                "failed to read Claude Code transcript {}: {error}",
                transcript_path.display()
            ));
            return precompact_result(
                false,
                hook_input.hook_event_name,
                hook_input.trigger,
                hook_input.session_id,
                Some(display_path(transcript_path)),
                Some(display_path(&cwd)),
                "transcript".to_string(),
                display_path(output_dir),
                Vec::new(),
                warnings,
            );
        }
    };

    let events = parse_claude_transcript(&transcript_text, &transcript_path);
    let documents = extract_mistake_documents(&events, ExtractOptions { max_docs });

    if documents.is_empty() {
        warnings.push("no mistake candidates found in Claude Code transcript".to_string());
        return precompact_result(
            true,
            hook_input.hook_event_name,
            hook_input.trigger,
            hook_input.session_id,
            Some(display_path(transcript_path)),
            Some(display_path(&cwd)),
            "transcript".to_string(),
            display_path(output_dir),
            Vec::new(),
            warnings,
        );
    }

    let saved_documents = if dry_run {
        Vec::new()
    } else {
        match write_documents(&documents, &output_dir, StoredDocumentFormat::Markdown) {
            Ok(saved) => saved,
            Err(error) => {
                warnings.push(format!("failed to write mistake documents: {error}"));
                return precompact_result(
                    false,
                    hook_input.hook_event_name,
                    hook_input.trigger,
                    hook_input.session_id,
                    Some(display_path(transcript_path)),
                    Some(display_path(&cwd)),
                    "transcript".to_string(),
                    display_path(output_dir),
                    Vec::new(),
                    warnings,
                );
            }
        }
    };

    let document_count = if dry_run {
        documents.len()
    } else {
        saved_documents.len()
    };

    ClaudePreCompactResult {
        ok: true,
        implemented: true,
        hook_event_name: hook_input.hook_event_name,
        trigger: hook_input.trigger,
        session_id: hook_input.session_id,
        transcript_path: Some(display_path(transcript_path)),
        cwd: Some(display_path(&cwd)),
        source: "transcript".to_string(),
        output_dir: display_path(output_dir),
        document_count,
        saved_documents,
        warnings,
    }
}

/// 构造 PreCompact hook 的统一结果。
#[allow(clippy::too_many_arguments)]
fn precompact_result(
    ok: bool,
    hook_event_name: Option<String>,
    trigger: Option<String>,
    session_id: Option<String>,
    transcript_path: Option<String>,
    cwd: Option<String>,
    source: String,
    output_dir: String,
    saved_documents: Vec<SavedDocument>,
    warnings: Vec<String>,
) -> ClaudePreCompactResult {
    ClaudePreCompactResult {
        ok,
        implemented: true,
        hook_event_name,
        trigger,
        session_id,
        transcript_path,
        cwd,
        source,
        output_dir,
        document_count: saved_documents.len(),
        saved_documents,
        warnings,
    }
}

/// 解析 Claude Code transcript JSONL，无法结构化解析时回退成普通文本事件。
fn parse_claude_transcript(input: &str, source_path: &Path) -> Vec<ConversationEvent> {
    let source = display_path(source_path);
    let mut events = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };

        let content = content_from_value(&value);

        if content.trim().is_empty() {
            continue;
        }

        events.push(ConversationEvent {
            role: role_from_value(&value),
            content,
            timestamp: string_at(&value, &["timestamp", "created_at"]),
            source: Some(source.clone()),
        });
    }

    if events.is_empty() && !input.trim().is_empty() {
        events.push(ConversationEvent {
            role: ConversationRole::Other,
            content: input.trim().to_string(),
            timestamp: None,
            source: Some(source),
        });
    }

    events
}

/// 从 Claude transcript 记录中尽量识别消息角色。
fn role_from_value(value: &Value) -> ConversationRole {
    let role = value
        .pointer("/message/role")
        .and_then(Value::as_str)
        .or_else(|| value.get("role").and_then(Value::as_str))
        .or_else(|| value.get("type").and_then(Value::as_str))
        .unwrap_or("other")
        .to_ascii_lowercase();

    match role.as_str() {
        "user" | "human" => ConversationRole::User,
        "assistant" => ConversationRole::Assistant,
        "tool" | "tool_use" | "tool_result" => ConversationRole::Tool,
        "system" => ConversationRole::System,
        _ => ConversationRole::Other,
    }
}

/// 从 Claude transcript 记录中尽量抽取可读文本。
fn content_from_value(value: &Value) -> String {
    if let Some(content) = value.pointer("/message/content") {
        return value_to_text(content);
    }

    if let Some(content) = value.get("content") {
        return value_to_text(content);
    }

    if let Some(message) = value.get("message") {
        return value_to_text(message);
    }

    serde_json::to_string(value).unwrap_or_default()
}

/// 把字符串、content block 数组或工具对象转换成文本。
fn value_to_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items
            .iter()
            .map(value_to_text)
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(object) => {
            if let Some(text) = object.get("text").and_then(Value::as_str) {
                return text.to_string();
            }

            if let Some(content) = object.get("content") {
                return value_to_text(content);
            }

            if let Some(name) = object.get("name").and_then(Value::as_str) {
                let input = object
                    .get("input")
                    .and_then(|input| serde_json::to_string(input).ok())
                    .unwrap_or_default();
                return format!("tool_use {name} {input}");
            }

            serde_json::to_string(value).unwrap_or_default()
        }
        _ => value.to_string(),
    }
}

/// 读取多个可能字段中的第一个字符串。
fn string_at(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(ToString::to_string)
}

/// 解析 hook 输入中的路径，支持相对路径和简单的用户目录前缀。
fn resolve_hook_path(cwd: &Path, path: &Path) -> PathBuf {
    let expanded = expand_home(path);

    if expanded.is_absolute() {
        expanded
    } else {
        cwd.join(expanded)
    }
}

/// 相对输出目录默认落在 Claude Code 当前工作目录下。
fn default_output_dir(cwd: &Path, out: Option<&Path>) -> PathBuf {
    let out = out
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".detour").join("mistakes"));

    if out.is_absolute() {
        out
    } else {
        cwd.join(out)
    }
}

/// 展开 `~` 开头的路径，避免 hook 输入里出现用户目录缩写时读不到文件。
fn expand_home(path: &Path) -> PathBuf {
    let Some(path_text) = path.to_str() else {
        return path.to_path_buf();
    };

    if path_text == "~" {
        return home_dir().unwrap_or_else(|| path.to_path_buf());
    }

    if let Some(rest) = path_text.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(rest);
        }
    }

    path.to_path_buf()
}

/// 获取当前用户目录，优先适配 Windows，再兼容类 Unix 环境。
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

/// 将路径转换成可展示字符串。
fn display_path(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

/// 压缩用于 warning 的长文本。
fn truncate(input: &str, max_chars: usize) -> String {
    let mut chars = input.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();

    if chars.next().is_some() {
        format!("{}...", truncated)
    } else {
        truncated
    }
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
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn run_precompact_reads_transcript_and_writes_documents() {
        let root = unique_temp_dir("detour-precompact-test");
        fs::create_dir_all(&root).unwrap();
        let transcript_path = root.join("session.jsonl");
        fs::write(
            &transcript_path,
            r#"{"type":"user","message":{"role":"user","content":"cargo is not recognized in VS Code"}}
{"type":"assistant","message":{"role":"assistant","content":"This failed because PATH did not refresh after Rust install."}}"#,
        )
        .unwrap();

        let hook_input = serde_json::json!({
            "session_id": "session-1",
            "transcript_path": transcript_path.display().to_string(),
            "cwd": root.display().to_string(),
            "hook_event_name": "PreCompact",
            "trigger": "auto"
        });

        let result = run_precompact(&hook_input.to_string(), 5, None, false);

        assert!(result.ok);
        assert!(result.implemented);
        assert_eq!(result.document_count, 1);
        assert_eq!(result.saved_documents.len(), 1);
        assert!(Path::new(&result.saved_documents[0].path).exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn run_precompact_reports_missing_transcript_path() {
        let hook_input = serde_json::json!({
            "session_id": "session-2",
            "cwd": ".",
            "hook_event_name": "PreCompact",
            "trigger": "manual"
        });

        let result = run_precompact(&hook_input.to_string(), 5, None, false);

        assert!(!result.ok);
        assert!(result.implemented);
        assert!(result
            .warnings
            .iter()
            .any(|warning| warning.contains("transcript_path")));
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be valid")
            .as_nanos();

        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }
}
