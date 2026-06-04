use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

// Top-level command parser for the `detour` binary.
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

// All first-level commands supported by `detour`.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize detour configuration and local storage.
    Init(InitArgs),

    /// Capture a session and render mistake-notebook documents.
    Capture(CaptureArgs),

    /// List saved mistake documents.
    List(ListArgs),

    /// Show one saved mistake document.
    Show(ShowArgs),

    /// Search saved mistake documents.
    Search(SearchArgs),

    /// Manage integrations with AI tools.
    Hook(HookArgs),

    /// Create, print, or install LLM skills.
    Skill(SkillArgs),
}

// Options accepted by `detour init`.
#[derive(Debug, Args)]
pub struct InitArgs {
    /// Directory where .detour should be created.
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    /// Overwrite existing generated files where supported.
    #[arg(long)]
    pub force: bool,
}

// Options accepted by `detour capture`.
#[derive(Debug, Args)]
pub struct CaptureArgs {
    /// Read conversation material from a file.
    #[arg(long = "from", value_name = "PATH")]
    pub from: Option<PathBuf>,

    /// Read conversation material from standard input.
    #[arg(long)]
    pub stdin: bool,

    /// Directory where mistake documents should be written.
    #[arg(long, value_name = "DIR")]
    pub out: Option<PathBuf>,

    /// Maximum number of documents to generate.
    #[arg(long, default_value_t = 5)]
    pub max_docs: usize,

    /// Output document format.
    #[arg(long, value_enum, default_value = "md")]
    pub format: CaptureFormat,

    /// Print what would happen without writing files.
    #[arg(long)]
    pub dry_run: bool,

    /// Print machine-readable JSON for the command result.
    #[arg(long)]
    pub json: bool,
}

// Document formats supported by the capture command.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CaptureFormat {
    #[value(name = "md")]
    Markdown,
    Json,
}

// Options accepted by `detour list`.
#[derive(Debug, Args)]
pub struct ListArgs {
    /// Project or detour root directory.
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    /// Print machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

// Options accepted by `detour show`.
#[derive(Debug, Args)]
pub struct ShowArgs {
    /// Document id or path to show.
    pub id_or_path: String,

    /// Print machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

// Options accepted by `detour search`.
#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Search query.
    pub query: String,

    /// Project or detour root directory.
    #[arg(long, default_value = ".")]
    pub root: PathBuf,

    /// Print machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

// Nested command group for AI-tool integrations.
#[derive(Debug, Args)]
pub struct HookArgs {
    #[command(subcommand)]
    pub command: HookCommands,
}

// AI tools that can be managed by `detour hook`.
#[derive(Debug, Subcommand)]
pub enum HookCommands {
    /// Manage Claude Code integration.
    Claude(ClaudeHookArgs),
}

// Nested command group for Claude Code hook integration.
#[derive(Debug, Args)]
pub struct ClaudeHookArgs {
    #[command(subcommand)]
    pub command: ClaudeHookCommands,
}

// Commands supported under `detour hook claude`.
#[derive(Debug, Subcommand)]
pub enum ClaudeHookCommands {
    /// Detect Claude Code and detour integration status.
    Detect(JsonFlag),

    /// Install Claude Code integration files.
    Install(ClaudeHookInstallArgs),

    /// Print Claude Code integration status.
    Status(JsonFlag),

    /// Print Claude Code settings snippets without writing files.
    PrintConfig(ClaudeHookPrintConfigArgs),

    /// Remove detour Claude Code integration files.
    Uninstall(ClaudeHookUninstallArgs),

    /// Entry point intended for Claude Code PreCompact hooks.
    RunPrecompact(JsonFlag),
}

// Reusable `--json` flag shared by simple status-like commands.
#[derive(Debug, Args)]
pub struct JsonFlag {
    /// Print machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

// Options accepted by `detour hook claude install`.
#[derive(Debug, Args)]
pub struct ClaudeHookInstallArgs {
    /// Integration mode to install.
    #[arg(long, value_enum, default_value = "pre-compact")]
    pub mode: ClaudeHookMode,

    /// Write user-level Claude Code settings instead of project-level settings.
    #[arg(long)]
    pub global: bool,

    /// Overwrite existing generated files where supported.
    #[arg(long)]
    pub force: bool,
}

// Options accepted by `detour hook claude print-config`.
#[derive(Debug, Args)]
pub struct ClaudeHookPrintConfigArgs {
    /// Hook event to print configuration for.
    #[arg(long, value_enum, default_value = "pre-compact")]
    pub event: ClaudeHookEvent,
}

// Options accepted by `detour hook claude uninstall`.
#[derive(Debug, Args)]
pub struct ClaudeHookUninstallArgs {
    /// Hook event to remove.
    #[arg(long, value_enum, default_value = "pre-compact")]
    pub event: ClaudeHookEvent,

    /// Remove user-level Claude Code settings instead of project-level settings.
    #[arg(long)]
    pub global: bool,
}

// Claude Code integration modes that detour can install.
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

// Claude Code hook events that detour can print or uninstall.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ClaudeHookEvent {
    #[value(name = "pre-compact")]
    PreCompact,
}

// Nested command group for skill-related actions.
#[derive(Debug, Args)]
pub struct SkillArgs {
    #[command(subcommand)]
    pub command: SkillCommands,
}

// Commands supported under `detour skill`.
#[derive(Debug, Subcommand)]
pub enum SkillCommands {
    /// Create a skill in the project .detour directory.
    Create(SkillInstallArgs),

    /// Install a skill into a target directory.
    Install(SkillInstallArgs),

    /// Print skill content to stdout.
    Print(SkillPrintArgs),

    /// Print the default skill path for a target.
    Path(SkillPrintArgs),
}

// Options accepted by skill commands that write files.
#[derive(Debug, Args)]
pub struct SkillInstallArgs {
    /// Skill target environment.
    #[arg(long, value_enum)]
    pub target: SkillTarget,

    /// Directory where skill files should be written.
    #[arg(long)]
    pub dir: Option<PathBuf>,

    /// Overwrite existing generated files where supported.
    #[arg(long)]
    pub force: bool,
}

// Options accepted by skill commands that only read or print information.
#[derive(Debug, Args)]
pub struct SkillPrintArgs {
    /// Skill target environment.
    #[arg(long, value_enum)]
    pub target: SkillTarget,
}

// Assistant environments that detour can generate skills for.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SkillTarget {
    Codex,
    Claude,
}
