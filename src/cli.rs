use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

// `detour` 二进制命令的顶层解析器。
#[derive(Debug, Parser)]
#[command(
    name = "detour",
    version,
    about = "Preserve AI-session mistakes before context compaction.",
    long_about = "Detour captures mistakes, wrong assumptions, fixes, and prevention rules from AI coding sessions before context compaction."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

// `detour` 支持的所有一级子命令。
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// 初始化 detour 配置和本地存储。
    Init(InitArgs),

    /// 捕获会话并生成错题集文档。
    Capture(CaptureArgs),

    /// 保存 LLM 已经生成好的错题集文档。
    Save(SaveArgs),

    /// 列出已保存的错题集文档。
    List(ListArgs),

    /// 显示一篇已保存的错题集文档。
    Show(ShowArgs),

    /// 搜索已保存的错题集文档。
    Search(SearchArgs),

    /// 列出最近保存的错题集文档。
    Recent(RecentArgs),

    /// 提取最近错题集中的下次预防规则。
    Rules(RulesArgs),

    /// 打印 LLM 调用 detour 时使用的输入 schema。
    Schema,

    /// 打印给 LLM 使用 detour 的操作说明。
    Prompt(PromptArgs),

    /// 管理与 AI 工具的集成。
    Hook(HookArgs),

    /// 创建、打印或安装 LLM skills。
    Skill(SkillArgs),
}

// `detour init` 接受的参数。
#[derive(Debug, Args)]
pub struct InitArgs {
    /// 创建 .detour 的目标目录。
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    /// 在支持的地方覆盖已有生成文件。
    #[arg(long)]
    pub force: bool,
}

// `detour capture` 接受的参数。
#[derive(Debug, Args)]
pub struct CaptureArgs {
    /// 从文件读取会话材料。
    #[arg(long = "from", value_name = "PATH")]
    pub from: Option<PathBuf>,

    /// 从标准输入读取会话材料。
    #[arg(long)]
    pub stdin: bool,

    /// 写入错题集文档的目录。
    #[arg(long, value_name = "DIR")]
    pub out: Option<PathBuf>,

    /// 最多生成多少篇文档。
    #[arg(long, default_value_t = 5)]
    pub max_docs: usize,

    /// 输出文档格式。
    #[arg(long, value_enum, default_value = "md")]
    pub format: CaptureFormat,

    /// 只打印将要发生的操作，不写入文件。
    #[arg(long)]
    pub dry_run: bool,

    /// 用机器可读 JSON 打印命令结果。
    #[arg(long)]
    pub json: bool,
}

// `detour save` 接受的参数。
#[derive(Debug, Args)]
pub struct SaveArgs {
    /// 从标准输入读取 LLM 生成的错题集 JSON。
    #[arg(long)]
    pub stdin: bool,

    /// 写入错题集文档的目录。
    #[arg(long, value_name = "DIR")]
    pub out: Option<PathBuf>,

    /// 最多保存多少篇文档。
    #[arg(long, default_value_t = 5)]
    pub max_docs: usize,

    /// 输出文档格式。
    #[arg(long, value_enum, default_value = "md")]
    pub format: CaptureFormat,

    /// 只打印将要发生的操作，不写入文件。
    #[arg(long)]
    pub dry_run: bool,

    /// 用机器可读 JSON 打印命令结果。
    #[arg(long)]
    pub json: bool,
}

// capture 命令支持的文档格式。
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CaptureFormat {
    #[value(name = "md")]
    Markdown,
    Json,
}

// `detour list` 接受的参数。
#[derive(Debug, Args)]
pub struct ListArgs {
    /// 项目目录或 detour 根目录。
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    /// 打印机器可读 JSON。
    #[arg(long)]
    pub json: bool,
}

// `detour show` 接受的参数。
#[derive(Debug, Args)]
pub struct ShowArgs {
    /// 要显示的文档 ID 或路径。
    pub id_or_path: String,

    /// 项目目录或 detour 根目录。
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    /// 打印机器可读 JSON。
    #[arg(long)]
    pub json: bool,
}

// `detour search` 接受的参数。
#[derive(Debug, Args)]
pub struct SearchArgs {
    /// 搜索关键词。
    pub query: String,

    /// 项目目录或 detour 根目录。
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    /// 打印机器可读 JSON。
    #[arg(long)]
    pub json: bool,
}

// `detour recent` 接受的参数。
#[derive(Debug, Args)]
pub struct RecentArgs {
    /// 项目目录或 detour 根目录。
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    /// 最多返回多少篇文档。
    #[arg(long, default_value_t = 5)]
    pub limit: usize,

    /// 打印机器可读 JSON。
    #[arg(long)]
    pub json: bool,
}

// `detour rules` 接受的参数。
#[derive(Debug, Args)]
pub struct RulesArgs {
    /// 项目目录或 detour 根目录。
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    /// 最多返回多少条规则。
    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    /// 打印机器可读 JSON。
    #[arg(long)]
    pub json: bool,
}

// `detour prompt` 接受的参数。
#[derive(Debug, Args)]
pub struct PromptArgs {
    /// prompt 目标助手环境。
    #[arg(long, value_enum, default_value = "codex")]
    pub target: SkillTarget,
}

