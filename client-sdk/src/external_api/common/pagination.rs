use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationRequest {
    pub direction: String,
    pub per_page: String,
    pub cursor: Cursor,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cursor {
    pub sorting_value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cursors {
    pub prev: Cursor,
    pub next: Cursor,
}

impl PaginationRequest {
    /// Create a new Pagination object with the given sorting value.
    pub fn from_sorting_value(sorting_value: &str) -> Self {
        PaginationRequest {
            direction: "next".to_string(),
            per_page: "100".to_string(), // TODO: make this configurable
            cursor: Cursor {
                sorting_value: sorting_value.to_string(),
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginationResponse {
    pub per_page: String,
    pub cursor: Cursors,
}
