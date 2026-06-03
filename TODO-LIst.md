# Detour CLI 实现步骤

## 目标概览

我们要用 Rust 开发一款 CLI 工具，暂名 `detour`。它的核心职责是在 AI 工具发生上下文压缩、摘要、清理或会话切换之前，把本轮会话里已经踩过的坑整理成“错题集”，按主题生成 `n` 个文档，并保存到一个指定文件夹中。之后，LLM 可以通过 CLI 或 skills 重新读取这些错题集，避免重复犯错。

这个工具需要覆盖三条主线：

1. 作为本地 CLI：能初始化配置、收集会话材料、生成错题集文档、检索和导出历史。
2. Hook 进 Claude Code / 类 Claude Code 工作流：参考 rust token killer 的“拦截/包装/触发”思路，在上下文压缩前自动执行。
3. 提供给 LLM 和 skills 使用：让模型知道什么时候调用 CLI、怎么调用、输出如何被消费，并提供可安装的 skill 文档。

下面的每一步都包含“实现什么、为什么这么写、如何写、验收方式”。你后续可以按步骤逐个和我确认、实现、验收。

## 阶段 0：确定边界与命名

### Step 0.1：定义产品名、命令名和核心概念

实现什么：

确定 CLI 名称、命令入口、文件夹结构和领域词汇。

建议初版命名：

- CLI 二进制：`detour`
- 错题集文件夹：`.detour/`
- 文档输出目录：`.detour/mistakes/`
- 会话输入目录：`.detour/sessions/`
- 配置文件：`.detour/config.toml`
- skills 目录：`.detour/skills/`

为什么得这么写：

这个项目以后会同时被人、LLM、Claude Code hook、skills 调用。如果一开始没有稳定命名，后续命令、文档、配置、skill 触发词都会反复变动，验收成本会很高。

如何写：

先在 README 或设计文档中固定这些术语，再让 Rust 代码和 skill 文档都引用同一套名称。

建议命令草案：

```bash
detour init
detour capture --from transcript.jsonl --out .detour/mistakes --max-docs 5
detour capture --stdin --max-docs 3
detour list
detour show <doc-id>
detour search "hook claude code"
detour skill install --target codex
detour skill install --target claude
```

验收方式：

- 我们能说清楚 `detour` 的输入、输出和默认目录。
- 后续步骤不再纠结“文件夹叫什么、命令叫什么”。

## 阶段 1：创建 Rust CLI 基础工程

### Step 1.1：初始化 Rust 项目

实现什么：

创建一个标准 Rust CLI 工程，提供可运行的 `detour` 命令。

为什么得这么写：

CLI 是整个系统的中心。先把命令骨架建好，后面 hook、skills、LLM 调用都只是围绕这个稳定入口扩展。

如何写：

使用 Cargo 初始化：

```bash
cargo init --bin
```

推荐依赖：

```toml
[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
time = { version = "0.3", features = ["formatting", "macros"] }
uuid = { version = "1", features = ["v4", "serde"] }
walkdir = "2"
thiserror = "1"
```

如果需要彩色终端输出，后续再加：

```toml
anstyle = "1"
```

建议目录：

```text
src/
  main.rs
  cli.rs
  config.rs
  capture.rs
  document.rs
  storage.rs
  llm.rs
  hooks/
    mod.rs
    claude.rs
  skills/
    mod.rs
```

验收方式：

- `cargo run -- --help` 能输出命令帮助。
- `detour init` 命令存在，即使暂时只打印提示。

### Step 1.2：实现命令行参数结构

实现什么：

用 `clap` 定义所有初版子命令和参数。

为什么得这么写：

Claude Code hook、LLM skill、用户手动调用都需要稳定、可文档化的命令接口。`clap` 可以自动生成 help，也方便后续测试。

如何写：

命令建议：

```text
detour init
detour capture
detour list
detour show
detour search
detour hook
detour skill
```

`capture` 参数建议：

```text
--from <PATH>         从会话文件读取
--stdin              从标准输入读取
--out <DIR>          输出错题集目录
--max-docs <N>       最多生成多少个文档
--format <md|json>   输出格式，初版默认 md
--dry-run            只展示将要生成什么，不写文件
```

验收方式：

