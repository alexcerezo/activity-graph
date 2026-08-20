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

# Inject graph into README if markers exist
if [ -f "README.md" ] && grep -q "<!-- BEGIN ACTIVITY-GRAPH -->" README.md; then
  echo "Injecting graph into README..."
  # Use awk to replace content between markers
  awk '
    /<!-- BEGIN ACTIVITY-GRAPH -->/ {
      print
      print "<picture>"
      print "  <source media=\"(max-width: 767px)\" srcset=\"activity-graph-mobile.svg\">"
      print "  <img src=\"activity-graph.svg\" alt=\"Activity Graph\" width=\"100%\">"
      print "</picture>"
      skip = 1
      next
    }
    /<!-- END ACTIVITY-GRAPH -->/ {
      skip = 0
    }
    !skip { print }
  ' README.md > README.md.tmp && mv README.md.tmp README.md
  echo "✅ Graph injected into README"
fi

# Commit and push if in GitHub Actions
if [ -n "$GITHUB_ACTIONS" ]; then
  git config --global user.name "github-actions[bot]"
  git config --global user.email "github-actions[bot]@users.noreply.github.com"
  git add activity-graph.svg activity-graph-mobile.svg README.md 2>/dev/null || true
  if ! git diff --staged --quiet; then
    git commit -m "chore: update activity graph [skip ci]"
    git push
    echo "✅ Changes committed and pushed"
  else
    echo "No changes to commit"
  fi
fi
