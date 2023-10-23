// TODO: Parameterize more
pub const ISSUES_QUERY: &str = r#" query ($page_size: Int!, $before: String) {
    repository(owner: "rust-lang", name: "rust") {
        issues(last: $page_size, before: $before) {
            nodes {
                number
                title
                createdAt
                timelineItems(itemTypes: [LABELED_EVENT, UNLABELED_EVENT], first: 200) {
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

pub const TIMELINE_QUERY: &str = r#" query ($number: Int!, $after: String!) {
    repository(owner: "rust-lang", name: "rust") {
        issue(number: $number) {
            number
            title
            createdAt
            timelineItems(itemTypes: [LABELED_EVENT, UNLABELED_EVENT, CLOSED_EVENT], first: 2, after: $after) {
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
