#!/usr/bin/env bash
# PostToolUse hook: formats the workspace after an Edit/Write on a Rust file.
file=$(jq -r '.tool_input.file_path // empty')
case "$file" in
  *.rs) cd "${CLAUDE_PROJECT_DIR:-.}" && cargo fmt --all >/dev/null 2>&1 || true ;;
esac
exit 0
