# Detour 代码地图

这份文档用于多人协作时快速定位功能实现位置。新成员可以先读这里，再进入具体源码文件。

## 项目入口

### `Cargo.toml`

Rust 项目配置文件。

主要负责：

- 定义包名、版本、Rust edition。
- 定义二进制命令名 `detour`。
- 管理依赖，例如 `clap`、`serde`、`serde_json`、`anyhow`。

协作注意：

- 新增第三方库时，在这里添加依赖。
- CLI 项目应提交 `Cargo.lock`，以锁定依赖版本。

### `Cargo.lock`

Cargo 自动生成的依赖锁定文件。

主要负责：

- 记录所有直接和间接依赖的精确版本。
- 保证不同开发者构建时依赖版本一致。

协作注意：

- 本项目是 CLI/binary 项目，`Cargo.lock` 应提交。
- 不要手动编辑，通常由 `cargo build`、`cargo test`、`cargo update` 生成或更新。

## 命令行入口

### `src/main.rs`

CLI 程序运行入口。

主要负责：

- 调用 `Cli::parse()` 解析命令行参数。
- 根据用户输入的子命令分发到对应功能。
- 控制最终输出到终端的内容。

当前已接入：

- `capture`
- `list`
- `show`
- `search`
- `recent`
- `rules`
- `schema`
- `prompt`
- `hook claude`
- `skill`

协作注意：

- 如果只是新增参数定义，优先改 `src/cli.rs`。
- 如果要让命令真正执行逻辑，再改 `src/main.rs`。
- `main.rs` 应尽量只做分发和输出，不堆复杂业务逻辑。

### `src/cli.rs`

命令行参数定义文件。

主要负责：

- 定义 `detour` 支持哪些命令。
- 定义每个命令有哪些参数。
- 使用 `clap` 自动生成 help 文案。

当前主要结构：

- `Cli`
- `Commands`
- `InitArgs`
- `CaptureArgs`
- `ListArgs`
- `ShowArgs`
- `SearchArgs`
- `RecentArgs`
- `RulesArgs`
- `HookArgs`
- `SkillArgs`
- `PromptArgs`

协作注意：

- 新增 CLI 子命令时，先在 `Commands` 中添加枚举项。
- 新增命令参数时，为该命令添加或修改 `Args` 结构体。
- 注释会影响 CLI help 输出，保持简短清晰。

## 库入口

### `src/lib.rs`

Rust 库入口文件。

主要负责：

- 导出可复用模块。
- 让 `main.rs`、测试、未来 hook、MCP server 能共用同一套逻辑。

当前导出：

- `capture`
- `claude`
- `cli`
- `extractor`
- `ledger`
- `llm`
- `models`
- `render`
- `skills`
- `storage`

协作注意：

- 新增可复用模块时，需要在这里 `pub mod xxx;`。
- 不要把业务逻辑直接写在 `lib.rs` 中。

## 数据模型

### `src/models/mod.rs`

模型模块出口。

主要负责：

- 导出 `conversation` 模块。
- 导出 `mistake` 模块。
- 通过 `pub use` 简化外部引用路径。

协作注意：

- 新增模型子模块时，在这里声明并导出。

### `src/models/conversation.rs`

输入会话模型。

主要负责：

- 定义会话事件结构。
- 定义会话角色。
- 定义输入格式。
- 解析 JSON、JSONL、普通文本。

核心类型和函数：

- `ConversationEvent`
- `ConversationRole`
- `ConversationFormat`
- `parse_conversation`
- `parse_json`
- `parse_json_lines`
- `parse_text`

协作注意：

- 如果要支持新的输入格式，例如 transcript 文件格式，应优先改这里。
- 当前 JSON schema 与这里的字段保持一致。

### `src/models/mistake.rs`

错题集模型。

主要负责：

- 定义错题集文档。
- 定义单条错题。
- 定义证据。
- 定义严重程度。

核心类型：

- `MistakeDocument`
- `Mistake`
- `Evidence`
- `EvidenceKind`
- `Severity`

协作注意：

