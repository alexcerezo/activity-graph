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
    contributions_collection: ContributionsCollection,
}

#[derive(Debug, Deserialize)]
struct ContributionsCollection {
    contribution_calendar: ContributionCalendar,
}

#[derive(Debug, Deserialize)]
struct ContributionCalendar {
    weeks: Vec<Week>,
}

#[derive(Debug, Deserialize)]
struct Week {
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

// --- Main ---

fn main() {
    let args: Vec<String> = env::args().collect();
    let test_mode = args.iter().any(|a| a == "--test");

    let days = if test_mode {
        println!("🧪 Running in test mode with mock data...");
        generate_mock_days()
    } else {
        let username = args.get(1).expect("Usage: activity-graph [--test] <github-username>");
        let token = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN environment variable is required");
        fetch_contributions(username, &token)
    };

    let username = if test_mode {
        "test-user".to_string()
    } else {
        args.get(1).unwrap_or(&"unknown".to_string()).clone()
    };

    let svg = generate_svg(&username, &days);
    fs::write("activity-graph.svg", &svg).expect("Failed to write SVG file");
    println!("✅ activity-graph.svg generated successfully!");
}

fn fetch_contributions(username: &str, token: &str) -> Vec<ContributionDay> {
    let client = Client::new();
    let query = json!({
        "query": format!(
            r#"{{
              user(login: "{username}") {{
                contributionsCollection {{
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

    let resp: GraphQLResponse = client
        .post("https://api.github.com/graphql")
        .bearer_auth(token)
        .header("User-Agent", "activity-graph-rust")
        .json(&query)
        .send()
        .expect("Failed to send request to GitHub API")
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
    // Go back ~53 weeks from today
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

fn generate_svg(username: &str, days: &[ContributionDay]) -> String {
    // Build a map of date -> count
    let mut date_map = std::collections::HashMap::new();
    for day in days {
        date_map.insert(day.date.as_str(), day.contribution_count);
    }

    // We'll generate 53 weeks x 7 rows
    // Each cell is 13x13 with 2px gap
    let cell_size = 13;
    let cell_gap = 2;
    let cell_total = cell_size + cell_gap;
    let label_width = 30;
    let header_height = 20;
    let right_padding = 20;
    let bottom_padding = 10;

    let num_weeks = 53;
    let num_days = 7;

    let svg_width = label_width + (num_weeks as i32) * cell_total + right_padding;
    let svg_height = header_height + (num_days as i32) * cell_total + bottom_padding;

    // Day labels (Mon, Wed, Fri)
    let day_labels = [
        (1, "Mon"),
        (3, "Wed"),
        (5, "Fri"),
    ];

    // Build the HTML cells
    let mut cells_html = String::new();

    // Month labels
    let mut month_positions: Vec<(String, usize)> = Vec::new();
    let mut last_month = "";
    for (week_idx, day_group) in days.chunks(7).enumerate() {
        if let Some(first_day) = day_group.first() {
            let month = &first_day.date[5..7]; // MM from YYYY-MM-DD
            if month != last_month {
                last_month = month;
                month_positions.push((month_name(month), week_idx));
            }
        }
    }

    // Month header labels
    let mut month_labels_html = String::new();
    for (month, week_idx) in &month_positions {
        let x = label_width + (*week_idx as i32) * cell_total;
        month_labels_html.push_str(&format!(
            r#"<text x="{x}" y="12" class="month-label">{month}</text>"#
        ));
    }

    // Day labels
    let mut day_labels_html = String::new();
    for (day_idx, label) in &day_labels {
        let y = header_height + (day_idx * cell_total) + cell_size - 2;
        day_labels_html.push_str(&format!(
            r#"<text x="0" y="{y}" class="day-label">{label}</text>"#
        ));
    }

    // Contribution cells
    let mut week_idx = 0;
    let mut day_in_week = 0;

    // Seed for deterministic pseudo-random delays
    let mut delay_seed: u32 = 42;
    for day in days {
        let count = day.contribution_count;
        let level = get_level(count);
        let x = label_width + (week_idx as i32) * cell_total;
        let y = header_height + (day_in_week as i32) * cell_total;

        // Cascade delay: waves from left to right (staggered per week)
        let cascade_delay = week_idx as f64 * 0.04;

        // Pseudo-random twinkle delay (deterministic based on position)
        delay_seed = delay_seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let twinkle_delay = ((delay_seed >> 16) % 6000) as f64 / 1000.0; // 0..6s

        // Twinkle duration varies by level: more active contributions twinkle faster
        let twinkle_duration = match level {
            0 => 0.0, // level-0 doesn't twinkle
            1 => 4.0 + (delay_seed % 2000) as f64 / 1000.0, // 4-6s
            2 => 3.0 + (delay_seed % 1500) as f64 / 1000.0, // 3-4.5s
            3 => 2.5 + (delay_seed % 1000) as f64 / 1000.0, // 2.5-3.5s
            _ => 2.0 + (delay_seed % 800) as f64 / 1000.0,  // 2-2.8s
        };

        // Glow intensity multiplier by level
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
            format!(
                r#" style="animation-delay: {:.2}s""#,
                cascade_delay
            )
        };

        let class_extra = if level > 0 { format!(" {glow_class}") } else { String::new() };

        cells_html.push_str(&format!(
            r#"<rect x="{x}" y="{y}" width="{cell_size}" height="{cell_size}" class="day level-{level}{class_extra}"{style_extra} rx="2" ry="2"><title>{} contributions on {}</title></rect>"#,
            count, day.date,
            class_extra = class_extra,
            style_extra = style_attr,
        ));

        day_in_week += 1;
        if day_in_week >= 7 {
            day_in_week = 0;
            week_idx += 1;
        }
    }

    // Total contributions
    let total: u32 = days.iter().map(|d| d.contribution_count).sum();

    // Generate decorative background star dots
    let mut bg_stars_html = String::new();
    let mut star_seed: u32 = 1337;
    for i in 0..40 {
        star_seed = star_seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let sx = (star_seed % (svg_width as u32 - 40) + 40) as i32;
        star_seed = star_seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let sy = (star_seed % (svg_height as u32 - 20) + 10) as i32;
        let sr = 0.5 + (i % 3) as f64 * 0.3; // 0.5 to 1.1 radius
        let delay = (i as f64) * 0.3;
        let dur = 2.0 + (i % 5) as f64 * 0.8;
        bg_stars_html.push_str(&format!(
            r#"<circle cx="{}" cy="{}" r="{:.1}" class="bg-star" style="animation-delay:{:.1}s;animation-duration:{:.1}s"/>"#,
            sx, sy, sr, delay, dur
        ));
    }

    let total_text_y = svg_height - 3;
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{svg_width}" height="{svg_height}" viewBox="0 0 {svg_width} {svg_height}">
  <style>
    /* --- Text styles --- */
    .month-label {{
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
      font-size: 10px;
      fill: #57606a;
      opacity: 0;
      animation: fadeIn 0.6s ease forwards;
    }}
    .day-label {{
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
      font-size: 10px;
      fill: #57606a;
      opacity: 0;
      animation: fadeIn 0.6s ease forwards;
    }}

    /* --- Cell base styles --- */
    .day {{
      stroke: rgba(27, 31, 36, 0.06);
      stroke-width: 1;
      opacity: 0;
      animation: cellAppear 0.4s ease forwards;
    }}
    .level-0 {{ fill: #ebedf0; }}
    .level-1 {{ fill: #9be9a8; }}
    .level-2 {{ fill: #40c463; }}
    .level-3 {{ fill: #30a14e; }}
    .level-4 {{ fill: #216e39; }}

    /* --- Keyframes --- */

    /* Fade-in for text labels */
    @keyframes fadeIn {{
      from {{ opacity: 0; }}
      to   {{ opacity: 1; }}
    }}

    /* Cells appear with scale + opacity cascade */
    @keyframes cellAppear {{
      0%   {{ opacity: 0; transform: scale(0.3); }}
      60%  {{ opacity: 1; transform: scale(1.1); }}
      100% {{ opacity: 1; transform: scale(1); }}
    }}

    /* --- Twinkle animations by intensity --- */
    /* Soft: subtle glow pulse (level 1) */
    @keyframes twinkleSoft {{
      0%, 100% {{ filter: brightness(1) drop-shadow(0 0 0px transparent); }}
      50%      {{ filter: brightness(1.3) drop-shadow(0 0 3px rgba(155,233,168,0.5)); }}
    }}
    .twinkle-soft {{
      animation: cellAppear 0.4s ease forwards, twinkleSoft var(--twinkle-dur, 5s) ease-in-out infinite;
    }}

    /* Medium: noticeable glow (level 2) */
    @keyframes twinkleMid {{
      0%, 100% {{ filter: brightness(1) drop-shadow(0 0 0px transparent); }}
      40%      {{ filter: brightness(1.5) drop-shadow(0 0 5px rgba(64,196,99,0.6)); }}
      70%      {{ filter: brightness(1.2) drop-shadow(0 0 2px rgba(64,196,99,0.3)); }}
    }}
    .twinkle-mid {{
      animation: cellAppear 0.4s ease forwards, twinkleMid var(--twinkle-dur, 4s) ease-in-out infinite;
    }}

    /* Bright: strong pulse (level 3) */
    @keyframes twinkleBright {{
      0%, 100% {{ filter: brightness(1) drop-shadow(0 0 0px transparent); }}
      30%      {{ filter: brightness(1.8) drop-shadow(0 0 8px rgba(48,161,78,0.7)); }}
      60%      {{ filter: brightness(1.1) drop-shadow(0 0 2px rgba(48,161,78,0.2)); }}
    }}
    .twinkle-bright {{
      animation: cellAppear 0.4s ease forwards, twinkleBright var(--twinkle-dur, 3s) ease-in-out infinite;
    }}

    /* Intense: dramatic star sparkle (level 4) */
    @keyframes twinkleIntense {{
      0%   {{ filter: brightness(1)    drop-shadow(0 0 0px transparent); }}
      25%  {{ filter: brightness(2.2)  drop-shadow(0 0 12px rgba(33,110,57,0.9)); }}
      50%  {{ filter: brightness(1.2)  drop-shadow(0 0 3px rgba(33,110,57,0.3)); }}
      75%  {{ filter: brightness(1.8)  drop-shadow(0 0 8px rgba(33,110,57,0.6)); }}
      100% {{ filter: brightness(1)    drop-shadow(0 0 0px transparent); }}
    }}
    .twinkle-intense {{
      animation: cellAppear 0.4s ease forwards, twinkleIntense var(--twinkle-dur, 2.5s) ease-in-out infinite;
    }}

    /* --- Background star field (subtle decorative layer) --- */
    @keyframes bgTwinkle {{
      0%, 100% {{ opacity: 0.15; }}
      50%      {{ opacity: 0.4; }}
    }}
    .bg-star {{
      fill: #fff;
      opacity: 0.15;
      animation: bgTwinkle 3s ease-in-out infinite;
    }}
  </style>
  <defs>
    <!-- Subtle radial gradient for background ambiance -->
    <radialGradient id="bgGlow" cx="50%" cy="50%" r="60%">
      <stop offset="0%" style="stop-color:#1a2332;stop-opacity:0.15"/>
      <stop offset="100%" style="stop-color:#1a2332;stop-opacity:0"/>
    </radialGradient>
  </defs>
  <!-- Soft background glow -->
  <rect width="{svg_width}" height="{svg_height}" fill="url(#bgGlow)" opacity="0.3"/>
  <!-- Tiny decorative star dots scattered in background -->
  {bg_stars_html}
  {month_labels_html}
  {day_labels_html}
  {cells_html}
  <text x="{label_width}" y="{total_text_y}" class="day-label">{total} contributions in the last year</text>
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
