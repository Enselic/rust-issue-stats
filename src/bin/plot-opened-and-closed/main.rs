use std::path::PathBuf;

type URI = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "schemas/github_schema.graphql",
    query_path = "issues_query.graphql",
    variables_derives = "Clone, Debug, Serialize",
    response_derives = "Clone, Debug, Serialize"
)]
pub struct OpenedAndClosedIssues;

#[derive(clap::Parser, Debug)]
pub struct Args {
    #[arg(long, default_value = "10")]
    page_size: u16,

    #[arg(long, default_value = "2")]
    pages: usize,

    #[arg(long, default_value = "persisted-data.json")]
    persisted_data_path: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log_init()?;

    let args = Args::parse();

    let github = GitHub::new();

    let variables = issues_query::Variables {
        page_size: args.page_size,
    };
    github
        .for_issues_with_timeline(
            variables,
            args.pages,
            |issue| {
                let (label_age, comment_age) = get_ages(issue).unwrap();
                let label_age_months = label_age.to_months();
                let comment_age_months = comment_age.to_months();

                let old_enough = label_age_months > args.label_months_considered_old
                    && comment_age_months > args.last_comment_months_considered_old;

                let labeled_triaged = issue
                    .labels
                    .nodes
                    .iter()
                    .any(|label| label.name.to_lowercase().contains("triaged"));

                if old_enough && !labeled_triaged {
                    println!(
                        "{} E-needs-mcve {} months old, last comment {} months ago",
                        issue.url, label_age_months, comment_age_months
                    );
                }
            },
            || {},
        )
        .await;

    Ok(())
}