- 每个命令都有 `--help`。
- 参数错误时给出清晰错误。
- `cargo test` 至少覆盖一次 CLI 参数解析。

## 阶段 2：定义错题集数据模型

### Step 2.1：设计输入会话模型

实现什么：

定义 CLI 接收的会话材料格式。

建议初版支持两种输入：

1. JSONL：每行一条消息。
2. Markdown / plain text：作为兜底格式。

JSONL 示例：

```jsonl
{"role":"user","content":"帮我改这个测试","timestamp":"2026-06-03T10:00:00Z"}
{"role":"assistant","content":"我改了 A 文件，但测试失败了","timestamp":"2026-06-03T10:01:00Z"}
{"role":"tool","content":"error: missing field title","timestamp":"2026-06-03T10:02:00Z"}
```

为什么得这么写：

不同 AI 工具有不同会话记录格式。我们不能一开始就绑定某个工具内部格式。JSONL 适合机器写入，Markdown 适合人工复制。

如何写：

Rust 类型建议：

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationEvent {
    pub role: String,
    pub content: String,
    pub timestamp: Option<String>,
    pub source: Option<String>,
}
```

解析策略：

- 如果输入是 `.jsonl`，逐行解析。
- 如果输入是 `.json`，尝试解析数组。
- 其他格式按纯文本读取，包装成一条 `ConversationEvent`。

验收方式：

- 能读取 JSONL。
- 能读取普通文本。
- 遇到坏 JSONL 行时，错误信息包含行号。

### Step 2.2：设计错题集模型

实现什么：

定义每条“错题”的结构，以及多个错题如何组成文档。

建议模型：

```rust
pub struct Mistake {
    pub id: String,
    pub title: String,
    pub symptom: String,
    pub root_cause: String,
    pub wrong_turn: String,
    pub correction: String,
    pub prevention_rule: String,
    pub evidence: Vec<String>,
    pub tags: Vec<String>,
    pub severity: Severity,
}

pub enum Severity {
    Low,
    Medium,
    High,
}

pub struct MistakeDoc {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub mistakes: Vec<Mistake>,
    pub created_at: String,
    pub source_session: Option<String>,
}
```

为什么得这么写：

“错题集”不是普通总结。它必须记录：

- 错在哪里。
- 当时为什么会走错。
- 正确做法是什么。
- 下次如何避免。
- 证据来自哪里。

这会让文档在下一次被 LLM 读取时更像规则库，而不是流水账。

如何写：

先实现 Markdown 渲染，不急着做数据库。

Markdown 模板建议：

```markdown
# {title}

生成时间：{created_at}
来源会话：{source_session}

## 摘要

{summary}

## 错题 1：{mistake.title}

### 现象

{symptom}

### 错误路径

{wrong_turn}

### 根因

{root_cause}

### 正确做法

{correction}

### 下次预防规则

{prevention_rule}

### 证据

- {evidence}

### 标签

