# detour-cli

detour 是一个给 Claude Code / 未来其他 AI 工具使用的错题集保存工具。

目标很简单：

```text
在 AI 上下文压缩前，让 LLM 把本轮会话踩过的坑生成错题集 JSON，
然后由 detour 保存成带 tags/frontmatter 的 Markdown 文件。
```

默认保存目录：

```text
.detour/mistakes/
```

## 安装到本机

在 detour 源码目录运行：

```bash
cargo install --path . --force
```

确认命令可用：

```bash
detour --version
detour --help
```

以后改了 detour 源码，需要重新执行 `cargo install --path . --force`，否则其他项目调用到的还是旧版本。

## 在 Claude Code 项目中接入

进入你真正要使用 detour 的项目：

```bash
cd <your-project>
```

推荐安装三层能力：

```bash
detour hook claude install --mode slash-command --force
detour hook claude install --mode reminder
detour hook claude install --mode pre-compact
```

它们分别负责：

```text
slash-command：提供 /detour-capture，让 Claude LLM 生成错题集 JSON，再调用 detour save 保存。
reminder：写入 .claude/CLAUDE.md，提醒 Claude 压缩前主动用 /detour-capture。
pre-compact：压缩前自动运行 detour 兜底，防止完全丢失。
```

检查状态：

```bash
detour hook claude status --json
```

你希望看到：

```json
{
  "precompact_installed": true,
  "reminder_installed": true,
  "capture_command_installed": true,
  "rules_command_installed": true
}
```

如果 Claude Code 已经打开，安装后建议重新进入项目或重启 Claude Code，让它重新加载 `.claude/` 配置。

## 日常使用

在 Claude Code 中运行：

```text
/detour-capture
```

这条路径是主路径：

```text
Claude LLM 分析当前会话
Claude LLM 生成错题集 JSON
detour save 校验、渲染、保存 Markdown
```

读取历史规则：

```text
/detour-rules
```

或者在终端里运行：

```bash
detour rules --limit 20 --json
detour recent --limit 5 --json
detour search <keyword> --json
```

## 自定义错题分类

项目可以定义自己的分类配置。默认情况下，这个配置是空白的注释模板；没有配置时，Claude/detour 使用内置兜底分类。

把本仓库的模板复制到目标项目：

```text
.detour/config.yoml
```

模板文件：

```text
config.yoml
```

内置兜底分类：

```text
claude-code
rust-cli
shell-env
llm-interface
filesystem
general
```

`config.yoml` 里提供的是注释模板。你可以取消注释并改成自己的项目分类，例如前端项目：

```yoml
# mistake_categories:
#   - id: style
#     title: "样式错题集"
#     tags: ["frontend", "style", "css"]
#     description: "CSS、布局、响应式、组件视觉、主题变量、设计还原相关问题。"
#
#   - id: build
#     title: "构建错误错题集"
#     tags: ["frontend", "build", "tooling"]
#     description: "Vite、Webpack、包管理器、依赖版本、构建脚本和 CI 构建失败。"
```

启用后应写成：

```yoml
mistake_categories:
  - id: style
    title: "样式错题集"
    tags: ["frontend", "style", "css"]
    description: "CSS、布局、响应式、组件视觉、主题变量、设计还原相关问题。"

  - id: build
    title: "构建错误错题集"
    tags: ["frontend", "build", "tooling"]
    description: "Vite、Webpack、包管理器、依赖版本、构建脚本和 CI 构建失败。"
```

Claude Code 的 `/detour-capture` 和 `.claude/CLAUDE.md` reminder 都会提醒 Claude：

```text
如果存在 .detour/config.yoml 且其中启用了 mistake_categories，
先读取它，并优先使用这些分类作为文档分类、title 和 tags。
如果没有配置，则使用内置兜底分类。
```

当前 detour 不强制解析 `config.yoml`；它是给 Claude / 团队看的分类规范。后续可以继续加 `detour save` 的强校验。

## LLM 生成结果保存

Claude 生成的 JSON 可以直接交给 detour：

```bash
type examples\llm-mistakes.json | detour save --stdin --json
```

源码开发时：

```bash
type examples\llm-mistakes.json | cargo run -- save --stdin --json
```

`detour save` 接收：

```json
{
  "documents": [
    {
      "title": "Claude Code Hook 错题集",
      "summary": "本文件记录 Claude Code hook 集成时踩过的坑。",
      "tags": ["claude-code", "precompact"],
      "mistakes": [
        {
          "title": "不要把 hook 命令误认为 LLM 推理",
          "symptom": "错误现象",
          "wrong_turn": "错误路径",
          "root_cause": "根因",
          "correction": "修正方式",
          "prevention_rule": "下次预防规则",
          "tags": ["llm-interface"],
          "severity": "high",
          "evidence": [
            {
              "kind": "message",
              "value": "证据来自当前会话。"
            }
          ]
        }
      ]
    }
  ]
}
```

生成的 Markdown 顶部会包含 frontmatter：

```markdown
---
detour_metadata_version: 1
generator: detour-cli
metadata_layers:
  - filesystem
  - markdown_frontmatter
id: "doc-example"
title: "示例错题集"
created_at: "unix:..."
tags:
  - "frontend"
---
```

## 各文件负责什么

想改 Claude 怎么生成错题集：

```text
src/claude.rs
fn claude_capture_command_content()
```

想改 Claude 什么时候被提醒运行 `/detour-capture`：

```text
src/claude.rs
fn reminder_block()
```

想改 LLM 输出 JSON 格式、字段、默认 tags：

```text
src/save.rs
LlmDocument
LlmMistake
normalize_document()
normalize_mistake()
```

想改 Markdown 长什么样：

```text
src/render.rs
render_markdown()
push_frontmatter()
```

想改文件名和防覆盖规则：

```text
src/storage.rs
unique_document_path()
slugify()
```

想改压缩前自动兜底规则：

```text
src/extractor.rs
```

注意：`extractor.rs` 是无 LLM 时的规则兜底，不是主生成路径。主生成路径是：

```text
Claude LLM -> detour save -> Markdown
```

## 常用命令

```bash
detour hook claude install --mode slash-command --force
detour hook claude install --mode reminder
detour hook claude install --mode pre-compact
detour hook claude status --json
detour save --stdin --json
detour recent --limit 5 --json
detour rules --limit 20 --json
detour search <keyword> --json
```
