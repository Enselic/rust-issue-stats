use tracing::subscriber::SetGlobalDefaultError;

use tracing_subscriber::FmtSubscriber;

/// GitHub GraphQL API wrapper.
pub struct GitHub {
    pub octocrab: octocrab::Octocrab,
}

impl Default for GitHub {
    fn default() -> Self {
        Self::new()
    }
}

fn github_api_token() -> String {
    let output = std::process::Command::new("git")
        .arg("config")
        .arg("--get")
        .arg("github.oauth-token")
        .output()
        .expect("Run: git config github.oauth-token <your-token>");

    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

impl GitHub {
    pub fn new() -> Self {
        Self {
            octocrab: octocrab::Octocrab::builder()
                .personal_token(github_api_token())
                .build()
                .unwrap(),
        }
    }
}

// impl GitHub {
//     async fn query(
//         &self,
//         query: &(impl serde::Serialize + ?Sized),
//         variables: serde_json::Value,
//     ) -> octocrab::Result<QueryResponse> {
//         let json = serde_json::json!({
//             "query": query,
//             "variables": variables,
//         });
//
//         warn!("making a GitHub API request (affecting rate limiting)");
//         trace!("Query: {}", &json);
//         self.octocrab.graphql(&json).await
//     }
//
//     pub async fn for_issues_with_timeline(
//         &self,
//         mut variables: serde_json::Value,
//         pages: usize,
//         mut issue_handler: impl FnMut(&IssueWithTimelineItems),
//         mut after_page_handler: impl FnMut(),
//     ) {
//         let mut pages_left = pages;
//         loop {
//             let mut issues: Issues = self
//                 .query(queries::ISSUES_WITH_TIMELINE_QUERY, variables.clone())
//                 .await
//                 .unwrap()
//                 .get(&["repository", "issues"])
//                 .unwrap();
//
//             pages_left -= 1;
//
//             for paged_issue in &mut issues.nodes {
//                 let issue = paged_issue.collect_pages(self).await.unwrap();
//
//                 issue_handler(&issue);
//             }
//
//             after_page_handler();
//
//             if pages_left == 0 {
//                 break;
//             }
//
//             if !issues.page_info.has_previous_page {
//                 debug!("No more pages left. Maybe unexpected. Raw data: {issues:#?}");
//                 break;
//             }
//
//             variables.as_object_mut().unwrap().insert(
//                 "before".to_owned(),
//                 serde_json::json!(issues
//                     .page_info
//                     .start_cursor
//                     .expect("has_previous_page is true")),
//             );
//         }
//     }
// }
//
// impl PagedIssueWithTimelineItems {
//     pub async fn collect_pages(
//         &mut self,
//         github: &GitHub,
//     ) -> octocrab::Result<IssueWithTimelineItems> {
//         let mut page_info = self.timeline_items.page_info.clone();
//
//         loop {
//             if !page_info.has_next_page {
//                 break;
//             }
//
//             let issue_data: PagedIssueWithTimelineItems = github
//                 .query(
//                     queries::TIMELINE_QUERY,
//                     serde_json::json!({
//                         "number": self.number,
//                         "after": page_info.end_cursor,
//                     }),
//                 )
//                 .await?
//                 .get(&["repository", "issue"])
//                 .unwrap();
//
//             assert_eq!(issue_data.number, self.number);
//             assert_eq!(issue_data.title, self.title);
//
//             self.timeline_items
//                 .nodes
//                 .extend(issue_data.timeline_items.nodes);
//
//             page_info = issue_data.timeline_items.page_info.clone();
//         }
//
//         Ok(IssueWithTimelineItems {
//             url: self.url.clone(),
//             number: self.number,
//             title: self.title.clone(),
//             labels: self.labels.clone(),
//             created_at: self.created_at,
//             timeline_items: self.timeline_items.nodes.clone(),
//         })
//     }
// }

pub fn log_init() -> Result<(), SetGlobalDefaultError> {
    // Enable like this: `RUST_LOG=rust_issue_stats=warn cargo run`
    tracing::subscriber::set_global_default(
        FmtSubscriber::builder()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_writer(std::io::stderr)
            .finish(),
    )
}
