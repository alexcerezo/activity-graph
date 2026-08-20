use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::fs;

// --- GitHub API types ---

#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<GraphData>,
    errors: Option<Vec<GraphError>>,
}

#[derive(Debug, Deserialize)]
struct GraphData {
    user: Option<User>,
}

#[derive(Debug, Deserialize)]
struct User {
    #[serde(rename = "contributionsCollection")]
    contributions_collection: ContributionsCollection,
}

#[derive(Debug, Deserialize)]
struct ContributionsCollection {
    #[serde(rename = "contributionCalendar")]
    contribution_calendar: ContributionCalendar,
}

#[derive(Debug, Deserialize)]
struct ContributionCalendar {
    weeks: Vec<Week>,
}

#[derive(Debug, Deserialize)]
struct Week {
    #[serde(rename = "contributionDays")]
    contribution_days: Vec<ContributionDay>,
}

#[derive(Debug, Deserialize)]
struct ContributionDay {
    #[serde(rename = "contributionCount")]
    contribution_count: u32,
    date: String,
}

#[derive(Debug, Deserialize)]
struct GraphError {
    message: String,
}

// --- SVG Configuration ---

struct SvgConfig {
    num_weeks: usize,
    cell_size: i32,
    cell_gap: i32,
    label_width: i32,
    header_height: i32,
    right_padding: i32,
    bottom_padding: i32,
}

impl SvgConfig {
    fn desktop() -> Self {
        Self {
            num_weeks: 53,
            cell_size: 13,
            cell_gap: 2,
            label_width: 30,
            header_height: 20,
            right_padding: 20,
            bottom_padding: 10,
        }
    }

    fn mobile() -> Self {
        Self {
            num_weeks: 18,
            cell_size: 15,
            cell_gap: 3,
            label_width: 35,
            header_height: 22,
            right_padding: 15,
            bottom_padding: 10,
        }
    }

    fn cell_total(&self) -> i32 {
        self.cell_size + self.cell_gap
    }

    fn svg_width(&self) -> i32 {
        self.label_width + (self.num_weeks as i32) * self.cell_total() + self.right_padding
    }

    fn svg_height(&self) -> i32 {
        self.header_height + 7 * self.cell_total() + self.bottom_padding
    }
}

// --- Main ---

fn main() {
    let args: Vec<String> = env::args().collect();
    let test_mode = args.iter().any(|a| a == "--test");

    let days = if test_mode {
        println!("Running in test mode with mock data...");
        generate_mock_days()
    } else {
        let username = args
            .get(1)
            .expect("Usage: activity-graph [--test] <github-username>");
        let token =
            env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN environment variable is required");
        fetch_contributions(username, &token)
    };

    // Generate desktop version (53 weeks, 13px cells)
    let desktop_svg = generate_svg(&days, &SvgConfig::desktop());
    fs::write("activity-graph.svg", &desktop_svg).expect("Failed to write activity-graph.svg");

    // Generate mobile version (18 weeks, 15px cells)
    let mobile_svg = generate_svg(&days, &SvgConfig::mobile());
    fs::write("activity-graph-mobile.svg", &mobile_svg)
        .expect("Failed to write activity-graph-mobile.svg");

    println!("Generated activity-graph.svg (desktop) and activity-graph-mobile.svg (mobile)");
}

fn fetch_contributions(username: &str, token: &str) -> Vec<ContributionDay> {
    let client = Client::new();

    // Calculate date range: last 365 days (GitHub API requires DateTime, not Date)
    let today = chrono::Utc::now().date_naive();
    let from = format!("{}T00:00:00Z", (today - chrono::Duration::days(364)).format("%Y-%m-%d"));
    let to = format!("{}T00:00:00Z", today.format("%Y-%m-%d"));

    let query = json!({
        "query": format!(
            r#"{{
              user(login: "{username}") {{
                contributionsCollection(from: "{from}", to: "{to}") {{
                  contributionCalendar {{
                    weeks {{
                      contributionDays {{
                        contributionCount
                        date
                      }}
                    }}
                  }}
                }}
              }}
            }}"#
        )
    });

    eprintln!("Querying GitHub API for user: {username}");

    let resp = client
        .post("https://api.github.com/graphql")
        .bearer_auth(token)
        .header("User-Agent", "activity-graph-rust")
        .json(&query)
        .send()
        .expect("Failed to send request to GitHub API");

    let status = resp.status();
    if !status.is_success() {
        eprintln!("GitHub API returned status: {status}");
        let body = resp.text().unwrap_or_default();
        eprintln!("Response body: {body}");
        std::process::exit(1);
    }

    let resp: GraphQLResponse = resp
        .json()
        .expect("Failed to parse GitHub API response");

    if let Some(errors) = resp.errors {
        eprintln!("GraphQL errors: {:?}", errors);
        std::process::exit(1);
    }

    let days = resp
        .data
        .and_then(|d| d.user)
        .map(|u| u.contributions_collection.contribution_calendar.weeks)
        .expect("No user data found")
        .into_iter()
        .flat_map(|w| w.contribution_days)
        .collect::<Vec<_>>();

    println!("Fetched {} days of contribution data", days.len());
    days
}

