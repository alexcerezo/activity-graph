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
<picture>
  <source media="(max-width: 767px)" srcset="activity-graph-mobile.svg">
  <img src="activity-graph.svg" alt="Activity Graph" width="100%">
</picture>
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
<picture>
  <source media="(max-width: 767px)" srcset="activity-graph-mobile.svg">
  <img src="activity-graph.svg" alt="Activity Graph" width="100%">
</picture>
            /<!-- BEGIN ACTIVITY-GRAPH -->/!{
<picture>
  <source media="(max-width: 767px)" srcset="activity-graph-mobile.svg">
  <img src="activity-graph.svg" alt="Activity Graph" width="100%">
</picture>
              /<!-- END ACTIVITY-GRAPH -->/!d
            }
            /<!-- BEGIN ACTIVITY-GRAPH -->/a\\
<picture>
  <source media="(max-width: 767px)" srcset="activity-graph-mobile.svg">
  <img src="activity-graph.svg" alt="Activity Graph" width="100%">
</picture>