{tags}
```

验收方式：

- 程序能把一个 `MistakeDoc` 写成 `.md` 文件。
- 文件名稳定，例如 `2026-06-03-context-compression-hooks.md`。

## 阶段 3：实现本地生成流程

### Step 3.1：先做规则版 mistake extractor

实现什么：

在不接入 LLM 的情况下，从会话文本中用简单规则提取可能的坑。

为什么得这么写：

初版不要直接依赖外部大模型。规则版可以让 CLI 在离线环境可测试，也能为后续 LLM 版提供基线。

如何写：

先识别这些关键词：

- 错误：`error`, `failed`, `panic`, `失败`, `报错`, `异常`
- 修复：`fix`, `resolved`, `改成`, `修复`, `解决`
- 约束：`不要`, `必须`, `注意`, `以后`, `下次`
- 工具问题：`sandbox`, `permission`, `hook`, `token`, `context`, `compression`

初版逻辑：

1. 读取所有事件。
2. 按时间顺序合并成文本块。
3. 找到包含错误关键词的片段。
4. 向前取 2 条消息作为上下文，向后取 3 条消息作为修复线索。
5. 生成临时 `Mistake`。
6. 按 tag 或关键词聚类成最多 `n` 个 `MistakeDoc`。

验收方式：

- 给一段包含失败和修复的文本，能生成至少一条错题。
- `--max-docs 1` 时只生成一个 Markdown 文件。
- `--dry-run` 不写文件，但打印将生成的文档标题。

### Step 3.2：实现文档分组策略

实现什么：

把会话中多个坑分成 `n` 个主题文档。

为什么得这么写：

用户明确要求“对应 n 个文档”。如果只是生成一个总文档，后续 skills 和检索会变得笨重。

如何写：

初版分组策略：

- 根据 tag 聚合：`hook`、`llm-interface`、`skills`、`rust-cli`、`filesystem`。
- 如果 tag 数量超过 `n`，保留数量最多的前 `n` 组。
- 如果 tag 数量不足 `n`，不要硬拆，生成实际需要的文档数量。

文档标题示例：

```text
Claude Code Hook 错题集
LLM 调用接口错题集
Skills 集成错题集
Rust CLI 实现错题集
```

验收方式：

- 同一类问题进入同一个文档。
- 文件名可读、稳定、不包含非法字符。
- `--max-docs 3` 不会生成超过 3 个文档。

## 阶段 4：接入 LLM 生成器

### Step 4.1：定义 LLM Provider 抽象

实现什么：

把“如何让大模型整理错题集”封装成可替换接口。

为什么得这么写：

这个 CLI 未来可能被不同环境使用：OpenAI、Claude、本地模型、或者完全离线。把 provider 抽象出来，避免核心逻辑和某一家 API 绑定。

如何写：

Rust trait 草案：

```rust
#[async_trait::async_trait]
pub trait MistakeGenerator {
    async fn generate(
        &self,
        session: &[ConversationEvent],
        options: GenerateOptions,
    ) -> anyhow::Result<Vec<MistakeDoc>>;
}
```

Provider 建议：

```text
RuleBasedGenerator
OpenAiGenerator
ClaudeGenerator
ExternalCommandGenerator
```

初版可以先只实现：

```text
RuleBasedGenerator
ExternalCommandGenerator
```

`ExternalCommandGenerator` 的意义是：CLI 把会话 JSON 发给一个外部命令，由外部命令返回结构化 JSON。这样可以在没有 SDK 的情况下接入任意 LLM。

验收方式：

- `detour capture --provider rules` 可用。
- `detour capture --provider command --cmd "..."` 的接口设计明确。
- LLM 输出 JSON 校验失败时，能回退到 rules 或给出清晰错误。

### Step 4.2：编写 LLM 提示词协议

实现什么：

定义 CLI 发给 LLM 的 prompt，以及 LLM 必须返回的 JSON schema。

为什么得这么写：

LLM 如果自由发挥，会生成风格漂亮但难以机器处理的文档。我们需要让它先返回结构化 JSON，再由 CLI 统一渲染 Markdown。

如何写：

Prompt 核心要求：

```text
你是一个会话复盘助手。请从输入会话中提取已经踩过的坑，并整理为错题集。

必须关注：
- 错误现象
- 错误路径
- 根因
- 正确做法
- 下次预防规则
- 来自会话的证据

