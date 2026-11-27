# Provider marks

The marks the assistant shows for each provider it can talk to. Each `*.svg` is the provider's
single-colour mark, unaltered; the `*.a8` beside it is that SVG rendered once at 96 × 96 pixels
into one byte of coverage per pixel (`resvg -w 96 -h 96`, alpha channel only), which
`src/ui/marks.rs` tints with the theme's colour.

| Mark | Source | Licence of the file |
|---|---|---|
| `claude.svg`, `gemini.svg`, `mistral.svg`, `openrouter.svg`, `ollama.svg` | [Simple Icons](https://github.com/simple-icons/simple-icons) (`claude`, `googlegemini`, `mistralai`, `openrouter`, `ollama`), copied from `master` on 2026-11-27 | CC0-1.0 |
| `openai.svg` | [Lobe Icons](https://github.com/lobehub/lobe-icons) `packages/static-svg/icons/openai.svg`, copied from `master` on 2026-11-27 | MIT, © 2023 LobeHub |

The licence of a file covers the file, not the mark. **Claude and Anthropic, OpenAI, Google and
Gemini, Mistral AI, OpenRouter and Ollama are trademarks of their owners.** They appear here only
to say which service the user has chosen to connect, with the user's own account and key. They do
not say that the owner made, approved or endorses Nodalia or its apps.

Rules for using them:

- Do not redraw, recolour beyond one flat theme colour, animate, combine or crop a mark.
- Never use one as an app icon, as part of a name or as decoration.
- If an owner asks for a mark to be changed or removed, do it. An app can replace any of them at
  run time with `ui::set_mark`.