- 修改这些字段会影响渲染、存储、schema、未来 LLM 输入输出契约。
- 改动模型时要同步检查 `render.rs`、`storage.rs`、`llm.rs`。

## 生成流程

### `src/capture.rs`

`detour capture` 的核心流程。

主要负责：

- 从文件或 stdin 读取输入。
- 自动识别输入格式。
- 调用会话解析函数。
- 调用规则提取器生成错题集。
- 根据 `--dry-run` 决定是否写入本地目录。
- 返回 `CaptureResult`。

核心函数：

- `capture_from_args`
- `read_input`
- `detect_format_from_path`
- `detect_format_from_text`

协作注意：

- `capture.rs` 是命令入口到业务流程的连接层。
- 不要在这里实现复杂提取规则，提取逻辑应放在 `extractor.rs`。
- 不要在这里实现 Markdown 细节，渲染逻辑应放在 `render.rs`。

### `src/extractor.rs`

本地规则版错题提取器。

主要负责：

- 从 `ConversationEvent` 中识别疑似踩坑消息。
- 根据关键词分类主题。
- 生成 `Mistake`。
- 按主题聚合成 `MistakeDocument`。

当前主题：

- `claude-code`
- `rust-cli`
- `shell-env`
- `llm-interface`
- `filesystem`
- `general`

核心函数：

- `extract_mistake_documents`
- `looks_like_mistake_candidate`
- `classify_tag`
- `document_title`
- `root_cause_for`
- `correction_for`
- `prevention_rule_for`

协作注意：

- 如果要改“什么算踩坑”，改关键词和 `looks_like_mistake_candidate`。
- 如果要改分类，改 `classify_tag`。
- 如果要改错题内容模板，改 `root_cause_for`、`correction_for`、`prevention_rule_for`。
- 未来接 LLM 生成器时，可以保留这里作为离线兜底。

## 渲染与存储

### `src/render.rs`

Markdown 渲染模块。

主要负责：

- 把 `MistakeDocument` 渲染为 Markdown。
- 控制错题集文档的小节结构。

当前 Markdown 结构：

- 标题
- 生成时间
- 标签
- 摘要
- 错题
- 错误现象
- 错误路径
- 根因
- 修正方式
- 下次预防规则
- 证据
- 标签

协作注意：

- 想调整 Markdown 文档样式，改这里。
- 不要在这里处理文件写入。
- 不要在这里处理提取逻辑。

### `src/storage.rs`

错题集写入模块。

主要负责：

- 创建输出目录。
- 把 Markdown 或 JSON 写入磁盘。
- 生成不会覆盖旧文件的路径。
- 返回已保存文档元数据。

核心类型和函数：

- `StoredDocumentFormat`
- `SavedDocument`
- `write_documents`

协作注意：

- 默认写入路径由 `capture.rs` 决定。
- 文件命名和防覆盖逻辑在这里。
- 不要在这里解析 Markdown 或搜索文档。

## 本地存折读取

### `src/ledger.rs`

本地错题集读取和检索模块。

主要负责：

- 列出 `.detour/mistakes/` 中的文档。
- 根据 ID 或路径展示文档。
- 搜索文档正文。
- 获取最近文档。
- 提取“下次预防规则”。

核心类型和函数：

- `LedgerDocument`
- `ShownDocument`
- `SearchHit`
- `RuleItem`
- `list_documents`
- `show_document`
- `search_documents`
- `recent_documents`
- `recent_rules`

协作注意：

- `list/show/search/recent/rules` 的业务逻辑在这里。
- 未来如果要支持索引、SQLite、向量搜索，应优先从这里扩展。
- 当前搜索是纯文本搜索。

## LLM 使用契约

### `src/llm.rs`

面向 LLM 的调用说明模块。

主要负责：

- 输出 `detour capture --stdin --json` 的输入 JSON schema。
- 输出给 Claude Code / Codex 使用 detour 的 prompt。

核心函数：

- `capture_schema`
- `usage_prompt`

协作注意：

- 如果模型输入格式变化，要同步改 `capture_schema`。
- 如果使用流程变化，要同步改 `usage_prompt`。
- 后续 skill、slash command、MCP 都会复用这里的说明。

## Skills 生成