请返回 JSON，不要返回 Markdown。
最多生成 {max_docs} 个文档。
```

返回 JSON 草案：

```json
{
  "documents": [
    {
      "title": "Claude Code Hook 错题集",
      "summary": "本文件记录 Claude Code hook 集成时的关键坑。",
      "mistakes": [
        {
          "title": "不要假设压缩前一定有公开 hook",
          "symptom": "希望在上下文压缩前自动触发 CLI，但目标工具未必暴露该生命周期事件。",
          "wrong_turn": "直接设计 pre-compress hook，忽略实际工具是否支持。",
          "root_cause": "没有先确认 Claude Code 的 hook 能力边界。",
          "correction": "先实现 wrapper / transcript watcher / slash command 三种路径，再按可用性选择。",
          "prevention_rule": "涉及第三方工具生命周期时，先做能力探测，再设计强绑定 hook。",
          "evidence": ["用户提到需要 hook 进 claudecode，可参考 rust token killer。"],
          "tags": ["claude-code", "hook"],
          "severity": "high"
        }
      ]
    }
  ]
}
```

验收方式：

- Provider 能解析 JSON。
- 缺字段时报错能指向具体字段。
- Markdown 仍由 CLI 渲染，而不是完全信任 LLM 文本。

## 阶段 5：Hook 进 Claude Code / 类 Claude Code 工作流

### Step 5.1：调研 Claude Code 可用集成点

实现什么：

确认 Claude Code 当前能否提供：

- hooks 配置。
- slash commands。
- MCP server。
- transcript / session 文件。
- wrapper 命令。
- 环境变量或事件通知。

为什么得这么写：

用户的关键需求是“在大模型压缩上下文之前”。这个点不能靠猜。根据 Claude Code 当前官方 hooks 文档，生命周期事件里已经包含 `PreCompact` 和 `PostCompact`，其中 `PreCompact` 的触发时机就是 context compaction 之前。因此第一优先级应该是官方 `PreCompact` hook，而不是先做 wrapper。

不过仍然需要保留降级方案，因为用户机器上的 Claude Code 版本、企业策略、settings 写入权限、项目配置方式都可能不同。

如何写：

调研顺序：

1. 查 Claude Code 官方文档里 hooks、slash commands、MCP 的能力。
2. 阅读 rust token killer 的实现，重点看它如何发现 token 状态、如何触发压缩或中断。
3. 确认是否能获得当前会话 transcript。
4. 确认是否能在压缩前运行外部命令。

预期接入优先级：

```text
第一优先级：Claude Code PreCompact hook，直接调用 detour capture。
第二优先级：Claude Code slash command，让用户或 LLM 显式触发错题集。
第三优先级：wrapper 或 watcher 监控 transcript/token 阈值，在接近压缩前提示或自动运行。
```

验收方式：

- 产出 `docs/claude-code-integration.md`。
- 明确写出“可直接 hook 的路径”和“不可直接 hook 时的兜底路径”。

### Step 5.2：参考 rust token killer 的触发模式

实现什么：

借鉴 rust token killer 的思路，设计 detour 的触发器。

为什么得这么写：

rust token killer 这类工具通常不是直接修改 Claude Code 内核，而是通过包装、监控 token、拦截命令或辅助生命周期事件来工作。我们的工具也应该避免依赖脆弱的私有实现。

RTK 官网和 GitHub README 的公开说明显示，它面向 Claude Code 的安装入口是 `rtk init --claude-code`，并通过 Claude Code 的 `PreToolUse` hook 透明改写 Bash 调用。detour 可以借鉴这个模式：提供 `detour hook claude install`，由 CLI 自动生成 Claude Code settings 片段，把 Rust 二进制接到指定 hook 事件上。

如何写：

需要重点参考：

- 它如何安装到用户环境。
- 它是否包装原始 Claude Code 命令。
- 它如何计算或感知 token。
- 它如何决定何时触发。
- 它如何处理失败和回退。

detour 的触发策略建议：

```text
detour hook claude install
detour hook claude status
detour hook claude uninstall
detour hook claude run --session <path> --max-docs 5
```

针对 Claude Code 第一版更建议：

```text
Claude Code PreCompact hook -> detour hook claude run-precompact -> detour capture
```

如果后续仍需要 wrapper：

```text
claude -> detour-claude-wrapper -> real claude
```

wrapper 的职责：

- 启动真实 Claude Code。
- 记录或定位 transcript。
- 监测 token / 上下文长度 / 压缩信号。
- 在接近阈值时运行 `detour capture`。
- 不阻塞主流程，失败时只报警。

验收方式：

- 能说明是否采用 wrapper、watcher、slash command 或 hook。
- `detour hook claude status` 能告诉用户当前集成是否可用。

### Step 5.3：实现 Claude Code PreCompact hook

实现什么：

在 Claude Code 项目级 settings 中安装一个 `PreCompact` hook，让上下文压缩发生前自动调用 detour。

为什么得这么写：

这正好对应用户提出的“在大模型压缩上下文之前”。官方 hook 比 wrapper 更直接，也更少侵入交互体验。

如何写：

命令设计：

```bash
detour hook claude install --event pre-compact
detour hook claude uninstall --event pre-compact
detour hook claude status --json
detour hook claude print-config --event pre-compact
```

生成的配置应优先写项目级：

```text
.claude/settings.json
```

不要默认写全局：

```text
~/.claude/settings.json
```

除非用户显式传：

```bash
detour hook claude install --global
```

配置思路：

```json
{
  "hooks": {
    "PreCompact": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "detour hook claude run-precompact --stdin --json"
          }
        ]
      }
    ]
  }
}
```

`run-precompact` 的职责：

- 从 stdin 读取 Claude Code 传入的 hook JSON。
- 尽量解析 compaction trigger，例如 `manual` 或 `auto`。
- 定位项目根目录和 `.detour/config.toml`。
- 触发 `detour capture` 或生成一个 capture request。
- 输出结构化 JSON。
- 失败时不要阻断 Claude Code 的压缩，除非用户配置了 strict 模式。

验收方式：

- `print-config` 能打印可复制的 PreCompact hook 配置。
- `install` 会备份已有 `.claude/settings.json`。
- hook 被触发时能创建错题集文档。
- detour 失败时 Claude Code 不会被硬卡死。

### Step 5.4：实现 watcher 兜底路径

实现什么：

当没有可靠 pre-compress hook 时，实现文件 watcher 或显式命令触发。

为什么得这么写：

第三方 AI 工具的生命周期 hook 不一定稳定。watcher 是最现实的兜底：只要能拿到 transcript 或用户能导出会话，就能生成错题集。

如何写：

可选依赖：

```toml
notify = "6"
```

watcher 行为：

```bash
detour hook claude watch --session-dir <DIR> --threshold 0.8 --max-docs 5
```

初版可以先不做精确 token 计算，只做：

- 文件大小阈值。
- 消息数量阈值。
- 手动触发。

后续再引入 tokenizer：

```text
tokenizers / tiktoken-rs / 模型相关 tokenizer
```

验收方式：

- 修改 session 文件时 watcher 能识别。
- 达到阈值后生成错题集。
- 重复触发时不会覆盖旧文件，或能以版本号保存。

## 阶段 6：设计 CLI 提供给 LLM 的接口

### Step 6.1：让 CLI 输出 LLM 友好的结果

实现什么：

为 LLM 调用场景提供稳定、短小、结构化的输出。

为什么得这么写：

LLM 使用工具时不适合读一大坨日志。它需要知道：

- 成功还是失败。
- 生成了哪些文件。
- 下次应该读哪个文件。
- 是否有需要用户介入的问题。

如何写：

所有关键命令支持：

```text
--json
--quiet
--no-color
```

示例：

```bash
detour capture --stdin --max-docs 3 --json
```

输出：

```json
{
  "ok": true,
  "documents": [
    {
      "id": "20260603-claude-hook",
      "title": "Claude Code Hook 错题集",
      "path": ".detour/mistakes/20260603-claude-hook.md",
      "mistake_count": 4
    }
  ],
  "warnings": []
}
```

验收方式：

- `--json` 输出没有多余人类日志。
- 失败时仍返回结构化错误。
- LLM 可以只根据 JSON 决定下一步。

### Step 6.2：提供读取与检索命令

实现什么：

让 LLM 后续能读取错题集，而不是只生成。

为什么得这么写：

错题集的价值在下一次会话。如果 LLM 只能写不能读，就无法形成持续改进闭环。

如何写：

命令：

```bash
detour list --json
detour show <id-or-path> --json
detour search "claude hook" --json
```

搜索初版可以用纯文本包含匹配，后续再做向量检索。

验收方式：

- `detour list` 能列出所有文档。
- `detour show` 能输出某个文档的内容或元数据。
- `detour search` 能按关键词找到相关文档。

## 阶段 7：创建使用这个 CLI 的 skills

### Step 7.1：设计 Codex Skill

实现什么：

创建一个给 Codex 使用的 skill，让 Codex 知道什么时候调用 `detour`。

为什么得这么写：

只提供 CLI 还不够。LLM 需要明确的操作规约：什么场景触发、调用什么命令、如何处理结果、不能做什么。

如何写：

skill 目录建议：

```text
skills/detour/
  SKILL.md
  examples/
    capture-session.md
    read-mistakes.md
