# GitHub Activity Graph Generator

Automatically generate a beautiful animated GitHub contribution graph SVG for your profile README. Features light/dark theme support, CSS-only animations, and responsive mobile layout.

<!-- BEGIN ACTIVITY-GRAPH -->
<picture>
  <source media="(max-width: 767px)" srcset="activity-graph-mobile.svg">
  <img src="activity-graph.svg" alt="Activity Graph" width="100%">
</picture>
<!-- END ACTIVITY-GRAPH -->

## Features

- 🎨 **Theme-aware** — Automatically adapts to light/dark mode via `prefers-color-scheme`
- ✨ **Animated** — CSS-only star rain effect with fade-in cascade and glow per contribution level
- 📱 **Responsive** — Two SVG variants: desktop (53 weeks) and mobile (18 weeks)
- 🔄 **Auto-updates** — Runs daily via GitHub Actions cron job
- 🚀 **Easy setup** — Just add to your workflow!

## Quick Setup

### 1. Add Comment Tags to Your README

In your profile README (or any README where you want to display the graph), add these markers:

```markdown
<!-- BEGIN ACTIVITY-GRAPH -->
<!-- END ACTIVITY-GRAPH -->
```

### 2. Create Workflow File

Create `.github/workflows/activity-graph.yml` in your repository:

```yaml
name: Activity Graph

on:
  schedule:
    - cron: "0 0 * * *"  # Runs daily at midnight
  workflow_dispatch:

permissions:
  contents: write

jobs:
  activity-graph:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: alexcerezo/activity-graph@main

      - name: Inject graph into README
        run: |
          GRAPH_DESKTOP=$(cat activity-graph.svg)
          GRAPH_MOBILE=$(cat activity-graph-mobile.svg)
          
          # Build the picture element
          PICTURE="<picture>
  <source media=\"(max-width: 767px)\" srcset=\"activity-graph-mobile.svg\">
  <img src=\"activity-graph.svg\" alt=\"Activity Graph\" width=\"100%\">
</picture>"
          
          # Replace between comment tags
          sed -i "/<!-- BEGIN ACTIVITY-GRAPH -->/,/<!-- END ACTIVITY-GRAPH -->/{
            /<!-- BEGIN ACTIVITY-GRAPH -->/!{
              /<!-- END ACTIVITY-GRAPH -->/!d
            }
            /<!-- BEGIN ACTIVITY-GRAPH -->/a\\
$PICTURE
          }" README.md

      - name: Commit changes
        run: |
          git config --local user.name "github-actions[bot]"
          git config --local user.email "github-actions[bot]@users.noreply.github.com"
          git add activity-graph.svg activity-graph-mobile.svg README.md
          if ! git diff --staged --quiet; then
            git commit -m "chore: update activity graph [skip ci]"
            git push
          fi
```

### 3. Run the Workflow

Go to **Actions** tab → **Activity Graph** → **Run workflow**

That's it! Your README will automatically update with your activity graph. 🎉

## Configuration Options

### Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `username` | GitHub username | ❌ No | `${{ github.repository_owner }}` |
| `token` | GitHub token | ❌ No | `${{ github.token }}` |

## How It Works

1. The GitHub Action runs on a schedule (daily by default)
2. Fetches your contribution data via GitHub GraphQL API
3. Generates two SVG files: desktop (53 weeks) and mobile (18 weeks)
4. Commits and pushes the updated SVGs to your repository

## Local Development

```bash
# Build the project
cargo build --release

# Run with test data (no API token needed)
cargo run --release -- --test

# Run with real GitHub data
GITHUB_TOKEN=your_token cargo run --release -- your_username
```

## License

MIT License - feel free to use and modify.

## Credits

Created by [Alejandro Cerezo](https://github.com/alexcerezo)
