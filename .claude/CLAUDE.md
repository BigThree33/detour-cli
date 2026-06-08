<!-- detour-cli:reminder:start -->
## Detour Context-Compression Reminder

When this project is being worked on with Claude Code, preserve mistake notes before context compaction.

Run `/detour-capture` before `/compact`, before summarizing or switching tasks, and whenever the session has accumulated failed commands, wrong assumptions, environment constraints, permission issues, path issues, API misunderstandings, or user preference corrections.

`/detour-capture` is the LLM-generated path: Claude must analyze the current conversation, generate structured mistake-notebook JSON, and call `detour save --stdin --json` so detour can render Markdown notes into `.detour/mistakes/`.

Use `detour rules --limit 20 --json` or `/detour-rules` before similar future work, and honor tags/frontmatter in the saved Markdown notes.
<!-- detour-cli:reminder:end -->