// AI 工具集成相关的嵌套命令组。
#[derive(Debug, Args)]
pub struct HookArgs {
    #[command(subcommand)]
    pub command: HookCommands,
}

// `detour hook` 可以管理的 AI 工具。
#[derive(Debug, Subcommand)]
pub enum HookCommands {
    /// 管理 Claude Code 集成。
    Claude(ClaudeHookArgs),
}

// Claude Code hook 集成的嵌套命令组。
#[derive(Debug, Args)]
pub struct ClaudeHookArgs {
    #[command(subcommand)]
    pub command: ClaudeHookCommands,
}

// `detour hook claude` 支持的命令。
#[derive(Debug, Subcommand)]
pub enum ClaudeHookCommands {
    /// 检测 Claude Code 和 detour 的集成状态。
    Detect(JsonFlag),

    /// 安装 Claude Code 集成文件。
    Install(ClaudeHookInstallArgs),

    /// 打印 Claude Code 集成状态。
    Status(JsonFlag),

    /// 打印 Claude Code settings 片段，不写入文件。
    PrintConfig(ClaudeHookPrintConfigArgs),

    /// 移除 detour 的 Claude Code 集成文件。
    Uninstall(ClaudeHookUninstallArgs),

    /// Claude Code PreCompact hook 调用的入口。
    RunPrecompact(ClaudeRunPrecompactArgs),
}

// 简单状态类命令复用的 `--json` 参数。
#[derive(Debug, Args)]
pub struct JsonFlag {
    /// 打印机器可读 JSON。
    #[arg(long)]
    pub json: bool,
}

// `detour hook claude install` 接受的参数。
#[derive(Debug, Args)]
pub struct ClaudeHookInstallArgs {
    /// 要安装的集成模式。
    #[arg(long, value_enum, default_value = "slash-command")]
    pub mode: ClaudeHookMode,

    /// 写入用户级 Claude Code settings，而不是项目级 settings。
    #[arg(long)]
    pub global: bool,

    /// 在支持的地方覆盖已有生成文件。
    #[arg(long)]
    pub force: bool,
}

// `detour hook claude print-config` 接受的参数。
#[derive(Debug, Args)]
pub struct ClaudeHookPrintConfigArgs {
    /// 要打印配置的 hook 事件。
    #[arg(long, value_enum, default_value = "pre-compact")]
    pub event: ClaudeHookEvent,
}

// `detour hook claude uninstall` 接受的参数。
#[derive(Debug, Args)]
pub struct ClaudeHookUninstallArgs {
    /// 要移除的集成模式。
    #[arg(long, value_enum, default_value = "slash-command")]
    pub mode: ClaudeHookMode,

    /// 从用户级 Claude Code settings 移除，而不是项目级 settings。
    #[arg(long)]
    pub global: bool,
}

// `detour hook claude run-precompact` 接受的参数。
#[derive(Debug, Args)]
pub struct ClaudeRunPrecompactArgs {
    /// 最多生成多少篇文档。
    #[arg(long, default_value_t = 5)]
    pub max_docs: usize,

    /// 写入错题集文档的目录。
    #[arg(long, value_name = "DIR")]
    pub out: Option<PathBuf>,

    /// 只打印将要发生的操作，不写入文件。
    #[arg(long)]
    pub dry_run: bool,

    /// 打印机器可读 JSON。
    #[arg(long)]
    pub json: bool,
}

// detour 可以安装的 Claude Code 集成模式。
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ClaudeHookMode {
    #[value(name = "pre-compact")]
    PreCompact,
    #[value(name = "slash-command")]
    SlashCommand,
    Wrapper,
    #[value(name = "skill-only")]
    SkillOnly,
}

// detour 可以打印或卸载的 Claude Code hook 事件。
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ClaudeHookEvent {
    #[value(name = "pre-compact")]
    PreCompact,
}

// skill 相关操作的嵌套命令组。
#[derive(Debug, Args)]
pub struct SkillArgs {
    #[command(subcommand)]
    pub command: SkillCommands,
}

// `detour skill` 支持的命令。
#[derive(Debug, Subcommand)]
pub enum SkillCommands {
    /// 在项目 .detour 目录中创建 skill。
    Create(SkillInstallArgs),

    /// 把 skill 安装到目标目录。
    Install(SkillInstallArgs),

    /// 把 skill 内容打印到 stdout。
    Print(SkillPrintArgs),

    /// 打印目标环境的默认 skill 路径。
    Path(SkillPrintArgs),
}

// 会写文件的 skill 命令接受的参数。
#[derive(Debug, Args)]
pub struct SkillInstallArgs {
    /// skill 目标环境。
    #[arg(long, value_enum, default_value = "codex")]
    pub target: SkillTarget,

    /// 写入 skill 文件的目录。
    #[arg(long)]
    pub dir: Option<PathBuf>,

    /// 在支持的地方覆盖已有生成文件。
    #[arg(long)]
    pub force: bool,
}

// 只读取或打印信息的 skill 命令接受的参数。
#[derive(Debug, Args)]
pub struct SkillPrintArgs {
    /// skill 目标环境。
    #[arg(long, value_enum, default_value = "codex")]
    pub target: SkillTarget,
}

// detour 可以生成 skill 的助手环境。
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SkillTarget {
    Codex,
    Claude,
}
