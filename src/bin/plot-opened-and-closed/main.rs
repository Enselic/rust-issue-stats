use std::path::PathBuf;

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

    #[arg(long, default_value = "persisted-data-dir")]
    persisted_data_dir: PathBuf,
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

    std::fs::create_dir_all(args.persisted_data_dir.clone())?;

    let mut page = 0;
    loop {
        page += 1;
        if page > args.pages {
            break;
        }

        let response: graphql_client::Response<ResponseData> = github
            .octocrab
            .graphql(
                &<OpenedAndClosedIssues as graphql_client::GraphQLQuery>::build_query(
                    variables.clone(),
                ),
            )
            .await?;
        eprintln!("errors: {:#?}", response.errors);

        let mut persited_data_path = args.persisted_data_dir.clone();
        persited_data_path.push(format!("page-{}_page-size-{}", page, args.page_size));
        println!(
            "Writing response to disk. path: {}",
            persited_data_path.display()
        );
        serde_json::to_writer(std::fs::File::create(persited_data_path)?, &response)?;

        let issues = &response
            .data
            .as_ref()
            .unwrap()
            .repository
            .as_ref()
            .unwrap()
            .issues;

        println!("{issues:?}");

        if issues.page_info.has_next_page {
            variables.after = issues.page_info.end_cursor.clone();
        }
    }

    Ok(())
}
