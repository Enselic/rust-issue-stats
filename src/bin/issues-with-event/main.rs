use clap::Parser;

use rust_issue_stats::*;

#[derive(clap::Parser, Debug)]
pub struct Args {
    #[arg(long, default_value = "10")]
    page_size: u16,

    #[arg(long, default_value = "2")]
    pages: usize,

    #[arg(long, default_value = "REOPENED_EVENT")]
    event: String,

    #[arg(long, default_value = "36")]
    last_comment_months_considered_old: i64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    log_init()?;

    let github = GitHub::new();

    let variables = serde_json::json!({
        "page_size": args.page_size,
        "states": ["OPEN"],
        "timelineItemTypes": [&args.event],
    });
    github
        .for_issues_with_timeline(
            variables,
            args.pages,
            |issue| {
                if !issue.timeline_items.is_empty() {
                    println!("{} {}", issue.url, issue.title);
                    println!("    {:?}", issue.timeline_items)
                }
            },
            || {},
        )
        .await;

    Ok(())
}
