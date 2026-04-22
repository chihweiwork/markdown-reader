---
name: Bug report
about: Report something that isn't working correctly
title: "[Bug] "
labels: bug
assignees: ""
---

## Description

A clear and concise description of what the bug is.

## Steps to Reproduce

1. Run `markdown-reader` with ...
2. Open file ...
3. Observe ...

If the bug is in `mermaid-text` rendering specifically, paste the Mermaid
source you're feeding it (and ideally the output of `mermaid-text < your.mmd`).

## Expected Behavior

What you expected to happen.

## Actual Behavior

What actually happened. Include any error messages, stack traces, or
screenshots. For TUI display issues, screenshots help a lot.

## Environment

| Field | Value |
|---|---|
| OS | e.g. macOS 15.2 / Ubuntu 24.04 |
| Terminal | e.g. iTerm2, Ghostty, kitty, tmux, Alacritty |
| Image protocol | e.g. iTerm2, Kitty, Sixel, none |
| Rust version | `rustc --version` |
| markdown-reader version | `markdown-reader --version` |
| mermaid-text version | check `Cargo.toml` or `cargo install` output |
| Installation method | `cargo install` / binary release / built from source |

## Additional Context

Any other context about the problem. For Mermaid-rendering issues, mention
which diagram type (flowchart, state, sequence, pie, erDiagram).
