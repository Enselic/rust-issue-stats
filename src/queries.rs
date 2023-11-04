pub const ISSUES_WITH_TIMELINE_QUERY: &str = r#" query ($page_size: Int!, $before: String, $states: [IssueState!], $filterBy: IssueFilters, $timeline_page_size: Int = 200, $timelineItemTypes: [IssueTimelineItemsItemType!]!) {
    repository(owner: "rust-lang", name: "rust") {
        issues(last: $page_size, before: $before, states: $states, filterBy: $filterBy, orderBy: { field: CREATED_AT, direction: ASC }) {
            nodes {
                url
                number
                title
                createdAt
                labels(first: 100) {
                    nodes {
                        name
                    }
                }
                timelineItems(first: $timeline_page_size, itemTypes: $timelineItemTypes) {
                    nodes {
                        ... on LabeledEvent {
                            __typename
                            label {
                                name
                            }
                            createdAt
                        }
                        ... on UnlabeledEvent {
                            __typename
                            label {
                                name
                            }
                            createdAt
                        }
                        ... on ClosedEvent {
                            __typename
                            createdAt
                        }
                        ... on ReopenedEvent {
                            __typename
                            actor {
                                login
                            }
                            createdAt
                        }
                        ... on IssueComment {
                            __typename
                            createdAt
                        }
                    }
                    pageInfo {
                        endCursor
                        hasNextPage
                        hasPreviousPage
                        startCursor
                    }
                }
            }
            pageInfo {
                endCursor
                hasNextPage
                hasPreviousPage
                startCursor
            }
        }
    }
} "#;

pub const TIMELINE_QUERY: &str = r#" query ($number: Int!, $after: String!, $timeline_page_size: Int = 200, $timelineItemTypes: [IssueTimelineItemsItemType!]!) {
    repository(owner: "rust-lang", name: "rust") {
        issue(number: $number) {
            number
            title
            createdAt
            labels(first: 100) {
                nodes {
                    name
                }
            }
            timelineItems(itemTypes: $timelineItemTypes, first: $timeline_page_size, after: $after) {
                nodes {
                    ... on LabeledEvent {
                        __typename
                        createdAt
                        label {
                            name
                        }
                    }
                    ... on UnlabeledEvent {
                        __typename
                        createdAt
                        label {
                            name
                        }
                    }
                    ... on ClosedEvent {
                        __typename
                        createdAt
                    }
                }
                pageInfo {
                    endCursor
                    hasNextPage
                    hasPreviousPage
                    startCursor
                }
            }
        }
    }
} "#;
