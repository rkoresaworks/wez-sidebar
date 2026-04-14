#!/usr/bin/env bash
# wez-sidebar skills を Claude Code の user skill としてインストール
# ~/.claude/skills/<skill_name> → <repo>/skills/<skill_name> の symlink を張る
#
# Usage:
#   ./scripts/install-skills.sh         # インストール
#   ./scripts/install-skills.sh --dry   # 何をするか表示するだけ
#   ./scripts/install-skills.sh --uninstall  # symlink を外す

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SKILLS_SRC="$REPO_ROOT/skills"
SKILLS_DST="$HOME/.claude/skills"

mode="install"
case "${1:-}" in
  --dry)       mode="dry" ;;
  --uninstall) mode="uninstall" ;;
  "")          mode="install" ;;
  *)
    echo "Usage: $0 [--dry|--uninstall]" >&2
    exit 1
    ;;
esac

if [ ! -d "$SKILLS_SRC" ]; then
  echo "skills source not found: $SKILLS_SRC" >&2
  exit 1
fi

mkdir -p "$SKILLS_DST"

for skill_dir in "$SKILLS_SRC"/*/; do
  [ -d "$skill_dir" ] || continue
  skill_name="$(basename "$skill_dir")"
  dst="$SKILLS_DST/$skill_name"

  case "$mode" in
    dry)
      if [ -L "$dst" ]; then
        current="$(readlink "$dst")"
        if [ "$current" = "$skill_dir" ]; then
          echo "OK     $skill_name (already linked)"
        else
          echo "REPLACE $skill_name ($current -> $skill_dir)"
        fi
      elif [ -e "$dst" ]; then
        echo "BLOCK  $skill_name (non-symlink exists at $dst)"
      else
        echo "LINK   $skill_name -> $skill_dir"
      fi
      ;;
    install)
      if [ -e "$dst" ] && [ ! -L "$dst" ]; then
        echo "skip (non-symlink exists): $dst" >&2
        continue
      fi
      ln -sfn "$skill_dir" "$dst"
      echo "installed: $skill_name"
      ;;
    uninstall)
      if [ -L "$dst" ]; then
        rm "$dst"
        echo "removed: $skill_name"
      fi
      ;;
  esac
done

if [ "$mode" = "install" ]; then
  echo
  echo "done. Claude Code を再起動すると skill が認識される。"
fi