```

`SKILL.md` 内容建议：

```markdown
# Detour Mistake Notebook

Use this skill when a conversation is approaching context compression,
when the user asks to preserve lessons learned, or when repeated mistakes
should be converted into reusable notes.

## Workflow

1. Gather the available conversation transcript or summary.
2. Run `detour capture --stdin --max-docs <n> --json`.
3. Read the JSON result and report generated files to the user.
4. Before similar future work, run `detour search <topic> --json`.

## Constraints

- Do not invent evidence that is not present in the conversation.
- Prefer structured JSON output when calling the CLI.
- If the CLI is unavailable, write the same Markdown format manually.
```

验收方式：

- Codex 能根据 skill 描述知道何时使用 detour。
- skill 包含至少一个 capture 示例和一个 search 示例。

### Step 7.2：设计 Claude Code 可用的命令说明

实现什么：

给 Claude Code 或其他 agent 环境提供一份可复制的 slash command / instruction 文档。

为什么得这么写：

不同 agent 对 skill 的格式不一定一样。我们需要把 CLI 能力翻译成 Claude Code 可理解的命令或项目说明。

如何写：

建议生成：

```text
.claude/commands/detour-capture.md
.claude/commands/detour-search.md
```

`detour-capture.md` 示例：

```markdown
请把当前会话中已经踩过的坑整理为错题集。

