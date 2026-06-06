# detour-capture

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
