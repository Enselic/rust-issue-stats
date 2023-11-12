use chrono::Utc;
use std::io::Write;
use std::{collections::HashMap, hash::Hash, path::PathBuf};

use tracing::*;

mod models;
use models::*;

use opened_and_closed_issues::*;

#[derive(clap::Parser, Debug)]
pub struct Args {
    #[arg(long, default_value = "10")]
    page_size: i64,

    #[arg(long, default_value = "2")]
    pages: usize,

    #[arg(long, default_value = "target/rust-issues-stats/persisted-data-dir")]
    persisted_data_dir: PathBuf,

    #[arg(long, default_value = "week.tsv")]
    week_stats_file: PathBuf,

    #[arg(long, default_value = "accumulated.tsv")]
    accumulated_stats_file: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rust_issue_stats::log_init().unwrap();

    let args = <Args as clap::Parser>::parse();

    let github = rust_issue_stats::GitHub::new();

    let mut variables = Variables {
        repository_owner: "rust-lang".to_owned(),
        repository_name: "rust".to_owned(),
        page_size: args.page_size,
        after: None,
    };

    let mut plot_data = PlotData::new();

    let mut page = 0;
    loop {
        page += 1;
        if page > args.pages {
            break;
        }

        let mut persited_data_path = args.persisted_data_dir.clone();
        persited_data_path.push(format!("page-size-{}", args.page_size));
        persited_data_path.push(format!("page-v4-{}.json", page));
        std::fs::create_dir_all(persited_data_path.parent().unwrap()).unwrap();

        let response: graphql_client::Response<ResponseData> = if persited_data_path.exists() {
            debug!(
                "Reading response from disk. path: {}",
                persited_data_path.display()
            );
            let file = std::fs::File::open(persited_data_path.clone()).unwrap();
            serde_json::from_reader(&file).unwrap()
        } else {
            info!("Making GitHub GraphQL API query (affects rate limit)");
            let response: graphql_client::Response<ResponseData> = github
                .octocrab
                .graphql(
                    &<OpenedAndClosedIssues as graphql_client::GraphQLQuery>::build_query(
                        variables.clone(),
                    ),
                )
                .await
                .unwrap();

            if let Some(errors) = response.errors {
                eprintln!("errors: {:#?}", errors);
                break;
            }

            println!(
                "Writing response to disk. path: {}",
                persited_data_path.display()
            );

            let mut tmp = persited_data_path.clone();
            tmp.set_extension("json.tmp");
            let file = std::fs::File::create(&tmp).unwrap();
            serde_json::to_writer(&file, &response).unwrap();
            file.sync_all().unwrap();
            std::fs::rename(tmp, &persited_data_path).unwrap();

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
        plot_data.analyze_issues(issues.nodes.as_ref().unwrap());

        if issues.page_info.has_next_page {
            variables.after = issues.page_info.end_cursor.clone();
        } else {
            break;
        }
    }

    let mut week_stats_file = std::fs::File::create(args.week_stats_file).unwrap();
    let mut accumulated_stats_file = std::fs::File::create(args.accumulated_stats_file).unwrap();

    writeln!(
        week_stats_file,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}",
        "Week",
        "opened Bugs",
        "opened Enhancements",
        "opened Others",
        "closed Bugs",
        "closed Enhancements",
        "closed Others",
    )
    .unwrap();
    writeln!(
        accumulated_stats_file,
        "{}\t{}\t{}\t{}\t{}\t{}",
        "Week", "Open bugs", "Open enhancements", "Open others", "Open total", "All"
    )
    .unwrap();

    let mut sorted_periods = plot_data.periods.keys().collect::<Vec<_>>();
    sorted_periods.sort();

    let mut total: HashMap<IssueCategory, i64> = HashMap::new();