如果可以调用 shell，请运行：

detour capture --stdin --max-docs 5 --json

输入应包含当前会话的关键消息、错误、修复尝试和最终结论。
生成后请告诉用户文件路径。
```

验收方式：

- Claude Code 中能通过 slash command 或手动命令触发。
- 文档明确告诉模型如何收集输入和解释输出。

### Step 7.3：实现 `detour skill install`

实现什么：

让 CLI 自动把 skills / commands 安装到目标目录。

为什么得这么写：

如果每次都让用户手动复制 skill 文件，工具很难推广，也容易装错路径。

如何写：

命令设计：

```bash
detour skill install --target codex
detour skill install --target claude
detour skill print --target codex
detour skill print --target claude
```

初版行为：

- `print`：把 skill 内容打印到 stdout。
- `install`：写入默认目录，或通过 `--dir` 指定目录。

目录示例：

```text
detour skill install --target codex --dir ~/.codex/skills/detour
detour skill install --target claude --dir .claude/commands
```

验收方式：

- 能打印 skill 内容。
- 能安装到指定目录。
- 已存在文件时默认不覆盖，除非传 `--force`。

## 阶段 8：配置、存储与文件安全

### Step 8.1：实现配置文件

实现什么：

支持 `.detour/config.toml`。

为什么得这么写：

hook、输出路径、文档数量、provider、skill 目标目录都需要可配置，否则命令会越来越长。

如何写：

配置草案：

```toml
[capture]
default_max_docs = 5
output_dir = ".detour/mistakes"
provider = "rules"

[hook.claude]
enabled = false
mode = "watch"
session_dir = ""
threshold = 0.8

