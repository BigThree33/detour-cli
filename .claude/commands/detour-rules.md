# detour-rules

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
