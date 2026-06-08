# detour-capture

请你作为 Claude Code 的 LLM，复盘当前会话中已经踩过的坑，并亲自生成错题集 JSON，然后调用 detour 保存成 Markdown 文档。

你必须：

1. 找出本会话中已经出现的错误假设、命令失败、环境约束、权限问题、路径问题、API 误解或用户偏好遗漏。
2. 不要编造不存在的错误或证据。
3. 由你来分析错误现象、错误路径、根因、修正方式和下次预防规则。
4. 按下面 JSON schema 生成最多 5 篇文档。
5. 把 JSON 通过 stdin 传给 detour 保存。

JSON 顶层必须是：

```json
{
  "documents": []
}
```

每篇 document 必须包含：

```json
{
  "title": "Claude Code Hook 错题集",
  "summary": "本文件记录 Claude Code hook 集成时踩过的坑。",
  "tags": ["claude-code", "precompact"],
  "mistakes": []
}
```

每条 mistake 必须包含：

```json
{
  "title": "不要把 hook 命令误认为 LLM 推理",
  "symptom": "误以为 detour extractor 会把规则发给 Claude Code。",
  "wrong_turn": "混淆了 Claude Code shell hook 和 Claude LLM 生成流程。",
  "root_cause": "没有拆开 Claude Code 宿主、hook 外部命令和 LLM 推理的职责。",
  "correction": "由 Claude 先生成结构化错题 JSON，再交给 detour save 保存。",
  "prevention_rule": "需要模型生成内容时，先让模型生成 JSON，再让 CLI 负责校验、渲染和存储。",
  "tags": ["llm-interface", "claude-code"],
  "severity": "high",
  "evidence": [
    {
      "kind": "message",
      "value": "用户明确要求让大模型生成错题集，而不是 Rust 规则生成。"
    }
  ]
}
```

severity 只能是：

```text
low
medium
high
```

evidence.kind 只能是：

```text
command
file
output
message
other
```

优先使用：

```bash
detour save --stdin --json
```

如果 `detour` 不在 PATH 中，且当前目录是 detour 源码项目，则改用：

```bash
cargo run -- save --stdin --json
```

stdin 输入必须是你生成的错题集 JSON，不是原始 transcript。

保存成功后，请告诉用户：

- 生成了几篇错题集。
- 每篇错题集的路径。
- 每篇错题集的 tags。
- 最重要的下次预防规则。