[llm]
provider = "command"
command = ""
```

命令优先级：

```text
CLI 参数 > 环境变量 > config.toml > 默认值
```

验收方式：

- `detour init` 能创建配置文件。
- CLI 参数能覆盖配置。
- 配置格式错误时给出文件路径和字段。

### Step 8.2：实现安全写文件策略

实现什么：

确保生成文档时不误删、不覆盖、不写到危险路径。

为什么得这么写：

这个工具会被 LLM 调用。LLM 生成路径时可能犯错，所以 CLI 必须自己兜底。

如何写：

策略：

- 默认只写 `.detour/`。
- `--out` 指向其他目录时，要求目录存在或显式 `--create-dir`。
- 文件名做 slug sanitize。
- 同名文件加序号或时间戳。
- 不跟随可疑 symlink 写入，除非显式允许。

验收方式：

- 重复运行不会覆盖旧文档。
- 非法文件名会被转换。
- `--dry-run` 能展示将写入的路径。

## 阶段 9：测试体系

### Step 9.1：单元测试

实现什么：

覆盖解析、分组、Markdown 渲染、文件名生成、配置合并。

为什么得这么写：

CLI 未来会被 hook 自动调用。如果这些底层逻辑不稳，自动化触发会产生大量坏文档。

如何写：

测试模块：

```text
tests/cli.rs
tests/capture.rs
tests/document.rs
tests/config.rs
tests/skills.rs
```

建议依赖：

```toml
[dev-dependencies]
assert_cmd = "2"
predicates = "3"
tempfile = "3"
insta = "1"
```

验收方式：

- `cargo test` 通过。
- Markdown 输出可用 snapshot 测试固定格式。

### Step 9.2：集成测试

实现什么：

模拟完整流程：输入会话、生成错题集、list、show、search。

为什么得这么写：

这个工具的价值是端到端闭环，不只是某个函数能跑。

如何写：

准备 fixture：

```text
tests/fixtures/session-basic.jsonl
tests/fixtures/session-claude-hook.md
```

测试命令：

```bash
detour capture --from tests/fixtures/session-basic.jsonl --out <tmp> --max-docs 2
detour list --root <tmp>
detour search "hook" --root <tmp>
```

验收方式：

- 临时目录中生成预期数量文档。
- JSON 输出可被解析。
- 搜索能命中标题或正文。

## 阶段 10：发布与安装

### Step 10.1：本地安装

实现什么：

让用户能把 CLI 安装到 PATH。

为什么得这么写：

hook 和 skills 调用都假设 `detour` 是一个可执行命令。

如何写：

本地安装：

```bash
cargo install --path .
```

验证：

```bash
detour --version
detour --help
```

验收方式：

- 新终端能运行 `detour`。
- `detour init` 能在任意项目目录创建 `.detour/`。

### Step 10.2：跨平台发布

实现什么：

准备 Windows、macOS、Linux 的 release。

为什么得这么写：

Claude Code / Codex 用户分布在不同系统，hook 路径和 shell 行为也不同。

如何写：

后续可引入：

```text
cargo-dist
GitHub Actions
shell completions
```

验收方式：

- 三个平台都能下载二进制。
- release 包含安装说明。

## 推荐实现顺序

第一轮先做最小可用闭环：

1. `cargo init --bin`
2. `clap` 命令骨架
3. `.detour/config.toml`
4. JSONL / 文本输入解析
5. rules provider
6. Markdown 文档生成
7. `capture --from / --stdin --max-docs --json`
8. `list / show / search`
9. Codex skill 文档
10. Claude Code slash command 文档

第二轮再做自动化集成：

1. 调研 Claude Code hooks / slash commands / MCP / transcript 能力。
2. 阅读 rust token killer 的触发方式。
3. 实现 `detour hook claude status`。
4. 实现 `detour hook claude watch`。
5. 视调研结果决定是否实现 wrapper。

第三轮再做 LLM 增强：

1. LLM JSON schema。
2. `ExternalCommandGenerator`。
3. OpenAI / Claude provider。
4. prompt 版本管理。
5. 更好的聚类和去重。

## 当前第一步建议

下一次对话建议从 `Step 0.1` 和 `Step 1.1` 开始验收：

我们先确认命令名、目录名和最小命令集合，然后初始化 Rust 项目。确认后，就可以实现：

```bash
detour init
detour capture --from <file> --max-docs <n>
detour list
detour show <id>
detour search <query>
```

这会形成第一条可运行主线：把会话材料变成可复用错题集文档。

## 参考资料

- Claude Code hooks reference：https://docs.anthropic.com/en/docs/claude-code/hooks
- Claude Code slash commands：https://docs.anthropic.com/en/docs/claude-code/slash-commands
- Claude Code settings：https://docs.anthropic.com/en/docs/claude-code/settings
- Claude Code skills：https://docs.anthropic.com/en/docs/claude-code/skills
- Model Context Protocol：https://modelcontextprotocol.io/
- RTK / Rust Token Killer：https://github.com/Mariozechner/claude-code-token-killer

参考要点：

- Claude Code hooks 文档列出了 `PreCompact`、`PostCompact`、`PreToolUse`、`SessionStart`、`SessionEnd` 等 hook 事件，并说明 command hook 会通过 stdin 接收 JSON。
- Claude Code slash commands 可用于生成 `.claude/commands/detour-capture.md` 这类显式触发入口。
- Claude Code settings 文档用于确认 project settings、local settings、user settings 的写入位置和优先级。
- RTK / Rust Token Killer 公开说明中提到 `rtk init --claude-code` 会安装 Claude Code `PreToolUse` hook，用 Rust CLI 在 Claude Code 与 shell 命令之间做透明处理；detour 可以借鉴它的安装和 hook 注入方式，但 hook 事件应优先选择 `PreCompact`。
