use chrono::{DateTime, FixedOffset};

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{Display, Formatter};

pub const E_NEEDS_MCVE: &str = "E-needs-mcve";

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResponse {
    pub data: Option<serde_json::Value>,
    pub errors: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issues {
    pub nodes: Vec<PagedIssueWithTimelineItems>,
    pub page_info: PreviousPageInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Labels {
    pub nodes: Vec<Label>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineItems {
    pub nodes: Vec<TimelineItem>,
    pub page_info: NextPageInfo,
}

/// TODO: Add more events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "__typename")]
pub enum TimelineItem {
    LabeledEvent {
        #[serde(rename = "createdAt", deserialize_with = "from_rfc3339_str")]
        created_at: DateTime<FixedOffset>,
        label: Label,
    },
    UnlabeledEvent {
        #[serde(rename = "createdAt", deserialize_with = "from_rfc3339_str")]
        created_at: DateTime<FixedOffset>,
        label: Label,
    },
    ClosedEvent {
        #[serde(rename = "createdAt", deserialize_with = "from_rfc3339_str")]
        created_at: DateTime<FixedOffset>,
    },
    ReopenedEvent {
        #[serde(rename = "createdAt", deserialize_with = "from_rfc3339_str")]
        created_at: DateTime<FixedOffset>,
        actor: Actor,
    },
    IssueComment {
        #[serde(rename = "createdAt", deserialize_with = "from_rfc3339_str")]
        created_at: DateTime<FixedOffset>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Actor {
    pub login: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextPageInfo {
    pub end_cursor: Option<String>,
    pub has_next_page: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviousPageInfo {
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedIssueWithTimelineItems {
    pub url: String,
    pub number: u32,
    pub title: String,
    pub labels: Labels,
    #[serde(rename = "createdAt", deserialize_with = "from_rfc3339_str")]
    pub created_at: DateTime<FixedOffset>,
    pub timeline_items: TimelineItems,
}

pub struct IssueWithTimelineItems {
    pub url: String,
    pub number: u32,
    pub title: String,
    pub labels: Labels,
    pub created_at: DateTime<FixedOffset>,
    pub timeline_items: Vec<TimelineItem>,
}

fn from_rfc3339_str<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(s).map_err(D::Error::custom)
}

impl QueryResponse {
    pub fn get<'de, T: Deserialize<'de>>(
        &'de self,
        path: &[&str],
    ) -> Result<T, <serde_json::Value as Deserializer<'de>>::Error> {
        if let Some(errors) = &self.errors {
            return Err(<serde_json::Value as Deserializer<'de>>::Error::custom(
                format!("Got errors: {:#?}", errors),
            ));
        }

        let mut value = self.data.as_ref().unwrap();
        for segment in path {
            let object = value.as_object().unwrap();
            value = object.get(*segment).unwrap();
        }

        T::deserialize(value)
    }
}

impl Display for Label {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

impl Display for TimelineItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TimelineItem::LabeledEvent { label, created_at } => {
                write!(f, "+{} {}", label, created_at.format("%Y-%m-%d"))
            }
            TimelineItem::UnlabeledEvent { label, created_at } => {
                write!(f, "-{} {}", label, created_at.format("%Y-%m-%d"))
            }
            TimelineItem::ClosedEvent { created_at } => {
                write!(f, "<CLOSED> {}", created_at.format("%Y-%m-%d"))
            }
            TimelineItem::ReopenedEvent { created_at, actor } => {
                write!(
                    f,
                    "<REOPENED by {}> {}",
                    actor.login,
                    created_at.format("%Y-%m-%d")
                )
            }
            TimelineItem::IssueComment { created_at } => {
                write!(f, "<COMMENT> {}", created_at.format("%Y-%m-%d"))
            }
        }
    }
}

impl Display for IssueWithTimelineItems {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\"#{} {}, created {}, timeline items: {}\"",
            self.number,
            self.title,
            self.created_at.format("%Y-%m-%d"),
            self.timeline_items
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}