    for period in sorted_periods {
        // Per week
        writeln!(
            week_stats_file,
            "{}\t{}\t{}",
            period,
            plot_data.get(period, IssueCategory::Bug, Counter::Opened),
            plot_data.get(period, IssueCategory::Bug, Counter::Closed),
        )
        .unwrap();

        // Accumulated
        for category in [
            IssueCategory::Bug,
            IssueCategory::Improvement,
            IssueCategory::Uncategorized,
        ] {
            let delta = plot_data.get(*period, category, Counter::Opened)
                - plot_data.get(*period, category, Counter::Closed);
            total
                .entry(category)
                .and_modify(|c| *c += delta)
                .or_insert(delta);
        }
        let sum = total.get(&IssueCategory::Bug).unwrap()
            + total.get(&IssueCategory::Improvement).unwrap()
            + total.get(&IssueCategory::Uncategorized).unwrap();
        writeln!(
            accumulated_stats_file,
            "{}\t{}\t{}\t{}\t{}",
            period,
            total.get(&IssueCategory::Bug).unwrap(),
            total.get(&IssueCategory::Improvement).unwrap(),
            total.get(&IssueCategory::Uncategorized).unwrap(),
            sum,
        )
        .unwrap();
    }

    Ok(())
}

impl PlotData {
    fn new() -> Self {
        Self {
            periods: HashMap::new(),
        }
    }

    fn increment(&mut self, period: Period, category: IssueCategory, counter: Counter) {
        let period_data = self
            .periods
            .entry(period)
            .or_default()
            .increment(category, counter);
    }

    fn analyze_issues(&mut self, issues: &[Option<OpenedAndClosedIssuesRepositoryIssuesNodes>]) {
        for issue in issues {
            let issue = issue.as_ref().unwrap();
            let category = issue.category();

            self.increment(issue.created_at.into(), category, Counter::Opened);

            if let Some(closed_week) = issue.closed_at().map(|date| date.into()) {
                self.increment(closed_week, category, Counter::Closed);
            }
        }
    }

    fn get(&self, period: Period, category: IssueCategory, counter: Counter) -> i64 {
        self.periods.get(&period).unwrap().get(category, counter)
    }
}

impl IssueCategory {
    fn from_c_labels(s: &[&String]) -> Self {
        if s.len() == 0 {
            return Self::Uncategorized;
        }

        if s.iter().any(|l| l == &"C-bug") {
            return Self::Bug;
        }

        if s.iter().any(|l| l == &"C-enhancement") {
            return Self::Improvement;
        }

        if s.iter().any(|l| l == &"C-feature-request") {
            return Self::Improvement;
        }

        if s.iter().any(|l| l == &"C-optimization") {
            return Self::Improvement;
        }

        if s.iter().any(|l| l == &"C-cleanup") {
            return Self::Improvement;
        }

        if s.iter().any(|l| l == &"C-feature-accepted") {
            return Self::Improvement;
        }

        if s.iter().any(|l| l == &"C-tracking-issue") {
            return Self::Improvement;
        }

        if s.iter().any(|l| l == &"C-future-compatibility") {
            return Self::Improvement;
        }

        if s.iter().any(|l| l == &"C-discussion") {
            return Self::Uncategorized;
        }

        unreachable!("Unknown category labels: {:?}", s);
    }
}

#[derive(Debug, Hash, Eq, PartialEq, PartialOrd, Ord, Copy)]
pub struct Period {
    year: i32,
    month: u32,
}

#[derive(Debug)]
pub struct PlotData {
    /// Maps a period such as "2023-07" (year and month) to period data.
    periods: HashMap<Period, PeriodData>,
}

impl OpenedAndClosedIssuesRepositoryIssuesNodes {
    fn category(&self) -> IssueCategory {
        let labels = self.labels.as_ref().unwrap();
        let category_labels: Vec<_> = labels
            .nodes
            .as_ref()
            .unwrap()
            .iter()
            .flatten()
            .filter(|label| label.name.starts_with("C-"))
            .map(|label| &label.name)
            .collect();
        IssueCategory::from_c_labels(&category_labels)
    }

    fn closed_at(&self) -> Option<DateTime> {
        if let Some(closed_at) = self.closed_at {
            return Some(closed_at);
        } else if self.state != IssueState::OPEN {
            eprintln!("strange state {:?} for issue: {}", self.state, self.url);
            return Some(self.created_at);
        }
        return None;
    }
}
