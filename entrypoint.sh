#!/bin/sh
set -e

USERNAME="${1:-${GITHUB_REPOSITORY_OWNER}}"

if [ -z "$USERNAME" ]; then
  echo "Error: username is required"
  exit 1
fi

echo "Generating activity graph for: $USERNAME"
activity-graph "$USERNAME"

echo "✅ Generated activity-graph.svg and activity-graph-mobile.svg"

# Commit and push if in GitHub Actions
if [ -n "$GITHUB_ACTIONS" ]; then
  git config --local user.name "github-actions[bot]"
  git config --local user.email "github-actions[bot]@users.noreply.github.com"
  git add activity-graph.svg activity-graph-mobile.svg
  if ! git diff --staged --quiet; then
    git commit -m "chore: update activity graphs [skip ci]"
    git push
    echo "✅ Changes committed and pushed"
  else
    echo "No changes to commit"
  fi
fi
