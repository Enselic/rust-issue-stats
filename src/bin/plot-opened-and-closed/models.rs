use std::collections::HashMap;

pub type URI = String;
pub type DateTime = chrono::DateTime<chrono::Utc>;

#[derive(graphql_client::GraphQLQuery)]
#[graphql(
    schema_path = "schemas/github_schema.graphql",
    query_path = "src/bin/plot-opened-and-closed/OpenedAndClosedIssues.graphql",
    variables_derives = "Clone, Debug",
    response_derives = "Clone, Debug, Serialize, Eq, PartialEq"
)]
pub struct OpenedAndClosedIssues;


#[derive(Debug, Hash, PartialEq, Eq)]
pub enum Counter {
    Opened,
    Closed,
}

#[derive(Debug)]
pub struct Counters(HashMap<Counter, i64>);

impl Default for Counters {
    fn default() -> Self {
        Self(HashMap::from([(Counter::Opened, 0), (Counter::Closed, 0)]))
    }
}

/// Represents a period of stats. Either one week or one month depending on user
/// preference.
#[derive(Debug)]
pub struct PeriodData(HashMap<IssueCategory, Counters>);

impl Default for PeriodData {
    fn default() -> Self {
        Self(HashMap::from([
            (IssueCategory::Bug, Counters::default()),
            (IssueCategory::Improvement, Counters::default()),
            (IssueCategory::Uncategorized, Counters::default()),
        ]))
    }
}

impl PeriodData {
    fn opened(&self, category: IssueCategory) -> i64 {
        *self
            .0
            .get(&category)
            .unwrap()
            .0
            .get(&Counter::Opened)
            .unwrap()
    }

    fn closed(&self, category: IssueCategory) -> i64 {
        *self
            .0
            .get(&category)
            .unwrap()
            .0
            .get(&Counter::Closed)
            .unwrap()
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone)]
pub enum IssueCategory {
    /// C-bug
    Bug,
    /// C-enhancement, C-feature-request, C-optimization, C-cleanup,
    /// C-feature-accepted, C-tracking-issue, C-future-compatibility.
    Improvement,
    /// C-discussion and issues without a C-* label.
    Uncategorized,
}
