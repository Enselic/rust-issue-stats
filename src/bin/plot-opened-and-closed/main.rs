use chrono::Utc;
use std::{collections::HashMap, hash::Hash, path::PathBuf};

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

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum Counter {
    Opened,
    Closed,
}

#[derive(Debug, Clone, Default)]
struct CategoryData {
    counters: HashMap<Counter, u64>,
}

#[derive(Debug, Default)]
struct WeekData {
    category_data: HashMap<Category, CategoryData>,
}

/*

C-bug
C-cleanup
C-discussion
C-enhancement
C-feature-accepted
C-feature-request
C-future-compatibility
C-optimization
C-tracking-issue

*/
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum Category {
    /// C-bug
    Bug,
    /// C-cleanup, C-discussion, C-enhancement, etc, etc.
    NotBug,
}

#[derive(Debug)]
struct PlotData {
    origin_of_time: DateTime,
    week_data: Vec<WeekData>,
}

impl OpenedAndClosedIssuesRepositoryIssuesNodes {
    fn category(&self) -> Category {
        let labels = self.labels.as_ref().unwrap();
        if labels
            .nodes
            .as_ref()
            .unwrap()
            .iter()
            .any(|label| label.as_ref().unwrap().name == "C-bug")
        {
            Category::Bug
        } else {
            Category::NotBug
        }
    }

    // TODO: Handle re-opened issues.
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

    fn ensure_len(&mut self, len: usize) {
        if self.week_data.len() <= len {
            self.week_data.resize_with(len + 1, WeekData::default);
        }
    }

    fn increment(&mut self, week: usize, category: Category, counter: Counter) {
        self.ensure_len(week);
        let week_data = self.week_data.get_mut(week).unwrap();
        week_data
            .category_data
            .entry(category)
            .or_default()
            .counters
            .entry(counter)
            .and_modify(|c| *c += 1)
            .or_insert(1);
    }

    fn analyze_issues(&mut self, issues: &[Option<OpenedAndClosedIssuesRepositoryIssuesNodes>]) {
        for issue in issues {
            let issue = issue.as_ref().unwrap();
            let opened_week = ((issue.created_at - self.origin_of_time).num_days() / 7) as usize;
            let closed_week = issue
                .closed_at()
                .map(|date| ((date - self.origin_of_time).num_days() / 7) as usize);

            let category = issue.category();
            self.increment(opened_week, category, Counter::Opened);

            if let Some(closed_week) = closed_week {
                self.increment(closed_week, category, Counter::Closed);
            }
        }
    }
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
        data.analyze_issues(issues.nodes.as_ref().unwrap());

        if issues.page_info.has_next_page {
            variables.after = issues.page_info.end_cursor.clone();
        } else {
            break;
        }
    }

    let mut total_open: HashMap<Category, i64> = HashMap::new();
    let mut total = 0;
    for (idx, week) in data.week_data.iter().enumerate() {
        for category in &[Category::Bug, Category::NotBug] {
            total_open
                .entry(category.clone())
                .and_modify(|c| *c += week.category_data.get(category).unwrap().counters[&Counter::Opened] as i64 - week.category_data.get(category).unwrap().counters[&Counter::Closed] as i64)
                .or_insert(week.category_data.get(category).unwrap().counters[&Counter::Opened] as i64 - week.category_data.get(category).unwrap().counters[&Counter::Closed] as i64);
        }
        let bugs_opened =
            week.category_data.get(&Category::Bug).unwrap().counters[&Counter::Opened];
        let bugs_closed =
            week.category_data.get(&Category::Bug).unwrap().counters[&Counter::Closed];

        let not_bugs_opened =
            week.category_data.get(&Category::NotBug).unwrap().counters[&Counter::Opened];
        let not_bugs_closed =
            week.category_data.get(&Category::NotBug).unwrap().counters[&Counter::Closed];

        let total_opened = bugs_opened + not_bugs_opened;
        let total_bugs = bugs_opened - bugs_closed;
        let 
        let total_closed = bugs_closed + not_bugs_closed;

        total_open += total_opened as i64 - total_closed as i64;
        total += total_opened as i64;
        println!(
            "{}\t{}\t{}\t{}\t{}",
            idx, bugs_opened, not_bugs_opened, bugs_closed, not_bugs_closed, total
        );
    }

    Ok(())
}
