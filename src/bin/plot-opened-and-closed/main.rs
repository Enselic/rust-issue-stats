use chrono::Utc;
use std::{collections::HashMap, path::PathBuf};

use tracing::*;

type URI = String;
type DateTime = chrono::DateTime<chrono::Utc>;

#[derive(graphql_client::GraphQLQuery)]
#[graphql(
    schema_path = "schemas/github_schema.graphql",
    query_path = "src/bin/plot-opened-and-closed/OpenedAndClosedIssues.graphql",
    variables_derives = "Clone, Debug",
    response_derives = "Clone, Debug, Serialize"
)]
pub struct OpenedAndClosedIssues;

use opened_and_closed_issues::*;

#[derive(clap::Parser, Debug)]
pub struct Args {
    #[arg(long, default_value = "10")]
    page_size: i64,

    #[arg(long, default_value = "2")]
    pages: usize,

    #[arg(long, default_value = "target/rust-issues-stats/persisted-data-dir")]
    persisted_data_dir: PathBuf,
}

#[derive(Debug, Default)]
struct WeekData {
    opened: u64,
    closed: u64,
    total_open: u64,
}

#[derive(Debug)]
struct PlotData {
    origin_of_time: DateTime,
    week_data: Vec<WeekData>,
}

impl OpenedAndClosedIssuesRepositoryIssuesNodes {
    fn closed_at(&self) -> Option<DateTime> {
        for item in self.timeline_items.nodes.as_ref().unwrap() {
            if let Some(
                OpenedAndClosedIssuesRepositoryIssuesNodesTimelineItemsNodes::ClosedEvent(event),
            ) = &item
            {
                return Some(event.created_at);
            }
        }
        return None;
    }
}

impl PlotData {
    fn new() -> Self {
        Self {
            origin_of_time: chrono::DateTime::parse_from_rfc3339("2010-06-21T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            week_data: vec![],
        }
    }

    fn analyze_issues(&mut self, issues: &[Option<OpenedAndClosedIssuesRepositoryIssuesNodes>]) {
        for issue in issues {
            let issue = issue.as_ref().unwrap();
            let opened_week = ((issue.created_at - self.origin_of_time).num_days() / 7) as usize;
            let closed_week = issue
                .closed_at()
                .map(|date| ((date - self.origin_of_time).num_days() / 7) as usize);

            if self.week_data.len() <= opened_week {
                self.week_data
                    .resize_with(opened_week + 1, || WeekData::default());
            }
            self.week_data.get_mut(opened_week).unwrap().opened += 1;

            if let Some(closed_week) = closed_week {
                if self.week_data.len() <= closed_week {
                    self.week_data
                        .resize_with(closed_week + 1, || WeekData::default());
                }
                self.week_data.get_mut(closed_week).unwrap().closed += 1;
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rust_issue_stats::log_init()?;

    let args = <Args as clap::Parser>::parse();

    let github = rust_issue_stats::GitHub::new();

    let mut variables = Variables {
        repository_owner: "rust-lang".to_owned(),
        repository_name: "rust".to_owned(),
        page_size: args.page_size,
        after: None,
    };

    let mut data = PlotData::new();

    let mut page = 0;
    loop {
        page += 1;
        if page > args.pages {
            break;
        }

        let mut persited_data_path = args.persisted_data_dir.clone();
        persited_data_path.push(format!("page-size-{}", args.page_size));
        persited_data_path.push(format!("page-{}.json", page));

        let response: graphql_client::Response<ResponseData> = if persited_data_path.exists() {
            debug!(
                "Reading response from disk. path: {}",
                persited_data_path.display()
            );
            serde_json::from_reader(std::fs::File::open(persited_data_path.clone())?)?
        } else {
            info!("Making GitHub GraphQL API query (affects rate limit)");
            let response: graphql_client::Response<ResponseData> = github
                .octocrab
                .graphql(
                    &<OpenedAndClosedIssues as graphql_client::GraphQLQuery>::build_query(
                        variables.clone(),
                    ),
                )
                .await?;

            if let Some(errors) = response.errors {
                eprintln!("errors: {:#?}", errors);
                break;
            }

            println!(
                "Writing response to disk. path: {}",
                persited_data_path.display()
            );
            std::fs::create_dir_all(persited_data_path.parent().unwrap())?;
            serde_json::to_writer(std::fs::File::create(persited_data_path)?, &response)?;

            response
        };

        let issues = &response
            .data
            .as_ref()
            .unwrap()
            .repository
            .as_ref()
            .unwrap()
            .issues;

        //println!("{issues:?}");
        data.analyze_issues(issues.nodes.as_ref().unwrap());

        if issues.page_info.has_next_page {
            variables.after = issues.page_info.end_cursor.clone();
        } else {
            break;
        }
    }

    let mut total_open = 0;
    let mut total = 0;
    for (idx, week) in data.week_data.iter().enumerate() {
        total_open += week.opened as i64 - week.closed as i64;
        total += week.opened as i64;
        println!(
            "{}\t{}\t{}\t{}\t{}",
            idx, week.opened, week.closed, total_open, total
        );
    }

    Ok(())
}
