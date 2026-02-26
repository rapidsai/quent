#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: $0 [options]"
  echo ""
  echo "Creates a diff of the current branch vs a source remote's main, then applies"
  echo "it onto a target remote's main as a new branch. Works even when the two remotes"
  echo "have unrelated histories (e.g. after a history rewrite)."
  echo ""
  echo "Options:"
  echo "  --source-remote   Remote the branch was developed against (default: gitlab)"
  echo "  --target-remote   Remote to push the new branch to (default: origin)"
  echo "  --base-branch     Base branch name on both remotes (default: main)"
  echo "  --branch-name     Name for the new branch (default: current branch + -migrate)"
  echo "  -h, --help        Show this help"
  exit 1
}

SOURCE_REMOTE="gitlab"
TARGET_REMOTE="origin"
BASE_BRANCH="main"
BRANCH_NAME=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --source-remote) SOURCE_REMOTE="$2"; shift 2 ;;
    --target-remote) TARGET_REMOTE="$2"; shift 2 ;;
    --base-branch)   BASE_BRANCH="$2"; shift 2 ;;
    --branch-name)   BRANCH_NAME="$2"; shift 2 ;;
    -h|--help)       usage ;;
    *) echo "Unknown option: $1"; usage ;;
  esac
done

CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
SOURCE_SHA=$(git rev-parse HEAD)
BRANCH_NAME="${BRANCH_NAME:-${CURRENT_BRANCH}-migrate}"

echo "Current branch:  $CURRENT_BRANCH ($SOURCE_SHA)"
echo "Source remote:    $SOURCE_REMOTE (diff against $SOURCE_REMOTE/$BASE_BRANCH)"
echo "Target remote:   $TARGET_REMOTE (new branch based on $TARGET_REMOTE/$BASE_BRANCH)"
echo "New branch:      $BRANCH_NAME"
echo ""

# Fetch both remotes
echo "==> Fetching $SOURCE_REMOTE/$BASE_BRANCH..."
git fetch "$SOURCE_REMOTE" "$BASE_BRANCH"
echo "==> Fetching $TARGET_REMOTE/$BASE_BRANCH..."
git fetch "$TARGET_REMOTE" "$BASE_BRANCH"

# Find merge base on the SOURCE remote (where the branch was developed)
MERGE_BASE=$(git merge-base "$SOURCE_REMOTE/$BASE_BRANCH" "$SOURCE_SHA")
echo "==> Merge base ($SOURCE_REMOTE/$BASE_BRANCH & $CURRENT_BRANCH): $(git log --oneline -1 "$MERGE_BASE")"

# Generate the diff from the source remote's merge base to the current branch
PATCH_FILE=$(mktemp "${TMPDIR:-/tmp}/patch-XXXXXXXX")
trap 'rm -f "$PATCH_FILE"' EXIT
git diff "$MERGE_BASE" "$SOURCE_SHA" > "$PATCH_FILE"
PATCH_SIZE=$(wc -c < "$PATCH_FILE" | tr -d ' ')
echo "==> Patch generated: ($PATCH_SIZE bytes)"

if [[ "$PATCH_SIZE" -eq 0 ]]; then
  echo "Error: empty patch — no differences found"
  exit 1
fi

# Create a new local branch from the target remote's main
echo "==> Creating local branch '$BRANCH_NAME' from $TARGET_REMOTE/$BASE_BRANCH..."
git checkout -B "$BRANCH_NAME" "$TARGET_REMOTE/$BASE_BRANCH"

# Apply the patch — use --reject so failed hunks become .rej files
echo "==> Applying patch..."
APPLY_FAILED=0
git apply --reject --whitespace=fix "$PATCH_FILE" || APPLY_FAILED=1

if [[ "$APPLY_FAILED" -eq 1 ]]; then
  REJ_COUNT=$(find . -name '*.rej' | wc -l | tr -d ' ')
  echo ""
  echo "==> Patch partially applied. $REJ_COUNT file(s) have .rej files with failed hunks."
  echo ""
  echo "  Rejected files:"
  find . -name '*.rej' -print | sed 's|^|    |'
  echo ""
  echo "  Resolve the .rej files manually, then run:"
  echo "    find . -name '*.rej' -delete"
  echo "    git add -A"
  echo "    git commit -m 'Apply changes from $CURRENT_BRANCH'"
  echo "    git push -u $TARGET_REMOTE $BRANCH_NAME"
  echo ""
  echo "  To abort:"
  echo "    git checkout $CURRENT_BRANCH"
  echo "    git branch -D $BRANCH_NAME"
  exit 1
fi

# Stage and commit
git add -A

if git diff --cached --quiet; then
  echo "Error: no changes after applying patch"
  git checkout "$CURRENT_BRANCH"
  git branch -D "$BRANCH_NAME"
  exit 1
fi

COMMIT_COUNT=$(git rev-list --count "$MERGE_BASE".."$SOURCE_SHA")
git commit -m "Apply changes from $CURRENT_BRANCH

Squashed $COMMIT_COUNT commit(s) onto $TARGET_REMOTE/$BASE_BRANCH."

echo ""
echo "==> New branch contents:"
git log --oneline -3
echo ""

# Push to target remote
echo "==> Pushing $BRANCH_NAME to $TARGET_REMOTE..."
git push -u "$TARGET_REMOTE" "$BRANCH_NAME"

echo ""
echo "Done! Branch '$BRANCH_NAME' pushed to $TARGET_REMOTE."
echo "Switch back with: git checkout $CURRENT_BRANCH"