fn generate_mock_days() -> Vec<ContributionDay> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut days = Vec::new();
    let today = chrono::Utc::now().date_naive();
    let start = today - chrono::Duration::days(364);
    let mut current = start;
    while current <= today {
        let count: u32 = rng.gen_range(0..=12);
        days.push(ContributionDay {
            contribution_count: count,
            date: current.format("%Y-%m-%d").to_string(),
        });
        current += chrono::Duration::days(1);
    }
    days
}

fn generate_svg(days: &[ContributionDay], cfg: &SvgConfig) -> String {
    let cell_total = cfg.cell_total();
    let svg_width = cfg.svg_width();
    let svg_height = cfg.svg_height();

    let day_labels = [(1, "Mon"), (3, "Wed"), (5, "Fri")];

    // Month labels
    let mut month_positions: Vec<(String, usize)> = Vec::new();
    let mut last_month = "";
    for (week_idx, day_group) in days.chunks(7).enumerate() {
        if let Some(first_day) = day_group.first() {
            let month = &first_day.date[5..7];
            if month != last_month {
                last_month = month;
                month_positions.push((month_name(month), week_idx));
            }
        }
    }

    let mut month_labels_html = String::new();
    for (month, week_idx) in &month_positions {
        if *week_idx < cfg.num_weeks {
            let x = cfg.label_width + (*week_idx as i32) * cell_total;
            month_labels_html.push_str(&format!(
                r#"<text x="{x}" y="12" class="month-label">{month}</text>"#
            ));
        }
    }

    // Day labels
    let mut day_labels_html = String::new();
    for &(day_idx, label) in &day_labels {
        let y = cfg.header_height + (day_idx as i32 * cell_total) + cfg.cell_size - 2;
        day_labels_html.push_str(&format!(
            r#"<text x="0" y="{y}" class="day-label">{label}</text>"#
        ));
    }

    // Contribution cells
    let mut cells_html = String::new();
    let mut week_idx: usize = 0;
    let mut day_in_week: usize = 0;
    let mut delay_seed: u32 = 42;

    for day in days {
        if week_idx >= cfg.num_weeks {
            break;
        }

        let count = day.contribution_count;
        let level = get_level(count);
        let x = cfg.label_width + (week_idx as i32) * cell_total;
        let y = cfg.header_height + (day_in_week as i32) * cell_total;

        let cascade_delay = week_idx as f64 * 0.04;
        delay_seed = delay_seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let twinkle_delay = ((delay_seed >> 16) % 6000) as f64 / 1000.0;

        let twinkle_duration = match level {
            0 => 0.0,
            1 => 4.0 + (delay_seed % 2000) as f64 / 1000.0,
            2 => 3.0 + (delay_seed % 1500) as f64 / 1000.0,
            3 => 2.5 + (delay_seed % 1000) as f64 / 1000.0,
            _ => 2.0 + (delay_seed % 800) as f64 / 1000.0,
        };

        let glow_class = match level {
            0 => "",
            1 => "twinkle-soft",
            2 => "twinkle-mid",
            3 => "twinkle-bright",
            _ => "twinkle-intense",
        };

        let style_attr = if level > 0 {
            format!(
                r#" style="animation-delay: {:.2}s, {:.2}s; --twinkle-dur: {:.1}s""#,
                cascade_delay, twinkle_delay, twinkle_duration
            )
        } else {
            format!(r#" style="animation-delay: {:.2}s""#, cascade_delay)
        };

        let class_extra = if level > 0 {
            format!(" {glow_class}")
        } else {
            String::new()
        };

        cells_html.push_str(&format!(
            r#"<rect x="{x}" y="{y}" width="{sz}" height="{sz}" class="day level-{level}{class_extra}"{style_extra} rx="2" ry="2"><title>{count} contributions on {date}</title></rect>"#,
            sz = cfg.cell_size,
            count = count,
            date = day.date,
            class_extra = class_extra,
            style_extra = style_attr,
        ));

        day_in_week += 1;
        if day_in_week >= 7 {
            day_in_week = 0;
            week_idx += 1;
        }
    }

    let total: u32 = days.iter().map(|d| d.contribution_count).sum();

    // Background star dots
    let mut bg_stars_html = String::new();
    let mut star_seed: u32 = 1337;
    for i in 0..40 {
        star_seed = star_seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let sx = (star_seed % (svg_width as u32 - 40) + 40) as i32;
        star_seed = star_seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let sy = (star_seed % (svg_height as u32 - 20) + 10) as i32;
        let sr = 0.5 + (i % 3) as f64 * 0.3;
        let delay = (i as f64) * 0.3;
        let dur = 2.0 + (i % 5) as f64 * 0.8;
        bg_stars_html.push_str(&format!(
            r#"<circle cx="{sx}" cy="{sy}" r="{:.1}" class="bg-star" style="animation-delay:{:.1}s;animation-duration:{:.1}s"/>"#,
            sr, delay, dur
        ));
    }

    let total_text_y = svg_height - 3;
    let lw = cfg.label_width;
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{svg_width}" height="{svg_height}" viewBox="0 0 {svg_width} {svg_height}">
  <style>
    /* ===== TEXT STYLES ===== */
    .month-label {{
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
      font-size: 10px;
      fill: #656d76;
      opacity: 0;
      animation: fadeIn 0.6s ease forwards;
    }}
    .day-label {{
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
      font-size: 10px;
      fill: #656d76;
      opacity: 0;
      animation: fadeIn 0.6s ease forwards;
    }}

    /* ===== CELL BASE ===== */
    .day {{
      stroke: none;
      opacity: 0;
      animation: cellAppear 0.4s ease forwards;
    }}

    /* ===== THEME: LIGHT (default) ===== */
    /* Pure green with transparency — level 0 invisible */
    .level-0 {{ fill: transparent; stroke: none; }}
    .level-1 {{ fill: rgba(0, 123, 58, 0.15); }}
    .level-2 {{ fill: rgba(0, 123, 58, 0.35); }}
    .level-3 {{ fill: rgba(0, 123, 58, 0.60); }}
    .level-4 {{ fill: rgba(0, 123, 58, 0.90); }}

    /* ===== THEME: DARK ===== */
    @media (prefers-color-scheme: dark) {{
      .month-label {{ fill: #8b949e; }}
      .day-label {{ fill: #8b949e; }}
      .level-0 {{ fill: transparent; stroke: none; }}
      .level-1 {{ fill: rgba(57, 211, 83, 0.25); }}
      .level-2 {{ fill: rgba(57, 211, 83, 0.50); }}
      .level-3 {{ fill: rgba(57, 211, 83, 0.75); }}
      .level-4 {{ fill: rgba(57, 211, 83, 1.0); }}

      .bg-star {{ fill: rgba(57, 211, 83, 0.6); }}
    }}

    /* ===== KEYFRAMES ===== */
    @keyframes fadeIn {{
      from {{ opacity: 0; }}
      to   {{ opacity: 1; }}
    }}

    @keyframes cellAppear {{
      0%   {{ opacity: 0; transform: scale(0.3); }}
      60%  {{ opacity: 1; transform: scale(1.1); }}
      100% {{ opacity: 1; transform: scale(1); }}
    }}

    /* --- Twinkle: soft (level 1) --- */
    @keyframes twinkleSoft {{
      0%, 100% {{ filter: drop-shadow(0 0 0px transparent); background: transparent; }}
      50%      {{ filter: drop-shadow(0 0 2px rgba(0,123,58,0.25)); background: transparent; }}
    }}
    .twinkle-soft {{
      animation: cellAppear 0.4s ease forwards, twinkleSoft var(--twinkle-dur, 5s) ease-in-out infinite;
    }}

    /* --- Twinkle: mid (level 2) --- */
    @keyframes twinkleMid {{
      0%, 100% {{ filter: drop-shadow(0 0 0px transparent); background: transparent; }}
      40%      {{ filter: drop-shadow(0 0 3px rgba(0,123,58,0.35)); background: rgba(0,123,58,0.03); }}
      70%      {{ filter: drop-shadow(0 0 1px rgba(0,123,58,0.2)); background: transparent; }}
    }}
    .twinkle-mid {{
      animation: cellAppear 0.4s ease forwards, twinkleMid var(--twinkle-dur, 4s) ease-in-out infinite;
    }}

    /* --- Twinkle: bright (level 3) --- */
    @keyframes twinkleBright {{
      0%, 100% {{ filter: drop-shadow(0 0 0px transparent); background: transparent; }}
      30%      {{ filter: drop-shadow(0 0 5px rgba(0,123,58,0.4)); background: rgba(0,123,58,0.05); }}
      60%      {{ filter: drop-shadow(0 0 2px rgba(0,123,58,0.2)); background: transparent; }}
    }}
    .twinkle-bright {{
      animation: cellAppear 0.4s ease forwards, twinkleBright var(--twinkle-dur, 3s) ease-in-out infinite;
    }}

    /* --- Twinkle: intense (level 4) --- */
    @keyframes twinkleIntense {{
      0%   {{ filter: drop-shadow(0 0 0px transparent); background: transparent; }}
      25%  {{ filter: drop-shadow(0 0 7px rgba(0,123,58,0.5)); background: rgba(0,123,58,0.07); }}
      50%  {{ filter: drop-shadow(0 0 2px rgba(0,123,58,0.25)); background: transparent; }}
      75%  {{ filter: drop-shadow(0 0 5px rgba(0,123,58,0.4)); background: rgba(0,123,58,0.04); }}
      100% {{ filter: drop-shadow(0 0 0px transparent); background: transparent; }}
    }}
    .twinkle-intense {{
      animation: cellAppear 0.4s ease forwards, twinkleIntense var(--twinkle-dur, 2.5s) ease-in-out infinite;
    }}

    /* --- Dark mode glow overrides --- */
    @media (prefers-color-scheme: dark) {{
      @keyframes twinkleSoft {{
        0%, 100% {{ filter: drop-shadow(0 0 0px transparent); }}
        50%      {{ filter: drop-shadow(0 0 3px rgba(57,211,83,0.4)); }}
      }}
      @keyframes twinkleMid {{
        0%, 100% {{ filter: drop-shadow(0 0 0px transparent); }}
        40%      {{ filter: drop-shadow(0 0 4px rgba(57,211,83,0.5)); }}
        70%      {{ filter: drop-shadow(0 0 2px rgba(57,211,83,0.3)); }}
      }}
      @keyframes twinkleBright {{
        0%, 100% {{ filter: drop-shadow(0 0 0px transparent); }}
        30%      {{ filter: drop-shadow(0 0 6px rgba(57,211,83,0.6)); }}
        60%      {{ filter: drop-shadow(0 0 3px rgba(57,211,83,0.3)); }}
      }}
      @keyframes twinkleIntense {{
        0%   {{ filter: drop-shadow(0 0 0px transparent); }}
        25%  {{ filter: drop-shadow(0 0 8px rgba(57,211,83,0.7)); }}
        50%  {{ filter: drop-shadow(0 0 3px rgba(57,211,83,0.35)); }}
        75%  {{ filter: drop-shadow(0 0 6px rgba(57,211,83,0.55)); }}
        100% {{ filter: drop-shadow(0 0 0px transparent); }}
      }}
    }}

    /* --- Background stars --- */
    @keyframes bgTwinkle {{
      0%, 100% {{ opacity: 0.1; }}
      50%      {{ opacity: 0.35; }}
    }}
    .bg-star {{
      fill: rgba(0, 123, 58, 0.3);
      opacity: 0.1;
      animation: bgTwinkle 3s ease-in-out infinite;
    }}
  </style>
  <defs>
    <radialGradient id="bgGlow" cx="50%" cy="50%" r="60%">
      <stop offset="0%" style="stop-color:currentColor;stop-opacity:0.04"/>
      <stop offset="100%" style="stop-color:currentColor;stop-opacity:0"/>
    </radialGradient>
  </defs>
  <rect width="{svg_width}" height="{svg_height}" fill="url(#bgGlow)"/>
  {bg_stars_html}
  {month_labels_html}
  {day_labels_html}
  {cells_html}
  <text x="{lw}" y="{total_text_y}" class="day-label">{total} contributions in the last year</text>
</svg>"#
    )
}

fn get_level(count: u32) -> u32 {
    match count {
        0 => 0,
        1..=3 => 1,
        4..=6 => 2,
        7..=9 => 3,
        _ => 4,
    }
}

fn month_name(m: &str) -> String {
    match m {
        "01" => "Jan".into(),
        "02" => "Feb".into(),
        "03" => "Mar".into(),
        "04" => "Apr".into(),
        "05" => "May".into(),
        "06" => "Jun".into(),
        "07" => "Jul".into(),
        "08" => "Aug".into(),
        "09" => "Sep".into(),
        "10" => "Oct".into(),
        "11" => "Nov".into(),
        "12" => "Dec".into(),
        _ => m.into(),
    }
}
