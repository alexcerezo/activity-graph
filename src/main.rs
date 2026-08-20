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
    for day in days {
        let count = day.contribution_count;
        let level = get_level(count);
        let x = label_width + (week_idx as i32) * cell_total;
        let y = header_height + (day_in_week as i32) * cell_total;

        cells_html.push_str(&format!(
            r#"<rect x="{x}" y="{y}" width="{cell_size}" height="{cell_size}" class="day level-{level}" rx="2" ry="2"><title>{} contributions on {}</title></rect>"#,
            count, day.date
        ));

        day_in_week += 1;
        if day_in_week >= 7 {
            day_in_week = 0;
            week_idx += 1;
        }
    }

    // Total contributions
    let total: u32 = days.iter().map(|d| d.contribution_count).sum();

    let total_text_y = svg_height - 3;
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="{svg_width}" height="{svg_height}" viewBox="0 0 {svg_width} {svg_height}">
  <style>
    .month-label {{
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
      font-size: 10px;
      fill: #57606a;
    }}
    .day-label {{
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
      font-size: 10px;
      fill: #57606a;
    }}
    .day {{
      stroke: rgba(27, 31, 36, 0.06);
      stroke-width: 1;
    }}
    .level-0 {{ fill: #ebedf0; }}
    .level-1 {{ fill: #9be9a8; }}
    .level-2 {{ fill: #40c463; }}
    .level-3 {{ fill: #30a14e; }}
    .level-4 {{ fill: #216e39; }}
  </style>
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
