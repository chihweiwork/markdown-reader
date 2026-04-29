# vhs script for the README demo GIF.
#
# Run: `vhs docs/assets/demo.tape`  → produces docs/assets/demo.gif
#
# Sequence (~35 seconds):
#   1. Open project root (file tree visible)
#   2. Open demo.md via search modal (filename mode)
#   3. PageDown 3x through demo.md — flowchart, sequence, state diagram
#   4. Settings: cycle through 2 themes
#   5. Outline picker — demo navigation
#   6. Hybrid live-preview editing
#   7. Quit

Output docs/assets/demo.gif

# Force bash so we don't depend on the user's fish/zsh startup time.
# When ttyd spawned fish, its plugin-laden startup ate the first ~30
# characters of the launch command, leaving just `> .` on screen.
Set Shell "bash"

Set Width 1500
Set Height 850
Set FontSize 16
Set Theme "Dracula"
Set Padding 20
Set TypingSpeed 50ms

# IMPORTANT: pre-build the release binary BEFORE running this script:
#
#   cargo build --release
#   vhs docs/assets/demo.tape
#
# Earlier versions tried to build inside a Hide block, but the release
# build can take >60s on a cold cache, and any commands typed while the
# build is still running get queued in stdin and silently lost. Building
# beforehand is the simplest reliable approach.

# Wait for the shell prompt to fully render before typing the launch
# command. Without this, the first characters of the command race the
# prompt and get dropped.
Hide
Sleep 2s
Show

# Step 1 — launch against project ROOT (file tree visible)
Type "./target/release/markdown-reader ."
Enter
Sleep 2s

# Step 2 — open demo.md via the search modal (reliable, deterministic).
# Tree navigation by j/l counts is too fragile — root directory order
# depends on what's checked in (and whether tests/, target/, examples/
# are filtered by .gitignore). Search by filename is the robust path.
Type "/"
Sleep 1000ms
Type "demo"
Sleep 1500ms
Enter
Sleep 1800ms

# Step 3 — PageDown three times. Each press shifts the viewport one full
# screen so the three mermaid diagrams (flowchart, sequence, state)
# come into view in sequence. We stop before the comparison table —
# tables aren't part of this demo.
PageDown
Sleep 2500ms
PageDown
Sleep 2500ms
PageDown
Sleep 2500ms

# Step 4 — settings: cycle through 2 themes
Type "c"
Sleep 1200ms
Type "j"
Sleep 300ms
Enter
Sleep 1500ms
Type "j"
Sleep 300ms
Enter
Sleep 1500ms
Escape
Sleep 800ms

# Step 5 — back to top, demo the outline picker
Type "g"
Sleep 200ms
Type "g"
Sleep 600ms
Type "o"
Sleep 1500ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 500ms
Enter
Sleep 1500ms

# Step 6 — hybrid live-preview editing
Type "g"
Sleep 200ms
Type "g"
Sleep 600ms
Type "i"
Sleep 1500ms
Down
Sleep 300ms
Down
Sleep 300ms
Right@40ms 60
Sleep 400ms
Type " (live!)"
Sleep 1500ms
Down
Sleep 300ms
Down
Sleep 1500ms
Type ":q!"
Enter
Sleep 1500ms

# Step 7 — quit
Type "q"
Sleep 1s
