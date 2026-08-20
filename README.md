# Activity Graph

A GitHub contribution graph SVG generated daily by a GitHub Action.

## How it works

1. A **Rust script** fetches your GitHub contribution data via the GraphQL API
2. It generates **two SVG files** — desktop (53 weeks) and mobile (18 weeks)
3. SVGs use `prefers-color-scheme` media queries to adapt to light/dark themes
4. A **GitHub Action** runs daily (cron job) to regenerate the graphs
5. CSS `<picture>` element in this README shows the right version per screen size

## Activity Graph

<picture>
  <source media="(max-width: 767px)" srcset="activity-graph-mobile.svg">
  <img src="activity-graph.svg" alt="Activity Graph" width="100%">
</picture>

> **Desktop:** Full 53-week view with 13px cells
> **Mobile:** Compact 18-week view with 15px cells — auto-switches at 768px breakpoint

## Features

- **Theme-aware:** Automatically adapts colors via `prefers-color-scheme`
  - Light mode: classic GitHub green palette
  - Dark mode: brighter greens with adjusted opacity for dark backgrounds
- **Animated:** CSS-only "star rain" effect with:
  - Fade-in cascade on load
  - Twinkle/glow animations per contribution level
  - Background decorative star dots
- **Responsive:** Two SVG variants for optimal viewing on any device

## GitHub Action

The workflow runs daily at 00:00 UTC and can also be triggered manually.

See [`.github/workflows/generate-graph.yml`](.github/workflows/generate-graph.yml)

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

MIT