### `src/skills.rs`

面向 Codex 和 Claude Code 生成 skill 文件的模块。

主要负责：

- 根据目标环境生成 `SKILL.md` 内容。
- 输出默认 skill 路径。
- 写入项目内 skill 文件。
- 防止未传 `--force` 时覆盖已有文件。

当前默认路径：

- Codex：`.detour/skills/detour-capture/SKILL.md`
- Claude Code：`.claude/skills/detour-capture/SKILL.md`

核心函数：

- `default_skill_path`
- `skill_content`
- `write_skill`

协作注意：

- Claude Code 目标使用 `.claude/skills/<name>/SKILL.md`。
- Codex 目标先写到项目内 `.detour/skills/`，避免误改全局环境。
- 如果 detour 的调用流程变化，应同步更新 `llm.rs` 和这里生成的 skill。

## Claude Code 项目级集成

### `src/claude.rs`

Claude Code 项目级集成模块。

主要负责：

- 检测当前项目是否已经安装 detour 的 Claude Code 集成文件。
- 生成 `.claude/commands/detour-capture.md`。
- 生成 `.claude/commands/detour-rules.md`。
- 安装或移除项目级 slash command 文件。
- 打印后续阶段接入 PreCompact hook 时可参考的 settings JSON 片段。

当前默认写入：

- `.claude/commands/detour-capture.md`
- `.claude/commands/detour-rules.md`

协作注意：

- 当前阶段只写项目目录内的 `.claude/commands/`，不自动修改用户级 `~/.claude/settings.json`。
- `PreCompact` 自动 hook 仍是后续阶段能力，当前只提供 `print-config` 和 `run-precompact` 占位。
- 如果 Claude Code slash command 内容变化，应同步检查 `skills.rs` 和 `llm.rs` 的调用说明。

## 示例数据

### `examples/session-basic.jsonl`

阶段三以后用于本地验收的示例会话。

主要负责：

- 提供一段最小 JSONL 会话。
- 触发 `rust-cli` 和 `claude-code` 两类错题。

协作注意：

- 新增测试场景时，可以继续放在 `examples/`。
- 不要把真实用户隐私、密钥或业务敏感内容放进示例文件。

## 生成内容目录

### `.detour/mistakes/`

本地生成的错题集目录。

主要负责：

- 存放 `detour capture` 生成的 Markdown 或 JSON 错题集。

协作注意：

- `.detour/` 已加入 `.gitignore`。
- 这里是本地生成内容，不默认提交。
- 如果某篇错题集需要共享，应人工复制到团队约定文档目录。

## 当前命令能力

### 生成

```bash
cargo run -- capture --from examples/session-basic.jsonl --max-docs 2
```

### 列出

```bash
cargo run -- list
cargo run -- list --json
```

### 查看

```bash
cargo run -- show doc-rust-cli
cargo run -- show .detour/mistakes/doc-rust-cli.md
```

### 搜索

```bash
cargo run -- search cargo
cargo run -- search cargo --json
```

### 最近文档

```bash
cargo run -- recent --limit 5
cargo run -- recent --limit 5 --json
```

### 预防规则

```bash
cargo run -- rules --limit 20
cargo run -- rules --limit 20 --json
```

### LLM 契约

```bash
cargo run -- schema
cargo run -- prompt --target codex
cargo run -- prompt --target claude
```

## 协作约定

- 代码注释使用中文。
- 保留必要技术术语，例如 CLI、JSON、MCP、Claude Code、PreCompact。
- `main.rs` 只做命令分发和终端输出。
- `cli.rs` 只做命令参数定义。
- 模型放在 `src/models/`。
- 生成逻辑放在 `capture.rs` 和 `extractor.rs`。
- 渲染逻辑放在 `render.rs`。
- 写文件逻辑放在 `storage.rs`。
- 读本地错题集逻辑放在 `ledger.rs`。
- 面向 LLM 的说明放在 `llm.rs`。
- skill 文件生成逻辑放在 `skills.rs`。
- Claude Code 项目级集成逻辑放在 `claude.rs`。
- 新增功能后运行：

```bash
cargo fmt
cargo test
```
